import {
  MNA_DECIMALS,
  QUOTE_DECIMALS,
  RATE_MNA,
  RATE_QUOTE,
} from './constants.js';

function parseUnits(value: string, decimals: number, label: string): bigint {
  if (!/^(0|[1-9]\d*)(\.\d+)?$/.test(value)) {
    throw new Error(`${label} must be a non-negative decimal string`);
  }
  const [whole, fraction = ''] = value.split('.');
  if (fraction.length > decimals) {
    throw new Error(`${label} has more than ${decimals} decimal places`);
  }
  const padded = fraction.padEnd(decimals, '0');
  return BigInt(whole) * 10n ** BigInt(decimals) + BigInt(padded || '0');
}

function formatUnits(value: bigint, decimals: number): string {
  if (value < 0n) throw new Error('Amount cannot be negative');
  const scale = 10n ** BigInt(decimals);
  const whole = value / scale;
  const fraction = (value % scale).toString().padStart(decimals, '0').replace(/0+$/, '');
  return fraction ? `${whole}.${fraction}` : whole.toString();
}

export function mnaToBaseUnits(value: string): bigint {
  return parseUnits(value, MNA_DECIMALS, 'MNA amount');
}

export function baseUnitsToMna(value: bigint): string {
  return formatUnits(value, MNA_DECIMALS);
}

export function quoteToBaseUnits(value: string): bigint {
  return parseUnits(value, QUOTE_DECIMALS, 'quote amount');
}

export function baseUnitsToQuote(value: bigint): string {
  return formatUnits(value, QUOTE_DECIMALS);
}

export function quoteToMnaBaseUnits(quoteBaseUnits: bigint): bigint {
  if (quoteBaseUnits <= 0n) throw new Error('Quote amount must be greater than zero');
  const numerator = quoteBaseUnits * RATE_MNA;
  if (numerator % RATE_QUOTE !== 0n) {
    throw new Error('Quote amount is not exactly representable at the fixed rate');
  }
  return numerator / RATE_QUOTE;
}

export function mnaToQuoteBaseUnits(mnaBaseUnits: bigint): bigint {
  if (mnaBaseUnits <= 0n) throw new Error('MNA amount must be greater than zero');
  return mnaBaseUnits * RATE_QUOTE / RATE_MNA;
}

export function quoteToMna(value: string): string {
  return baseUnitsToMna(quoteToMnaBaseUnits(quoteToBaseUnits(value)));
}

export function mnaToQuote(value: string): string {
  return baseUnitsToQuote(mnaToQuoteBaseUnits(mnaToBaseUnits(value)));
}
