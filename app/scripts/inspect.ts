import { getAccount, getMint, getTokenMetadata } from '@solana/spl-token';
import { Connection, PublicKey } from '@solana/web3.js';
import { loadConfig } from '../src/config.js';
import { deriveConfigPda } from '../src/controller.js';
import { baseUnitsToMna, baseUnitsToQuote } from '../src/amounts.js';
import { SPL_TOKEN_PROGRAM_ID, TOKEN_2022_PROGRAM_ID } from '../src/constants.js';

async function main() {
  const config = loadConfig();
  const mnaMint = config.mnaMint;
  const quoteMint = config.quoteMint;
  if (!mnaMint || !quoteMint) throw new Error('Set MNA_MINT_ADDRESS and QUOTE_MINT_ADDRESS');
  const connection = new Connection(config.rpcUrl, 'confirmed');
  const [configPda] = deriveConfigPda(config.programId);
  const quoteVault = await import('@solana/spl-token').then(({ getAssociatedTokenAddressSync }) =>
    getAssociatedTokenAddressSync(quoteMint, configPda, true, SPL_TOKEN_PROGRAM_ID));
  const [mnaInfo, quoteInfo, mnaState, quoteState, metadata, vaultInfo] = await Promise.all([
    connection.getAccountInfo(mnaMint),
    connection.getAccountInfo(quoteMint),
    getMint(connection, mnaMint, 'confirmed', TOKEN_2022_PROGRAM_ID),
    getMint(connection, quoteMint, 'confirmed', SPL_TOKEN_PROGRAM_ID),
    getTokenMetadata(connection, mnaMint, 'confirmed'),
    connection.getAccountInfo(quoteVault),
  ]);
  const vault = vaultInfo
    ? await getAccount(connection, quoteVault, 'confirmed', SPL_TOKEN_PROGRAM_ID)
    : undefined;
  console.log(JSON.stringify({
    network: config.network,
    controllerProgram: config.programId.toBase58(),
    controllerConfig: configPda.toBase58(),
    mnaMint: {
      address: mnaMint.toBase58(),
      ownerProgram: mnaInfo?.owner.toBase58() ?? null,
      decimals: mnaState.decimals,
      supply: baseUnitsToMna(mnaState.supply),
      supplyBaseUnits: mnaState.supply.toString(),
      mintAuthority: mnaState.mintAuthority?.toBase58() ?? null,
      metadata,
    },
    quoteMint: {
      address: quoteMint.toBase58(),
      ownerProgram: quoteInfo?.owner.toBase58() ?? null,
      decimals: quoteState.decimals,
      supply: baseUnitsToQuote(quoteState.supply),
      supplyBaseUnits: quoteState.supply.toString(),
    },
    quoteVault: {
      address: quoteVault.toBase58(),
      exists: Boolean(vault),
      amount: vault ? baseUnitsToQuote(vault.amount) : null,
      amountBaseUnits: vault?.amount.toString() ?? null,
      authority: vault?.owner.toBase58() ?? null,
    },
  }, null, 2));
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : error);
  process.exitCode = 1;
});
