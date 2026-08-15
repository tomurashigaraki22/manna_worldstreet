import { createHash } from 'node:crypto';
import { AccountMeta, PublicKey, TransactionInstruction } from '@solana/web3.js';
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  CONFIG_SEED,
  CONTROLLER_PROGRAM_ID,
  SPL_TOKEN_PROGRAM_ID,
  SYSTEM_PROGRAM_ID,
  TOKEN_2022_PROGRAM_ID,
} from './constants.js';

function discriminator(name: string): Buffer {
  return createHash('sha256').update(`global:${name}`).digest().subarray(0, 8);
}

function u64(value: bigint): Buffer {
  const buffer = Buffer.alloc(8);
  buffer.writeBigUInt64LE(value);
  return buffer;
}

export function deriveConfigPda(programId: PublicKey = CONTROLLER_PROGRAM_ID): [PublicKey, number] {
  return PublicKey.findProgramAddressSync([Buffer.from(CONFIG_SEED)], programId);
}

function meta(pubkey: PublicKey, isSigner = false, isWritable = false): AccountMeta {
  return { pubkey, isSigner, isWritable };
}

export function buildInitializeInstruction(args: {
  payer: PublicKey;
  admin: PublicKey;
  config: PublicKey;
  mnaMint: PublicKey;
  quoteMint: PublicKey;
  quoteVault: PublicKey;
  programId?: PublicKey;
  quoteTokenProgramId?: PublicKey;
}): TransactionInstruction {
  const programId = args.programId ?? CONTROLLER_PROGRAM_ID;
  const quoteTokenProgramId = args.quoteTokenProgramId ?? SPL_TOKEN_PROGRAM_ID;
  return new TransactionInstruction({
    programId,
    keys: [
      meta(args.payer, true, true),
      meta(args.admin, true),
      meta(args.config, false, true),
      meta(args.mnaMint),
      meta(args.quoteMint),
      meta(args.quoteVault, false, true),
      meta(TOKEN_2022_PROGRAM_ID),
      meta(quoteTokenProgramId),
      meta(ASSOCIATED_TOKEN_PROGRAM_ID),
      meta(SYSTEM_PROGRAM_ID),
    ],
    data: discriminator('initialize'),
  });
}

export function buildMintInstruction(args: {
  user: PublicKey;
  config: PublicKey;
  mnaMint: PublicKey;
  quoteMint: PublicKey;
  userQuoteAccount: PublicKey;
  quoteVault: PublicKey;
  userMnaAccount: PublicKey;
  quoteAmount: bigint;
  programId?: PublicKey;
  quoteTokenProgramId?: PublicKey;
}): TransactionInstruction {
  const programId = args.programId ?? CONTROLLER_PROGRAM_ID;
  const quoteTokenProgramId = args.quoteTokenProgramId ?? SPL_TOKEN_PROGRAM_ID;
  return new TransactionInstruction({
    programId,
    keys: [
      meta(args.user, true),
      meta(args.config),
      // Writable: MintTo increments the mint's supply.
      meta(args.mnaMint, false, true),
      meta(args.quoteMint),
      meta(args.userQuoteAccount, false, true),
      meta(args.quoteVault, false, true),
      meta(args.userMnaAccount, false, true),
      meta(TOKEN_2022_PROGRAM_ID),
      meta(quoteTokenProgramId),
    ],
    data: Buffer.concat([discriminator('mint_mna'), u64(args.quoteAmount)]),
  });
}

export function buildRedeemInstruction(args: {
  user: PublicKey;
  config: PublicKey;
  mnaMint: PublicKey;
  quoteMint: PublicKey;
  userMnaAccount: PublicKey;
  quoteVault: PublicKey;
  userQuoteAccount: PublicKey;
  mnaAmount: bigint;
  programId?: PublicKey;
  quoteTokenProgramId?: PublicKey;
}): TransactionInstruction {
  const programId = args.programId ?? CONTROLLER_PROGRAM_ID;
  const quoteTokenProgramId = args.quoteTokenProgramId ?? SPL_TOKEN_PROGRAM_ID;
  return new TransactionInstruction({
    programId,
    keys: [
      meta(args.user, true),
      meta(args.config),
      // Writable: Burn decrements the mint's supply.
      meta(args.mnaMint, false, true),
      meta(args.quoteMint),
      meta(args.userMnaAccount, false, true),
      meta(args.quoteVault, false, true),
      meta(args.userQuoteAccount, false, true),
      meta(TOKEN_2022_PROGRAM_ID),
      meta(quoteTokenProgramId),
    ],
    data: Buffer.concat([discriminator('redeem_mna'), u64(args.mnaAmount)]),
  });
}

export function buildSetPausedInstruction(
  admin: PublicKey,
  config: PublicKey,
  paused: boolean,
  programId: PublicKey = CONTROLLER_PROGRAM_ID,
): TransactionInstruction {
  return new TransactionInstruction({
    programId,
    keys: [meta(config, false, true), meta(admin, true)],
    data: Buffer.concat([discriminator('set_paused'), Buffer.from([paused ? 1 : 0])]),
  });
}
