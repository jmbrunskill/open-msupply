import React from 'react';
import { rankWith, isBooleanControl, ControlProps } from '@jsonforms/core';
import { withJsonFormsControlProps } from '@jsonforms/react';
import {
  Switch,
  DetailInputWithLabelRow,
  useTranslation,
  LocaleKey,
} from '@openmsupply-client/common';
import { FORM_LABEL_WIDTH, DefaultFormRowSx } from '../styleConstants';

export const booleanTester = rankWith(4, isBooleanControl);

// Whether the schema allows null, e.g. `"type": ["boolean", "null"]`
const isNullable = (schema: ControlProps['schema']) =>
  Array.isArray(schema?.type) && schema.type.includes('null');

/**
 * Whether the switch is on for the stored value. With `options.invert` the
 * switch shows the opposite of the stored value (e.g. `isActive` shown as
 * "Include inactive"); for a nullable field "on" means no filter (null).
 */
export const booleanChecked = (
  data: unknown,
  invert: boolean,
  nullable: boolean
): boolean => {
  if (!invert) return !!data;
  return nullable ? data === null : data !== true;
};

/**
 * The value to store for the switch state. Turning an inverted nullable field
 * on clears it (null) rather than storing false, which would select only the
 * records the field excludes.
 */
export const booleanValue = (
  checked: boolean,
  invert: boolean,
  nullable: boolean
): boolean | null => {
  if (!invert) return checked;
  return nullable ? (checked ? null : true) : !checked;
};

const UIComponent = (props: ControlProps) => {
  const { data, handleChange, label, path, enabled, uischema, schema } = props;
  const invert = !!uischema.options?.['invert'];
  const nullable = isNullable(schema);
  const t = useTranslation();

  if (!props.visible) {
    return null;
  }

  return (
    <DetailInputWithLabelRow
      // On most json form elements, we want the label at the start as
      // the input may span multiple lines. However, for a boolean field,
      // the input is a single line, so we want the label centred.
      sx={{ ...DefaultFormRowSx, alignItems: 'center !important' }}
      label={t(label as LocaleKey)}
      labelWidthPercentage={FORM_LABEL_WIDTH}
      inputAlignment={'start'}
      Input={
        <Switch
          labelPlacement="end"
          onChange={(_, checked) => {
            handleChange(path, booleanValue(checked, invert, nullable));
          }}
          value={data ?? ''}
          checked={booleanChecked(data, invert, nullable)}
          disabled={!enabled}
        />
      }
    />
  );
};

export const BooleanField = withJsonFormsControlProps(UIComponent);
