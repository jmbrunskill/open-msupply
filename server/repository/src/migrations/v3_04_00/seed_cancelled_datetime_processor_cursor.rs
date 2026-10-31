use crate::migrations::*;

/// Seed the cancelled-datetime processor's cursor at the current changelog head, so its
/// first run doesn't walk the entire historical changelog synchronously (the #12547
/// trap). Consequence: cancellations already synced before this version are not
/// backfilled; only new or re-synced invoices are processed.
///
/// Own fragment (own transaction) so Postgres can use the enum value added before it.
pub(crate) struct Migrate;

impl MigrationFragment for Migrate {
    fn identifier(&self) -> &'static str {
        "seed_cancelled_datetime_processor_cursor"
    }

    fn migrate(&self, connection: &StorageConnection) -> anyhow::Result<()> {
        sql!(
            connection,
            r#"
            INSERT INTO key_value_store (id, value_int)
            SELECT 'CANCELLED_DATETIME_PROCESSOR_CURSOR', COALESCE(MAX(cursor), 0)
            FROM changelog;
            "#
        )?;

        Ok(())
    }
}
