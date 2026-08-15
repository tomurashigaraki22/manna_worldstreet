# MNA Security Notes

## Authority boundaries

- The MNA mint authority is the controller config PDA.
- The quote reserve vault is controlled by the same config PDA.
- The admin can pause minting and redemption.
- There is no admin reserve-withdrawal instruction in V1.
- The Anchor upgrade authority must be protected separately from the controller admin.

## Program checks

The controller must validate the MNA mint, quote mint, token programs, vault address, token-account mints, token-account owners, fixed rate, pause state, and checked arithmetic on every operation.

## Main risks

### Fake asset accounts

An attacker can create another token called Manna or MNA. Clients must compare the exact mint address, token program, and cluster.

### Authority compromise

Compromise of the Anchor upgrade authority or admin can permit malicious program upgrades or pausing. Keep deployer, admin, and upgrade keys separate before mainnet.

### Reserve insolvency

Redemption is permissionless but limited by the on-chain quote reserve. The program must check the vault balance before burning and rely on Solana transaction atomicity.

### Decimal errors

All amounts are integer base units. Odd quote base-unit amounts are rejected because they cannot represent an exact half-MNA result at the six-decimal rate.

### RPC and retry errors

Clients must confirm transactions and inspect resulting account state before retrying. A network timeout is not proof that a transaction failed.

### Devnet impersonation

`USDT-DEV` is a project-created Devnet test token, not official Tether USDT. Never reuse its address on mainnet.

## Incident response

If a key is compromised, pause mint/redeem, stop distribution, preserve signatures and account state, rotate or replace authorities where possible, and decide whether a fresh mint/controller deployment is required. Existing tokens cannot be erased by rotating an authority.

## Before mainnet

Require an independent program review, tested upgrade process, multisig or institutional key custody, monitoring for supply/reserve divergence, incident runbooks, and a legal review appropriate to the intended asset and jurisdictions.

