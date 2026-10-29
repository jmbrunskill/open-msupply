use crate::migrations::*;

pub(crate) struct Migrate;

impl MigrationFragment for Migrate {
    fn identifier(&self) -> &'static str {
        "add_not_functioning_asset_log_reasons"
    }

    // WHO terminology defaults for the 'Not functioning' status ('Needs spare
    // parts' already exists). asset_log_reason syncs down from central, so skip
    // reasons that are already there (by id or by name).
    fn migrate(&self, connection: &StorageConnection) -> anyhow::Result<()> {
        sql!(
            connection,
            r#"
            INSERT INTO asset_log_reason (id, asset_log_status, reason, comments_required)
            SELECT 'c0a6f6c2-5d38-4a5e-9f0b-2b1f6a7e1d01', 'NOT_FUNCTIONING', 'Multiple temperature excursions', false
            WHERE NOT EXISTS (
                SELECT 1 FROM asset_log_reason WHERE asset_log_status = 'NOT_FUNCTIONING' AND reason = 'Multiple temperature excursions'
            ) ON CONFLICT (id) DO NOTHING;
            INSERT INTO asset_log_reason (id, asset_log_status, reason, comments_required)
            SELECT 'c0a6f6c2-5d38-4a5e-9f0b-2b1f6a7e1d02', 'NOT_FUNCTIONING', 'Solar panel failure', false
            WHERE NOT EXISTS (
                SELECT 1 FROM asset_log_reason WHERE asset_log_status = 'NOT_FUNCTIONING' AND reason = 'Solar panel failure'
            ) ON CONFLICT (id) DO NOTHING;
            INSERT INTO asset_log_reason (id, asset_log_status, reason, comments_required)
            SELECT 'c0a6f6c2-5d38-4a5e-9f0b-2b1f6a7e1d03', 'NOT_FUNCTIONING', 'Other', true
            WHERE NOT EXISTS (
                SELECT 1 FROM asset_log_reason WHERE asset_log_status = 'NOT_FUNCTIONING' AND reason = 'Other'
            ) ON CONFLICT (id) DO NOTHING;
            "#
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use crate::migrations::*;
    use crate::test_db::*;
    use diesel::{
        RunQueryDsl, sql_query,
        sql_types::{Bool, Text},
    };

    #[derive(QueryableByName, Debug, PartialEq)]
    struct Reason {
        #[diesel(sql_type = Text)]
        reason: String,
        #[diesel(sql_type = Bool)]
        comments_required: bool,
    }

    fn not_functioning(connection: &StorageConnection) -> Vec<Reason> {
        sql_query(
            "SELECT reason, comments_required FROM asset_log_reason \
             WHERE asset_log_status = 'NOT_FUNCTIONING' ORDER BY reason",
        )
        .load::<Reason>(connection.lock().connection())
        .unwrap()
    }

    #[actix_rt::test]
    async fn migration_add_not_functioning_asset_log_reasons() {
        let previous_version = v3_02_00::V3_02_00.version();
        let version = v3_04_00::V3_04_00.version();

        let SetupResult { connection, .. } = setup_test(SetupOption {
            db_name: &format!("migration_{version}_add_not_functioning_asset_log_reasons"),
            version: Some(previous_version),
            ..Default::default()
        })
        .await;

        // a reason already present by name (e.g. synced from central) isn't duplicated
        sql!(
            &connection,
            r#"
            INSERT INTO asset_log_reason (id, asset_log_status, reason) VALUES ('central-solar', 'NOT_FUNCTIONING', 'Solar panel failure');
            "#
        )
        .unwrap();

        migrate(&connection, Some(version), MigrationConfig::default()).unwrap();

        let reasons = not_functioning(&connection);
        let get = |name: &str| {
            reasons
                .iter()
                .filter(|r| r.reason == name)
                .collect::<Vec<_>>()
        };
        assert_eq!(get("Needs spare parts").len(), 1);
        assert_eq!(get("Multiple temperature excursions").len(), 1);
        assert_eq!(get("Solar panel failure").len(), 1);
        assert_eq!(get("Other").len(), 1);
        assert!(get("Other")[0].comments_required);
        assert!(!get("Multiple temperature excursions")[0].comments_required);
    }
}
