import React from 'react';
import { act, fireEvent, render } from '@testing-library/react';
import { TestingProvider } from '@openmsupply-client/common';
import { NumberInputCell } from './NumberInputCell';

const stubCell = (value: number) =>
  ({
    getValue: () => value,
    column: {},
    row: {},
  }) as unknown as Parameters<typeof NumberInputCell>[0]['cell'];

describe('NumberInputCell — updateOnBlur', () => {
  beforeEach(() => jest.useFakeTimers());
  afterEach(() => jest.useRealTimers());

  it('does not call updateFn while typing, only on blur', () => {
    const updateFn = jest.fn();
    const { getByRole } = render(
      <TestingProvider>
        <NumberInputCell cell={stubCell(0)} updateFn={updateFn} updateOnBlur />
      </TestingProvider>
    );
    const input = getByRole('textbox');

    fireEvent.change(input, { target: { value: '3' } });
    fireEvent.change(input, { target: { value: '35' } });
    act(() => {
      jest.advanceTimersByTime(2000);
    });
    expect(updateFn).not.toHaveBeenCalled();

    fireEvent.change(input, { target: { value: '35960' } });
    fireEvent.blur(input);
    expect(updateFn).toHaveBeenCalledTimes(1);
    expect(updateFn).toHaveBeenCalledWith(35960);
  });

  it('does not call updateFn on blur if nothing was typed', () => {
    const updateFn = jest.fn();
    const { getByRole } = render(
      <TestingProvider>
        <NumberInputCell cell={stubCell(5)} updateFn={updateFn} updateOnBlur />
      </TestingProvider>
    );
    fireEvent.blur(getByRole('textbox'));
    expect(updateFn).not.toHaveBeenCalled();
  });
});
