import {
  AutocompleteOptionRenderer,
  FilterOptionsState,
  Formatter,
  getGenderTranslationKey,
  LocaleKey,
  RegexUtils,
  SxProps,
  TypedTFunction,
} from '@openmsupply-client/common';
import { PatientRowFragment } from './api';

export interface NameSearchProps {
  open: boolean;
  onClose: () => void;
  onChange: (name: SearchInputPatient) => void;
}

export interface SearchInputPatient {
  id: string;
  name: string;
  code: string;
  isDeceased: boolean;
  firstName?: string | null;
  lastName?: string | null;
  dateOfBirth?: string | null;
}

export interface NameSearchInputProps {
  autoFocus?: boolean;
  onChange: (name: SearchInputPatient) => void;
  width?: number;
  value: SearchInputPatient | null;
  disabled?: boolean;
  sx?: SxProps;
  NoOptionsRenderer?: AutocompleteOptionRenderer<SearchInputPatient>;
}

export interface PatientSearchModalProps {
  open: boolean;
  onClose: () => void;
  onChange: (name: SearchInputPatient) => void;
  openPatientModal: () => void;
}

export const basicFilterOptions = {
  stringify: (name: SearchInputPatient) => `${name.code} ${name.name}`,
  limit: 100,
};

export const filterByNameAndCode = (
  options: SearchInputPatient[],
  state: FilterOptionsState<SearchInputPatient>
) =>
  options.filter(option =>
    RegexUtils.matchObjectProperties(state.inputValue, option, ['name', 'code'])
  );

// Program names (document registry) of the patient's enrolments, e.g. "HIV; TB"
const programEnrolmentNames = (patient: PatientRowFragment) => {
  const enrolments = patient.programEnrolments;
  if (enrolments?.__typename !== 'ProgramEnrolmentConnector') return '';
  return enrolments.nodes
    .map(node => node.document.documentRegistry?.name)
    .filter(Boolean)
    .join('; ');
};

export const patientsToCsv = (
  invoices: PatientRowFragment[],
  t: TypedTFunction<LocaleKey>
) => {
  const fields: string[] = [
    t('label.patient-id'),
    t('label.first-name'),
    t('label.last-name'),
    t('label.gender'),
    t('label.date-of-birth'),
    t('label.next-of-kin'),
    t('label.program-enrolments'),
    t('label.deceased'),
  ];

  const data = invoices.map(node => [
    node.code,
    node.firstName,
    node.lastName,
    node.gender ? t(getGenderTranslationKey(node.gender)) : '',
    Formatter.csvDateString(node.dateOfBirth),
    node.nextOfKinName ?? '',
    programEnrolmentNames(node),
    node.isDeceased,
  ]);
  return Formatter.csv({ fields, data });
};
