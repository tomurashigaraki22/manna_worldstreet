import { Connection, PublicKey } from '@solana/web3.js';
import { createMnaMint } from '../src/token.js';
import { deriveConfigPda } from '../src/controller.js';
import { loadConfig, requiredEnv } from '../src/config.js';

async function main() {
  const config = loadConfig();
  const payer = config.feePayer ?? config.admin;
  if (!payer) throw new Error('Set FEE_PAYER_KEYPAIR_PATH or ADMIN_KEYPAIR_PATH');
  const metadataUri = requiredEnv('MNA_METADATA_URI');
  const [controllerConfig] = deriveConfigPda(config.programId);
  const connection = new Connection(config.rpcUrl, 'confirmed');
  const result = await createMnaMint({
    connection,
    payer,
    metadataUri,
    controllerAuthority: controllerConfig,
    metadataUpdateAuthority: config.admin?.publicKey ?? payer.publicKey,
  });
  console.log(JSON.stringify({
    name: 'Manna',
    symbol: 'MNA',
    decimals: 6,
    mint: result.mint.toBase58(),
    controllerConfig: controllerConfig.toBase58(),
    mintAccountBytes: result.mintLength,
    signature: result.signature,
    next: 'Set MNA_MINT_ADDRESS to this mint, then run initialize-controller',
  }, null, 2));
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : error);
  process.exitCode = 1;
});
