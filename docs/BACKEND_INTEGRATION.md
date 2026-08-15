# Backend Boundary

No backend is required for V1 minting, redemption, holding, or ordinary transfers. Users interact with the controller directly through wallets or compatible client applications.

## Future backend responsibilities

A future service may provide indexing, notifications, analytics, or an application workflow, but it must not become the source of truth for the on-chain mint/redeem rate. The program enforces the fixed rate and reserve movement on-chain.

Any future service should:

- identify MNA by exact mint, program, and cluster;
- record transaction signatures and confirmation state;
- query the controller quote vault and MNA mint supply;
- reconcile `supply × 2` against the quote vault balance;
- treat RPC timeouts as unknown until reconciled;
- never ask users for seed phrases or mint-authority keys.

No fiat processor, bank integration, KYC provider, or payment webhook is part of this contract.

