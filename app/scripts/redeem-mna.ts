import { getOperationContext, readScenarioState, redeemMna } from '../src/operations.js';

function readFlag(name: string): string {
  const index = process.argv.indexOf(name);
  const value = index >= 0 ? process.argv[index + 1] : undefined;
  if (!value) throw new Error(`Usage: npm run redeem-mna -- --mna <amount>`);
  return value;
}

async function main() {
  const mnaAmount = readFlag('--mna');
  const ctx = await getOperationContext();
  const result = await redeemMna(ctx, mnaAmount);
  const state = await readScenarioState(ctx);
  console.log(JSON.stringify({
    operation: 'redeem_mna',
    ...result,
    postState: state,
  }, null, 2));
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : error);
  process.exitCode = 1;
});
