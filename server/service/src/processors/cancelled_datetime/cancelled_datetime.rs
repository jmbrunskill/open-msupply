use async_trait::async_trait;
use repository::{
    ChangelogCondition, ChangelogRow, ChangelogTableName, EqualFilter, FilterBuilder,
    InvoiceFilter, InvoiceRepository, InvoiceRow, InvoiceRowRepository, KeyType,
};

use crate::{
    cursor_controller::CursorType,
    processors::general_processor::{Processor, ProcessorError},
    service_provider::{ServiceContext, ServiceProvider},
    sync::ActiveStoresOnSite,
};

const DESCRIPTION: &str =
    "Set cancelled_datetime on invoices that have a linked cancellation invoice";

pub(crate) struct CancelledDatetimeProcessor;

#[async_trait]
impl Processor for CancelledDatetimeProcessor {
    fn get_description(&self) -> String {
        DESCRIPTION.to_string()
    }

    /// mSupply represents a cancelled prescription as the original invoice plus a
    /// linked reversal invoice (`is_cancellation`), and never populates
    /// `cancelled_datetime` on the original. OMS hides the reversal and shows the
    /// original as cancelled, so copy the reversal's `created_datetime` onto the
    /// original. The pair can sync in either order, so a changelog for either side
    /// resolves the pair. Idempotent: an original that already has a
    /// `cancelled_datetime` is left alone.
    async fn try_process_record(
        &self,
        ctx: &ServiceContext,
        _service_provider: &ServiceProvider,
        changelog: &ChangelogRow,
    ) -> Result<Option<String>, ProcessorError> {
        let invoice_repo = InvoiceRowRepository::new(&ctx.connection);

        // A delete changelog can point at a gone invoice — nothing to do
        let Some(invoice) = invoice_repo.find_one_by_id(&changelog.record_id)? else {
            return Ok(None);
        };

        let (original, cancellation) = if invoice.is_cancellation {
            let Some(original_id) = invoice.linked_invoice_id.clone() else {
                return Ok(None);
            };
            // The original may not have synced yet; its own changelog will resolve the pair
            let Some(original) = invoice_repo.find_one_by_id(&original_id)? else {
                return Ok(None);
            };
            (original, invoice)
        } else {
            let Some(cancellation) = InvoiceRepository::new(&ctx.connection)
                .query_by_filter(
                    InvoiceFilter::new()
                        .linked_invoice_id(EqualFilter::equal_to(invoice.id.to_string()))
                        .is_cancellation(true),
                )?
                .into_iter()
                .map(|invoice| invoice.invoice_row)
                // Earliest reversal wins, deterministic if there is ever more than one
                .min_by_key(|invoice| invoice.created_datetime)
            else {
                return Ok(None);
            };
            (invoice, cancellation)
        };

        // Only an original can be cancelled; skip a cancellation linking to a cancellation
        if original.is_cancellation || original.cancelled_datetime.is_some() {
            return Ok(None);
        }

        let updated = InvoiceRow {
            cancelled_datetime: Some(cancellation.created_datetime),
            ..original
        };
        invoice_repo.upsert_one(&updated)?;

        Ok(Some(format!(
            "invoice ({}) cancelled_datetime set from cancellation invoice ({})",
            updated.id, cancellation.id
        )))
    }

    async fn changelogs_filter(
        &self,
        ctx: &ServiceContext,
    ) -> Result<ChangelogCondition::Inner, ProcessorError> {
        let active_stores = ActiveStoresOnSite::get(&ctx.connection)
            .map_err(ProcessorError::GetActiveStoresOnSiteError)?;

        Ok(ChangelogCondition::And(vec![
            ChangelogCondition::table_name::equal(ChangelogTableName::Invoice),
            ChangelogCondition::store_id::any(active_stores.store_ids()),
        ]))
    }

    fn cursor_type(&self) -> CursorType {
        CursorType::Standard(KeyType::CancelledDatetimeProcessorCursor)
    }
}
