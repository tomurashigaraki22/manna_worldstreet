# MNA Client Integration

## Official asset identity

Clients must identify MNA by all of the following:

- cluster: `devnet` for this deployment;
- exact MNA mint address: set after deployment in `MNA_MINT_ADDRESS`;
- token program: `TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb`;
- decimals: `6`.

Never identify the token by `Manna` or `MNA` alone. Another account can use the same name and symbol.

## Wallet balances and display

Use the Token-2022 program when locating the mint and associated token account. Read the raw integer amount and format it with six decimals. Do not use floating-point arithmetic for calculations.

```text
display MNA = raw base units / 1,000,000
```

Phantom and other wallets may create or discover the associated token account when the user receives MNA. A wallet must be connected to Devnet to see the Devnet asset. The first token-account creation requires a small SOL rent deposit, normally paid by the transaction payer.

## Transfers

MNA uses standard Token-2022 transfers. This repository provides a transfer smoke-test command:

```powershell
npm run transfer-mna -- --recipient <OWNER_PUBLIC_KEY> --mna 1
npm run check-wallet -- --owner <OWNER_PUBLIC_KEY>
```

The sender must own or control the source MNA token account. The destination account must use the MNA mint and Token-2022 program.

## On-chain minting

Minting is performed through the controller, not with a client-held mint authority:

```text
mint_mna(quote_amount)
```

At the current fixed rate:

- `1` quote token → `0.5 MNA`;
- `2` quote tokens → `1 MNA`.

The user signs the quote-token transfer and receives MNA atomically. Clients must use the exact configured quote mint and controller config PDA.

## On-chain redemption

Redemption is also performed through the controller:

```text
redeem_mna(mna_amount)
```

The controller burns the user’s MNA and transfers the corresponding quote amount from its reserve vault. If the reserve is insufficient or the controller is paused, the transaction fails atomically.

Clients must display transaction confirmation state and must not assume a timeout means failure. Query the transaction and resulting token balances before retrying.

## Operations clients must never perform

- use a name or symbol as token identity;
- submit transactions against a different cluster;
- mint directly with an authority key;
- change MNA mint authority or metadata authority;
- assume Devnet tokens have monetary value;
- assume the controller’s fixed redemption rate equals a DEX market price;
- treat a failed or unconfirmed transaction as completed.

