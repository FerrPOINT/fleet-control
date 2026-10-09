use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

async fn snapshot_versions(manager: &SchemaManager<'_>, mapped: bool) -> Result<(), DbErr> {
    let (from, to) = if mapped {
        ("= '2'::text", "= ANY (ARRAY['2'::text, '3'::text])")
    } else {
        ("= ANY (ARRAY['2'::text, '3'::text])", "= '2'::text")
    };
    let rows = manager
        .get_connection()
        .query_all(sea_orm::Statement::from_string(
            sea_orm::DbBackend::Postgres,
            "SELECT conname,pg_get_constraintdef(oid) AS definition FROM pg_constraint
         WHERE conrelid='runtime_container_launches'::regclass AND contype='c'
           AND pg_get_constraintdef(oid) LIKE '%contract_version%'",
        ))
        .await?;
    if rows.len() != 1 {
        return Err(DbErr::Custom("Original snapshot constraint changed".into()));
    }
    let name: String = rows[0].try_get("", "conname")?;
    let definition: String = rows[0].try_get("", "definition")?;
    if definition.matches(from).count() != 1
        || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
    {
        return Err(DbErr::Custom(
            "Original snapshot version guard changed".into(),
        ));
    }
    manager.get_connection().execute_unprepared(&format!(
        "ALTER TABLE runtime_container_launches DROP CONSTRAINT {name}; ALTER TABLE runtime_container_launches ADD CONSTRAINT {name} {}",
        definition.replacen(from, to, 1))).await?;
    Ok(())
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        snapshot_versions(manager, true).await?;
        manager.get_connection().execute_unprepared(r#"
        CREATE TABLE runtime_container_recoveries (
            id uuid PRIMARY KEY,
            generation uuid NOT NULL REFERENCES runtime_container_launches(generation),
            epoch bigint NOT NULL CHECK(epoch BETWEEN 1 AND 1024),
            command jsonb NOT NULL CHECK(jsonb_typeof(command)='object' AND octet_length(command::text)<=16384),
            receipt jsonb CHECK(receipt IS NULL OR (jsonb_typeof(receipt)='object' AND octet_length(receipt::text)<=32768)),
            lease jsonb NOT NULL CHECK(jsonb_typeof(lease)='object' AND octet_length(lease::text)<=16384),
            lease_receipt jsonb CHECK(lease_receipt IS NULL OR (jsonb_typeof(lease_receipt)='object' AND octet_length(lease_receipt::text)<=32768)),
            expires_at timestamptz NOT NULL,
            UNIQUE(generation,epoch),
            CHECK((command #>> '{request,id}') IS NOT DISTINCT FROM id::text),
            CHECK((command #>> '{request,launch_id}') IS NOT DISTINCT FROM generation::text),
            CHECK((command->>'epoch') IS NOT DISTINCT FROM epoch::text),
            CHECK((command->>'lease_version') IS NOT DISTINCT FROM '1'),
            CHECK((lease->'request') IS NOT DISTINCT FROM (command->'request') AND (lease->>'epoch') IS NOT DISTINCT FROM epoch::text),
            CHECK(((lease->>'lease_version')::bigint>0) IS TRUE),
            CHECK((lease->>'lease_expires_at')::timestamptz IS NOT DISTINCT FROM expires_at),
            CHECK(receipt IS NULL OR ((receipt->>'state'='controller_recovered'
                AND receipt->'recovery'=command AND receipt->>'contract_version'='1') IS TRUE)),
            CHECK(lease_receipt IS NULL OR (receipt IS NOT NULL AND ((lease_receipt #>> '{ack,state}'='controller_heartbeat'
                AND lease_receipt #>> '{ack,recovery_id}'=id::text AND lease_receipt #>> '{ack,lease_version}'=lease->>'lease_version'
                AND lease_receipt #>> '{ack,lease_expires_at}'=lease->>'lease_expires_at') IS TRUE)))
        );
        CREATE FUNCTION fleet_guard_container_recovery() RETURNS trigger AS $$
        DECLARE c runtime_container_launches; p runtime_container_recoveries;
        BEGIN
            IF TG_OP='DELETE' THEN RAISE EXCEPTION 'Original recovery history cannot be deleted' USING ERRCODE='23514'; END IF;
            SELECT generation,agent_id,controller_id,prepared,state,snapshot,origin,stop_id,created_at INTO STRICT c
                FROM runtime_container_launches WHERE generation=NEW.generation FOR UPDATE;
            IF c.state NOT IN ('running','stopping') OR (c.prepared #>> '{container,registration,contract_version}') IS DISTINCT FROM '3'
                OR (NEW.command #>> '{request,agent_id}') IS DISTINCT FROM c.agent_id::text
                OR (NEW.command #>> '{request,original_controller_id}') IS DISTINCT FROM c.controller_id::text
                OR (NEW.command #>> '{request,controller_id}') IS NOT DISTINCT FROM c.controller_id::text
                OR (NEW.command #>> '{request,agent_pid}') IS DISTINCT FROM (c.snapshot->>'init_pid')
                OR (NEW.command #>> '{request,controller_snapshot,container_id}') IS DISTINCT FROM (c.prepared #>> '{container,mapped,mapping,controller,container_id}')
                OR (NEW.command #>> '{request,controller_snapshot,inventory_sha256}') IS DISTINCT FROM (c.prepared #>> '{container,mapped,mapping,snapshot,inventory_sha256}')
                OR (NEW.command #>> '{request,controller_snapshot,started_at}') IS NOT DISTINCT FROM (c.prepared #>> '{container,mapped,mapping,snapshot,started_at}') THEN
                RAISE EXCEPTION 'Recovery differs from original physical custody' USING ERRCODE='23514';
            END IF;
            IF TG_OP='INSERT' THEN
                IF NEW.receipt IS NOT NULL OR NEW.lease_receipt IS NOT NULL OR NEW.lease IS DISTINCT FROM NEW.command
                    OR NEW.expires_at<=clock_timestamp() OR NEW.expires_at>clock_timestamp()+interval '30 seconds' THEN
                    RAISE EXCEPTION 'Recovery requires bounded pre-effect claim' USING ERRCODE='23514';
                END IF;
                SELECT id,generation,epoch,command,receipt,lease,lease_receipt,expires_at INTO p
                    FROM runtime_container_recoveries WHERE generation=NEW.generation ORDER BY epoch DESC LIMIT 1;
                IF FOUND THEN
                    IF p.receipt IS NULL OR p.lease_receipt IS NULL OR p.expires_at>clock_timestamp()
                        OR NEW.epoch<>p.epoch+1 OR (NEW.command #>> '{request,predecessor_id}') IS DISTINCT FROM p.id::text
                        OR (NEW.command #>> '{request,controller_id}') IS NOT DISTINCT FROM (p.command #>> '{request,controller_id}')
                        OR (NEW.command #>> '{request,controller_snapshot,started_at}') IS NOT DISTINCT FROM (p.command #>> '{request,controller_snapshot,started_at}') THEN
                        RAISE EXCEPTION 'Previous owner or unknown delivery remains held' USING ERRCODE='23514';
                    END IF;
                    IF ((NEW.command->'request')-'id'-'controller_id'-'predecessor_id'-'controller_snapshot') IS DISTINCT FROM
                        ((p.command->'request')-'id'-'controller_id'-'predecessor_id'-'controller_snapshot') THEN
                        RAISE EXCEPTION 'Recovery predecessor identity changed' USING ERRCODE='23514';
                    END IF;
                ELSIF NEW.epoch<>1 OR NEW.command #>> '{request,predecessor_id}' IS NOT NULL THEN
                    RAISE EXCEPTION 'Recovery predecessor is missing' USING ERRCODE='23514';
                END IF;
            ELSE
                IF (to_jsonb(NEW)-'receipt'-'lease'-'lease_receipt'-'expires_at') IS DISTINCT FROM (to_jsonb(OLD)-'receipt'-'lease'-'lease_receipt'-'expires_at')
                    OR (OLD.receipt IS NOT NULL AND NEW.receipt IS DISTINCT FROM OLD.receipt)
                    OR EXISTS(SELECT 1 FROM runtime_container_recoveries WHERE generation=NEW.generation AND epoch>NEW.epoch) THEN
                    RAISE EXCEPTION 'Original recovery identity is immutable' USING ERRCODE='23514';
                END IF;
                IF NEW.lease IS DISTINCT FROM OLD.lease THEN
                    IF OLD.receipt IS NULL OR OLD.lease_receipt IS NULL OR NEW.lease_receipt IS NOT NULL OR OLD.expires_at<=clock_timestamp()
                        OR NEW.expires_at<=OLD.expires_at OR NEW.expires_at>clock_timestamp()+interval '30 seconds'
                        OR (NEW.lease->>'lease_version')::bigint<>(OLD.lease->>'lease_version')::bigint+1 THEN
                        RAISE EXCEPTION 'Lease must be claimed before native heartbeat' USING ERRCODE='23514';
                    END IF;
                ELSIF NEW.expires_at IS DISTINCT FROM OLD.expires_at OR (OLD.lease_receipt IS NOT NULL AND NEW.lease_receipt IS DISTINCT FROM OLD.lease_receipt) THEN
                    RAISE EXCEPTION 'Lease acknowledgement is immutable' USING ERRCODE='23514';
                END IF;
            END IF;
            IF (NEW.receipt IS NOT NULL AND ((NEW.receipt #> '{witness,receipt,snapshot}') IS DISTINCT FROM c.snapshot
                    OR (NEW.receipt->>'request_sha256') IS NULL
                    OR (NEW.receipt #>> '{witness,registration_sha256}') IS DISTINCT FROM (NEW.command #>> '{request,registration_sha256}')
                    OR (NEW.receipt #>> '{witness,original_mapping_sha256}') IS DISTINCT FROM (NEW.command #>> '{request,mapping_sha256}')
                    OR (NEW.receipt #> '{witness,current_controller_snapshot}') IS DISTINCT FROM (NEW.command #> '{request,controller_snapshot}')))
                OR (NEW.lease_receipt IS NOT NULL AND (NEW.lease_receipt #> '{receipt,snapshot}') IS DISTINCT FROM c.snapshot) THEN
                RAISE EXCEPTION 'Native receipt differs from original custody' USING ERRCODE='23514';
            END IF;
            RETURN NEW;
        END; $$ LANGUAGE plpgsql;
        CREATE TRIGGER fleet_container_recovery_guard BEFORE INSERT OR UPDATE OR DELETE ON runtime_container_recoveries
            FOR EACH ROW EXECUTE FUNCTION fleet_guard_container_recovery();
        CREATE FUNCTION fleet_fence_container_launch_recovery() RETURNS trigger AS $$
        BEGIN
            IF EXISTS(SELECT 1 FROM runtime_container_recoveries WHERE generation=OLD.generation)
                AND NOT EXISTS(SELECT 1 FROM runtime_container_recoveries r WHERE r.generation=OLD.generation
                    AND r.id::text=current_setting('fleet.container_recovery_id',true)
                    AND r.receipt IS NOT NULL AND r.lease_receipt IS NOT NULL AND r.expires_at>clock_timestamp()
                    AND NOT EXISTS(SELECT 1 FROM runtime_container_recoveries n WHERE n.generation=r.generation AND n.epoch>r.epoch)) THEN
                RAISE EXCEPTION 'Original owner is fenced after recovery claim' USING ERRCODE='23514';
            END IF;
            RETURN NEW;
        END; $$ LANGUAGE plpgsql;
        CREATE TRIGGER fleet_container_launch_recovery_fence BEFORE UPDATE ON runtime_container_launches
            FOR EACH ROW EXECUTE FUNCTION fleet_fence_container_launch_recovery();
        CREATE FUNCTION fleet_container_custody_live(g uuid) RETURNS boolean AS $$
            SELECT NOT EXISTS(SELECT 1 FROM runtime_container_recoveries WHERE generation=g)
            OR COALESCE((SELECT receipt IS NOT NULL AND lease_receipt IS NOT NULL AND expires_at>clock_timestamp()
                FROM runtime_container_recoveries WHERE generation=g ORDER BY epoch DESC LIMIT 1),false)
        $$ LANGUAGE sql;
        CREATE OR REPLACE FUNCTION fleet_container_origin(agent uuid, endpoint text, port integer, caps jsonb) RETURNS boolean AS $$
            SELECT CASE WHEN EXISTS(SELECT 1 FROM runtime_container_launches WHERE agent_id=agent)
            THEN EXISTS(SELECT 1 FROM runtime_container_launches c WHERE c.agent_id=agent AND c.state='running'
                AND c.origin=endpoint AND c.prepared->>'api_port'=port::text
                AND caps->>'fleet_container_generation'=c.generation::text AND fleet_container_custody_live(c.generation))
            ELSE endpoint='http://127.0.0.1:' || port::text AND NOT caps ? 'fleet_container_generation' END
        $$ LANGUAGE sql;
        "#).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(r#"
        LOCK TABLE runtime_container_launches,runtime_container_recoveries IN ACCESS EXCLUSIVE MODE;
        DO $$ BEGIN
            IF EXISTS(SELECT 1 FROM runtime_container_recoveries) OR EXISTS(SELECT 1 FROM runtime_container_launches
                WHERE prepared #>> '{container,registration,contract_version}'='3') THEN
                RAISE EXCEPTION 'Mapped custody history prevents downgrade' USING ERRCODE='23514';
            END IF;
        END $$;
        CREATE OR REPLACE FUNCTION fleet_container_origin(agent uuid, endpoint text, port integer, caps jsonb) RETURNS boolean AS $$
            SELECT CASE WHEN EXISTS(SELECT 1 FROM runtime_container_launches WHERE agent_id=agent)
            THEN EXISTS(SELECT 1 FROM runtime_container_launches c WHERE c.agent_id=agent AND c.state='running'
                AND c.origin=endpoint AND c.prepared->>'api_port'=port::text
                AND caps->>'fleet_container_generation'=c.generation::text)
            ELSE endpoint='http://127.0.0.1:' || port::text AND NOT caps ? 'fleet_container_generation' END
        $$ LANGUAGE sql STABLE;
        DROP FUNCTION fleet_container_custody_live(uuid);
        DROP TRIGGER fleet_container_launch_recovery_fence ON runtime_container_launches;
        DROP FUNCTION fleet_fence_container_launch_recovery();
        DROP TABLE runtime_container_recoveries;
        DROP FUNCTION fleet_guard_container_recovery();
        "#).await?;
        snapshot_versions(manager, false).await
    }
}
