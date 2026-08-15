# Manna (MNA) V1 On-Chain Implementation Plan

## 1. Scope correction

Manna (MNA) V1 is an on-chain Solana token project. The first version should be deployable, mintable, redeemable, transferable, and usable in an on-chain liquidity pool.

This plan does **not** include:

- banks or bank-account integrations;
- Stripe or payment gateways;
- fiat APIs or payment webhooks;
- KYC, AML, identity, or compliance providers;
- a frontend, wallet UI, mobile app, or admin dashboard;
- a production off-chain reserve database;
- a fiat redemption/payout service;
- an algorithmic peg, oracle, rebasing, bond, or volatile companion token.

The word “USD” in the original specification is replaced by an on-chain quote asset: USDT on mainnet, and a clearly labeled mock quote token on Devnet because official Tether USDT is not a general Devnet testing asset.

## 2. Economic specification

The original conversion rate is preserved, with USDT replacing the off-chain dollar unit:

> 1 MNA = 2 quote tokens (USDT on mainnet; mock USDT on Devnet)

Examples:

| Quote deposited | MNA minted |
|---:|---:|
| 1 USDT | 0.5 MNA |
| 2 USDT | 1 MNA |
| 10 USDT | 5 MNA |
| 30 USDT | 15 MNA |
| 200 USDT | 100 MNA |

Redemption is the reverse:

| MNA burned | Quote released |
|---:|---:|
| 0.5 MNA | 1 USDT |
| 1 MNA | 2 USDT |
| 10 MNA | 20 USDT |

If the intended rate is instead 1 USDT = 1 MNA, change the immutable rate constants before deployment. Do not deploy with an assumed rate and change it later without a migration decision.

| Parameter | V1 value |
|---|---:|
| Name | Manna |
| Symbol | MNA |
| Network | Solana Devnet first |
| MNA token program | Token-2022 |
| Quote token on Devnet | Dedicated mock USDT-DEV token |
| Quote token on mainnet | Official Solana USDT, configured only after verification |
| MNA decimals | 6 |
| Quote decimals | 6 expected for USDT-style tokens |
| Initial quote reserve fixture | 30 quote tokens |
| Initial MNA issuance | 15 MNA |
| Supply | Elastic through the controller’s on-chain mint path |

MNA has no fiat value merely because the metadata says “USDT” or “Manna.” On Devnet, both SOL and the mock quote token are for testing.

## 3. Architecture decision

### Use Token-2022 plus a small Anchor controller

The previous plan correctly questioned whether a custom program was needed for a simple token, but the requirements have changed: V1 must support **permissionless on-chain minting and redemption against a token reserve**. That requires a small controller program.

The controller is needed to atomically:

1. receive quote tokens from a user;
2. calculate the exact MNA amount at the fixed rate;
3. mint MNA through the controller-controlled MNA mint authority;
4. receive MNA from a user for redemption;
5. burn that MNA;
6. release quote tokens from the controller reserve vault.

Token-2022 remains responsible for standard token behavior. Anchor is used only for the reserve vault, fixed-rate accounting, atomic mint/redeem flows, and pause protection. No custom DEX or payment processor is built.

### Why Token-2022 alone is not enough now

Token-2022 can create MNA, mint, burn, and transfer tokens, but it does not by itself atomically link a quote-token deposit to MNA issuance or a burn to a quote-token release. Without a controller, mint authority would have to be operated manually and redemption would not be a trust-minimized on-chain contract flow.

### DEX trading

MNA remains a normal transferable Token-2022 asset. DEX trading is separate from redemption:

- redemption uses the MNA controller and its quote reserve;
- trading uses an MNA/quote liquidity pool on a compatible Solana DEX;
- the controller does not implement swaps or price discovery;
- no frontend is required for a user to trade if they use an existing compatible wallet/DEX interface.

The implementation should include a Devnet pool-bootstrap runbook or adapter only after selecting a currently available Devnet DEX. It must not add a custom AMM.

## 4. On-chain account model

Keep the account set small:

1. **MNA mint** — Token-2022 mint with native metadata; mint authority is the controller PDA.
2. **Controller config PDA** — stores the MNA mint, quote mint, quote token program, admin, pause flag, and immutable rate parameters.
3. **Quote reserve vault** — an associated token account owned by the controller PDA; holds quote tokens deposited for minting and available for redemption.
4. **User MNA token account** — standard associated Token-2022 account.
5. **User quote token account** — standard quote-token account.
6. **Optional mock quote mint on Devnet** — created only for testing the on-chain reserve flow.

No reserve PDA ledger, payment-event account, user account, oracle account, or per-mint receipt account is required for V1.

## 5. Controller instruction surface

The Anchor program should expose only the following core instructions.

### `initialize`

Creates/configures the controller and reserve vault.

Inputs:

- MNA mint;
- quote mint;
- quote token program;
- admin authority;
- rate numerator/denominator, fixed at `1 MNA : 2 quote` for this deployment.

Checks:

- MNA mint is owned by Token-2022;
- quote mint is owned by the configured token program;
- MNA mint authority is or becomes the controller PDA;
- decimals are compatible with the conversion helpers;
- the config has not already been initialized.

### `mint_mna(quote_amount)`

Permissionless and atomic:

1. user signs;
2. controller validates the quote mint and user accounts;
3. controller transfers quote tokens from the user to the reserve vault;
4. controller computes `mna_amount = quote_amount / 2` in base-unit-safe arithmetic;
5. controller rejects non-exact or zero conversions;
6. controller mints MNA to the user’s MNA account.

Examples:

- `1,000,000` quote base units → `500,000` MNA base units;
- `2,000,000` quote base units → `1,000,000` MNA base units;
- `30,000,000` quote base units → `15,000,000` MNA base units.

No backend approval or private mint key is involved in this path.

### `redeem_mna(mna_amount)`

Permissionless and atomic, subject to reserve liquidity:

1. user signs;
2. controller verifies the MNA account and MNA mint;
3. controller transfers/burns the user’s MNA through Token-2022;
4. controller calculates `quote_amount = mna_amount × 2`;
5. controller verifies the reserve vault has enough quote tokens;
6. controller transfers quote tokens from the reserve vault to the user.

The preferred implementation burns directly from the user’s MNA account using the user signer, avoiding an unnecessary redemption holding account. If the chosen CPI flow requires an intermediate account, the program must make the transfer and burn atomic in one instruction.

If the reserve vault is empty or insufficient, redemption fails without burning the user’s MNA.

### `set_paused(paused)`

Admin-only emergency control. When paused, `mint_mna` and `redeem_mna` fail. Standard MNA transfers should remain available unless a separate, explicitly approved transfer restriction is added. There is no admin withdrawal instruction in V1; reserve tokens are not a general treasury controlled by the admin.

### Optional `set_admin`

Include only if authority migration is implemented cleanly. Otherwise keep the admin authority as a documented upgrade/migration operation and avoid adding an unnecessary instruction.

## 6. Fixed-rate and decimal safety

Use 6 decimals for MNA and the Devnet mock quote token. Use `u64` on-chain where safe and checked arithmetic for token base units. Use `bigint` in TypeScript scripts.

For equal 6-decimal tokens and a rate of 1 MNA per 2 quote:

```text
mna_base_units = quote_base_units / 2
quote_base_units = mna_base_units × 2
```

The program must reject odd quote base-unit amounts rather than round. This prevents silent value loss. It must also reject zero amounts, arithmetic overflow, wrong mint accounts, wrong token programs, and mismatched token-account authorities.

Required TypeScript helpers:

- `mnaToBaseUnits()`;
- `baseUnitsToMna()`;
- `quoteToBaseUnits()`;
- `baseUnitsToQuote()`;
- `quoteToMnaBaseUnits()`;
- `mnaToQuoteBaseUnits()`;
- `assertExactRateConversion()`.

Required Rust tests:

- 1 quote → 0.5 MNA;
- 2 quote → 1 MNA;
- 30 quote → 15 MNA;
- 200 quote → 100 MNA;
- odd quote base-unit rejection;
- zero and overflow rejection;
- redemption round-trip invariants.

## 7. Token and metadata configuration

### MNA token

- Name: `Manna`.
- Symbol: `MNA`.
- Program: Token-2022.
- Decimals: 6.
- Mint authority: controller config PDA.
- Freeze authority: disabled unless a later requirement justifies it.
- Extensions: native metadata pointer and Token-2022 metadata only.
- No transfer hook, permanent delegate, pause extension, confidential transfer, or rebasing extension.

Use the native Token-2022 metadata extension on the mint with a small public JSON URI. The metadata JSON must use `Manna`/`MNA`, not the previous name/symbol.

```json
{
  "name": "Manna",
  "symbol": "MNA",
  "description": "Manna (MNA) Devnet test token.",
  "image": "<LOGO_URL>",
  "properties": {
    "category": "fungible",
    "network": "devnet",
    "token_program": "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb"
  }
}
```

The logo URL remains a required configuration value. Do not invent one.

### Devnet quote token

Do not label an invented Devnet mint as official Tether USDT. Create a dedicated mock quote mint with:

- name: `Mock USDT Devnet`;
- symbol: `USDT-DEV`;
- 6 decimals;
- a clearly documented test-only mint authority.

Mint test quote tokens to the test wallet and use them as the on-chain reserve asset. This provides the exact same token-account and CPI behavior needed for development without pretending that an unofficial Devnet token is official Tether USDT.

### Mainnet quote token

For a future mainnet deployment, configure the verified official Solana USDT mint address as `QUOTE_MINT_ADDRESS`. The Tether-supported Solana address is documented on Tether’s official supported-protocols page; it must be re-verified at migration time and must never be reused on Devnet. [Tether supported protocols](https://tether.to/en/supported-protocols/)

## 8. Repository layout

```text
shekel_coin/
├─ programs/
│  └─ manna-controller/
│     ├─ src/lib.rs
│     └─ Cargo.toml
├─ app/
│  ├─ src/
│  │  ├─ amounts.ts
│  │  ├─ config.ts
│  │  ├─ constants.ts
│  │  ├─ token.ts
│  │  └─ controller.ts
│  ├─ scripts/
│  │  ├─ create-mock-quote.ts
│  │  ├─ create-mna.ts
│  │  ├─ initialize-controller.ts
│  │  ├─ mint-mna.ts
│  │  ├─ redeem-mna.ts
│  │  ├─ inspect.ts
│  │  ├─ verify.ts
│  │  └─ bootstrap-devnet.ts
│  └─ tests/
│     ├─ amounts.test.ts
│     └─ client.test.ts
├─ metadata/
│  ├─ manna.json
│  └─ manna.example.json
├─ tests/
│  ├─ manna-controller.ts
│  └─ invariant-tests.ts
├─ docs/
│  ├─ ARCHITECTURE.md
│  ├─ ONCHAIN_MINT_REDEEM.md
│  ├─ CLIENT_INTEGRATION.md
│  ├─ DEX_TRADING.md
│  ├─ SECURITY.md
│  ├─ MAINNET_MIGRATION.md
│  └─ COSTS.md
├─ Anchor.toml
├─ Cargo.toml
├─ package.json
├─ tsconfig.json
├─ .env.example
├─ .gitignore
├─ README.md
└─ IMPLEMENTATION_PLAN.md
```

Do not add a backend server, database, frontend, payment SDK, or compliance dependency.

## 9. Configuration

Create `.env.example`:

```dotenv
SOLANA_NETWORK=devnet
SOLANA_RPC_URL=https://api.devnet.solana.com
ANCHOR_PROVIDER_URL=https://api.devnet.solana.com
ANCHOR_WALLET=
PROGRAM_ID=
ADMIN_KEYPAIR_PATH=
FEE_PAYER_KEYPAIR_PATH=
MNA_MINT_ADDRESS=
QUOTE_MINT_ADDRESS=
QUOTE_TOKEN_PROGRAM_ID=TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA
MNA_METADATA_URI=
LOGO_URL=
MNA_DECIMALS=6
QUOTE_DECIMALS=6
RATE_MNA=1
RATE_QUOTE=2
```

Rules:

- Devnet scripts must reject mainnet RPCs.
- Mainnet addresses must never be silently substituted into Devnet commands.
- Private key files, `.env`, wallet files, and Anchor deploy keys must be ignored by Git.
- The program must validate all mint and token-program addresses supplied by clients.
- The controller must use stored config values, not caller-provided rate values.

## 10. Implementation phases

### Phase 0 — Tooling and preflight

1. Install Rust, Solana CLI, Anchor CLI, Node.js dependencies, and compatible Anchor/Solana crates.
2. Configure Solana CLI for Devnet.
3. Generate or select a Devnet-only deployer wallet outside the repository.
4. Obtain Devnet SOL from an official faucet for deployment, account rent, and transactions.
5. Pin package/crate versions.
6. Confirm the workspace builds before adding protocol logic.

Exit criteria: `anchor build`, `anchor test` against a Devnet-compatible setup, TypeScript type checking, and read-only RPC access work.

### Phase 1 — Controller program

1. Create the Anchor program and config account.
2. Define fixed rate constants and checked conversion helpers.
3. Implement `initialize`.
4. Implement `mint_mna` with quote-token CPI transfer followed by MNA mint CPI.
5. Implement `redeem_mna` with MNA burn CPI followed by quote-token vault transfer.
6. Implement pause/unpause.
7. Add account, mint, authority, token-program, and reserve-balance checks.
8. Add Rust unit and program tests for success and failure paths.

Important atomicity rule: if any CPI fails, the entire transaction must fail, so the user cannot lose MNA without receiving the quote token or deposit quote without receiving MNA.

### Phase 2 — MNA and mock quote deployment scripts

Implement TypeScript scripts that:

1. create the Token-2022 MNA mint with metadata extensions;
2. create the Devnet mock quote mint with 6 decimals;
3. initialize the controller;
4. set the MNA mint authority to the controller PDA;
5. create associated token accounts;
6. mint test mock quote tokens to the test wallet;
7. print all addresses and signatures;
8. verify that the MNA mint and quote mint are the expected accounts.

The MNA mint should not be independently mintable after controller initialization. The admin must not keep a second mint authority.

### Phase 3 — On-chain issuance

Use the mock quote token as the reserve asset:

1. mint 30 mock quote tokens to the test user;
2. call `mint_mna(30 quote)`;
3. verify the controller vault holds 30 quote tokens;
4. verify the user receives 15 MNA;
5. verify total MNA supply is 15;
6. verify the conversion invariant on-chain and in client tests.

### Phase 4 — Additional on-chain issuance

1. mint or transfer an additional 200 mock quote tokens to the test user;
2. call `mint_mna(200 quote)`;
3. verify total quote reserve is 230;
4. verify total MNA supply is 115;
5. verify the user receives an additional 100 MNA.

This is an on-chain token deposit, not an off-chain reserve event.

### Phase 5 — On-chain redemption

1. call `redeem_mna(10 MNA)` from the user;
2. atomically burn 10 MNA;
3. transfer 20 quote tokens from the reserve vault to the user;
4. verify MNA supply is 105;
5. verify reserve vault balance is 210 quote tokens;
6. verify `105 MNA × 2 quote = 210 quote`.

Also test an underfunded vault: redemption must fail and MNA supply/user balance must remain unchanged.

### Phase 6 — Trading readiness

1. Verify MNA transfers between ordinary wallets.
2. Verify wallets can create/read the correct Token-2022 associated accounts.
3. Document how to use MNA with a compatible DEX.
4. If a Devnet DEX with Token-2022 support is available, provide a separate optional pool-bootstrap script/runbook for MNA/USDT-DEV.
5. Do not make the controller depend on a specific DEX.

The controller’s fixed redemption rate is not the same thing as a DEX market price. A DEX pool may trade above or below the redemption rate depending on liquidity and configuration.

## 11. Documentation deliverables

### `README.md`

Practical commands for installing, building, testing, deploying to Devnet, creating the mock quote token, initializing the controller, minting, redeeming, inspecting, and verifying. Record final MNA mint, quote mint, controller program, config PDA, and reserve vault addresses after deployment.

### `docs/ARCHITECTURE.md`

Explain Token-2022, the Anchor controller, PDA/vault accounts, fixed rate, atomic CPI flow, authority model, and why no backend/payment system is present.

### `docs/ONCHAIN_MINT_REDEEM.md`

Document exact transaction flows, account requirements, formulas, error conditions, pause behavior, reserve sufficiency, and example instructions.

### `docs/CLIENT_INTEGRATION.md`

Document how clients identify MNA by exact mint + Token-2022 program + cluster, query balances, construct transfers, call `mint_mna`, call `redeem_mna`, confirm transactions, and handle failures. State clearly that clients must not use only the name or symbol.

### `docs/DEX_TRADING.md`

Explain that MNA is a standard transferable Token-2022 asset. Document pool requirements, quote mint identity, Token-2022 compatibility, liquidity risks, slippage, and the difference between DEX price and controller redemption rate.

### `docs/SECURITY.md`

Cover PDA authority correctness, CPI account validation, fake mint/quote token substitution, arbitrary-token attacks, vault drain risk, integer overflow, rounding, reentrancy-style CPI assumptions, pause/admin compromise, upgrade authority, reserve insolvency, and insufficient Devnet test coverage.

### `docs/MAINNET_MIGRATION.md`

Document fresh redeployment, new addresses, official USDT verification, program audit, upgrade-authority custody, admin migration, metadata hosting, DEX liquidity, and mainnet SOL/rent costs. Do not deploy mainnet as part of V1.

### `docs/COSTS.md`

Record Devnet lamports spent on program deployment, MNA mint creation, quote mint creation, token accounts, metadata space, initialization, minting, redemption, and optional liquidity-pool setup. Do not hardcode SOL/USD prices.

## 12. Security requirements

The Anchor program must:

- use PDA signer seeds only for the configured MNA mint authority and quote vault authority;
- verify every mint account against the config account;
- verify every token account’s mint and owner;
- verify the expected token program for MNA and the configured quote token program;
- use checked arithmetic for every conversion;
- reject zero, overflow, odd, or unrepresentable quote amounts;
- enforce the pause flag before mint/redeem;
- ensure the quote vault is the controller-owned reserve account;
- ensure the destination is the transaction signer’s intended token account;
- never accept a caller-supplied exchange rate;
- never expose an admin withdrawal path in V1;
- emit events for initialize, mint, redeem, pause, and admin changes;
- include a clear upgrade-authority policy.

Threats to test:

- passing a fake MNA mint;
- passing a fake quote mint;
- passing a regular SPL program where Token-2022 is required;
- passing someone else’s token account;
- using a wrong vault;
- attempting to redeem more quote than the vault holds;
- replaying or resubmitting a transaction;
- using odd base units that would round;
- minting after pause;
- redeeming after pause;
- exploiting unchecked multiplication/division;
- confusing Devnet mock USDT-DEV with mainnet USDT.

## 13. Devnet and mainnet asset separation

Devnet uses a project-created mock quote token because a random token labeled “USDT” is not an authoritative Tether asset. Its mint address must be printed in every command and documentation example.

Mainnet must use a separately verified official Solana USDT mint address. The MNA Devnet mint, mock quote mint, controller config, vault, and DEX pool cannot be reused as mainnet deployments.

## 14. Acceptance criteria

- [ ] Token name is `Manna` everywhere.
- [ ] Token symbol is `MNA` everywhere.
- [ ] No references to Shekel/SHK remain in implementation or metadata.
- [ ] No bank, Stripe, fiat API, KYC, or payment integration exists.
- [ ] MNA is a Token-2022 mint with 6 decimals.
- [ ] Anchor controller is deployed to Solana Devnet.
- [ ] Controller owns MNA mint authority.
- [ ] Devnet mock quote mint is clearly labeled and documented.
- [ ] `1 quote → 0.5 MNA` works on-chain.
- [ ] `2 quote → 1 MNA` works on-chain.
- [ ] `30 quote → 15 MNA` works on-chain.
- [ ] `200 quote → 100 MNA` works on-chain.
- [ ] `10 MNA → 20 quote` redeems atomically on-chain.
- [ ] Final scenario ends with 105 MNA supply and 210 quote tokens in reserve.
- [ ] Underfunded redemption fails without burning MNA.
- [ ] Ordinary MNA transfers work.
- [ ] Client and DEX documentation identifies the exact mint/program/cluster.
- [ ] All private keys and `.env` files are excluded from Git.
- [ ] No mainnet deployment occurs.

## 15. Current execution blockers

1. The logo URL is still missing; keep it configurable and do not fabricate it.
2. Solana CLI and Anchor are not currently installed in the workspace.
3. A Devnet deployer wallet and Devnet SOL are required for deployment.
4. The exact Devnet mock quote mint, MNA mint, controller program ID, config PDA, and reserve vault addresses will only exist after deployment.
5. If the intended fixed rate is 1 USDT = 1 MNA rather than the original 2 USDT = 1 MNA, change the rate before implementation.

## 16. Technical references

- [Solana Token Extensions](https://solana.com/solutions/token-extensions)
- [Solana Token-2022 extension guide](https://solana.com/developers/guides/token-extensions/getting-started)
- [SPL Token-2022 documentation](https://spl.solana.com/token-2022)
- [Solana Devnet faucet guidance](https://solana.com/developers/guides/getstarted/solana-token-airdrop-and-faucets)
- [Tether official supported protocols](https://tether.to/en/supported-protocols/)

