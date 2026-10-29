use crate::migrations::*;

pub(crate) struct Migrate;

impl MigrationFragment for Migrate {
    fn identifier(&self) -> &'static str {
        "add_stock_relocation_activity_log_types"
    }

    fn migrate(&self, connection: &StorageConnection) -> anyhow::Result<()> {
        if cfg!(feature = "postgres") {
            sql!(
                connection,
                r#"
                    ALTER TYPE activity_log_type ADD VALUE IF NOT EXISTS 'STOCK_RELOCATION_CREATED';
                    ALTER TYPE activity_log_type ADD VALUE IF NOT EXISTS 'STOCK_RELOCATION_CONFIRMED';
                    ALTER TYPE activity_log_type ADD VALUE IF NOT EXISTS 'STOCK_RELOCATION_FINALISED';
                    ALTER TYPE activity_log_type ADD VALUE IF NOT EXISTS 'STOCK_RELOCATION_DELETED';
                "#
            )?;
        }
        Ok(())
    }
}
