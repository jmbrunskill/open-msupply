use crate::sync::translations::{
    form_schema::FormSchemaTranslation, master_list::MasterListTranslation,
};

use repository::{
    DocumentRegistryCategory, DocumentRegistryRow, DocumentRegistryRowDelete, StorageConnection,
    SyncBufferRow,
};
use serde::Deserialize;
use serde_json::Value;
use util::sync_serde::empty_str_as_option_string;

use super::{FkField, PullTranslateResult, SyncTranslation};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum LegacyDocumentCategory {
    Patient,
    ProgramEnrolment,
    Encounter,
    ContactTrace,
    Custom,
    #[serde(other)]
    Others,
}

#[allow(non_snake_case)]
#[derive(Deserialize)]
struct LegacyDocumentRegistryRow {
    #[serde(rename = "ID")]
    pub id: String,
    pub category: LegacyDocumentCategory,
    pub document_type: String,
    #[serde(rename = "context_ID")]
    pub context_id: String,
    pub name: Option<String>,
    #[serde(deserialize_with = "empty_str_as_option_string")]
    #[serde(rename = "form_schema_ID")]
    pub form_schema_id: Option<String>,
    pub config: Option<Value>,
}

// Needs to be added to all_translators()
#[deny(dead_code)]
pub(crate) fn boxed() -> Box<dyn SyncTranslation> {
    Box::new(DocumentRegistryTranslation)
}

pub(super) struct DocumentRegistryTranslation;
impl SyncTranslation for DocumentRegistryTranslation {
    fn table_name(&self) -> &str {
        "om_document_registry"
    }

    fn pull_dependencies(&self) -> Vec<&str> {
        vec![
            FormSchemaTranslation.table_name(),
            // The program context is synced via the program master list
            MasterListTranslation.table_name(),
        ]
    }

    fn try_translate_from_upsert_sync_record(
        &self,
        connection: &StorageConnection,
        fk_checker: &crate::sync::translations::FkChecker,
        sync_record: &SyncBufferRow,
    ) -> Result<PullTranslateResult, anyhow::Error> {
        let LegacyDocumentRegistryRow {
            id,
            document_type,
            context_id,
            category,
            name,
            form_schema_id,
            config,
        } = sync_record.deserialize()?;

        let fk_check = fk_checker.with_table(connection, "document_registry", &id);
        let check_fk = fk_checker.with_table_required(connection, "document_registry", &id);

        let form_schema_id = fk_check(form_schema_id, "form_schema_id", FkField::FormSchema)?;

        let config_str = match config {
            Some(config) => Some(serde_json::to_string(&config)?),
            None => None,
        };

        let result = DocumentRegistryRow {
            id,
            document_type,
            context_id: check_fk(context_id, "context_id", FkField::Context)?,
            category: match category {
                LegacyDocumentCategory::Patient => DocumentRegistryCategory::Patient,
                LegacyDocumentCategory::ProgramEnrolment => {
                    DocumentRegistryCategory::ProgramEnrolment
                }
                LegacyDocumentCategory::Encounter => DocumentRegistryCategory::Encounter,
                LegacyDocumentCategory::ContactTrace => DocumentRegistryCategory::ContactTrace,
                LegacyDocumentCategory::Custom => DocumentRegistryCategory::Custom,
                LegacyDocumentCategory::Others => {
                    return Ok(PullTranslateResult::Ignored(
                        "Unsupported report type".to_string(),
                    ));
                }
            },
            name,
            form_schema_id,
            config: config_str,
        };

        Ok(PullTranslateResult::upsert(result))
    }

    fn try_translate_from_delete_sync_record(
        &self,
        _: &StorageConnection,
        sync_record: &SyncBufferRow,
    ) -> Result<PullTranslateResult, anyhow::Error> {
        Ok(PullTranslateResult::delete(DocumentRegistryRowDelete(
            sync_record.record_id.clone(),
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use repository::{
        DocumentRegistryRowRepository, SyncAction,
        mock::{MockDataInserts, document_registry_b},
        sync_buffer::SyncRecordData,
        test_db::setup_all,
    };

    use crate::sync::translations::IntegrationOperation;

    /// A delete of a document registry entry in the central server must remove
    /// it here too, otherwise deleted config lingers (and re-uploaded config
    /// ends up duplicated).
    #[actix_rt::test]
    async fn test_document_registry_pull_delete() {
        let translator = DocumentRegistryTranslation {};
        let (_, connection, _, _) =
            setup_all("test_document_registry_pull_delete", MockDataInserts::all()).await;

        let registry = document_registry_b();
        let repo = DocumentRegistryRowRepository::new(&connection);
        assert!(repo.find_one_by_id(&registry.id).unwrap().is_some());

        let sync_record = SyncBufferRow {
            table_name: translator.table_name().to_string(),
            record_id: registry.id.clone(),
            data: SyncRecordData(serde_json::json!({})),
            action: SyncAction::Delete,
            ..Default::default()
        };
        assert!(translator.should_translate_from_sync_record(&sync_record));

        let PullTranslateResult::IntegrationOperations(operations) = translator
            .try_translate_from_delete_sync_record(&connection, &sync_record)
            .unwrap()
        else {
            panic!("delete record was not translated");
        };
        for operation in operations {
            let IntegrationOperation::Delete(delete) = operation else {
                panic!("expected a delete operation");
            };
            delete
                .delete_sync(
                    &connection,
                    repository::ChangelogSyncType::SyncTypeV5V6 {
                        source_site_id: None,
                    },
                )
                .unwrap();
        }

        assert_eq!(repo.find_one_by_id(&registry.id).unwrap(), None);
    }
}
