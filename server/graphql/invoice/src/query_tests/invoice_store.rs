#[cfg(test)]
mod test {
    use async_graphql::EmptyMutation;

    use graphql_core::assert_graphql_query;
    use graphql_core::test_helpers::setup_graphql_test;
    use repository::mock::{
        MockDataInserts, mock_name_store_a, mock_name_store_b, mock_outbound_shipment_a,
        mock_store_b,
    };
    use serde_json::json;

    use crate::InvoiceQueries;

    /// The `store` node of an invoice must describe the store that owns the
    /// invoice, not mix the owning store's code with the other party's name.
    #[actix_rt::test]
    async fn test_graphql_invoice_store_node_is_owning_store() {
        let (_, _, _, settings) = setup_graphql_test(
            InvoiceQueries,
            EmptyMutation,
            "test_graphql_invoice_store_node_is_owning_store",
            MockDataInserts::all(),
        )
        .await;

        let query = r#"
        query {
          invoice(id: \"outbound_shipment_a\", storeId: \"store_b\") {
            ... on InvoiceNode {
              otherPartyName
              store {
                id
                code
                storeName
              }
            }
          }
        }
        "#;

        // outbound_shipment_a: owned by store_b (name_store_b), other party is name_store_a
        let invoice = mock_outbound_shipment_a();
        assert_eq!(invoice.store_id, mock_store_b().id);

        let expected = json!({
            "invoice": {
                "otherPartyName": mock_name_store_a().name,
                "store": {
                    "id": mock_store_b().id,
                    "code": mock_store_b().code,
                    "storeName": mock_name_store_b().name,
                }
            }
        });

        assert_graphql_query!(&settings, query, &None, &expected, None);
    }
}
