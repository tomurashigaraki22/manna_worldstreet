import { Connection, PublicKey, Transaction, sendAndConfirmTransaction } from '@solana/web3.js';
import { loadConfig, requiredEnv } from '../src/config.js';
import { buildInitializeInstruction, deriveConfigPda } from '../src/controller.js';
import { getAssociatedTokenAddressSync } from '@solana/spl-token';
import { SPL_TOKEN_PROGRAM_ID } from '../src/constants.js';

async function main() {
  const config = loadConfig();
  const payer = config.feePayer ?? config.admin;
  if (!payer) throw new Error('Set FEE_PAYER_KEYPAIR_PATH or ADMIN_KEYPAIR_PATH');
  const mnaMint = new PublicKey(requiredEnv('MNA_MINT_ADDRESS'));
  const quoteMint = new PublicKey(requiredEnv('QUOTE_MINT_ADDRESS'));
  const connection = new Connection(config.rpcUrl, 'confirmed');
  const [controllerConfig] = deriveConfigPda(config.programId);
  const quoteVault = getAssociatedTokenAddressSync(
    quoteMint,
    controllerConfig,
    true,
    SPL_TOKEN_PROGRAM_ID,
  );
  const ix = buildInitializeInstruction({
    payer: payer.publicKey,
    config: controllerConfig,
    mnaMint,
    quoteMint,
    quoteVault,
    programId: config.programId,
    quoteTokenProgramId: SPL_TOKEN_PROGRAM_ID,
  });
  const signature = await sendAndConfirmTransaction(
    connection,
    new Transaction().add(ix),
    [payer],
  );
  console.log(JSON.stringify({
    controllerProgram: config.programId.toBase58(),
    config: controllerConfig.toBase58(),
    mnaMint: mnaMint.toBase58(),
    quoteMint: quoteMint.toBase58(),
    quoteVault: quoteVault.toBase58(),
    rate: '2 quote = 1 MNA',
    signature,
  }, null, 2));
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : error);
  process.exitCode = 1;
});
