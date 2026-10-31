import { t, exportDate } from '../../../intl';
import { toCsv } from '../../../domain/reportFiles';
import { genderLabel } from '../../../domain/patient';
import type { PatientsResult } from './patients.generated';

type PatientRow = PatientsResult['patients']['nodes'][number];

// Registry names of the patient's enrolments, e.g. "HIV; TB".
const programEnrolmentTypes = (row: PatientRow): string => {
  const enrolments = row.programEnrolments;
  if (enrolments?.__typename !== 'ProgramEnrolmentConnector') return '';
  return enrolments.nodes.map(node => node.type).join('; ');
};

// The patient list → CSV (spec/patients S1 "Export CSV"). Columns match the
// list's visible fields; headers are translated, dates localised, gender via
// the fixed gender labels, booleans Yes/No. Feeds a direct .csv download or the
// server's csvToExcel conversion (domain/reportFiles).
export const patientsToCsv = (rows: PatientRow[]): string => {
  const fields = [
    t('label.patient-id'),
    t('label.patient-nuic'),
    t('label.created'),
    t('label.first-name'),
    t('label.last-name'),
    t('label.gender'),
    t('label.date-of-birth'),
    t('label.next-of-kin'),
    t('label.program-enrolments'),
    t('label.deceased'),
  ];
  const data = rows.map(row => [
    row.code,
    row.code2 ?? '',
    row.createdDatetime ? exportDate(row.createdDatetime) : '',
    row.firstName ?? '',
    row.lastName ?? '',
    row.gender ? genderLabel(row.gender) : '',
    row.dateOfBirth ? exportDate(row.dateOfBirth) : '',
    row.nextOfKinName ?? '',
    programEnrolmentTypes(row),
    row.isDeceased ? t('messages.yes') : t('messages.no'),
  ]);
  return toCsv(fields, data);
};
