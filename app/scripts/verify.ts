import { getMint, getTokenMetadata, getAccount } from '@solana/spl-token';
import { Connection, PublicKey } from '@solana/web3.js';
import { loadConfig } from '../src/config.js';
import { deriveConfigPda } from '../src/controller.js';
import { mnaToBaseUnits } from '../src/amounts.js';
import { SPL_TOKEN_PROGRAM_ID, TOKEN_2022_PROGRAM_ID } from '../src/constants.js';

function optionalFlag(name: string): string | undefined {
  const index = process.argv.indexOf(name);
  return index >= 0 ? process.argv[index + 1] : undefined;
}

async function main() {
  const config = loadConfig();
  if (!config.mnaMint || !config.quoteMint) throw new Error('Set MNA_MINT_ADDRESS and QUOTE_MINT_ADDRESS');
  const connection = new Connection(config.rpcUrl, 'confirmed');
  const [configPda] = deriveConfigPda(config.programId);
  const quoteVault = (await import('@solana/spl-token')).getAssociatedTokenAddressSync(
    config.quoteMint,
    configPda,
    true,
    SPL_TOKEN_PROGRAM_ID,
  );
  const [mnaAccount, quoteAccount, mna, quote, metadata, vault] = await Promise.all([
    connection.getAccountInfo(config.mnaMint),
    connection.getAccountInfo(config.quoteMint),
    getMint(connection, config.mnaMint, 'confirmed', TOKEN_2022_PROGRAM_ID),
    getMint(connection, config.quoteMint, 'confirmed', SPL_TOKEN_PROGRAM_ID),
    getTokenMetadata(connection, config.mnaMint, 'confirmed'),
    getAccount(connection, quoteVault, 'confirmed', SPL_TOKEN_PROGRAM_ID),
  ]);
  const failures: string[] = [];
  if (!mnaAccount?.owner.equals(TOKEN_2022_PROGRAM_ID)) failures.push('MNA mint is not owned by Token-2022');
  if (!quoteAccount?.owner.equals(SPL_TOKEN_PROGRAM_ID)) failures.push('Quote mint is not owned by the configured SPL Token program');
  if (mna.decimals !== 6) failures.push(`MNA decimals are ${mna.decimals}, expected 6`);
  if (quote.decimals !== 6) failures.push(`Quote decimals are ${quote.decimals}, expected 6`);
  if (!mna.mintAuthority?.equals(configPda)) failures.push('MNA mint authority is not the controller config PDA');
  if (!metadata || metadata.name !== 'Manna' || metadata.symbol !== 'MNA') failures.push('MNA metadata name/symbol mismatch');
  if (!vault.owner.equals(configPda)) failures.push('Quote vault authority is not the controller config PDA');
  const expectedSupply = optionalFlag('--expected-supply');
  if (expectedSupply !== undefined && mna.supply !== mnaToBaseUnits(expectedSupply)) {
    failures.push(`Supply is ${mna.supply.toString()} base units, expected ${mnaToBaseUnits(expectedSupply).toString()}`);
  }
  const report = {
    ok: failures.length === 0,
    controllerProgram: config.programId.toBase58(),
    config: configPda.toBase58(),
    mnaMint: config.mnaMint.toBase58(),
    quoteMint: config.quoteMint.toBase58(),
    quoteVault: quoteVault.toBase58(),
    mnaSupplyBaseUnits: mna.supply.toString(),
    quoteReserveBaseUnits: vault.amount.toString(),
    failures,
  };
  console.log(JSON.stringify(report, null, 2));
  if (failures.length > 0) process.exitCode = 1;
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : error);
  process.exitCode = 1;
});
