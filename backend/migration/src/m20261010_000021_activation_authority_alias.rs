use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

// OLD is a PL/pgSQL trigger record, not a safe SQL relation alias.
const ORIGINAL: &str = "SELECT 1 FROM runtime_container_activation_authorities old WHERE old.activation_id=a.id AND old.claim=NEW.claim AND old.plan_sha256=NEW.plan_sha256";
const REPAIRED: &str = "SELECT 1 FROM runtime_container_activation_authorities prior_authority WHERE prior_authority.activation_id=a.id AND prior_authority.claim=NEW.claim AND prior_authority.plan_sha256=NEW.plan_sha256";

fn definition(body: &str, up: bool) -> Result<String, DbErr> {
    let (from, to) = if up {
        (ORIGINAL, REPAIRED)
    } else {
        (REPAIRED, ORIGINAL)
    };
    match (body.matches(from).count(), body.matches(to).count()) {
        (1, 0) => Ok(body.replacen(from, to, 1)),
        (0, 1) => Ok(body.to_owned()),
        _ => Err(DbErr::Custom(
            "Activation authority alias guard changed".into(),
        )),
    }
}

async fn replace_alias(manager: &SchemaManager<'_>, up: bool) -> Result<(), DbErr> {
    let row = manager
        .get_connection()
        .query_one(sea_orm::Statement::from_sql_and_values(
            sea_orm::DbBackend::Postgres,
            "SELECT pg_get_functiondef(to_regprocedure($1)) AS body",
            ["fleet_guard_activation_authority()".into()],
        ))
        .await?
        .ok_or_else(|| DbErr::Custom("Missing activation authority guard".into()))?;
    let body: String = row.try_get("", "body")?;
    let repaired = definition(&body, up)?;
    if repaired != body {
        manager
            .get_connection()
            .execute_unprepared(&repaired)
            .await?;
    }
    Ok(())
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        replace_alias(manager, true).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                "LOCK TABLE runtime_container_activation_authorities IN ACCESS EXCLUSIVE MODE;
                 DO $$ BEGIN
                    IF EXISTS(SELECT 1 FROM runtime_container_activation_authorities) THEN
                        RAISE EXCEPTION 'Recovered activation history prevents alias repair downgrade';
                    END IF;
                 END $$;",
            )
            .await?;
        replace_alias(manager, false).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn original() -> &'static str {
        let source = include_str!("m20261009_000019_recovered_activation.rs");
        source
            .split_once("CREATE FUNCTION fleet_guard_activation_authority()")
            .unwrap()
            .1
            .split_once("END $$;")
            .unwrap()
            .0
    }

    #[test]
    fn authority_alias_repair_changes_only_ambiguous_relation_references() {
        let before = original();
        let after = definition(before, true).unwrap();
        assert_eq!(after, before.replacen(ORIGINAL, REPAIRED, 1));
        assert_eq!(after.replace("prior_authority", "old"), before);
        assert_eq!(definition(&after, false).unwrap(), before);
    }

    #[test]
    fn authority_alias_repair_is_repeatable_in_both_directions() {
        let before = original();
        let after = definition(before, true).unwrap();
        assert_eq!(definition(&after, true).unwrap(), after);
        assert_eq!(definition(before, false).unwrap(), before);
    }

    #[test]
    fn authority_alias_repair_rejects_missing_duplicate_and_mixed_guard() {
        for body in [
            String::new(),
            original().replace("old.claim=NEW.claim", "true"),
            format!("{ORIGINAL}\n{ORIGINAL}"),
            format!("{ORIGINAL}\n{REPAIRED}"),
        ] {
            assert!(definition(&body, true).is_err());
            assert!(definition(&body, false).is_err());
        }
    }
}
