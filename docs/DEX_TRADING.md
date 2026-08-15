# DEX Trading Readiness

No DEX pool is created in the current phase. MNA is nevertheless a normal transferable Token-2022 asset, so it can be used with a compatible Solana DEX once liquidity funding and a compatible venue are available.

## What is ready

- MNA has no transfer hook, transfer fee, non-transferable extension, or confidential-transfer requirement.
- Standard Token-2022 token accounts and checked transfers are used.
- `transfer-mna.ts` verifies ordinary wallet-to-wallet movement.
- The quote asset is explicit: `USDT-DEV` on Devnet, a verified quote mint on mainnet.

## What remains out of scope

- pool creation;
- liquidity provision;
- swap routing;
- price oracles;
- market making;
- DEX-specific SDKs;
- a custom AMM.

## Future pool checklist

Before creating a pool:

1. Confirm the DEX supports Token-2022 MNA and the exact MNA extensions.
2. Confirm the quote mint and cluster.
3. Decide whether the pool is MNA/USDT or MNA/USDT-DEV.
4. Fund liquidity separately from the controller reserve vault.
5. Record the pool address and vault addresses.
6. Test deposits, swaps, withdrawals, slippage, and token-account creation.

The controller’s redemption rate is `2 quote = 1 MNA`; a DEX price is determined by pool liquidity and may differ. A DEX pool does not prove that a pool’s market price is the controller redemption rate.

