import { Connection } from '@solana/web3.js';
import { createMockQuoteMint, ensureAta, mintTokens } from '../src/token.js';
import { loadConfig } from '../src/config.js';
import { SPL_TOKEN_PROGRAM_ID } from '../src/constants.js';
import { quoteToBaseUnits } from '../src/amounts.js';

async function main() {
  const config = loadConfig();
  const payer = config.feePayer ?? config.admin;
  if (!payer) throw new Error('Set FEE_PAYER_KEYPAIR_PATH or ADMIN_KEYPAIR_PATH');
  const connection = new Connection(config.rpcUrl, 'confirmed');
  const mint = await createMockQuoteMint({ connection, payer });
  const destination = await ensureAta({
    connection,
    payer,
    mint,
    owner: payer.publicKey,
    tokenProgramId: SPL_TOKEN_PROGRAM_ID,
  });
  const signature = await mintTokens({
    connection,
    payer,
    mint,
    destination,
    amount: quoteToBaseUnits('230'),
    tokenProgramId: SPL_TOKEN_PROGRAM_ID,
  });
  console.log(JSON.stringify({
    name: 'Mock USDT Devnet',
    symbol: 'USDT-DEV',
    decimals: 6,
    mint: mint.toBase58(),
    ownerTokenAccount: destination.toBase58(),
    mintedForScenario: '230',
    signature,
  }, null, 2));
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : error);
  process.exitCode = 1;
});
