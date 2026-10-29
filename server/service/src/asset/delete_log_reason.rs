use crate::{activity_log::activity_log_entry, service_provider::ServiceContext};
use repository::asset_log_reason_row::AssetLogReasonRowRepository;
use repository::{
    ActivityLogType, EqualFilter, RepositoryError, StorageConnection,
    assets::asset_log::{AssetLogFilter, AssetLogRepository},
};

use super::validate::check_asset_log_reason_exists;

#[derive(PartialEq, Debug)]
pub enum DeleteAssetLogReasonError {
    ReasonDoesNotExist,
    ReasonInUse,
    DatabaseError(RepositoryError),
}
impl From<RepositoryError> for DeleteAssetLogReasonError {
    fn from(error: RepositoryError) -> Self {
        DeleteAssetLogReasonError::DatabaseError(error)
    }
}

pub fn delete_log_reason(
    ctx: &ServiceContext,
    reason_id: String,
) -> Result<String, DeleteAssetLogReasonError> {
    ctx.connection
        .transaction_sync(|connection| {
            validate(connection, &reason_id)?;

            activity_log_entry(
                ctx,
                ActivityLogType::AssetLogReasonDeleted,
                Some(reason_id.clone()),
                None,
                None,
            )?;

            AssetLogReasonRowRepository::new(connection)
                .delete(&reason_id)
                .map_err(DeleteAssetLogReasonError::from)
        })
        .map_err(|error| error.to_inner_error())?;
    Ok(reason_id)
}

pub fn validate(
    connection: &StorageConnection,
    reason_id: &str,
) -> Result<(), DeleteAssetLogReasonError> {
    check_asset_log_reason_exists(reason_id, connection)?
        .ok_or(DeleteAssetLogReasonError::ReasonDoesNotExist)?;

    // Deleting a reason that logs refer to would blank it out of those logs
    let in_use = AssetLogRepository::new(connection).count(Some(
        AssetLogFilter::new().reason_id(EqualFilter::equal_to(reason_id.to_string())),
    ))?;
    if in_use > 0 {
        return Err(DeleteAssetLogReasonError::ReasonInUse);
    }
    Ok(())
}
