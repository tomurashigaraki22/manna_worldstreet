# Devnet deployment status

Generated: 2026-08-15 UTC

## Public addresses

- Fee payer: `CrN2YAo3Mwk5trWZwud2M6bBRjBabcsewJYNUBm9UBLy`
- Admin: `7XTFdsNkAzz9fQiZ5SKtCD6fZVj8XHxLFZSJ9EokvKyu`
- Controller program: `7QZLtciaBCeLy5uyooZERzBNbPLNuQ6Ppc4iA2kqEsyR`

Keypair files are stored outside the repository under `/home/ubuntu/.config/solana/manna-worldstreet/`, with directory mode 700 and file mode 600. `.env` contains their paths and public addresses, not raw private-key material.

## Compiled program

- Artifact: `target/deploy/manna_controller.so`
- Size: 308,872 bytes
- SHA-256: `59d48a2dcf98f70f9fb01adf453dbe8d9f128caea1c7953c79d6d2c5d3e2dc09`
- Toolchain: Anchor 0.32.1, Solana 2.3.0, platform-tools 1.48
- Docker image: `solanafoundation/anchor:v0.32.1`

## Devnet rent

- ProgramData (308,917 bytes): 2.15095320 SOL
- Program account (36 bytes): 0.00114144 SOL
- Final program rent: 2.15209464 SOL
- Temporary upload buffer (308,909 bytes): 2.15089752 SOL
- MNA Token-2022 mint (420 bytes for the proposed GitHub metadata URI): 0.00381408 SOL
- Mock quote mint (82 bytes): 0.00146160 SOL
- Controller config (188 bytes): 0.00219936 SOL
- Three token accounts (165 bytes each): 0.00611784 SOL
- Non-program account rent subtotal: 0.01359288 SOL
- Final locked rent total: 2.16568752 SOL, excluding transaction fees
- Suggested fee-payer balance before deployment: at least 4.4 devnet SOL, allowing for the temporary upload buffer and transaction fees

## Funding and blockers

The public RPC faucet request was attempted and rate-limited. Obtain free devnet SOL from https://faucet.solana.com/ and send it to the fee-payer address above. Devnet SOL has no monetary value.

The documented `30 quote -> 15 MNA` flow uses 30 mock USDT-DEV created by the repository; it is not a deposit of $30 or real USDT. Do not send real USDT to any address for the devnet scenario.

Deployment has not been broadcast because the fee payer is unfunded. The configured logo is https://watchup.site/manna.png and the metadata URI points to the public GitHub metadata file.
