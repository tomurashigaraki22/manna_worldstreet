import {
  Connection,
  Keypair,
  PublicKey,
  Transaction,
  sendAndConfirmTransaction,
} from '@solana/web3.js';
import { createTransferCheckedInstruction, getAssociatedTokenAddressSync, getMint } from '@solana/spl-token';
import { loadConfig } from './config.js';
import {
  buildMintInstruction,
  buildRedeemInstruction,
  deriveConfigPda,
} from './controller.js';
import { SPL_TOKEN_PROGRAM_ID, TOKEN_2022_PROGRAM_ID } from './constants.js';
import {
  baseUnitsToMna,
  baseUnitsToQuote,
  mnaToBaseUnits,
  mnaToQuoteBaseUnits,
  quoteToBaseUnits,
  quoteToMnaBaseUnits,
} from './amounts.js';
import { ensureAta, readTokenAccount } from './token.js';

export type OperationContext = {
  config: ReturnType<typeof loadConfig>;
  connection: Connection;
  user: Keypair;
  mnaMint: PublicKey;
  quoteMint: PublicKey;
  configPda: PublicKey;
  quoteVault: PublicKey;
  userMnaAccount: PublicKey;
  userQuoteAccount: PublicKey;
};

export async function getOperationContext(): Promise<OperationContext> {
  const config = loadConfig();
  const user = config.admin ?? config.feePayer;
  if (!user) throw new Error('Set ADMIN_KEYPAIR_PATH or FEE_PAYER_KEYPAIR_PATH');
  if (!config.mnaMint || !config.quoteMint) {
    throw new Error('Set MNA_MINT_ADDRESS and QUOTE_MINT_ADDRESS');
  }
  const connection = new Connection(config.rpcUrl, 'confirmed');
  const [configPda] = deriveConfigPda(config.programId);
  const quoteVault = getAssociatedTokenAddressSync(
    config.quoteMint,
    configPda,
    true,
    SPL_TOKEN_PROGRAM_ID,
  );
  const userMnaAccount = await ensureAta({
    connection,
    payer: user,
    mint: config.mnaMint,
    owner: user.publicKey,
    tokenProgramId: TOKEN_2022_PROGRAM_ID,
  });
  const userQuoteAccount = await ensureAta({
    connection,
    payer: user,
    mint: config.quoteMint,
    owner: user.publicKey,
    tokenProgramId: SPL_TOKEN_PROGRAM_ID,
  });
  return {
    config,
    connection,
    user,
    mnaMint: config.mnaMint,
    quoteMint: config.quoteMint,
    configPda,
    quoteVault,
    userMnaAccount,
    userQuoteAccount,
  };
}

export async function mintMna(ctx: OperationContext, quoteAmount: string) {
  const quoteBaseUnits = quoteToBaseUnits(quoteAmount);
  const expectedMnaBaseUnits = quoteToMnaBaseUnits(quoteBaseUnits);
  const instruction = buildMintInstruction({
    user: ctx.user.publicKey,
    config: ctx.configPda,
    mnaMint: ctx.mnaMint,
    quoteMint: ctx.quoteMint,
    userQuoteAccount: ctx.userQuoteAccount,
    quoteVault: ctx.quoteVault,
    userMnaAccount: ctx.userMnaAccount,
    quoteAmount: quoteBaseUnits,
    programId: ctx.config.programId,
    quoteTokenProgramId: SPL_TOKEN_PROGRAM_ID,
  });
  const signature = await sendAndConfirmTransaction(
    ctx.connection,
    new Transaction().add(instruction),
    [ctx.user],
  );
  return {
    signature,
    quoteAmount,
    quoteBaseUnits,
    expectedMna: baseUnitsToMna(expectedMnaBaseUnits),
    expectedMnaBaseUnits,
  };
}

export async function redeemMna(ctx: OperationContext, mnaAmount: string) {
  const mnaBaseUnits = mnaToBaseUnits(mnaAmount);
  const expectedQuoteBaseUnits = mnaToQuoteBaseUnits(mnaBaseUnits);
  const instruction = buildRedeemInstruction({
    user: ctx.user.publicKey,
    config: ctx.configPda,
    mnaMint: ctx.mnaMint,
    quoteMint: ctx.quoteMint,
    userMnaAccount: ctx.userMnaAccount,
    quoteVault: ctx.quoteVault,
    userQuoteAccount: ctx.userQuoteAccount,
    mnaAmount: mnaBaseUnits,
    programId: ctx.config.programId,
    quoteTokenProgramId: SPL_TOKEN_PROGRAM_ID,
  });
  const signature = await sendAndConfirmTransaction(
    ctx.connection,
    new Transaction().add(instruction),
    [ctx.user],
  );
  return {
    signature,
    mnaAmount,
    mnaBaseUnits,
    expectedQuote: baseUnitsToQuote(expectedQuoteBaseUnits),
    expectedQuoteBaseUnits,
  };
}

export async function transferMna(ctx: OperationContext, recipient: PublicKey, mnaAmount: string) {
  const destination = await ensureAta({
    connection: ctx.connection,
    payer: ctx.user,
    mint: ctx.mnaMint,
    owner: recipient,
    tokenProgramId: TOKEN_2022_PROGRAM_ID,
  });
  const amountBaseUnits = mnaToBaseUnits(mnaAmount);
  const instruction = createTransferCheckedInstruction(
    ctx.userMnaAccount,
    ctx.mnaMint,
    destination,
    ctx.user.publicKey,
    amountBaseUnits,
    6,
    [],
    TOKEN_2022_PROGRAM_ID,
  );
  const signature = await sendAndConfirmTransaction(
    ctx.connection,
    new Transaction().add(instruction),
    [ctx.user],
  );
  return {
    signature,
    recipient: recipient.toBase58(),
    destinationTokenAccount: destination.toBase58(),
    mnaAmount,
    amountBaseUnits,
  };
}

export async function readScenarioState(ctx: OperationContext) {
  const [mnaMint, userMna, userQuote, quoteVault] = await Promise.all([
    getMint(ctx.connection, ctx.mnaMint, 'confirmed', TOKEN_2022_PROGRAM_ID),
    readTokenAccount(ctx.connection, ctx.userMnaAccount, TOKEN_2022_PROGRAM_ID),
    readTokenAccount(ctx.connection, ctx.userQuoteAccount, SPL_TOKEN_PROGRAM_ID),
    readTokenAccount(ctx.connection, ctx.quoteVault, SPL_TOKEN_PROGRAM_ID),
  ]);
  return {
    supply: baseUnitsToMna(mnaMint.supply),
    supplyBaseUnits: mnaMint.supply,
    userMna: baseUnitsToMna(userMna.amount),
    userMnaBaseUnits: userMna.amount,
    userQuote: baseUnitsToQuote(userQuote.amount),
    userQuoteBaseUnits: userQuote.amount,
    reserve: baseUnitsToQuote(quoteVault.amount),
    reserveBaseUnits: quoteVault.amount,
  };
}

export function assertScenarioInvariant(state: Awaited<ReturnType<typeof readScenarioState>>) {
  const expectedReserve = state.supplyBaseUnits * 2n;
  if (state.reserveBaseUnits !== expectedReserve) {
    throw new Error(`Reserve invariant failed: ${state.reserve} quote != ${baseUnitsToQuote(expectedReserve)} quote`);
  }
}
