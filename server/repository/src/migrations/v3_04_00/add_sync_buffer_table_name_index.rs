use crate::migrations::*;

pub(crate) struct Migrate;

impl MigrationFragment for Migrate {
    fn identifier(&self) -> &'static str {
        "add_sync_buffer_table_name_index"
    }

    fn migrate(&self, connection: &StorageConnection) -> anyhow::Result<()> {
        // Reintegrating by table_name (migrations that re-flag buffered rows after a
        // translator fix, a new column backfill, etc.) filters on table_name alone.
        // The only other sync_buffer index is the partial pending one, led by
        // source_site_id/reference_id, so table_name can't be seeked and the large
        // integrated/archived side was scanned in full.
        //
        // Postgres: an index on the partitioned parent is created on every partition
        // (pending and archive), and on any partition attached later. SQLite: a plain
        // index on the table.
        sql!(
            connection,
            r#"
            CREATE INDEX IF NOT EXISTS index_sync_buffer_table_name
                ON sync_buffer (table_name);
            "#
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use crate::migrations::*;
    use crate::test_db::*;
    use diesel::{prelude::*, sql_query, sql_types::Text};

    #[actix_rt::test]
    async fn migration_add_sync_buffer_table_name_index() {
        let previous_version = v3_03_01::V3_03_01.version();
        let version = v3_04_00::V3_04_00.version();

        let SetupResult { connection, .. } = setup_test(SetupOption {
            db_name: &format!("migration_{version}_add_sync_buffer_table_name_index"),
            version: Some(previous_version),
            ..Default::default()
        })
        .await;

        #[derive(QueryableByName)]
        struct NameRow {
            #[diesel(sql_type = Text)]
            name: String,
        }
        // Postgres lists the parent and one index per partition; all must exist
        let lookup = if cfg!(feature = "postgres") {
            "SELECT indexname AS name FROM pg_indexes WHERE tablename LIKE 'sync_buffer%' AND indexdef LIKE '%(table_name)%'"
        } else {
            "SELECT name FROM sqlite_master WHERE type = 'index' AND tbl_name = 'sync_buffer' AND sql LIKE '%(table_name)%'"
        };
        let indexes = |connection: &StorageConnection| -> Vec<String> {
            sql_query(lookup)
                .load::<NameRow>(connection.lock().connection())
                .unwrap()
                .into_iter()
                .map(|r| r.name)
                .collect()
        };

        assert!(indexes(&connection).is_empty());

        migrate(&connection, Some(version), MigrationConfig::default()).unwrap();

        let found = indexes(&connection);
        assert!(found.contains(&"index_sync_buffer_table_name".to_string()));
        if cfg!(feature = "postgres") {
            // parent + pending + archive
            assert_eq!(found.len(), 3);
        }
    }
}
