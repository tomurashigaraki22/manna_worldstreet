import { getOperationContext, mintMna, readScenarioState } from '../src/operations.js';

function readFlag(name: string): string {
  const index = process.argv.indexOf(name);
  const value = index >= 0 ? process.argv[index + 1] : undefined;
  if (!value) throw new Error(`Usage: npm run mint-mna -- --quote <amount>`);
  return value;
}

async function main() {
  const quoteAmount = readFlag('--quote');
  const ctx = await getOperationContext();
  const result = await mintMna(ctx, quoteAmount);
  const state = await readScenarioState(ctx);
  console.log(JSON.stringify({
    operation: 'mint_mna',
    ...result,
    postState: state,
  }, null, 2));
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : error);
  process.exitCode = 1;
});
