import {
  AuthorityType,
  ExtensionType,
  LENGTH_SIZE,
  TOKEN_2022_PROGRAM_ID as SPL_TOKEN_2022_PROGRAM_ID,
  TYPE_SIZE,
  createInitializeMetadataPointerInstruction,
  createInitializeMintInstruction,
  createSetAuthorityInstruction,
  getAccount,
  getMint,
  getMintLen,
  getOrCreateAssociatedTokenAccount,
  getTokenMetadata,
  mintTo,
} from '@solana/spl-token';
import { createInitializeInstruction } from '@solana/spl-token-metadata';
import {
  Connection,
  Keypair,
  PublicKey,
  SendOptions,
  SystemProgram,
  Transaction,
  sendAndConfirmTransaction,
} from '@solana/web3.js';
import { pack } from '@solana/spl-token-metadata';
import { MNA_DECIMALS, MNA_NAME, MNA_SYMBOL, TOKEN_2022_PROGRAM_ID } from './constants.js';

export async function createMnaMint(args: {
  connection: Connection;
  payer: Keypair;
  metadataUri: string;
  controllerAuthority: PublicKey;
  metadataUpdateAuthority?: PublicKey;
  sendOptions?: SendOptions;
}): Promise<{ mint: PublicKey; signature: string; mintLength: number }> {
  const mint = Keypair.generate();
  const metadataUpdateAuthority = args.metadataUpdateAuthority ?? args.payer.publicKey;
  const metadataLength = pack({
    updateAuthority: metadataUpdateAuthority,
    mint: mint.publicKey,
    name: MNA_NAME,
    symbol: MNA_SYMBOL,
    uri: args.metadataUri,
    additionalMetadata: [],
  }).length;
  const mintLength =
    getMintLen([ExtensionType.MetadataPointer]) + TYPE_SIZE + LENGTH_SIZE + metadataLength;
  const lamports = await args.connection.getMinimumBalanceForRentExemption(mintLength);

  const transaction = new Transaction().add(
    SystemProgram.createAccount({
      fromPubkey: args.payer.publicKey,
      newAccountPubkey: mint.publicKey,
      space: mintLength,
      lamports,
      programId: TOKEN_2022_PROGRAM_ID,
    }),
    createInitializeMetadataPointerInstruction(
      mint.publicKey,
      args.payer.publicKey,
      mint.publicKey,
      TOKEN_2022_PROGRAM_ID,
    ),
    createInitializeMintInstruction(
      mint.publicKey,
      MNA_DECIMALS,
      args.payer.publicKey,
      null,
      TOKEN_2022_PROGRAM_ID,
    ),
    createInitializeInstruction({
      programId: TOKEN_2022_PROGRAM_ID,
      metadata: mint.publicKey,
      updateAuthority: metadataUpdateAuthority,
      mint: mint.publicKey,
      mintAuthority: args.payer.publicKey,
      name: MNA_NAME,
      symbol: MNA_SYMBOL,
      uri: args.metadataUri,
    }),
    createSetAuthorityInstruction(
      mint.publicKey,
      args.payer.publicKey,
      AuthorityType.MintTokens,
      args.controllerAuthority,
      [],
      TOKEN_2022_PROGRAM_ID,
    ),
  );

  const signature = await sendAndConfirmTransaction(
    args.connection,
    transaction,
    [args.payer, mint],
    args.sendOptions,
  );
  return { mint: mint.publicKey, signature, mintLength };
}

export async function createMockQuoteMint(args: {
  connection: Connection;
  payer: Keypair;
}): Promise<PublicKey> {
  const { createMint } = await import('@solana/spl-token');
  return createMint(
    args.connection,
    args.payer,
    args.payer.publicKey,
    null,
    6,
    undefined,
    undefined,
  );
}

export async function ensureAta(args: {
  connection: Connection;
  payer: Keypair;
  mint: PublicKey;
  owner: PublicKey;
  tokenProgramId: PublicKey;
  allowOwnerOffCurve?: boolean;
}): Promise<PublicKey> {
  const account = await getOrCreateAssociatedTokenAccount(
    args.connection,
    args.payer,
    args.mint,
    args.owner,
    args.allowOwnerOffCurve ?? false,
    'confirmed',
    undefined,
    args.tokenProgramId,
  );
  return account.address;
}

export async function mintTokens(args: {
  connection: Connection;
  payer: Keypair;
  mint: PublicKey;
  destination: PublicKey;
  amount: bigint;
  tokenProgramId: PublicKey;
}): Promise<string> {
  return mintTo(
    args.connection,
    args.payer,
    args.mint,
    args.destination,
    args.payer,
    args.amount,
    [],
    undefined,
    args.tokenProgramId,
  );
}

export async function readMna(connection: Connection, mint: PublicKey) {
  const state = await getMint(connection, mint, 'confirmed', SPL_TOKEN_2022_PROGRAM_ID);
  const metadata = await getTokenMetadata(connection, mint, 'confirmed');
  return { state, metadata };
}

export async function readTokenAccount(
  connection: Connection,
  address: PublicKey,
  tokenProgramId: PublicKey,
) {
  return getAccount(connection, address, 'confirmed', tokenProgramId);
}
