# Manna (MNA)

Manna is a Solana Devnet Token-2022 asset with a small Anchor controller for an on-chain fixed-rate quote-token reserve.

Current rate: `2` quote tokens = `1 MNA`.

This repository currently covers phases 0–7:

- toolchain/project bootstrap;
- the controller program source;
- MNA and mock quote-token deployment scripts;
- on-chain issuance, additional issuance, and redemption scripts.
- transfer readiness, wallet ATA checks, and integration/security documentation.

No DEX pool or liquidity setup is included.

## Setup

```powershell
npm install
Copy-Item .env.example .env
npm run typecheck
npm test
```

The Rust/Solana/Anchor toolchain is required for program build and deployment. See `IMPLEMENTATION_PLAN.md` for the Devnet sequence.

## Devnet sequence

1. Build/deploy the controller with Anchor.
2. Set `MNA_METADATA_URI` to a public metadata JSON URI and run `npm run create-mna`.
3. Set `MNA_MINT_ADDRESS` to the resulting mint.
4. Run `npm run create-mock-quote`.
5. Set `QUOTE_MINT_ADDRESS` to the mock quote mint.
6. Run `npm run initialize-controller`.
7. Run `npm run mint-mna -- --quote 30` to issue 15 MNA against 30 USDT-DEV.
8. Run `npm run mint-mna -- --quote 200` to issue another 100 MNA.
9. Run `npm run redeem-mna -- --mna 10` to burn 10 MNA and receive 20 USDT-DEV.
10. Run `npm run verify -- --expected-supply 105`.
11. Run `npm run transfer-mna -- --recipient <OWNER_PUBLIC_KEY> --mna 1` to smoke-test Token-2022 transfers.
12. Run `npm run check-wallet -- --owner <OWNER_PUBLIC_KEY>` to inspect the recipient’s token accounts.

For a clean, funded controller, `npm run bootstrap-devnet` runs the complete 30 → 15, +200 → +100, 10 → 20 scenario and checks the final 105 MNA / 210 USDT-DEV invariant.

The mock quote token is named `USDT-DEV`; it is not official Tether USDT.

Read [docs/ONCHAIN_MINT_REDEEM.md](docs/ONCHAIN_MINT_REDEEM.md) for account requirements and failure behavior.

Read the full documentation set:

- [Architecture](docs/ARCHITECTURE.md)
- [Client integration](docs/CLIENT_INTEGRATION.md)
- [On-chain mint and redemption](docs/ONCHAIN_MINT_REDEEM.md)
- [Reserve model](docs/RESERVE_MODEL.md)
- [Backend boundary](docs/BACKEND_INTEGRATION.md)
- [DEX trading readiness](docs/DEX_TRADING.md)
- [Security](docs/SECURITY.md)
- [Costs](docs/COSTS.md)
- [Mainnet migration](docs/MAINNET_MIGRATION.md)

## Security notes

Do not commit `.env`, wallet files, keypairs, or seed phrases. Always verify the exact mint, token program, controller program, and Devnet cluster before signing transactions.
