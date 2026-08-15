# Costs

The project has not yet been deployed from this workspace, so exact transaction signatures and lamport totals are still pending.

## One-time Devnet setup

- Anchor controller program deployment: depends on the compiled `.so` byte size.
- MNA Token-2022 mint account: rent-exempt balance based on the mint plus metadata extensions.
- Mock quote mint: standard token mint account rent.
- Controller config PDA: rent-exempt balance based on the config account size.
- Quote reserve vault: associated token-account rent.
- User MNA and quote accounts: associated token-account rent when created.

## Per-operation fees

- `mint_mna`: one transaction with quote transfer and MNA mint CPIs;
- `redeem_mna`: one transaction with MNA burn and quote transfer CPIs;
- ordinary MNA transfer: one Token-2022 transaction;
- account creation: additional rent if the destination ATA does not exist.

The transaction fee is paid in SOL. Priority fees may be added by the sender under network conditions.

## Measurement commands

After installing Solana CLI:

```powershell
solana program show <PROGRAM_ID>
solana rent <PROGRAM_BYTES>
solana balance
```

The TypeScript scripts should also record `getMinimumBalanceForRentExemption` values for created accounts and transaction signatures.

Rent-exempt balances are account deposits, not a hardcoded USD price. Closing an eligible account can return its rent balance according to Solana account rules. DEX pools and liquidity funding are intentionally excluded from this phase.

