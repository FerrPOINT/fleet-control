use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

// Preserve every original16 predicate and its OID; widen only the state predicate.
async fn anchor_state(manager: &SchemaManager<'_>, up: bool) -> Result<(), DbErr> {
    let row = manager
        .get_connection()
        .query_one(sea_orm::Statement::from_string(
            sea_orm::DbBackend::Postgres,
            "SELECT pg_get_functiondef('fleet_guard_container_recovery()'::regprocedure) AS body",
        ))
        .await?
        .ok_or_else(|| DbErr::Custom("Missing recovery guard".into()))?;
    let body: String = row.try_get("", "body")?;
    let old = "c.state NOT IN ('running','stopping')";
    let new = "(c.state NOT IN ('running','stopping') AND NOT (c.state='exited' AND fleet_activation_anchor(c.generation)))";
    let (from, to) = if up { (old, new) } else { (new, old) };
    if body.matches(from).count() != 1 {
        return Err(DbErr::Custom("Recovery state guard changed".into()));
    }
    manager
        .get_connection()
        .execute_unprepared(&body.replacen(from, to, 1))
        .await?;
    Ok(())
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(r#"
CREATE TABLE runtime_container_activation_authorities (
 activation_id uuid NOT NULL REFERENCES runtime_container_activations(id),
 recovery_id uuid NOT NULL REFERENCES runtime_container_recoveries(id),
 controller_id uuid NOT NULL,
 plan_sha256 text NOT NULL CHECK(plan_sha256 ~ '^[a-f0-9]{64}$'),
 claim jsonb NOT NULL,
 PRIMARY KEY(activation_id,recovery_id)
);
CREATE FUNCTION fleet_activation_anchor(g uuid) RETURNS boolean LANGUAGE sql AS $$
 SELECT EXISTS(SELECT 1 FROM runtime_container_activations a
 JOIN runtime_container_launches l ON l.generation=g AND l.agent_id=a.agent_id
 JOIN agents agent ON agent.id=a.agent_id
 JOIN agent_config_heads h ON h.agent_id=a.agent_id
 JOIN agent_config_revisions r ON r.agent_id=a.agent_id AND r.revision=a.revision
 WHERE a.record#>>'{claim,previous,prepared,container,registration,generation}'=g::text
 AND l.prepared=a.record#>'{claim,previous,prepared}'
 AND l.snapshot=a.record#>'{claim,previous,snapshot}'
 AND l.origin=a.record#>>'{claim,previous,origin}'
 AND l.controller_id::text=a.record#>>'{claim,controller_id}'
 AND l.stop_id::text=a.record#>>'{claim,previous,stop_id}'
 AND agent.kind='hermes' AND agent.archived_at IS NULL
 AND agent.api_port::text=l.prepared->>'api_port'
 AND agent.runtime_path=l.prepared#>>'{paths,runtime}'
 AND agent.config_path=l.prepared#>>'{paths,config}'
 AND agent.workspace_path=l.prepared#>>'{paths,workspace}'
 AND agent.logs_path=l.prepared#>>'{paths,logs}'
 AND r.claimed_at IS NOT NULL AND r.validation_errors='[]'::jsonb
 AND (l.state IN ('running','stopping') OR (l.state='exited'
      AND a.record#>>'{previous_stop,observation}'='namespace_exited'
      AND a.record#>>'{previous_stop,generation}'=g::text
      AND a.record#>>'{previous_stop,operation_id}'=l.stop_id::text
      AND a.record#>>'{previous_stop,container_id}'=l.prepared#>>'{container,registration,container_id}'))
 AND ((a.record->>'phase' NOT IN ('committed','rolled_back') AND h.desired_revision=a.revision AND h.draining AND r.state='activating'
 AND h.effective_revision::text IS NOT DISTINCT FROM a.record#>>'{claim,previous_revision}'
 AND NOT EXISTS(SELECT 1 FROM session_agent_runs WHERE agent_id=a.agent_id AND state IN ('pending','running','waiting','stopping'))
 AND NOT EXISTS(SELECT 1 FROM hermes_dispatch_journal WHERE agent_id=a.agent_id AND state IN ('prepared','submitted'))
 AND NOT EXISTS(SELECT 1 FROM message_dispatch_outbox WHERE agent_id=a.agent_id AND state IN ('dispatching','uncertain')))
 OR (a.record->>'phase' IN ('committed','rolled_back')
 AND EXISTS(SELECT 1 FROM runtime_container_activation_authorities x WHERE x.activation_id=a.id)
 AND EXISTS(SELECT 1 FROM runtime_container_launches published WHERE published.agent_id=a.agent_id AND published.state IN ('running','stopping','exited')
   AND published.prepared=CASE WHEN a.record->>'phase'='committed' THEN a.record#>'{candidate,prepared}' ELSE a.record#>'{rollback,prepared}' END
   AND published.snapshot=CASE WHEN a.record->>'phase'='committed' THEN a.record#>'{candidate,snapshot}' ELSE a.record#>'{rollback,snapshot}' END
   AND published.origin=CASE WHEN a.record->>'phase'='committed' THEN a.record#>>'{candidate,origin}' ELSE a.record#>>'{rollback,origin}' END
   AND published.controller_id::text=a.record#>>'{claim,controller_id}'
   AND published.generation::text=a.record#>>'{readiness,generation}'
   AND h.effective_revision::text IS NOT DISTINCT FROM published.prepared->>'configuration_revision'))))
$$;
CREATE FUNCTION fleet_guard_activation_authority() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP<>'INSERT' THEN RAISE EXCEPTION 'Recovered activation authority is immutable'; END IF;
 IF NOT EXISTS(SELECT 1 FROM runtime_container_activations a JOIN runtime_container_recoveries r
    ON r.id=NEW.recovery_id WHERE a.id=NEW.activation_id AND a.record->'claim'=NEW.claim
    AND a.record#>>'{claim,intent_sha256}'=NEW.plan_sha256
    AND r.generation::text=NEW.claim#>>'{previous,prepared,container,registration,generation}'
    AND r.command#>>'{request,controller_id}'=NEW.controller_id::text
    AND r.receipt IS NOT NULL AND r.lease_receipt IS NOT NULL AND r.expires_at>clock_timestamp()
    AND NOT EXISTS(SELECT 1 FROM runtime_container_recoveries n WHERE n.generation=r.generation AND n.epoch>r.epoch)
    AND fleet_activation_anchor(r.generation)
    AND (a.record->>'phase' NOT IN ('committed','rolled_back') OR EXISTS(
       SELECT 1 FROM runtime_container_activation_authorities old WHERE old.activation_id=a.id AND old.claim=NEW.claim AND old.plan_sha256=NEW.plan_sha256)))
 THEN RAISE EXCEPTION 'Original activation and live native recovery required'; END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER runtime_container_activation_authority_guard BEFORE INSERT OR UPDATE OR DELETE
 ON runtime_container_activation_authorities FOR EACH ROW EXECUTE FUNCTION fleet_guard_activation_authority();
CREATE FUNCTION fleet_activation_parent(g uuid) RETURNS uuid LANGUAGE sql AS $$
 SELECT (a.record#>>'{claim,previous,prepared,container,registration,generation}')::uuid
 FROM runtime_container_activations a
 WHERE (a.record#>>'{claim,candidate,generation}'=g::text OR a.record#>>'{claim,rollback,generation}'=g::text)
 AND EXISTS(SELECT 1 FROM runtime_container_activation_authorities x WHERE x.activation_id=a.id
   AND x.claim=a.record->'claim' AND x.plan_sha256=a.record#>>'{claim,intent_sha256}')
$$;
CREATE OR REPLACE FUNCTION fleet_container_custody_live(g uuid) RETURNS boolean LANGUAGE plpgsql AS $$
DECLARE anchor uuid; latest runtime_container_recoveries;
BEGIN
 -- All repository mutations use the same agent -> anchor -> latest lease lock order.
 PERFORM agent.id FROM agents agent JOIN runtime_container_launches l ON l.agent_id=agent.id
   WHERE l.generation=g FOR UPDATE OF agent;
 anchor := fleet_activation_parent(g);
 IF anchor IS NOT NULL AND (NOT fleet_activation_anchor(anchor)
   OR EXISTS(SELECT 1 FROM runtime_container_recoveries WHERE generation=g)
   OR NOT EXISTS(SELECT 1 FROM runtime_container_launches l JOIN runtime_container_activations a ON a.agent_id=l.agent_id
     WHERE l.generation=g AND l.state='running'
     AND a.record#>>'{claim,previous,prepared,container,registration,generation}'=anchor::text
     AND l.controller_id::text=a.record#>>'{claim,controller_id}'
     AND ((l.prepared=a.record#>'{candidate,prepared}' AND l.stop_id::text=a.record#>>'{claim,candidate,stop_id}')
       OR (l.prepared=a.record#>'{rollback,prepared}' AND l.stop_id::text=a.record#>>'{claim,rollback,stop_id}'))))
 THEN RETURN false; END IF;
 PERFORM generation FROM runtime_container_launches WHERE generation=COALESCE(anchor,g) FOR UPDATE;
 SELECT * INTO latest FROM runtime_container_recoveries WHERE generation=COALESCE(anchor,g) ORDER BY epoch DESC LIMIT 1 FOR UPDATE;
 IF NOT FOUND THEN RETURN anchor IS NULL; END IF;
 RETURN latest.receipt IS NOT NULL AND latest.lease_receipt IS NOT NULL AND latest.expires_at>clock_timestamp()
 AND (anchor IS NULL OR EXISTS(SELECT 1 FROM runtime_container_activation_authorities x
   JOIN runtime_container_activations a ON a.id=x.activation_id
   WHERE x.recovery_id=latest.id AND x.controller_id::text=latest.command#>>'{request,controller_id}'
   AND x.claim=a.record->'claim' AND x.plan_sha256=a.record#>>'{claim,intent_sha256}'
   AND (a.record#>>'{claim,candidate,generation}'=g::text OR a.record#>>'{claim,rollback,generation}'=g::text)));
END $$;
CREATE FUNCTION fleet_fence_activation_child() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE anchor uuid; bound runtime_container_launches; latest runtime_container_recoveries;
BEGIN
 IF TG_OP='DELETE' THEN bound:=OLD; ELSE bound:=NEW; END IF;
 anchor:=fleet_activation_parent(bound.generation);
 IF anchor IS NULL THEN RETURN bound; END IF;
 PERFORM id FROM agents WHERE id=bound.agent_id FOR UPDATE;
 PERFORM generation FROM runtime_container_launches WHERE generation=anchor FOR UPDATE;
 SELECT * INTO latest FROM runtime_container_recoveries WHERE generation=anchor ORDER BY epoch DESC LIMIT 1 FOR UPDATE;
 IF latest.id IS NULL OR latest.id::text IS DISTINCT FROM current_setting('fleet.container_recovery_id',true)
   OR latest.receipt IS NULL OR latest.lease_receipt IS NULL OR latest.expires_at<=clock_timestamp()
   OR NOT fleet_activation_anchor(anchor)
   OR NOT EXISTS(SELECT 1 FROM runtime_container_activations a WHERE a.agent_id=bound.agent_id
     AND a.record#>>'{claim,previous,prepared,container,registration,generation}'=anchor::text
     AND bound.controller_id::text=a.record#>>'{claim,controller_id}'
     AND EXISTS(SELECT 1 FROM runtime_container_activation_authorities x WHERE x.activation_id=a.id
       AND x.recovery_id=latest.id AND x.controller_id::text=latest.command#>>'{request,controller_id}'
       AND x.claim=a.record->'claim' AND x.plan_sha256=a.record#>>'{claim,intent_sha256}')
     AND ((bound.prepared=a.record#>'{candidate,prepared}' AND bound.stop_id::text=a.record#>>'{claim,candidate,stop_id}')
       OR (bound.prepared=a.record#>'{rollback,prepared}' AND bound.stop_id::text=a.record#>>'{claim,rollback,stop_id}')))
 THEN RAISE EXCEPTION 'Replacement requires latest original native custody' USING ERRCODE='23514'; END IF;
 IF TG_OP='DELETE' THEN RAISE EXCEPTION 'Replacement history is immutable' USING ERRCODE='23514'; END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER fleet_activation_child_fence BEFORE INSERT OR UPDATE OR DELETE ON runtime_container_launches
 FOR EACH ROW EXECUTE FUNCTION fleet_fence_activation_child();
CREATE FUNCTION fleet_forbid_child_recovery() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF fleet_activation_parent(NEW.generation) IS NOT NULL THEN
   RAISE EXCEPTION 'Replacement cannot become another custody anchor' USING ERRCODE='23514';
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER fleet_activation_child_recovery_fence BEFORE INSERT ON runtime_container_recoveries
 FOR EACH ROW EXECUTE FUNCTION fleet_forbid_child_recovery();
"#).await?;
        anchor_state(manager, true).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                r#"
LOCK TABLE runtime_container_activation_authorities IN ACCESS EXCLUSIVE MODE;
DO $$ BEGIN
 IF EXISTS(SELECT 1 FROM runtime_container_activation_authorities)
 THEN RAISE EXCEPTION 'Recovered activation history prevents downgrade'; END IF;
END $$;
"#,
            )
            .await?;
        anchor_state(manager, false).await?;
        manager.get_connection().execute_unprepared(r#"
DROP TRIGGER fleet_activation_child_fence ON runtime_container_launches;
DROP TRIGGER fleet_activation_child_recovery_fence ON runtime_container_recoveries;
DROP FUNCTION fleet_fence_activation_child();
DROP FUNCTION fleet_forbid_child_recovery();
CREATE OR REPLACE FUNCTION fleet_container_custody_live(g uuid) RETURNS boolean AS $$
            SELECT NOT EXISTS(SELECT 1 FROM runtime_container_recoveries WHERE generation=g)
            OR COALESCE((SELECT receipt IS NOT NULL AND lease_receipt IS NOT NULL AND expires_at>clock_timestamp()
                FROM runtime_container_recoveries WHERE generation=g ORDER BY epoch DESC LIMIT 1),false)
        $$ LANGUAGE sql;
DROP FUNCTION fleet_activation_parent(uuid);
DROP FUNCTION fleet_activation_anchor(uuid);
DROP TABLE runtime_container_activation_authorities;
DROP FUNCTION fleet_guard_activation_authority();
"#).await?;
        Ok(())
    }
}
