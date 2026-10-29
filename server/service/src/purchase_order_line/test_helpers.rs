use repository::{
    MasterListLineRow, MasterListLineRowRepository, MasterListNameJoinRepository,
    MasterListNameJoinRow, MasterListRow, MasterListRowRepository, StorageConnection,
    mock::mock_name_store_a,
};

/// Puts the items on a master list assigned to store_a, so they are visible
/// there (purchase order lines can only be added for visible items).
pub(crate) fn make_items_visible_in_store_a(connection: &StorageConnection, item_ids: &[&str]) {
    MasterListRowRepository::new(connection)
        .upsert_one(&MasterListRow {
            id: "po_visible_items".to_string(),
            name: "po_visible_items".to_string(),
            code: "po_visible_items".to_string(),
            description: "".to_string(),
            is_active: true,
            ..Default::default()
        })
        .unwrap();
    MasterListNameJoinRepository::new(connection)
        .upsert_one(&MasterListNameJoinRow {
            id: "po_visible_items".to_string(),
            master_list_id: "po_visible_items".to_string(),
            name_id: mock_name_store_a().id,
        })
        .unwrap();
    for item_id in item_ids {
        MasterListLineRowRepository::new(connection)
            .upsert_one(&MasterListLineRow {
                id: format!("po_visible_items_{item_id}"),
                item_id: item_id.to_string(),
                master_list_id: "po_visible_items".to_string(),
                ..Default::default()
            })
            .unwrap();
    }
}
