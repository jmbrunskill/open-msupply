import { booleanChecked, booleanValue } from '../Boolean';

describe('Boolean json form control', () => {
  it('stores and shows the value as-is when not inverted', () => {
    expect(booleanValue(true, false, false)).toBe(true);
    expect(booleanValue(false, false, false)).toBe(false);
    expect(booleanChecked(true, false, false)).toBe(true);
    expect(booleanChecked(undefined, false, false)).toBe(false);
  });

  it('inverted nullable: "on" clears the filter instead of storing false', () => {
    // e.g. isActive shown as "Include inactive items"
    expect(booleanValue(true, true, true)).toBeNull();
    expect(booleanValue(false, true, true)).toBe(true);
    expect(booleanChecked(null, true, true)).toBe(true);
    // default isActive = true => active only => "include inactive" is off
    expect(booleanChecked(true, true, true)).toBe(false);
  });

  it('inverted non-nullable: plain negation', () => {
    expect(booleanValue(true, true, false)).toBe(false);
    expect(booleanChecked(false, true, false)).toBe(true);
  });
});
