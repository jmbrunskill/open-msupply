use chrono::NaiveDate;
use repository::{
    InvoiceRow, InvoiceRowRepository, InvoiceStatus, InvoiceType, KeyType, KeyValueStoreRow,
    NameRow, StoreRow,
    mock::{MockData, MockDataInserts},
};
use util::uuid::uuid;

use crate::{
    processors::ProcessorType,
    test_helpers::{ServiceTestContext, setup_all_with_data_and_service_provider},
};

/// An mSupply-synced cancellation is the original invoice plus a linked
/// `is_cancellation` reversal. The processor copies the reversal's
/// `created_datetime` onto the original, whichever side of the pair changed,
/// and is idempotent.
#[tokio::test]
async fn sets_cancelled_datetime_from_linked_cancellation() {
    let site_id = 27;
    let patient_name = NameRow {
        id: uuid(),
        ..Default::default()
    };
    let store_name = NameRow {
        id: uuid(),
        ..Default::default()
    };
    let store = StoreRow {
        id: uuid(),
        name_id: store_name.id.clone(),
        site_id,
        ..Default::default()
    };
    let site_id_settings = KeyValueStoreRow {
        id: KeyType::SettingsSyncSiteId,
        value_int: Some(site_id),
        ..Default::default()
    };

    let ServiceTestContext {
        service_provider, ..
    } = setup_all_with_data_and_service_provider(
        "sets_cancelled_datetime_from_linked_cancellation",
        MockDataInserts::none().stores().names(),
        MockData {
            names: vec![patient_name.clone(), store_name.clone()],
            stores: vec![store.clone()],
            key_value_store_rows: vec![site_id_settings],
            ..Default::default()
        },
    )
    .await;

    let ctx = service_provider.basic_context().unwrap();
    let invoice_repo = InvoiceRowRepository::new(&ctx.connection);

    let dt = |day: u32| {
        NaiveDate::from_ymd_opt(2026, 1, day)
            .unwrap()
            .and_hms_opt(9, 0, 0)
            .unwrap()
    };
    let base = InvoiceRow {
        name_id: patient_name.id.clone(),
        store_id: store.id.clone(),
        r#type: InvoiceType::Prescription,
        status: InvoiceStatus::Verified,
        created_datetime: dt(1),
        ..Default::default()
    };

    // Cancelled pair
    let cancelled = InvoiceRow {
        id: uuid(),
        ..base.clone()
    };
    let reversal = InvoiceRow {
        id: uuid(),
        linked_invoice_id: Some(cancelled.id.clone()),
        is_cancellation: true,
        created_datetime: dt(5),
        ..base.clone()
    };
    // Control: verified, never cancelled
    let untouched = InvoiceRow {
        id: uuid(),
        ..base.clone()
    };
    // Control: already has a cancelled_datetime, must not be overwritten
    let already = InvoiceRow {
        id: uuid(),
        cancelled_datetime: Some(dt(3)),
        ..base.clone()
    };
    let already_reversal = InvoiceRow {
        id: uuid(),
        linked_invoice_id: Some(already.id.clone()),
        is_cancellation: true,
        created_datetime: dt(7),
        ..base.clone()
    };
    // Control: reversal whose original has not synced yet, must not error
    let orphan_reversal = InvoiceRow {
        id: uuid(),
        linked_invoice_id: Some(uuid()),
        is_cancellation: true,
        ..base.clone()
    };
    for row in [
        &cancelled,
        &reversal,
        &untouched,
        &already,
        &already_reversal,
        &orphan_reversal,
    ] {
        invoice_repo.upsert_one(row).unwrap();
    }

    let run = || async {
        // inserting invoices directly doesn't trigger the processor
        ctx.processors_trigger
            .general_processor
            .try_send(ProcessorType::CancelledDatetime)
            .unwrap();
        ctx.processors_trigger.await_events_processed().await;
    };
    run().await;

    let get = |id: &str| invoice_repo.find_one_by_id(id).unwrap().unwrap();
    assert_eq!(get(&cancelled.id).cancelled_datetime, Some(dt(5)));
    assert_eq!(get(&untouched.id).cancelled_datetime, None);
    assert_eq!(get(&already.id).cancelled_datetime, Some(dt(3)));
    assert_eq!(get(&reversal.id).cancelled_datetime, None);

    // Re-run: idempotent
    run().await;
    assert_eq!(get(&cancelled.id).cancelled_datetime, Some(dt(5)));

    // Reversal syncs after the original was already processed: original's
    // changelog arrives first, reversal later, still resolves
    let late_original = InvoiceRow {
        id: uuid(),
        ..base.clone()
    };
    invoice_repo.upsert_one(&late_original).unwrap();
    run().await;
    assert_eq!(get(&late_original.id).cancelled_datetime, None);
    let late_reversal = InvoiceRow {
        id: uuid(),
        linked_invoice_id: Some(late_original.id.clone()),
        is_cancellation: true,
        created_datetime: dt(9),
        ..base.clone()
    };
    invoice_repo.upsert_one(&late_reversal).unwrap();
    run().await;
    assert_eq!(get(&late_original.id).cancelled_datetime, Some(dt(9)));
}
