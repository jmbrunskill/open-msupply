import {
  useQuery,
  SortBy,
  ActivityLogSortInput,
  ActivityLogSortFieldInput,
  ActivityLogNodeType,
  isEnumValue,
  useAuthContext,
} from '@openmsupply-client/common';
import { ActivityLogRowFragment } from '../operations.generated';
import { useActivityLogGraphQL } from '../useActivityLogGraphQL';
import { ACTIVITY_LOG } from './keys';

const ACTIVITY_LOG_POLL_INTERVAL_MS = 10 * 1000;

export function useActivityLog(
  recordId: string,
  enabled: boolean = true,
  logType?: ActivityLogNodeType,
  sortBy?: SortBy<ActivityLogRowFragment>
) {
  const { activityLogApi } = useActivityLogGraphQL();
  const { storeId } = useAuthContext();

  const queryKey = [ACTIVITY_LOG, recordId, sortBy];
  const queryFn = async (): Promise<{
    nodes: ActivityLogRowFragment[];
    totalCount: number;
  }> => {
    const filter = {
      recordId: { equalTo: recordId },
      ...(logType ? { type: { equalTo: logType } } : {}),
    };

    const query = await activityLogApi.activityLogs({
      storeId,
      offset: 0,
      first: 1000,
      sort: sortBy ? getSortInput(sortBy) : undefined,
      filter,
    });
    const { nodes, totalCount } = query?.activityLogs ?? {};
    return { nodes: nodes ?? [], totalCount: totalCount ?? 0 };
  };

  const query = useQuery({
    queryKey,
    queryFn,
    enabled,
    // Logs are often a server-side side effect of other actions (e.g. status
    // changes) so can't be invalidated reliably. Poll while the log is shown.
    refetchInterval: ACTIVITY_LOG_POLL_INTERVAL_MS,
  });
  return query;
}

const getSortInput = (
  sortBy: SortBy<ActivityLogRowFragment>
): ActivityLogSortInput => ({
  desc: sortBy.isDesc,
  key: isEnumValue(ActivityLogSortFieldInput, sortBy.key)
    ? sortBy.key
    : ActivityLogSortFieldInput.ActivityLogType,
});
