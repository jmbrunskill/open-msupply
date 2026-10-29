import { describe, expect, it } from 'vitest';
import { sortDocuments, titleForFile, titleForUpload } from './helpDocumentsLogic';

describe('titleForUpload (OMS-REG-HLP-01.29/.30)', () => {
  it('trims surrounding whitespace on a valid title', () => {
    expect(titleForUpload('  Cold chain guide  ')).toEqual({
      ok: true,
      title: 'Cold chain guide',
    });
  });

  it('rejects an empty title', () => {
    expect(titleForUpload('')).toEqual({ ok: false });
  });

  it('rejects a whitespace-only title', () => {
    expect(titleForUpload('   ')).toEqual({ ok: false });
  });
});

describe('titleForFile (#12468)', () => {
  it('keeps a typed title, trimmed', () => {
    expect(titleForFile('  Guide ', 'a.pdf')).toEqual({
      ok: true,
      title: 'Guide',
    });
  });

  it('defaults an empty title to the file name without extension', () => {
    expect(titleForFile('', 'Cold chain guide.v2.pdf')).toEqual({
      ok: true,
      title: 'Cold chain guide.v2',
    });
    expect(titleForFile('   ', 'notes.pdf')).toEqual({
      ok: true,
      title: 'notes',
    });
  });

  it('is still refused when there is no title and no usable file name', () => {
    expect(titleForFile('', '')).toEqual({ ok: false });
    expect(titleForFile('', '.pdf')).toEqual({ ok: false });
  });
});

describe('sortDocuments (OMS-REG-HLP-01.33)', () => {
  const rows = [{ title: 'Banana' }, { title: 'apple' }, { title: 'Cherry' }];

  it('preserves the server order (newest-first) when unsorted', () => {
    // Same reference — no copy, no reorder.
    expect(sortDocuments(rows, undefined)).toBe(rows);
  });

  it('sorts by title ascending, locale-aware (case-insensitive order)', () => {
    expect(sortDocuments(rows, { desc: false }).map(r => r.title)).toEqual([
      'apple',
      'Banana',
      'Cherry',
    ]);
  });

  it('sorts by title descending', () => {
    expect(sortDocuments(rows, { desc: true }).map(r => r.title)).toEqual([
      'Cherry',
      'Banana',
      'apple',
    ]);
  });

  it('does not mutate the source rows', () => {
    const before = rows.map(r => r.title);
    sortDocuments(rows, { desc: true });
    expect(rows.map(r => r.title)).toEqual(before);
  });
});
