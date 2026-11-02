import React, { useState } from 'react';
import {
  useTranslation,
  InputWithLabelRow,
  ConfirmationModalLayout,
  Grid,
  DialogButton,
  getReasonOptionTypes,
  useAuthContext,
  StoreModeNodeType,
  ReasonOptionNodeType,
} from '@openmsupply-client/common';
import {
  ReasonOptionRowFragment,
  ReasonOptionsSearchInput,
  useReasonOptions,
} from '@openmsupply-client/system';
import { StocktakeLineFragment, useStocktakeOld } from '../api';

interface ReduceLinesToZeroConfirmationModalProps {
  isOpen: boolean;
  selectedRows: StocktakeLineFragment[];
  onCancel: () => void;
  clearSelected: () => void;
}

export const ReduceLinesToZeroConfirmationModal = ({
  isOpen,
  selectedRows,
  onCancel,
  clearSelected,
}: ReduceLinesToZeroConfirmationModalProps) => {
  const t = useTranslation();
  const { store } = useAuthContext();

  const [vaccineReason, setVaccineReason] =
    useState<ReasonOptionRowFragment | null>(null);
  const [nonVaccineReason, setNonVaccineReason] =
    useState<ReasonOptionRowFragment | null>(null);

  const { onZeroQuantities, hasVaccineItems, hasNonVaccineItems } =
    useStocktakeOld.line.zeroQuantities(selectedRows);

  const { data: reasonOptions } = useReasonOptions();
  const isDispensary = store?.storeMode === StoreModeNodeType.Dispensary;
  const vaccineReasonTypes = getReasonOptionTypes({
    isInventoryReduction: true,
    isVaccine: true,
    isDispensary,
  });
  const nonVaccineReasonTypes = getReasonOptionTypes({
    isInventoryReduction: true,
    isVaccine: false,
    isDispensary,
  });
  const reasonIsRequired = (types: ReasonOptionNodeType[]) =>
    !!reasonOptions?.nodes.some(
      reasonOption =>
        types.includes(reasonOption.type) ||
        reasonOption.type === ReasonOptionNodeType.NegativeInventoryAdjustment
    );
  const vaccineReasonRequired =
    hasVaccineItems && reasonIsRequired(vaccineReasonTypes);
  const nonVaccineReasonRequired =
    hasNonVaccineItems && reasonIsRequired(nonVaccineReasonTypes);

  const reasonInput = (
    label: string,
    types: ReasonOptionNodeType[],
    value: ReasonOptionRowFragment | null,
    onChange: (reason: ReasonOptionRowFragment | null) => void
  ) => (
    <InputWithLabelRow
      label={label}
      labelWidth="200px"
      Input={
        <ReasonOptionsSearchInput
          type={types}
          fallbackType={ReasonOptionNodeType.NegativeInventoryAdjustment}
          value={value}
          onChange={onChange}
          width={250}
        />
      }
      sx={{
        '.MuiFormControl-root > .MuiInput-root, > input': {
          width: '250px',
        },
      }}
    />
  );

  return (
    <ConfirmationModalLayout
      isOpen={isOpen}
      title={t('heading.are-you-sure')}
      message={t('messages.confirm-reduce-lines-to-zero')}
      buttons={
        <>
          <Grid>
            <DialogButton variant="cancel" onClick={onCancel} />
          </Grid>
          <Grid>
            <DialogButton
              variant="ok"
              disabled={
                (vaccineReasonRequired && !vaccineReason) ||
                (nonVaccineReasonRequired && !nonVaccineReason)
              }
              onClick={async () => {
                await onZeroQuantities({ vaccineReason, nonVaccineReason });
                clearSelected();
                onCancel();
              }}
            />
          </Grid>
        </>
      }
    >
      {vaccineReasonRequired &&
        reasonInput(
          hasNonVaccineItems
            ? t('label.reason-vaccine-items')
            : t('label.reason'),
          vaccineReasonTypes,
          vaccineReason,
          setVaccineReason
        )}
      {nonVaccineReasonRequired &&
        reasonInput(
          hasVaccineItems
            ? t('label.reason-non-vaccine-items')
            : t('label.reason'),
          nonVaccineReasonTypes,
          nonVaccineReason,
          setNonVaccineReason
        )}
    </ConfirmationModalLayout>
  );
};
