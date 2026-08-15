import { PublicKey } from '@solana/web3.js';
import { getOperationContext, readScenarioState, transferMna } from '../src/operations.js';

function readFlag(name: string): string {
  const index = process.argv.indexOf(name);
  const value = index >= 0 ? process.argv[index + 1] : undefined;
  if (!value) throw new Error(`Usage: npm run transfer-mna -- --recipient <OWNER> --mna <amount>`);
  return value;
}

async function main() {
  const recipient = new PublicKey(readFlag('--recipient'));
  const mnaAmount = readFlag('--mna');
  const ctx = await getOperationContext();
  const result = await transferMna(ctx, recipient, mnaAmount);
  console.log(JSON.stringify({
    operation: 'transfer_mna',
    ...result,
    postState: await readScenarioState(ctx),
  }, null, 2));
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : error);
  process.exitCode = 1;
});
