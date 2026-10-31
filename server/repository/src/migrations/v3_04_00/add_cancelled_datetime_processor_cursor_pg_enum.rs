use crate::migrations::*;

/// Cursor for the processor that sets cancelled_datetime on invoices with a linked
/// cancellation invoice (synced from mSupply).
pub(crate) struct Migrate;

impl MigrationFragment for Migrate {
    fn identifier(&self) -> &'static str {
        "add_cancelled_datetime_processor_cursor_pg_enum"
    }

    fn migrate(&self, connection: &StorageConnection) -> anyhow::Result<()> {
        if cfg!(feature = "postgres") {
            sql!(
                connection,
                r#"
                    ALTER TYPE key_type ADD VALUE IF NOT EXISTS 'CANCELLED_DATETIME_PROCESSOR_CURSOR';
                "#
            )?;
        }

        Ok(())
    }
}
