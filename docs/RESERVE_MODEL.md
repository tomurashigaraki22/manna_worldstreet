# On-Chain Reserve Model

MNA uses an on-chain quote-token reserve, not a bank or fiat reserve in this V1.

## Fixed rate

```text
1 MNA = 2 quote tokens
```

The Devnet quote token is `USDT-DEV`. The mainnet quote token must be configured separately after verifying the official Solana USDT mint.

## Invariant

```text
Required quote reserve = circulating MNA × 2
```

At six decimals, the program uses base units:

```text
quote_base_units = mna_base_units × 2
mna_base_units = quote_base_units / 2
```

The program rejects quote amounts that cannot be divided exactly into MNA base units.

## Scenario

```text
Deposit 30 quote → mint 15 MNA
Deposit 200 quote → mint 100 MNA
Redeem 10 MNA → release 20 quote
Final supply: 105 MNA
Final reserve: 210 quote
```

The reserve is the controller’s quote vault. The controller does not accept a caller-supplied exchange rate and has no general admin withdrawal path in V1.

