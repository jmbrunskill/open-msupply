use crate::migrations::constants::COLD_ROOMS_AND_FREEZER_ROOMS_UUID;
use crate::migrations::*;

pub(crate) struct Migrate;

/// WHO PQS: cold rooms and freezer rooms have a 20 year lifespan (other CCE
/// stay at the 10 year default set in v2.4.0).
const COLD_ROOM_EXPECTED_LIFESPAN: i32 = 20;

impl MigrationFragment for Migrate {
    fn identifier(&self) -> &'static str {
        "update_cold_room_expected_lifespan"
    }

    // properties are stored as JSON text, so the JSON object is updated in place
    fn migrate(&self, connection: &StorageConnection) -> anyhow::Result<()> {
        if cfg!(feature = "postgres") {
            sql!(
                connection,
                r#"
                UPDATE asset_catalogue_item
                SET properties = jsonb_set(properties::jsonb, '{{expected_lifespan}}', '{COLD_ROOM_EXPECTED_LIFESPAN}')::text
                WHERE asset_category_id = '{COLD_ROOMS_AND_FREEZER_ROOMS_UUID}' AND properties IS NOT NULL;
                "#
            )?;
        } else {
            sql!(
                connection,
                r#"
                UPDATE asset_catalogue_item
                SET properties = json_set(properties, '$.expected_lifespan', {COLD_ROOM_EXPECTED_LIFESPAN})
                WHERE asset_category_id = '{COLD_ROOMS_AND_FREEZER_ROOMS_UUID}' AND properties IS NOT NULL;
                "#
            )?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use crate::migrations::constants::COLD_ROOMS_AND_FREEZER_ROOMS_UUID;
    use crate::migrations::*;
    use crate::test_db::*;
    use diesel::{
        RunQueryDsl, sql_query,
        sql_types::{BigInt, Text},
    };

    #[derive(QueryableByName)]
    struct Count {
        #[diesel(sql_type = BigInt)]
        c: i64,
    }

    fn count(connection: &StorageConnection, in_category: bool, lifespan: i32) -> i64 {
        let op = if in_category { "=" } else { "<>" };
        sql_query(format!(
            "SELECT COUNT(*) AS c FROM asset_catalogue_item \
             WHERE asset_category_id {op} '{COLD_ROOMS_AND_FREEZER_ROOMS_UUID}' \
             AND replace(properties, ' ', '') LIKE '%\"expected_lifespan\":{lifespan}%'"
        ))
        .get_result::<Count>(connection.lock().connection())
        .unwrap()
        .c
    }

    #[actix_rt::test]
    async fn migration_update_cold_room_expected_lifespan() {
        let previous_version = v3_02_00::V3_02_00.version();
        let version = v3_04_00::V3_04_00.version();

        let SetupResult { connection, .. } = setup_test(SetupOption {
            db_name: &format!("migration_{version}_update_cold_room_expected_lifespan"),
            version: Some(previous_version),
            ..Default::default()
        })
        .await;

        let cold_rooms_before = count(&connection, true, 10);
        let others_before = count(&connection, false, 10);
        assert!(cold_rooms_before > 0, "catalogue has cold room items");
        assert!(others_before > 0, "catalogue has other CCE items");

        migrate(&connection, Some(version), MigrationConfig::default()).unwrap();

        assert_eq!(count(&connection, true, 20), cold_rooms_before);
        assert_eq!(count(&connection, true, 10), 0);
        assert_eq!(count(&connection, false, 10), others_before);
        assert_eq!(count(&connection, false, 20), 0);
    }
}
