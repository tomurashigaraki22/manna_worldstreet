import { PublicKey } from '@solana/web3.js';

export const DEVNET_RPC_URL = 'https://api.devnet.solana.com';
export const TOKEN_2022_PROGRAM_ID = new PublicKey(
  'TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb',
);
export const SPL_TOKEN_PROGRAM_ID = new PublicKey(
  'TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA',
);
export const ASSOCIATED_TOKEN_PROGRAM_ID = new PublicKey(
  'ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL',
);
export const SYSTEM_PROGRAM_ID = new PublicKey(
  '11111111111111111111111111111111',
);
export const CONTROLLER_PROGRAM_ID = new PublicKey(
  'G5g6sekcjmuxNriHN2K6kca2tFVXXqL1C6yd5uLpvoTj',
);
export const CONFIG_SEED = 'config';

export const MNA_NAME = 'Manna';
export const MNA_SYMBOL = 'MNA';
export const MNA_DECIMALS = 6;
export const QUOTE_DECIMALS = 6;
export const RATE_MNA = 1n;
export const RATE_QUOTE = 2n;

export const INITIAL_QUOTE = '30';
export const INITIAL_MNA = '15';
