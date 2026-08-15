# On-Chain Mint and Redemption

Manna’s controller uses a fixed on-chain rate:

```text
2 USDT-DEV base-value = 1 MNA
```

Both tokens use six decimals in Devnet tests.

## Issuance

`mint_mna(quote_amount)` is permissionless. The user signs a transaction that:

1. transfers quote tokens from the user’s quote account into the controller quote vault;
2. calculates the exact MNA amount;
3. mints MNA from the controller-owned Token-2022 mint authority to the user’s MNA account.

Examples:

```text
30 USDT-DEV -> 15 MNA
200 USDT-DEV -> 100 MNA
```

Issuance does not use a backend approval or a private mint key. The controller’s PDA is the only MNA mint authority.

## Redemption

`redeem_mna(mna_amount)` is permissionless while the controller is funded and unpaused. The user signs a transaction that:

1. verifies the reserve vault has enough quote tokens;
2. burns MNA from the user’s MNA account;
3. transfers the corresponding quote amount from the reserve vault to the user.

Example:

```text
10 MNA -> 20 USDT-DEV
```

The operation is atomic. If the vault is underfunded or a CPI fails, the whole transaction fails and the MNA is not burned.

## Commands

```powershell
npm run mint-mna -- --quote 30
npm run mint-mna -- --quote 200
npm run redeem-mna -- --mna 10
npm run inspect
npm run verify -- --expected-supply 105
npm run bootstrap-devnet
```

These commands require `.env` with the Devnet RPC, keypair path, controller program ID, MNA mint, and mock quote mint.

## Important account identity rules

- MNA must be the exact Token-2022 mint configured in the controller.
- The quote mint must be the exact configured `USDT-DEV` Devnet mint.
- The quote vault is the associated token account owned by the controller config PDA.
- User token accounts must use the correct mint and token program.
- Do not substitute a token based only on the name or symbol.
