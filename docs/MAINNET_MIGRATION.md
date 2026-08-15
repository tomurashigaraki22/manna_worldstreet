# Mainnet Migration

Mainnet is a fresh deployment. Devnet addresses and the Devnet mock quote token must not be reused.

## Redeploy

1. Build and independently review the Anchor program.
2. Deploy the controller to mainnet with a protected upgrade authority.
3. Create a fresh Token-2022 MNA mint.
4. Initialize metadata with the final public URI and logo.
5. Configure the verified official Solana USDT mint as the quote mint.
6. Initialize the controller and confirm the MNA mint authority is the config PDA.
7. Verify the reserve vault and all account identities from a second independent wallet/RPC.

## Address changes

The program ID, config PDA, MNA mint, quote vault, user token accounts, metadata URI, and any future DEX pool address will be different from Devnet.

## Security changes

- move upgrade authority to reviewed multisig custody;
- separate admin, fee payer, and operational signer roles;
- protect metadata update authority;
- decide whether the controller should be immutable after audit;
- establish monitoring for total MNA supply and quote-vault balance;
- publish the canonical mint/program/cluster identity.

## Cost points

Mainnet SOL is needed for program deployment, mint/metadata account rent, controller config, reserve vault, user token accounts, and transaction fees. Measure every account size dynamically and fund only the accounts actually required.

## Product and legal readiness

Before mainnet, complete the required legal, reserve, custody, market, disclosure, and operational work for the actual MNA design. This document does not provide legal advice and does not authorize a mainnet launch.

