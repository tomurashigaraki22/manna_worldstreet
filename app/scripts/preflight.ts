import { Connection } from '@solana/web3.js';
import { loadConfig } from '../src/config.js';

async function main() {
  const config = loadConfig();
  const connection = new Connection(config.rpcUrl, 'confirmed');
  const [version, epoch] = await Promise.all([connection.getVersion(), connection.getEpochInfo()]);
  console.log(JSON.stringify({
    network: config.network,
    rpcUrl: config.rpcUrl,
    solanaCore: version['solana-core'],
    epoch: epoch.epoch,
    feePayerConfigured: Boolean(config.feePayer),
    adminConfigured: Boolean(config.admin),
  }, null, 2));
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : error);
  process.exitCode = 1;
});
