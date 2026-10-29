use async_graphql::*;
use graphql_core::{
    ContextExt,
    generic_filters::EqualFilterStringInput,
    map_filter,
    pagination::PaginationInput,
    standard_graphql_error::{StandardGraphqlError, validate_auth},
};
use graphql_types::types::{ActivityLogConnector, ActivityLogNodeType};
use repository::{
    ActivityLogType,
    activity_log::{ActivityLogFilter, ActivityLogSort, ActivityLogSortField},
};
use repository::{EqualFilter, PaginationOption};
use service::{
    activity_log::get_activity_logs,
    auth::{Resource, ResourceAccessRequest},
};

#[derive(Enum, Copy, Clone, PartialEq, Eq)]
#[graphql(rename_items = "camelCase")]
#[graphql(remote = "repository::activity_log::ActivityLogSortField")]
pub enum ActivityLogSortFieldInput {
    Id,
    ActivityLogType,
    UserId,
    RecordId,
}

#[derive(InputObject)]
pub struct ActivityLogSortInput {
    /// Sort query result by `key`
    key: ActivityLogSortFieldInput,
    /// Sort query result is sorted descending or ascending (if not provided the default is
    /// ascending)
    desc: Option<bool>,
}

#[derive(InputObject, Clone)]
pub struct EqualFilterActivityLogTypeInput {
    pub equal_to: Option<ActivityLogNodeType>,
    pub equal_any: Option<Vec<ActivityLogNodeType>>,
    pub not_equal_to: Option<ActivityLogNodeType>,
    pub not_equal_all: Option<Vec<ActivityLogNodeType>>,
}

#[derive(InputObject, Clone)]
pub struct ActivityLogFilterInput {
    pub id: Option<EqualFilterStringInput>,
    pub r#type: Option<EqualFilterActivityLogTypeInput>,
    pub user_id: Option<EqualFilterStringInput>,
    pub store_id: Option<EqualFilterStringInput>,
    pub record_id: Option<EqualFilterStringInput>,
}

#[derive(Union)]
pub enum ActivityLogResponse {
    Response(ActivityLogConnector),
}

pub fn activity_logs(
    ctx: &Context<'_>,
    store_id: String,
    page: Option<PaginationInput>,
    filter: Option<ActivityLogFilterInput>,
    sort: Option<Vec<ActivityLogSortInput>>,
) -> Result<ActivityLogResponse> {
    // Logs for a specific record (the Log tab of a detail view) only need access to the store;
    // LOG_QUERY is for browsing admin-level logs across records.
    let for_specific_records = filter
        .as_ref()
        .and_then(|f| f.record_id.as_ref())
        .is_some_and(|r| r.equal_to.is_some() || r.equal_any.is_some());
    validate_auth(
        ctx,
        &ResourceAccessRequest {
            resource: if for_specific_records {
                Resource::QueryRecordLog
            } else {
                Resource::QueryLog
            },
            store_id: Some(store_id),
            require_central_standalone: false,
        },
    )?;

    let connection_manager = ctx.get_connection_manager();
    let items = get_activity_logs(
        connection_manager,
        page.map(PaginationOption::from),
        filter.map(|filter| filter.to_domain()),
        // Currently only one sort option is supported, use the first from the list.
        sort.and_then(|mut sort_list| sort_list.pop())
            .map(|sort| sort.to_domain()),
    )
    .map_err(StandardGraphqlError::from_list_error)?;

    Ok(ActivityLogResponse::Response(
        ActivityLogConnector::from_domain(items),
    ))
}

impl ActivityLogFilterInput {
    pub fn to_domain(self) -> ActivityLogFilter {
        let ActivityLogFilterInput {
            id,
            r#type,
            user_id,
            store_id,
            record_id,
        } = self;

        ActivityLogFilter {
            id: id.map(EqualFilter::from),
            r#type: r#type.map(|t| map_filter!(t, ActivityLogType::from)),
            user_id: user_id.map(EqualFilter::from),
            store_id: store_id.map(EqualFilter::from),
            record_id: record_id.map(EqualFilter::from),
        }
    }
}

impl ActivityLogSortInput {
    pub fn to_domain(&self) -> ActivityLogSort {
        ActivityLogSort {
            key: ActivityLogSortField::from(self.key),
            desc: self.desc,
        }
    }
}
