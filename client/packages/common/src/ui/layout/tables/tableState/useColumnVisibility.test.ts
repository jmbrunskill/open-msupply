/* eslint-disable @typescript-eslint/no-require-imports */
// The mocks need registering before the hook (and everything it pulls in) loads
let mockGlobalDefaults:
  | { columnVisibility?: Record<string, boolean> }
  | undefined;

jest.mock('./useGlobalTableConfig', () => ({
  useGlobalTableDefaults: () => mockGlobalDefaults,
}));
jest.mock('@common/hooks', () => ({
  useSimplifiedTabletUI: () => false,
}));

jest.resetModules(); // jest-setup may already have loaded the real modules
const { renderHook } = require('@testing-library/react');
const { useColumnVisibility } = require('./useColumnVisibility');

const columns = [
  { id: 'a', header: 'A' },
  { id: 'b', header: 'B', defaultHidden: true },
];

describe('useColumnVisibility', () => {
  beforeEach(() => {
    localStorage.clear();
    mockGlobalDefaults = undefined;
  });

  it('applies the global default when no local state is saved', () => {
    mockGlobalDefaults = { columnVisibility: { a: false } };
    const { result } = renderHook(() => useColumnVisibility('table', columns));
    expect(result.current.state).toEqual({ a: false });
  });

  it('applies the global default when it loads after mount', () => {
    const { result, rerender } = renderHook(() =>
      useColumnVisibility('table', columns)
    );
    expect(result.current.state).toEqual({ b: false });

    mockGlobalDefaults = { columnVisibility: { a: false } };
    rerender();
    expect(result.current.state).toEqual({ a: false });
  });

  it('prefers locally saved state over the global default', () => {
    localStorage.setItem(
      '@openmsupply-client/tables/table',
      JSON.stringify({ columnVisibility: { b: true } })
    );
    mockGlobalDefaults = { columnVisibility: { a: false } };
    const { result } = renderHook(() => useColumnVisibility('table', columns));
    expect(result.current.state).toEqual({ b: true });
  });
});
