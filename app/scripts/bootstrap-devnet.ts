import {
  assertScenarioInvariant,
  getOperationContext,
  mintMna,
  readScenarioState,
  redeemMna,
} from '../src/operations.js';

function assertState(state: Awaited<ReturnType<typeof readScenarioState>>, supply: string, reserve: string) {
  if (state.supply !== supply || state.reserve !== reserve) {
    throw new Error(`Scenario state mismatch: expected ${supply} MNA / ${reserve} quote, got ${state.supply} MNA / ${state.reserve} quote`);
  }
  assertScenarioInvariant(state);
}

async function main() {
  const ctx = await getOperationContext();
  const before = await readScenarioState(ctx);
  if (before.supply !== '0' || before.reserve !== '0') {
    throw new Error(`Bootstrap expects an empty controller; found ${before.supply} MNA and ${before.reserve} quote in circulation/reserve`);
  }

  const initial = await mintMna(ctx, '30');
  const afterInitial = await readScenarioState(ctx);
  assertState(afterInitial, '15', '30');

  const additional = await mintMna(ctx, '200');
  const afterAdditional = await readScenarioState(ctx);
  assertState(afterAdditional, '115', '230');

  const redemption = await redeemMna(ctx, '10');
  const afterRedemption = await readScenarioState(ctx);
  assertState(afterRedemption, '105', '210');

  console.log(JSON.stringify({
    scenario: '30 quote -> 15 MNA; +200 quote -> +100 MNA; 10 MNA -> 20 quote',
    initial,
    afterInitial,
    additional,
    afterAdditional,
    redemption,
    afterRedemption,
  }, null, 2));
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : error);
  process.exitCode = 1;
});
