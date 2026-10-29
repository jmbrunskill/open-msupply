#[cfg(test)]
mod delete_log_reason {
    use crate::{
        asset::{
            delete_log_reason::DeleteAssetLogReasonError, insert_log::InsertAssetLog,
            insert_log_reason::InsertAssetLogReason,
        },
        service_provider::ServiceProvider,
    };
    use repository::{
        asset_log_reason_row::AssetLogReasonRowRepository,
        asset_log_row::AssetLogStatus,
        mock::{MockDataInserts, mock_asset_a, mock_store_a, mock_user_account_a},
        test_db::setup_all,
    };

    #[actix_rt::test]
    async fn asset_log_reason_in_use_cannot_be_deleted() {
        let (_, connection, connection_manager, _) = setup_all(
            "asset_log_reason_in_use_cannot_be_deleted",
            MockDataInserts::none().user_accounts().assets(),
        )
        .await;

        let service_provider = ServiceProvider::new(connection_manager);
        let ctx = service_provider
            .context(mock_store_a().id, mock_user_account_a().id)
            .unwrap();
        let service = service_provider.asset_service;

        for id in ["used_reason", "unused_reason"] {
            service
                .insert_asset_log_reason(
                    &ctx,
                    InsertAssetLogReason {
                        id: id.to_string(),
                        asset_log_status: AssetLogStatus::NotFunctioning,
                        reason: id.to_string(),
                        comments_required: false,
                    },
                )
                .unwrap();
        }

        service
            .insert_asset_log(
                &ctx,
                InsertAssetLog {
                    id: "log_using_reason".to_string(),
                    asset_id: mock_asset_a().id,
                    status: Some(AssetLogStatus::NotFunctioning),
                    comment: None,
                    r#type: None,
                    reason_id: Some("used_reason".to_string()),
                    log_datetime: None,
                },
            )
            .unwrap();

        // A reason referenced by an asset log is refused, and stays
        assert_eq!(
            service.delete_log_reason(&ctx, "used_reason".to_string()),
            Err(DeleteAssetLogReasonError::ReasonInUse)
        );
        assert!(
            AssetLogReasonRowRepository::new(&connection)
                .find_one_by_id("used_reason")
                .unwrap()
                .is_some_and(|reason| reason.deleted_datetime.is_none())
        );

        // An unused reason can still be deleted
        assert_eq!(
            service.delete_log_reason(&ctx, "unused_reason".to_string()),
            Ok("unused_reason".to_string())
        );
    }
}
