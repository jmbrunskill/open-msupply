import { describe, expect, it } from 'vitest';
import { patientsToCsv } from './patientsToCsv';
import type { PatientsResult } from './patients.generated';

type Row = PatientsResult['patients']['nodes'][number];

const row = (overrides: Partial<Row> = {}): Row =>
  ({
    id: 'p1',
    code: 'P-001',
    firstName: 'Ada',
    lastName: 'Lovelace',
    nextOfKinName: 'Charles',
    isDeceased: false,
    programEnrolments: {
      __typename: 'ProgramEnrolmentConnector',
      nodes: [
        { type: 'HIV', programEnrolmentId: 'a' },
        { type: 'TB', programEnrolmentId: 'b' },
      ],
    },
    ...overrides,
  }) as unknown as Row;

describe('patientsToCsv', () => {
  it('exports the program enrolments alongside next of kin', () => {
    const [header, line] = patientsToCsv([row()]).trim().split('\n');
    expect(header).toContain('label.program-enrolments');
    expect(header).toContain('label.next-of-kin');
    expect(line).toContain('Charles');
    expect(line).toContain('HIV; TB');
  });
});
