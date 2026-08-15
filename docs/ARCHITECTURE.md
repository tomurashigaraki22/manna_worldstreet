# Manna V1 Architecture

Manna uses Token-2022 for the MNA mint and a small Anchor controller for atomic on-chain minting and redemption.

The controller owns the MNA mint authority and owns a quote-token reserve vault through its config PDA. `mint_mna` transfers quote tokens from the user into the vault and mints MNA to the user. `redeem_mna` burns MNA from the user and transfers the corresponding quote amount from the vault.

The fixed rate is `2 quote tokens = 1 MNA`, with six decimals. The Devnet quote asset is a project-created `USDT-DEV` mock token. It is not official Tether USDT.

No fiat, bank, Stripe, KYC, payment webhook, frontend, backend, DEX, or off-chain reserve ledger is part of this phase.

Phase 6 adds only standard Token-2022 transfer readiness and wallet associated-token-account checks. Phase 7 documents client integration, reserve behavior, security, costs, migration, the future DEX boundary, and the future backend boundary. No DEX pool is created until liquidity funding is available.

The controller has no admin withdrawal instruction. The admin can pause and unpause mint/redeem while ordinary token transfers remain standard Token-2022 transfers.
