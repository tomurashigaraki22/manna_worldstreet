import { describe, expect, it } from 'vitest';
import {
  baseUnitsToMna,
  baseUnitsToQuote,
  mnaToBaseUnits,
  mnaToQuote,
  mnaToQuoteBaseUnits,
  quoteToBaseUnits,
  quoteToMna,
  quoteToMnaBaseUnits,
} from '../src/amounts.js';

describe('Manna fixed-rate integer conversions', () => {
  it('converts the initial 30 quote reserve to 15 MNA', () => {
    expect(quoteToMna('30')).toBe('15');
    expect(quoteToMnaBaseUnits(quoteToBaseUnits('30'))).toBe(15_000_000n);
  });

  it('converts one quote token to half an MNA', () => {
    expect(quoteToMna('1')).toBe('0.5');
    expect(mnaToQuote('0.5')).toBe('1');
  });

  it('round-trips the issuance and redemption scenario', () => {
    const initialSupply = quoteToMnaBaseUnits(quoteToBaseUnits('30'));
    const additionalSupply = quoteToMnaBaseUnits(quoteToBaseUnits('200'));
    const redeemed = mnaToQuoteBaseUnits(mnaToBaseUnits('10'));
    expect(baseUnitsToMna(initialSupply + additionalSupply - mnaToBaseUnits('10'))).toBe('105');
    expect(baseUnitsToQuote(quoteToBaseUnits('30') + quoteToBaseUnits('200') - redeemed)).toBe('210');
  });

  it('rejects non-exact base-unit conversions', () => {
    expect(() => quoteToMnaBaseUnits(1n)).toThrow(/not exactly representable/);
  });

  it('rejects malformed and over-precise amounts', () => {
    expect(() => mnaToBaseUnits('-1')).toThrow();
    expect(() => quoteToBaseUnits('1.0000001')).toThrow();
    expect(() => mnaToBaseUnits('')).toThrow();
  });
});
