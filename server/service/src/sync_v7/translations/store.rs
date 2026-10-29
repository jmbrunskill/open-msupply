// V7 store translation: deserialise the incoming `StoreRow`, then — only on
// the remote — emit a local sync_request when the row reassigns the store to
// this site. The sync_request is consumed by the auxiliary sync_request_runner
// on the next tick to pull all data for the newly-owned store.

use repository::{
    ChangeLogInsertRow, ChangelogFilter, ChangelogRow, Description, KeyType,
    KeyValueStoreRepository, StorageConnection, StoreLogoRow, StoreRow, StoreRowRepository,
    SyncRequestFilter, SyncRequestRow, Upsert,
    syncv7::{SyncError, SyncRecordSerializeError},
};

use crate::sync_v7::{
    serde::{DeserializeResult, serialize},
    sync::SyncRecordV7,
    translations::SendContext,
};

// `logo` is not a field of the lean `StoreRow` (it is a large base64 image kept
// off every store join), so it has to be carried on the wire next to the row.
const LOGO_KEY: &str = "logo";

/// Outgoing store: the lean row plus its logo, when it has one, so a store logo
/// added or changed on central reaches the site the store lives on.
pub(crate) fn translate_outgoing_store(
    context: &SendContext,
    changelog: ChangelogRow,
    store: StoreRow,
) -> Result<SyncRecordV7, SyncError> {
    let logo = StoreRowRepository::new(context.connection)
        .find_logo_by_id(&store.id)?
        .and_then(|row| row.logo);
    let mut data = serialize(&repository::Row::Store(store))?;
    if let (Some(logo), Some(object)) = (logo, data.as_object_mut()) {
        object.insert(LOGO_KEY.to_string(), serde_json::Value::String(logo));
    }
    Ok(SyncRecordV7::upsert(changelog, data))
}

pub fn translate_store(
    connection: &StorageConnection,
    changelog_insert: ChangeLogInsertRow,
    data: &serde_json::Value,
) -> DeserializeResult {
    let store: StoreRow = serde_json::from_value(data.clone())
        .map_err(|e| SyncRecordSerializeError::SerdeError(e.to_string()))?;

    let sync_request = sync_request_for_site_change(connection, &store)?;
    let logo = data
        .get(LOGO_KEY)
        .and_then(|logo| logo.as_str())
        .map(str::to_string);
    let logo_row = logo.map(|logo| StoreLogoRow {
        id: store.id.clone(),
        logo: Some(logo),
    });

    let mut out = vec![(Box::new(store) as Box<dyn Upsert>, changelog_insert)];
    if let Some(logo_row) = logo_row {
        // Must follow the store row: the logo is an UPDATE of the existing row.
        // The logo upsert ignores the changelog.
        out.push((
            Box::new(logo_row) as Box<dyn Upsert>,
            ChangeLogInsertRow::default(),
        ));
    }
    if let Some(sync_request) = sync_request {
        // Upsert from sync_request will ignore changelog, for now just provide default value
        // in the future probably need upsert with changelog on Upsert trait, and optional ChangelogInsertRow here
        out.push((
            Box::new(sync_request) as Box<dyn Upsert>,
            ChangeLogInsertRow::default(),
        ));
    }
    Ok(out)
}

/// Returns Some(sync_request) iff:
/// - this site already has the store row locally with a different `site_id`
/// - the new `site_id` is this site's id
///
/// Pre-existing row + site change is what distinguishes a transfer from the
/// initial arrival of a store; first sync wouldn't have a local row yet.
fn sync_request_for_site_change(
    connection: &StorageConnection,
    new_store: &StoreRow,
) -> Result<Option<SyncRequestRow>, SyncRecordSerializeError> {
    let Some(this_site_id) =
        KeyValueStoreRepository::new(connection).get_i32(KeyType::SettingsSyncSiteId)?
    else {
        // Site id not configured yet (early initialisation) — skip.
        return Ok(None);
    };

    if new_store.site_id != this_site_id {
        return Ok(None);
    }

    let Some(existing) = StoreRowRepository::new(connection).find_one_by_id(&new_store.id)? else {
        return Ok(None);
    };
    if existing.site_id == new_store.site_id {
        return Ok(None);
    }

    let store_name = new_store.code.clone();
    let pull_filter = ChangelogFilter::data_for_store(&new_store.id);

    Ok(Some(SyncRequestRow {
        id: util::uuid::uuid(),
        reference_id: None,
        description: Description::AllStoreData { store_name },
        pull_filter: Some(SyncRequestFilter(pull_filter)),
        push_filter: None,
        created_datetime: chrono::Utc::now().naive_utc(),
        finished_datetime: None,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use repository::{
        ChangelogSyncType, mock::MockDataInserts, mock::mock_store_a, test_db::setup_all,
    };

    #[actix_rt::test]
    async fn v7_store_logo_round_trips() {
        let (_, central, _, _) = setup_all("v7_store_logo_central", MockDataInserts::all()).await;
        let (_, remote, _, _) = setup_all("v7_store_logo_remote", MockDataInserts::none()).await;

        let store = mock_store_a();
        let repo = StoreRowRepository::new(&central);
        repo.update_logo(&store.id, Some("base64logo")).unwrap();
        let store_row = repo.find_one_by_id(&store.id).unwrap().unwrap();

        let context = SendContext {
            connection: &central,
            receiver_version: None,
        };
        let record = translate_outgoing_store(
            &context,
            ChangelogRow {
                record_id: store.id.clone(),
                ..Default::default()
            },
            store_row,
        )
        .unwrap();
        assert_eq!(record.data["logo"], "base64logo");

        // Plain serialize (the old wire format) has no logo
        let plain = serialize(&repository::Row::Store(
            StoreRowRepository::new(&central)
                .find_one_by_id(&store.id)
                .unwrap()
                .unwrap(),
        ))
        .unwrap();
        assert!(plain.get("logo").is_none());

        // Integrate on the remote: needs the name the store links to
        repository::NameRowRepository::new(&remote)
            .upsert_one(&repository::mock::mock_name_store_a())
            .unwrap();
        for (upsert, changelog_row) in
            translate_store(&remote, ChangeLogInsertRow::default(), &record.data).unwrap()
        {
            upsert
                .upsert_sync(&remote, ChangelogSyncType::SyncTypeV7 { changelog_row })
                .unwrap();
        }
        let logo = StoreRowRepository::new(&remote)
            .find_logo_by_id(&store.id)
            .unwrap()
            .unwrap();
        assert_eq!(logo.logo.as_deref(), Some("base64logo"));
    }
}
