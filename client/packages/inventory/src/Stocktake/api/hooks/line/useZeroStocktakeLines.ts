import { useNotification, useTranslation } from '@openmsupply-client/common';
import { useSaveStocktakeLines } from './useStocktakeSaveLines';
import { ReasonOptionRowFragment } from '@openmsupply-client/system';
import { StocktakeLineFragment } from '../../operations.generated';

export const useZeroStocktakeLines = (
  selectedRows: StocktakeLineFragment[]
) => {
  const { saveAndMapStructuredErrors } = useSaveStocktakeLines();
  const t = useTranslation();
  const { error, success } = useNotification();

  // Vaccine and non-vaccine items take different reasons (closed/open vial
  // wastage vs negative inventory adjustment), so a mixed selection needs one
  // reason per group
  const onZeroQuantities = async ({
    vaccineReason,
    nonVaccineReason,
  }: {
    vaccineReason: ReasonOptionRowFragment | null;
    nonVaccineReason: ReasonOptionRowFragment | null;
  }) => {
    try {
      const { errorMessages } = await saveAndMapStructuredErrors(
        selectedRows.map(line => ({
          ...line,
          countedNumberOfPacks: 0,
          isUpdated: true,
          countThisLine: true,
          reasonOption: line.item.isVaccine ? vaccineReason : nonVaccineReason,
        }))
      );

      if (errorMessages) {
        errorMessages.forEach(errorMessage => error(errorMessage)());
        return;
      }

      success(t('messages.reduced-to-zero', { count: selectedRows.length }))();
    } catch (err) {
      throw err;
    }
  };

  const hasVaccineItems = selectedRows.some(row => row.item.isVaccine);
  const hasNonVaccineItems = selectedRows.some(row => !row.item.isVaccine);

  return { onZeroQuantities, hasVaccineItems, hasNonVaccineItems };
};
