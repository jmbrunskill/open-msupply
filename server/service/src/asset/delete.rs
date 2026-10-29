use super::validate::check_asset_exists;
use crate::sync::CentralServerConfig;
use crate::sync_file_reference::delete_attached_sync_files;
use crate::{activity_log::activity_log_entry, service_provider::ServiceContext};
use repository::assets::asset_internal_location_row::AssetInternalLocationRowRepository;
use repository::{
    ActivityLogType, RepositoryError, StorageConnection, assets::asset_row::AssetRowRepository,
};

#[derive(PartialEq, Debug)]
pub enum DeleteAssetError {
    AssetDoesNotExist,
    AssetDoesNotBelongToCurrentStore,
    DatabaseError(RepositoryError),
}
impl From<RepositoryError> for DeleteAssetError {
    fn from(error: RepositoryError) -> Self {
        DeleteAssetError::DatabaseError(error)
    }
}

pub fn delete_asset(ctx: &ServiceContext, asset_id: String) -> Result<String, DeleteAssetError> {
    ctx.connection
        .transaction_sync(|connection| {
            validate(connection, &ctx.store_id, &asset_id)?;

            activity_log_entry(
                ctx,
                ActivityLogType::AssetDeleted,
                Some(asset_id.clone()),
                None,
                None,
            )?;

            // This means that on soft deletion of asset, location assignments will be hard deleted.
            // If we bring assets back, they won't retain their location assignments.
            // This was done to make validation checks easier for assigning locations to assets
            let _deleted_location = AssetInternalLocationRowRepository::new(connection)
                .delete_all_for_asset_id(&asset_id);

            delete_attached_sync_files(connection, "asset", &asset_id)?;

            AssetRowRepository::new(connection)
                .mark_deleted(&asset_id)
                .map_err(DeleteAssetError::from)
        })
        .map_err(|error| error.to_inner_error())?;
    Ok(asset_id)
}

pub fn validate(
    connection: &StorageConnection,
    ctx_store_id: &str,
    asset_id: &str,
) -> Result<(), DeleteAssetError> {
    let asset_row = match check_asset_exists(asset_id, connection)? {
        Some(asset_row) => asset_row,
        None => return Err(DeleteAssetError::AssetDoesNotExist),
    };

    if let Some(store_id) = &asset_row.store_id
        && ctx_store_id != store_id
        && !CentralServerConfig::is_central_server()
    {
        return Err(DeleteAssetError::AssetDoesNotBelongToCurrentStore);
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use repository::{
        SyncFileReferenceRowRepository,
        mock::{MockDataInserts, mock_asset_a, mock_store_a, mock_user_account_a},
        sync_file_reference_row::SyncFileReferenceRow,
        test_db::setup_all,
    };

    use crate::service_provider::ServiceProvider;

    #[actix_rt::test]
    async fn delete_asset_soft_deletes_attached_sync_files() {
        let (_, connection, connection_manager, _) = setup_all(
            "delete_asset_soft_deletes_attached_sync_files",
            MockDataInserts::all(),
        )
        .await;
        let service_provider = ServiceProvider::new(connection_manager);
        let ctx = service_provider
            .context(mock_store_a().id, mock_user_account_a().id)
            .unwrap();

        let file_row = |id: &str, table_name: &str, record_id: &str| SyncFileReferenceRow {
            id: id.to_string(),
            table_name: table_name.to_string(),
            record_id: record_id.to_string(),
            file_name: format!("{id}.pdf"),
            ..Default::default()
        };
        let repo = SyncFileReferenceRowRepository::new(&connection);
        repo.upsert_one(&file_row("asset_file", "asset", &mock_asset_a().id))
            .unwrap();
        repo.upsert_one(&file_row("other_asset_file", "asset", "some_other_asset"))
            .unwrap();

        service_provider
            .asset_service
            .delete_asset(&ctx, mock_asset_a().id)
            .unwrap();

        let deleted = repo.find_one_by_id("asset_file").unwrap().unwrap();
        assert!(deleted.deleted_datetime.is_some());
        // files attached to other records are untouched
        let other = repo.find_one_by_id("other_asset_file").unwrap().unwrap();
        assert!(other.deleted_datetime.is_none());
    }
}
