use repository::{
    EqualFilter, RepositoryError, StorageConnection, SyncFileReferenceFilter,
    SyncFileReferenceRepository, SyncFileReferenceRowRepository,
};

/// Soft-deletes (with changelog, so the deletion syncs) every live file attached to the
/// given record. Call from the delete path of any record that has Documents attached,
/// inside the delete's transaction.
pub fn delete_attached_sync_files(
    connection: &StorageConnection,
    table_name: &str,
    record_id: &str,
) -> Result<(), RepositoryError> {
    let files = SyncFileReferenceRepository::new(connection).query_by_filter(
        SyncFileReferenceFilter::new()
            .table_name(EqualFilter::equal_to(table_name.to_string()))
            .record_id(EqualFilter::equal_to(record_id.to_string())),
    )?;
    let row_repo = SyncFileReferenceRowRepository::new(connection);

    for file in files {
        row_repo.delete(&file.sync_file_reference_row.id)?;
    }

    Ok(())
}
