use sea_orm::{DbBackend, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const ORIGINAL_ORIGIN: &str = "NEW.origin='http://127.0.0.1:' || a.api_port::text";
const BOUND_ORIGIN: &str = "fleet_container_origin(a.id, NEW.origin, a.api_port, NEW.capabilities)";

async fn replace_origin(manager: &SchemaManager<'_>, from: &str, to: &str) -> Result<(), DbErr> {
    let row = manager
        .get_connection()
        .query_one(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT pg_get_functiondef(to_regprocedure($1)) AS definition",
            ["fleet_guard_hermes_dispatch()".into()],
        ))
        .await?
        .ok_or_else(|| DbErr::Custom("Hermes guard is missing".into()))?;
    let definition: String = row.try_get("", "definition")?;
    if definition.matches(from).count() != 1 {
        return Err(DbErr::Custom(
            "Hermes origin guard differs from accepted history".into(),
        ));
    }
    manager
        .get_connection()
        .execute_unprepared(&definition.replacen(from, to, 1))
        .await?;
    Ok(())
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(
            "CREATE TABLE runtime_container_launches (
                generation uuid PRIMARY KEY,
                agent_id uuid NOT NULL REFERENCES agents(id),
                controller_id uuid NOT NULL,
                prepared jsonb NOT NULL CHECK(jsonb_typeof(prepared)='object' AND octet_length(prepared::text)<=65536),
                state text NOT NULL CHECK(state IN ('claimed','running','stopping','exited')),
                snapshot jsonb,
                origin text,
                stop_id uuid NOT NULL UNIQUE,
                created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
                CHECK((prepared->>'agent_id') IS NOT DISTINCT FROM agent_id::text),
                CHECK((prepared #>> '{container,registration,generation}') IS NOT DISTINCT FROM generation::text),
                CHECK((prepared #>> '{container,registration,resource_id}') IS NOT DISTINCT FROM agent_id::text),
                CHECK((state='claimed' AND snapshot IS NULL AND origin IS NULL)
                    OR (state<>'claimed' AND snapshot IS NOT NULL AND jsonb_typeof(snapshot)='object' AND origin IS NOT NULL)),
                CHECK(snapshot IS NULL OR (((snapshot->>'contract_version')='2'
                    AND (snapshot->>'generation') IS NULL
                    AND (snapshot->>'container_id') IS NOT DISTINCT FROM (prepared #>> '{container,registration,container_id}')
                    AND (snapshot->'engine') IS NOT DISTINCT FROM (prepared #> '{container,registration,engine}')
                    AND (snapshot->>'inventory_sha256') IS NOT DISTINCT FROM (prepared #>> '{container,registration,running_inventory_sha256}')
                    AND (snapshot->>'policy_sha256') IS NOT DISTINCT FROM (prepared #>> '{container,registration,policy_sha256}')
                    AND (snapshot->>'network_sha256') IS NOT DISTINCT FROM (prepared #>> '{container,registration,network_sha256}')
                    AND (snapshot->>'init_pid')::bigint > 0 AND (snapshot->>'started_at') NOT LIKE '0001-%') IS TRUE)),
                CHECK(origin IS NULL OR origin ~ '^http://(10[.][0-9]{1,3}[.][0-9]{1,3}[.][0-9]{1,3}|172[.](1[6-9]|2[0-9]|3[01])[.][0-9]{1,3}[.][0-9]{1,3}|192[.]168[.][0-9]{1,3}[.][0-9]{1,3}):[0-9]{4,5}$')
             );
             CREATE UNIQUE INDEX runtime_container_open_agent ON runtime_container_launches(agent_id) WHERE state<>'exited';
             CREATE INDEX runtime_container_agent_history ON runtime_container_launches(agent_id,created_at DESC,generation);
             CREATE FUNCTION fleet_guard_container_launch() RETURNS trigger AS $$
             BEGIN
                IF TG_OP='DELETE' THEN
                    RAISE EXCEPTION 'Original container history cannot be deleted' USING ERRCODE='23514';
                ELSIF TG_OP='INSERT' THEN
                    IF NEW.state<>'claimed' THEN
                        RAISE EXCEPTION 'Original container must be claimed before start' USING ERRCODE='23514';
                    END IF;
                ELSE
                    IF (to_jsonb(NEW)-'state'-'snapshot'-'origin') IS DISTINCT FROM (to_jsonb(OLD)-'state'-'snapshot'-'origin')
                        OR (OLD.snapshot IS NOT NULL AND (NEW.snapshot IS DISTINCT FROM OLD.snapshot OR NEW.origin IS DISTINCT FROM OLD.origin))
                        OR (NEW.state IS DISTINCT FROM OLD.state AND NOT
                            ((OLD.state='claimed' AND NEW.state='running') OR (OLD.state='running' AND NEW.state IN ('stopping','exited'))
                             OR (OLD.state='stopping' AND NEW.state='exited')))
                        OR (NEW.state=OLD.state AND NEW IS DISTINCT FROM OLD) THEN
                        RAISE EXCEPTION 'Original container identity and progress are immutable' USING ERRCODE='23514';
                    END IF;
                END IF;
                RETURN NEW;
             END; $$ LANGUAGE plpgsql;
             CREATE TRIGGER fleet_container_launch_guard BEFORE INSERT OR UPDATE OR DELETE ON runtime_container_launches
                FOR EACH ROW EXECUTE FUNCTION fleet_guard_container_launch();
             CREATE FUNCTION fleet_container_origin(agent uuid, endpoint text, port integer, caps jsonb) RETURNS boolean AS $$
                SELECT CASE WHEN EXISTS(SELECT 1 FROM runtime_container_launches WHERE agent_id=agent)
                THEN EXISTS(SELECT 1 FROM runtime_container_launches c WHERE c.agent_id=agent AND c.state='running'
                    AND c.origin=endpoint AND c.prepared->>'api_port'=port::text
                    AND caps->>'fleet_container_generation'=c.generation::text)
                ELSE endpoint='http://127.0.0.1:' || port::text AND NOT caps ? 'fleet_container_generation' END
             $$ LANGUAGE sql STABLE;
             ALTER TABLE hermes_dispatch_journal DROP CONSTRAINT hermes_dispatch_journal_origin_check;
             ALTER TABLE hermes_dispatch_journal ADD CONSTRAINT hermes_dispatch_journal_origin_check
                CHECK(origin ~ '^http://[0-9]{1,3}[.][0-9]{1,3}[.][0-9]{1,3}[.][0-9]{1,3}:[0-9]{4,5}$');"
        ).await?;
        replace_origin(manager, ORIGINAL_ORIGIN, BOUND_ORIGIN).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(
            "LOCK TABLE runtime_container_launches,hermes_dispatch_journal IN ACCESS EXCLUSIVE MODE;
             DO $$ BEGIN
                IF EXISTS(SELECT 1 FROM runtime_container_launches)
                    OR EXISTS(SELECT 1 FROM hermes_dispatch_journal WHERE origin !~ '^http://127[.]0[.]0[.]1:[0-9]{4,5}$'
                        OR capabilities ? 'fleet_container_generation') THEN
                    RAISE EXCEPTION 'Original container history prevents downgrade' USING ERRCODE='23514';
                END IF;
             END $$;"
        ).await?;
        replace_origin(manager, BOUND_ORIGIN, ORIGINAL_ORIGIN).await?;
        manager.get_connection().execute_unprepared(
            "ALTER TABLE hermes_dispatch_journal DROP CONSTRAINT hermes_dispatch_journal_origin_check;
             ALTER TABLE hermes_dispatch_journal ADD CONSTRAINT hermes_dispatch_journal_origin_check
                CHECK(origin ~ '^http://127[.]0[.]0[.]1:[0-9]{4,5}$');
             DROP FUNCTION fleet_container_origin(uuid,text,integer,jsonb);
             DROP TABLE runtime_container_launches;
             DROP FUNCTION fleet_guard_container_launch();"
        ).await?;
        Ok(())
    }
}
