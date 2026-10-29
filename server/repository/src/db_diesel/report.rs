use super::{
    ContextType, Pagination, ReportMetaDataRow, ReportRow, StorageConnection,
    form_schema_row::{self, form_schema},
    report_row::report,
};

use crate::{EqualFilter, Sort, StringFilter};
use crate::{
    FormSchema, FormSchemaRow,
    diesel_macros::{apply_equal_filter, apply_sort_no_case},
    migrations::Version,
    schema_from_row,
};

use crate::{DBType, RepositoryError, diesel_macros::apply_string_filter};

use diesel::{dsl::IntoBoxed, helper_types::LeftJoin, prelude::*};

#[derive(Debug, Clone, PartialEq)]
pub struct Report {
    pub report_row: ReportRow,
    pub argument_schema: Option<FormSchema>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReportMetaData {
    pub id: String,
    pub is_custom: bool,
    pub version: Version,
    pub code: String,
    pub is_active: bool,
}

impl ReportMetaData {
    pub fn from_domain(report: ReportMetaDataRow) -> ReportMetaData {
        ReportMetaData {
            id: report.id,
            is_custom: report.is_custom,
            version: Version::from_str(&report.version),
            code: report.code,
            is_active: report.is_active,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ReportFilter {
    pub id: Option<EqualFilter<String>>,
    pub name: Option<StringFilter>,
    pub context: Option<EqualFilter<ContextType>>,
    pub sub_context: Option<EqualFilter<String>>,
    pub code: Option<StringFilter>,
    pub is_custom: Option<bool>,
    pub is_active: Option<bool>,
}

#[derive(PartialEq, Debug)]
pub enum ReportSortField {
    Id,
    Name,
    Code,
    Version,
}

pub type ReportSort = Sort<ReportSortField>;

impl ReportFilter {
    pub fn new() -> ReportFilter {
        ReportFilter::default()
    }

    pub fn id(mut self, filter: EqualFilter<String>) -> Self {
        self.id = Some(filter);
        self
    }

    pub fn name(mut self, filter: StringFilter) -> Self {
        self.name = Some(filter);
        self
    }

    pub fn context(mut self, filter: EqualFilter<ContextType>) -> Self {
        self.context = Some(filter);
        self
    }

    pub fn code(mut self, filter: StringFilter) -> Self {
        self.code = Some(filter);
        self
    }

    pub fn is_custom(mut self, value: bool) -> Self {
        self.is_custom = Some(value);
        self
    }

    pub fn is_active(mut self, value: bool) -> Self {
        self.is_active = Some(value);
        self
    }
}

type ReportJoin = (ReportRow, Option<FormSchemaRow>);

pub struct ReportRepository<'a> {
    connection: &'a StorageConnection,
}

impl<'a> ReportRepository<'a> {
    pub fn new(connection: &'a StorageConnection) -> Self {
        ReportRepository { connection }
    }

    pub fn count(&self, filter: Option<ReportFilter>) -> Result<i64, RepositoryError> {
        let query = create_filtered_query(filter);
        Ok(query
            .count()
            .get_result(self.connection.lock().connection())?)
    }

    pub fn query_by_filter(&self, filter: ReportFilter) -> Result<Vec<Report>, RepositoryError> {
        self.query(Pagination::all(), Some(filter), None)
    }

    pub fn query(
        &self,
        pagination: Pagination,
        filter: Option<ReportFilter>,
        sort: Option<ReportSort>,
    ) -> Result<Vec<Report>, RepositoryError> {
        let mut query = create_filtered_query(filter);

        // Versions are stored as text, so sorting in SQL would be lexicographic
        // ("2.10.0" < "2.9.0"). Sort numerically in memory (and paginate after).
        if let Some(sort) = sort.as_ref().filter(|s| s.key == ReportSortField::Version) {
            let mut rows = query
                .order_by(report::id.asc())
                .load::<ReportJoin>(self.connection.lock().connection())?;
            rows.sort_by(|(a, _), (b, _)| {
                let ordering = Version::from_str(&a.version).cmp(&Version::from_str(&b.version));
                if sort.desc.unwrap_or(false) {
                    ordering.reverse()
                } else {
                    ordering
                }
            });
            return rows
                .into_iter()
                .skip(pagination.offset as usize)
                .take(pagination.limit as usize)
                .map(map_report_row_join_to_report)
                .collect();
        }

        if let Some(sort) = sort {
            match sort.key {
                ReportSortField::Id => {
                    apply_sort_no_case!(query, sort, report::id);
                }
                ReportSortField::Name => {
                    apply_sort_no_case!(query, sort, report::name);
                }
                ReportSortField::Code => {
                    apply_sort_no_case!(query, sort, report::code);
                }
                ReportSortField::Version => {}
            }
        }

        // Stable tiebreaker so paginated results don't shuffle or drop rows
        // when the primary sort column has ties (e.g. multiple versions sharing
        // the same `code`).
        let final_query = query
            .then_order_by(report::id.asc())
            .offset(pagination.offset as i64)
            .limit(pagination.limit as i64);

        let result = final_query.load::<ReportJoin>(self.connection.lock().connection())?;

        result
            .into_iter()
            .map(map_report_row_join_to_report)
            .collect::<Result<Vec<Report>, RepositoryError>>()
    }

    pub fn query_meta_data(
        &self,
        filter: Option<ReportFilter>,
        sort: Option<ReportSort>,
    ) -> Result<Vec<ReportMetaData>, RepositoryError> {
        let mut query = create_filtered_query(filter);
        let sort_version = sort.as_ref().filter(|s| s.key == ReportSortField::Version);
        if let Some(sort) = sort.as_ref().filter(|s| s.key != ReportSortField::Version) {
            match sort.key {
                ReportSortField::Id => {
                    apply_sort_no_case!(query, sort, report::id);
                }
                ReportSortField::Name => {
                    apply_sort_no_case!(query, sort, report::name);
                }
                ReportSortField::Code => {
                    apply_sort_no_case!(query, sort, report::code);
                }
                ReportSortField::Version => {}
            }
        }

        let mut result = query
            .select(ReportMetaDataRow::as_select())
            .load::<ReportMetaDataRow>(self.connection.lock().connection())?;
        if let Some(sort) = sort_version {
            // Text column, see `query`
            result.sort_by(|a, b| {
                let ordering = Version::from_str(&a.version).cmp(&Version::from_str(&b.version));
                if sort.desc.unwrap_or(false) {
                    ordering.reverse()
                } else {
                    ordering
                }
            });
        }
        Ok(result
            .into_iter()
            .map(ReportMetaData::from_domain)
            .collect())
    }
}

type BoxedStoreQuery =
    IntoBoxed<'static, LeftJoin<report::table, form_schema_row::form_schema::table>, DBType>;

fn create_filtered_query(filter: Option<ReportFilter>) -> BoxedStoreQuery {
    let mut query = report::table.left_join(form_schema::table).into_boxed();

    if let Some(f) = filter {
        let ReportFilter {
            id,
            name,
            context,
            sub_context,
            code,
            is_custom,
            is_active,
        } = f;

        apply_equal_filter!(query, id, report::id);
        apply_string_filter!(query, name, report::name);
        apply_equal_filter!(query, context, report::context);
        apply_equal_filter!(query, sub_context, report::sub_context);
        apply_string_filter!(query, code, report::code);
        if let Some(is_custom) = is_custom {
            query = query.filter(report::is_custom.eq(is_custom));
        }
        if let Some(is_active) = is_active {
            query = query.filter(report::is_active.eq(is_active));
        }
    }

    query
}

fn map_report_row_join_to_report(
    (report_row, argument_schema): ReportJoin,
) -> Result<Report, RepositoryError> {
    Ok(Report {
        report_row,
        argument_schema: argument_schema.map(schema_from_row).transpose()?,
    })
}

#[cfg(test)]
mod test {
    use crate::{
        Pagination, ReportRepository, ReportRow, ReportRowRepository, ReportSort, ReportSortField,
        mock::MockDataInserts, test_db::setup_all,
    };

    #[actix_rt::test]
    async fn report_sort_by_version_is_numeric() {
        let (_, connection, _, _) =
            setup_all("report_sort_by_version_is_numeric", MockDataInserts::none()).await;

        let versions = ["2.9.0", "2.10.0", "2.2.0", "10.0.0", "2.10.1"];
        let repo = ReportRowRepository::new(&connection);
        for (i, version) in versions.iter().enumerate() {
            repo.upsert_one(&ReportRow {
                id: format!("report{i}"),
                code: format!("code{i}"),
                version: version.to_string(),
                ..Default::default()
            })
            .unwrap();
        }

        let report_repo = ReportRepository::new(&connection);
        let sort = |desc| ReportSort {
            key: ReportSortField::Version,
            desc: Some(desc),
        };
        let query = |desc, offset, limit| {
            report_repo
                .query(Pagination { offset, limit }, None, Some(sort(desc)))
                .unwrap()
                .into_iter()
                .map(|r| r.report_row.version)
                .collect::<Vec<_>>()
        };

        assert_eq!(
            query(true, 0, 100),
            ["10.0.0", "2.10.1", "2.10.0", "2.9.0", "2.2.0"]
        );
        assert_eq!(
            query(false, 0, 100),
            ["2.2.0", "2.9.0", "2.10.0", "2.10.1", "10.0.0"]
        );
        // Pagination applies after sorting
        assert_eq!(query(true, 2, 2), ["2.10.0", "2.9.0"]);

        let meta = report_repo
            .query_meta_data(None, Some(sort(true)))
            .unwrap()
            .into_iter()
            .map(|r| r.version.to_string())
            .collect::<Vec<_>>();
        assert_eq!(meta[0], "10.0.0");
        assert_eq!(meta[4], "2.2.0");
    }
}
