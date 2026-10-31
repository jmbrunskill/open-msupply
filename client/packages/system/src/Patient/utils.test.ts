import { patientsToCsv } from './utils';
import { PatientRowFragment } from './api';

const t = ((key: string) => key) as Parameters<typeof patientsToCsv>[1];

const patient = (overrides: Partial<PatientRowFragment>) =>
  ({
    id: 'p1',
    code: 'P-001',
    firstName: 'Ada',
    lastName: 'Lovelace',
    gender: null,
    dateOfBirth: null,
    nextOfKinName: 'Charles',
    isDeceased: false,
    programEnrolments: {
      __typename: 'ProgramEnrolmentConnector',
      totalCount: 2,
      nodes: [
        {
          programEnrolmentId: 'a',
          document: { documentRegistry: { name: 'HIV' } },
        },
        {
          programEnrolmentId: 'b',
          document: { documentRegistry: { name: 'TB' } },
        },
      ],
    },
    ...overrides,
  }) as unknown as PatientRowFragment;

describe('patientsToCsv', () => {
  it('includes Patient ID, next of kin, program enrolments and deceased', () => {
    const [header, row] = patientsToCsv([patient({})], t)
      .trim()
      .split('\n');
    expect(header).toContain('label.patient-id');
    expect(header).not.toContain('label.code');
    expect(header).toContain('label.next-of-kin');
    expect(header).toContain('label.program-enrolments');
    expect(header).toContain('label.gender');
    expect(header).toContain('label.deceased');
    expect(row).toContain('Charles');
    expect(row).toContain('HIV; TB');
    expect(row).toContain('false');
  });

  it('leaves program enrolments blank when there are none', () => {
    const csv = patientsToCsv(
      [
        patient({
          programEnrolments: {
            __typename: 'ProgramEnrolmentConnector',
            totalCount: 0,
            nodes: [],
          },
        }),
      ],
      t
    );
    expect(csv).toContain('Charles');
    expect(csv).not.toContain('HIV');
  });
});
