use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

async fn replace_ack_constraint(manager: &SchemaManager<'_>, up: bool) -> Result<(), DbErr> {
    // Let PostgreSQL canonicalize both trusted shapes, rather than guessing its deparser format.
    manager.get_connection().execute_unprepared(&format!(r#"
        DO $repair$
        DECLARE original text; repaired text; source_shape text; target_shape text;
                source_count bigint; target_count bigint; constraint_name text;
        BEGIN
            LOCK TABLE pm_dispatch_journal IN ACCESS EXCLUSIVE MODE;
            IF NOT {up} AND EXISTS(SELECT 1 FROM pm_dispatch_journal) THEN
                RAISE EXCEPTION 'PM dispatch custody prevents ACK bounds downgrade';
            END IF;
            CREATE TEMPORARY TABLE fleet_pm_ack_shapes_v24 (
                submitted boolean, hermes_run_ref text,
                CONSTRAINT original_ack CHECK(hermes_run_ref IS NULL OR
                    (submitted AND hermes_run_ref ~ '^[A-Za-z0-9_-]{{1,512}}$')),
                CONSTRAINT repaired_ack CHECK(hermes_run_ref IS NULL OR
                    (submitted AND length(hermes_run_ref) BETWEEN 1 AND 512
                     AND hermes_run_ref ~ '^[A-Za-z0-9_-]+$'))
            ) ON COMMIT DROP;
            SELECT pg_get_constraintdef(oid) INTO STRICT original FROM pg_constraint
                WHERE conrelid='pg_temp.fleet_pm_ack_shapes_v24'::regclass AND conname='original_ack';
            SELECT pg_get_constraintdef(oid) INTO STRICT repaired FROM pg_constraint
                WHERE conrelid='pg_temp.fleet_pm_ack_shapes_v24'::regclass AND conname='repaired_ack';
            source_shape := CASE WHEN {up} THEN original ELSE repaired END;
            target_shape := CASE WHEN {up} THEN repaired ELSE original END;
            SELECT count(*), min(conname::text) INTO source_count, constraint_name FROM pg_constraint
                WHERE conrelid='pm_dispatch_journal'::regclass AND contype='c'
                  AND convalidated AND NOT connoinherit AND pg_get_constraintdef(oid)=source_shape;
            SELECT count(*) INTO target_count FROM pg_constraint
                WHERE conrelid='pm_dispatch_journal'::regclass AND contype='c'
                  AND convalidated AND NOT connoinherit AND pg_get_constraintdef(oid)=target_shape;
            IF source_count=1 AND target_count=0 THEN
                EXECUTE format('ALTER TABLE pm_dispatch_journal DROP CONSTRAINT %I, ADD CONSTRAINT %I %s',
                    constraint_name, constraint_name, target_shape);
            ELSIF NOT (source_count=0 AND target_count=1) THEN
                RAISE EXCEPTION 'PM ACK constraint shape changed';
            END IF;
            DROP TABLE pg_temp.fleet_pm_ack_shapes_v24;
        END $repair$;
    "#)).await?;
    Ok(())
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        replace_ack_constraint(manager, true).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        replace_ack_constraint(manager, false).await
    }
}
