import 'dotenv/config';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { Keypair, PublicKey } from '@solana/web3.js';
import { CONTROLLER_PROGRAM_ID, DEVNET_RPC_URL } from './constants.js';

function expandHome(value: string): string {
  return value.startsWith('~/') || value.startsWith('~\\')
    ? path.join(os.homedir(), value.slice(2))
    : value;
}

export function requiredEnv(name: string): string {
  const value = process.env[name]?.trim();
  if (!value) throw new Error(`Missing required environment variable: ${name}`);
  return value;
}

export function loadKeypair(filePath: string): Keypair {
  const resolved = expandHome(filePath);
  const secret = JSON.parse(fs.readFileSync(resolved, 'utf8')) as number[];
  return Keypair.fromSecretKey(Uint8Array.from(secret));
}

export function loadPublicKey(value: string, name: string): PublicKey {
  try {
    return new PublicKey(value);
  } catch {
    throw new Error(`${name} is not a valid Solana public key`);
  }
}

export function loadConfig() {
  const network = process.env.SOLANA_NETWORK ?? 'devnet';
  const rpcUrl = process.env.SOLANA_RPC_URL ?? DEVNET_RPC_URL;
  if (network !== 'devnet') throw new Error('This V1 implementation only permits SOLANA_NETWORK=devnet');
  if (!rpcUrl.includes('devnet')) throw new Error('Devnet scripts require a Devnet RPC URL');
  return {
    network,
    rpcUrl,
    programId: loadPublicKey(process.env.PROGRAM_ID ?? CONTROLLER_PROGRAM_ID.toBase58(), 'PROGRAM_ID'),
    feePayer: process.env.FEE_PAYER_KEYPAIR_PATH ? loadKeypair(process.env.FEE_PAYER_KEYPAIR_PATH) : undefined,
    admin: process.env.ADMIN_KEYPAIR_PATH ? loadKeypair(process.env.ADMIN_KEYPAIR_PATH) : undefined,
    mnaMint: process.env.MNA_MINT_ADDRESS ? loadPublicKey(process.env.MNA_MINT_ADDRESS, 'MNA_MINT_ADDRESS') : undefined,
    quoteMint: process.env.QUOTE_MINT_ADDRESS ? loadPublicKey(process.env.QUOTE_MINT_ADDRESS, 'QUOTE_MINT_ADDRESS') : undefined,
  };
}
