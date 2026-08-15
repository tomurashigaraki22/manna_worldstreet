import {
  AuthorityType,
  createCreateMetadataAccountV3Instruction,
} from '@metaplex-foundation/mpl-token-metadata';
import {
  AuthorityType as SplAuthorityType,
  createMint,
  createSetAuthorityInstruction,
  getOrCreateAssociatedTokenAccount,
  mintTo,
  TOKEN_PROGRAM_ID,
} from '@solana/spl-token';
import {
  Connection,
  Keypair,
  PublicKey,
  SystemProgram,
  Transaction,
  sendAndConfirmTransaction,
} from '@solana/web3.js';
import { loadConfig } from '../src/config.js';

const NAME = 'WManna';
const SYMBOL = 'WMNA';
const DECIMALS = 6;
const SUPPLY_BASE_UNITS = 50_000_000_000n;
const METADATA_URI =
  'https://raw.githubusercontent.com/tomurashigaraki22/manna_worldstreet/main/metadata/wmanna.json';
const METADATA_PROGRAM_ID = new PublicKey(
  'metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s',
);

async function main() {
  const config = loadConfig();
  const payer = config.feePayer;
  if (!payer) throw new Error('FEE_PAYER_KEYPAIR_PATH is required');
  const connection = new Connection(config.rpcUrl, 'confirmed');

  const payerBalance = await connection.getBalance(payer.publicKey, 'confirmed');
  console.log(`Fee payer: ${payer.publicKey.toBase58()}`);
  console.log(`Fee payer balance: ${(payerBalance / 1_000_000_000).toFixed(9)} SOL`);
  if (payerBalance < 20_000_000) {
    throw new Error('Fee payer has less than 0.02 SOL; refusing to create the mint');
  }

  // Legacy SPL Token is intentionally used for broad Phantom/DEX compatibility.
  // Freeze authority is null from creation; mint authority is revoked after minting
  // the fixed 50,000 WMNA supply.
  const mint = await createMint(
    connection,
    payer,
    payer.publicKey,
    null,
    DECIMALS,
  );
  const ownerAta = await getOrCreateAssociatedTokenAccount(
    connection,
    payer,
    mint,
    payer.publicKey,
    false,
    'confirmed',
    undefined,
    TOKEN_PROGRAM_ID,
  );
  const mintSignature = await mintTo(
    connection,
    payer,
    mint,
    ownerAta.address,
    payer,
    SUPPLY_BASE_UNITS,
    [],
    undefined,
    TOKEN_PROGRAM_ID,
  );

  const [metadata] = PublicKey.findProgramAddressSync(
    [Buffer.from('metadata'), METADATA_PROGRAM_ID.toBuffer(), mint.toBuffer()],
    METADATA_PROGRAM_ID,
  );
  const metadataInstruction = createCreateMetadataAccountV3Instruction(
    {
      metadata,
      mint,
      mintAuthority: payer.publicKey,
      payer: payer.publicKey,
      updateAuthority: payer.publicKey,
      systemProgram: SystemProgram.programId,
    },
    {
      createMetadataAccountArgsV3: {
        data: {
          name: NAME,
          symbol: SYMBOL,
          uri: METADATA_URI,
          sellerFeeBasisPoints: 0,
          creators: null,
          collection: null,
          uses: null,
        },
        isMutable: true,
        collectionDetails: null,
      },
    },
  );
  const metadataSignature = await sendAndConfirmTransaction(
    connection,
    new Transaction().add(metadataInstruction),
    [payer],
    { commitment: 'confirmed' },
  );

  const revokeSignature = await sendAndConfirmTransaction(
    connection,
    new Transaction().add(
      createSetAuthorityInstruction(
        mint,
        payer.publicKey,
        SplAuthorityType.MintTokens,
        null,
        [],
        TOKEN_PROGRAM_ID,
      ),
    ),
    [payer],
    { commitment: 'confirmed' },
  );

  console.log(
    JSON.stringify(
      {
        network: config.network,
        tokenProgram: TOKEN_PROGRAM_ID.toBase58(),
        name: NAME,
        symbol: SYMBOL,
        decimals: DECIMALS,
        supplyBaseUnits: SUPPLY_BASE_UNITS.toString(),
        supply: '50,000 WMNA',
        mint: mint.toBase58(),
        feePayer: payer.publicKey.toBase58(),
        ownerAta: ownerAta.address.toBase58(),
        metadataAccount: metadata.toBase58(),
        metadataUri: METADATA_URI,
        mintSignature,
        metadataSignature,
        revokeMintAuthoritySignature: revokeSignature,
        poolCreated: false,
      },
      null,
      2,
    ),
  );
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : error);
  process.exitCode = 1;
});
