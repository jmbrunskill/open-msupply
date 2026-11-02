use std::collections::HashMap;

use repository::{
    EqualFilter, InvoiceFilter, InvoiceLineFilter, InvoiceLineRepository, InvoiceLineType,
    InvoiceRepository, InvoiceRow, InvoiceStatus, InvoiceType, Name, RepositoryError,
    StorageConnection,
};

use crate::invoice::{check_invoice_exists, check_invoice_type, check_store};
use crate::validate::{CheckOtherPartyType, OtherPartyErrors, check_other_party};

use super::{InsertCustomerReturn, InsertCustomerReturnError};

pub fn validate(
    connection: &StorageConnection,
    store_id: &str,
    input: &InsertCustomerReturn,
) -> Result<Name, InsertCustomerReturnError> {
    use InsertCustomerReturnError::*;
    if (check_invoice_exists(&input.id, connection)?).is_some() {
        return Err(InvoiceAlreadyExists);
    }

    if let Some(outbound_shipment_id) = &input.outbound_shipment_id {
        let outbound_shipment = check_invoice_exists(outbound_shipment_id, connection)?
            .ok_or(OutboundShipmentDoesNotExist)?;

        if !check_store(&outbound_shipment, store_id) {
            return Err(OutboundShipmentDoesNotBelongToCurrentStore);
        }
        if !check_invoice_type(&outbound_shipment, InvoiceType::OutboundShipment) {
            return Err(OriginalInvoiceNotAnOutboundShipment);
        }
        if !check_outbound_shipment_is_returnable(&outbound_shipment) {
            return Err(CannotReturnOutboundShipment);
        }
        // Returning more than was issued is allowed (e.g. the original shipment may have been
        // edited elsewhere), but flag it
        for over in find_items_returned_more_than_issued(connection, &outbound_shipment, input)? {
            log::warn!(
                "Customer return {} returns more of item {} than was issued on outbound shipment {}: {} returned in total (including this return), {} issued (units)",
                input.id,
                over.item_id,
                outbound_shipment.id,
                over.returned_units,
                over.issued_units
            );
        }
    }

    let other_party = check_other_party(
        connection,
        store_id,
        &input.other_party_id,
        match input.is_patient_return {
            true => CheckOtherPartyType::Patient,
            false => CheckOtherPartyType::Customer,
        },
    )
    .map_err(|e| match e {
        OtherPartyErrors::OtherPartyDoesNotExist => OtherPartyDoesNotExist {},
        OtherPartyErrors::OtherPartyNotVisible => OtherPartyNotVisible,
        OtherPartyErrors::TypeMismatched => OtherPartyNotACustomer,
        OtherPartyErrors::DatabaseError(repository_error) => DatabaseError(repository_error),
    })?;

    Ok(other_party)
}

fn check_outbound_shipment_is_returnable(outbound_shipment: &InvoiceRow) -> bool {
    matches!(
        outbound_shipment.status,
        InvoiceStatus::Shipped
            | InvoiceStatus::Delivered
            | InvoiceStatus::Received
            | InvoiceStatus::Verified
    )
}

pub struct OverReturnedItem {
    pub item_id: String,
    pub issued_units: f64,
    /// Already returned against the outbound shipment, plus this return
    pub returned_units: f64,
}

/// Returns the items where (already returned against this outbound shipment + this return)
/// is more than was issued on the outbound shipment, compared in units. Items that aren't on
/// the outbound shipment are not checked.
pub(crate) fn find_items_returned_more_than_issued(
    connection: &StorageConnection,
    outbound_shipment: &InvoiceRow,
    input: &InsertCustomerReturn,
) -> Result<Vec<OverReturnedItem>, RepositoryError> {
    let line_repo = InvoiceLineRepository::new(connection);

    let mut issued_units: HashMap<String, f64> = HashMap::new();
    for line in line_repo.query_by_filter(
        InvoiceLineFilter::new()
            .invoice_id(EqualFilter::equal_to(outbound_shipment.id.clone()))
            .r#type(InvoiceLineType::StockOut.equal_to()),
    )? {
        let row = line.invoice_line_row;
        *issued_units.entry(line.item_row.id).or_default() += row.number_of_packs * row.pack_size;
    }

    let previous_returns = InvoiceRepository::new(connection).query_by_filter(
        InvoiceFilter::new()
            .r#type(InvoiceType::CustomerReturn.equal_to())
            .original_shipment_id(EqualFilter::equal_to(outbound_shipment.id.clone())),
    )?;

    let mut returned_units: HashMap<String, f64> = HashMap::new();
    if !previous_returns.is_empty() {
        let previous_return_ids = previous_returns
            .into_iter()
            .map(|invoice| invoice.invoice_row.id)
            .collect();
        for line in line_repo.query_by_filter(
            InvoiceLineFilter::new().invoice_id(EqualFilter::equal_any(previous_return_ids)),
        )? {
            let row = line.invoice_line_row;
            *returned_units.entry(line.item_row.id).or_default() +=
                row.number_of_packs * row.pack_size;
        }
    }
    for line in &input.customer_return_lines {
        *returned_units.entry(line.item_id.clone()).or_default() +=
            line.number_of_packs * line.pack_size;
    }

    Ok(returned_units
        .into_iter()
        .filter_map(|(item_id, returned_units)| {
            let issued_units = *issued_units.get(&item_id)?;
            (returned_units > issued_units + 1e-6).then_some(OverReturnedItem {
                item_id,
                issued_units,
                returned_units,
            })
        })
        .collect())
}
