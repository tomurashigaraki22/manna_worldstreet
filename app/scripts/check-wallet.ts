import { PublicKey } from '@solana/web3.js';
import { getOperationContext } from '../src/operations.js';
import { ensureAta, readTokenAccount } from '../src/token.js';
import { SPL_TOKEN_PROGRAM_ID, TOKEN_2022_PROGRAM_ID } from '../src/constants.js';

function readFlag(name: string): string {
  const index = process.argv.indexOf(name);
  const value = index >= 0 ? process.argv[index + 1] : undefined;
  if (!value) throw new Error(`Usage: npm run check-wallet -- --owner <OWNER>`);
  return value;
}

async function main() {
  const owner = new PublicKey(readFlag('--owner'));
  const ctx = await getOperationContext();
  const mnaAccount = await ensureAta({
    connection: ctx.connection,
    payer: ctx.user,
    mint: ctx.mnaMint,
    owner,
    tokenProgramId: TOKEN_2022_PROGRAM_ID,
  });
  const quoteAccount = await ensureAta({
    connection: ctx.connection,
    payer: ctx.user,
    mint: ctx.quoteMint,
    owner,
    tokenProgramId: SPL_TOKEN_PROGRAM_ID,
  });
  const [mnaState, quoteState] = await Promise.all([
    readTokenAccount(ctx.connection, mnaAccount, TOKEN_2022_PROGRAM_ID),
    readTokenAccount(ctx.connection, quoteAccount, SPL_TOKEN_PROGRAM_ID),
  ]);
  console.log(JSON.stringify({
    owner: owner.toBase58(),
    mnaTokenAccount: mnaAccount.toBase58(),
    mnaAmountBaseUnits: mnaState.amount.toString(),
    quoteTokenAccount: quoteAccount.toBase58(),
    quoteAmountBaseUnits: quoteState.amount.toString(),
  }, null, 2));
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : error);
  process.exitCode = 1;
});
