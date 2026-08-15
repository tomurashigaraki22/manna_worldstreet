use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
use anchor_lang::solana_program::program::{invoke, invoke_signed};

declare_id!("7QZLtciaBCeLy5uyooZERzBNbPLNuQ6Ppc4iA2kqEsyR");

const CONFIG_SEED: &[u8] = b"config";
const TOKEN_2022_PROGRAM_ID: Pubkey = pubkey!("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");
const TOKEN_PROGRAM_ID: Pubkey = pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
const ASSOCIATED_TOKEN_PROGRAM_ID: Pubkey =
    pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");
const MNA_DECIMALS: u8 = 6;
const QUOTE_DECIMALS: u8 = 6;
const RATE_MNA: u64 = 1;
const RATE_QUOTE: u64 = 2;

// ---------------------------------------------------------------------------
// Raw SPL Token / Token-2022 account access.
//
// Token-2022 keeps the legacy layouts as its base, so the same offsets read
// both programs' accounts:
//
//   Mint  (82 bytes): mint_authority COption<Pubkey> [0..36], supply u64
//                     [36..44], decimals u8 [44], is_initialized u8 [45],
//                     freeze_authority COption<Pubkey> [46..82]
//   Token (165 bytes): mint [0..32], owner [32..64], amount u64 [64..72],
//                      delegate COption<Pubkey> [72..108], state u8 [108]
//
// When a Token-2022 account carries extensions it is longer than the base and
// stores a discriminant byte at offset 165 (1 = Mint, 2 = Account). That byte
// is what disambiguates an extended mint from a token account, since an
// extended mint can otherwise reach exactly the 165-byte token account size.
// ---------------------------------------------------------------------------

const MINT_LEN: usize = 82;
const TOKEN_LEN: usize = 165;
const ACCOUNT_TYPE_OFFSET: usize = 165;
const ACCOUNT_TYPE_MINT: u8 = 1;
const ACCOUNT_TYPE_TOKEN: u8 = 2;

fn read_pubkey(data: &[u8], offset: usize) -> Pubkey {
    let mut key = [0u8; 32];
    key.copy_from_slice(&data[offset..offset + 32]);
    Pubkey::new_from_array(key)
}

fn read_u64(data: &[u8], offset: usize) -> u64 {
    let mut value = [0u8; 8];
    value.copy_from_slice(&data[offset..offset + 8]);
    u64::from_le_bytes(value)
}

/// Fields we need off a mint, read without deserializing the whole account.
struct MintView {
    decimals: u8,
    mint_authority: Option<Pubkey>,
}

fn load_mint(account: &AccountInfo, token_program: &Pubkey) -> Result<MintView> {
    require_keys_eq!(*account.owner, *token_program, MannaError::InvalidTokenAccount);
    let data = account.try_borrow_data()?;
    let len = data.len();
    require!(len >= MINT_LEN, MannaError::InvalidTokenAccount);
    // A longer-than-base account is only a mint if it is tagged as one.
    if len > MINT_LEN {
        require!(
            len > ACCOUNT_TYPE_OFFSET && data[ACCOUNT_TYPE_OFFSET] == ACCOUNT_TYPE_MINT,
            MannaError::InvalidTokenAccount
        );
    }
    require!(data[45] == 1, MannaError::InvalidTokenAccount);

    let mint_authority = match data[0..4] {
        [1, 0, 0, 0] => Some(read_pubkey(&data, 4)),
        _ => None,
    };
    Ok(MintView {
        decimals: data[44],
        mint_authority,
    })
}

/// Fields we need off a token account, read without deserializing extensions.
struct TokenView {
    mint: Pubkey,
    owner: Pubkey,
    amount: u64,
}

fn load_token_account(account: &AccountInfo, token_program: &Pubkey) -> Result<TokenView> {
    require_keys_eq!(*account.owner, *token_program, MannaError::InvalidTokenAccount);
    let data = account.try_borrow_data()?;
    let len = data.len();
    require!(len >= TOKEN_LEN, MannaError::InvalidTokenAccount);
    if len > TOKEN_LEN {
        require!(
            len > ACCOUNT_TYPE_OFFSET && data[ACCOUNT_TYPE_OFFSET] == ACCOUNT_TYPE_TOKEN,
            MannaError::InvalidTokenAccount
        );
    }
    // state: 0 = uninitialized, 1 = initialized, 2 = frozen.
    require!(data[108] != 0, MannaError::InvalidTokenAccount);

    Ok(TokenView {
        mint: read_pubkey(&data, 0),
        owner: read_pubkey(&data, 32),
        amount: read_u64(&data, 64),
    })
}

/// Assert a token account belongs to `mint` and is controlled by `authority`.
fn require_token_account(
    account: &AccountInfo,
    token_program: &Pubkey,
    mint: &Pubkey,
    authority: &Pubkey,
) -> Result<TokenView> {
    let view = load_token_account(account, token_program)?;
    require_keys_eq!(view.mint, *mint, MannaError::InvalidTokenAccount);
    require_keys_eq!(view.owner, *authority, MannaError::InvalidTokenAccount);
    Ok(view)
}

fn require_token_program(program: &AccountInfo) -> Result<()> {
    require!(
        *program.key == TOKEN_PROGRAM_ID || *program.key == TOKEN_2022_PROGRAM_ID,
        MannaError::InvalidTokenProgram
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Hand-rolled SPL Token CPIs. Instruction tags are stable across both the
// legacy Token program and Token-2022.
// ---------------------------------------------------------------------------

const IX_MINT_TO: u8 = 7;
const IX_BURN: u8 = 8;
const IX_TRANSFER_CHECKED: u8 = 12;

fn amount_data(tag: u8, amount: u64) -> [u8; 9] {
    let mut data = [0u8; 9];
    data[0] = tag;
    data[1..9].copy_from_slice(&amount.to_le_bytes());
    data
}

fn transfer_checked<'info>(
    token_program: &AccountInfo<'info>,
    from: &AccountInfo<'info>,
    mint: &AccountInfo<'info>,
    to: &AccountInfo<'info>,
    authority: &AccountInfo<'info>,
    amount: u64,
    decimals: u8,
    signer_seeds: Option<&[&[&[u8]]]>,
) -> Result<()> {
    let mut data = [0u8; 10];
    data[0] = IX_TRANSFER_CHECKED;
    data[1..9].copy_from_slice(&amount.to_le_bytes());
    data[9] = decimals;

    let ix = Instruction {
        program_id: *token_program.key,
        accounts: vec![
            AccountMeta::new(*from.key, false),
            AccountMeta::new_readonly(*mint.key, false),
            AccountMeta::new(*to.key, false),
            AccountMeta::new_readonly(*authority.key, true),
        ],
        data: data.to_vec(),
    };
    let infos = [
        from.clone(),
        mint.clone(),
        to.clone(),
        authority.clone(),
        token_program.clone(),
    ];
    match signer_seeds {
        Some(seeds) => invoke_signed(&ix, &infos, seeds)?,
        None => invoke(&ix, &infos)?,
    }
    Ok(())
}

fn mint_to<'info>(
    token_program: &AccountInfo<'info>,
    mint: &AccountInfo<'info>,
    to: &AccountInfo<'info>,
    authority: &AccountInfo<'info>,
    amount: u64,
    signer_seeds: &[&[&[u8]]],
) -> Result<()> {
    let ix = Instruction {
        program_id: *token_program.key,
        accounts: vec![
            AccountMeta::new(*mint.key, false),
            AccountMeta::new(*to.key, false),
            AccountMeta::new_readonly(*authority.key, true),
        ],
        data: amount_data(IX_MINT_TO, amount).to_vec(),
    };
    invoke_signed(
        &ix,
        &[mint.clone(), to.clone(), authority.clone(), token_program.clone()],
        signer_seeds,
    )?;
    Ok(())
}

fn burn<'info>(
    token_program: &AccountInfo<'info>,
    from: &AccountInfo<'info>,
    mint: &AccountInfo<'info>,
    authority: &AccountInfo<'info>,
    amount: u64,
) -> Result<()> {
    let ix = Instruction {
        program_id: *token_program.key,
        accounts: vec![
            AccountMeta::new(*from.key, false),
            AccountMeta::new(*mint.key, false),
            AccountMeta::new_readonly(*authority.key, true),
        ],
        data: amount_data(IX_BURN, amount).to_vec(),
    };
    invoke(
        &ix,
        &[from.clone(), mint.clone(), authority.clone(), token_program.clone()],
    )?;
    Ok(())
}

/// Associated Token Program `Create` (tag 0) — fails if the ATA already exists,
/// matching the semantics of Anchor's `init` constraint.
#[allow(clippy::too_many_arguments)]
fn create_associated_token_account<'info>(
    ata_program: &AccountInfo<'info>,
    payer: &AccountInfo<'info>,
    ata: &AccountInfo<'info>,
    owner: &AccountInfo<'info>,
    mint: &AccountInfo<'info>,
    system_program: &AccountInfo<'info>,
    token_program: &AccountInfo<'info>,
) -> Result<()> {
    let ix = Instruction {
        program_id: *ata_program.key,
        accounts: vec![
            AccountMeta::new(*payer.key, true),
            AccountMeta::new(*ata.key, false),
            AccountMeta::new_readonly(*owner.key, false),
            AccountMeta::new_readonly(*mint.key, false),
            AccountMeta::new_readonly(*system_program.key, false),
            AccountMeta::new_readonly(*token_program.key, false),
        ],
        data: vec![0],
    };
    invoke(
        &ix,
        &[
            payer.clone(),
            ata.clone(),
            owner.clone(),
            mint.clone(),
            system_program.clone(),
            token_program.clone(),
        ],
    )?;
    Ok(())
}

#[program]
pub mod manna_controller {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        require_keys_eq!(
            ctx.accounts.mna_token_program.key(),
            TOKEN_2022_PROGRAM_ID,
            MannaError::InvalidMnaTokenProgram
        );
        require_keys_eq!(
            ctx.accounts.associated_token_program.key(),
            ASSOCIATED_TOKEN_PROGRAM_ID,
            MannaError::InvalidTokenProgram
        );
        require_token_program(&ctx.accounts.quote_token_program)?;

        let config_key = ctx.accounts.config.key();

        let mna_mint = load_mint(&ctx.accounts.mna_mint, &TOKEN_2022_PROGRAM_ID)?;
        require!(
            mna_mint.mint_authority == Some(config_key),
            MannaError::InvalidMnaMintAuthority
        );
        require!(
            mna_mint.decimals == MNA_DECIMALS,
            MannaError::InvalidMnaDecimals
        );

        let quote_mint = load_mint(
            &ctx.accounts.quote_mint,
            ctx.accounts.quote_token_program.key,
        )?;
        require!(
            quote_mint.decimals == QUOTE_DECIMALS,
            MannaError::InvalidQuoteDecimals
        );

        // The Associated Token Program derives the address itself and rejects a
        // mismatch, so creating through it is what pins `quote_vault` to the
        // canonical ATA of (config, quote_mint).
        create_associated_token_account(
            &ctx.accounts.associated_token_program,
            &ctx.accounts.payer,
            &ctx.accounts.quote_vault,
            &ctx.accounts.config.to_account_info(),
            &ctx.accounts.quote_mint,
            &ctx.accounts.system_program,
            &ctx.accounts.quote_token_program,
        )?;
        require_token_account(
            &ctx.accounts.quote_vault,
            ctx.accounts.quote_token_program.key,
            ctx.accounts.quote_mint.key,
            &config_key,
        )?;

        let config = &mut ctx.accounts.config;
        config.admin = ctx.accounts.admin.key();
        config.mna_mint = ctx.accounts.mna_mint.key();
        config.quote_mint = ctx.accounts.quote_mint.key();
        config.quote_vault = ctx.accounts.quote_vault.key();
        config.quote_token_program = ctx.accounts.quote_token_program.key();
        config.mna_decimals = MNA_DECIMALS;
        config.quote_decimals = QUOTE_DECIMALS;
        config.rate_mna = RATE_MNA;
        config.rate_quote = RATE_QUOTE;
        config.paused = false;
        config.bump = ctx.bumps.config;

        emit!(ControllerInitialized {
            admin: config.admin,
            mna_mint: config.mna_mint,
            quote_mint: config.quote_mint,
            quote_vault: config.quote_vault,
        });

        Ok(())
    }

    pub fn mint_mna(ctx: Context<MintMna>, quote_amount: u64) -> Result<()> {
        let config = &ctx.accounts.config;
        require!(!config.paused, MannaError::Paused);
        require!(quote_amount > 0, MannaError::ZeroAmount);

        let quote_token_program = &ctx.accounts.quote_token_program;
        require_keys_eq!(
            quote_token_program.key(),
            config.quote_token_program,
            MannaError::InvalidTokenProgram
        );
        require_keys_eq!(
            ctx.accounts.mna_token_program.key(),
            TOKEN_2022_PROGRAM_ID,
            MannaError::InvalidMnaTokenProgram
        );
        require_keys_eq!(
            ctx.accounts.mna_mint.key(),
            config.mna_mint,
            MannaError::InvalidTokenAccount
        );
        require_keys_eq!(
            ctx.accounts.quote_mint.key(),
            config.quote_mint,
            MannaError::InvalidTokenAccount
        );
        require_keys_eq!(
            ctx.accounts.quote_vault.key(),
            config.quote_vault,
            MannaError::InvalidTokenAccount
        );

        let user_key = ctx.accounts.user.key();
        let config_key = config.key();
        require_token_account(
            &ctx.accounts.user_quote_account,
            quote_token_program.key,
            &config.quote_mint,
            &user_key,
        )?;
        require_token_account(
            &ctx.accounts.quote_vault,
            quote_token_program.key,
            &config.quote_mint,
            &config_key,
        )?;
        require_token_account(
            &ctx.accounts.user_mna_account,
            &TOKEN_2022_PROGRAM_ID,
            &config.mna_mint,
            &user_key,
        )?;

        let mna_amount = quote_amount_to_mna(quote_amount)?;
        let quote_decimals = config.quote_decimals;
        let bump = config.bump;

        transfer_checked(
            quote_token_program,
            &ctx.accounts.user_quote_account,
            &ctx.accounts.quote_mint,
            &ctx.accounts.quote_vault,
            &ctx.accounts.user,
            quote_amount,
            quote_decimals,
            None,
        )?;

        let signer_seeds: &[&[&[u8]]] = &[&[CONFIG_SEED, &[bump]]];
        mint_to(
            &ctx.accounts.mna_token_program,
            &ctx.accounts.mna_mint,
            &ctx.accounts.user_mna_account,
            &ctx.accounts.config.to_account_info(),
            mna_amount,
            signer_seeds,
        )?;

        emit!(MnaMinted {
            user: user_key,
            quote_amount,
            mna_amount,
        });
        Ok(())
    }

    pub fn redeem_mna(ctx: Context<RedeemMna>, mna_amount: u64) -> Result<()> {
        let config = &ctx.accounts.config;
        require!(!config.paused, MannaError::Paused);
        require!(mna_amount > 0, MannaError::ZeroAmount);

        let quote_token_program = &ctx.accounts.quote_token_program;
        require_keys_eq!(
            quote_token_program.key(),
            config.quote_token_program,
            MannaError::InvalidTokenProgram
        );
        require_keys_eq!(
            ctx.accounts.mna_token_program.key(),
            TOKEN_2022_PROGRAM_ID,
            MannaError::InvalidMnaTokenProgram
        );
        require_keys_eq!(
            ctx.accounts.mna_mint.key(),
            config.mna_mint,
            MannaError::InvalidTokenAccount
        );
        require_keys_eq!(
            ctx.accounts.quote_mint.key(),
            config.quote_mint,
            MannaError::InvalidTokenAccount
        );
        require_keys_eq!(
            ctx.accounts.quote_vault.key(),
            config.quote_vault,
            MannaError::InvalidTokenAccount
        );

        let user_key = ctx.accounts.user.key();
        let config_key = config.key();
        require_token_account(
            &ctx.accounts.user_mna_account,
            &TOKEN_2022_PROGRAM_ID,
            &config.mna_mint,
            &user_key,
        )?;
        let vault = require_token_account(
            &ctx.accounts.quote_vault,
            quote_token_program.key,
            &config.quote_mint,
            &config_key,
        )?;
        require_token_account(
            &ctx.accounts.user_quote_account,
            quote_token_program.key,
            &config.quote_mint,
            &user_key,
        )?;

        let quote_amount = mna_to_quote_amount(mna_amount)?;
        require!(
            vault.amount >= quote_amount,
            MannaError::InsufficientReserve
        );

        let quote_decimals = config.quote_decimals;
        let bump = config.bump;

        burn(
            &ctx.accounts.mna_token_program,
            &ctx.accounts.user_mna_account,
            &ctx.accounts.mna_mint,
            &ctx.accounts.user,
            mna_amount,
        )?;

        let signer_seeds: &[&[&[u8]]] = &[&[CONFIG_SEED, &[bump]]];
        transfer_checked(
            quote_token_program,
            &ctx.accounts.quote_vault,
            &ctx.accounts.quote_mint,
            &ctx.accounts.user_quote_account,
            &ctx.accounts.config.to_account_info(),
            quote_amount,
            quote_decimals,
            Some(signer_seeds),
        )?;

        emit!(MnaRedeemed {
            user: user_key,
            mna_amount,
            quote_amount,
        });
        Ok(())
    }

    pub fn set_paused(ctx: Context<SetPaused>, paused: bool) -> Result<()> {
        ctx.accounts.config.paused = paused;
        emit!(PauseChanged { paused });
        Ok(())
    }
}

fn quote_amount_to_mna(quote_amount: u64) -> Result<u64> {
    require!(quote_amount % RATE_QUOTE == 0, MannaError::InexactConversion);
    quote_amount
        .checked_mul(RATE_MNA)
        .ok_or_else(|| error!(MannaError::ArithmeticOverflow))
        .map(|amount| amount / RATE_QUOTE)
}

fn mna_to_quote_amount(mna_amount: u64) -> Result<u64> {
    mna_amount
        .checked_mul(RATE_QUOTE)
        .ok_or_else(|| error!(MannaError::ArithmeticOverflow))
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    pub admin: Signer<'info>,
    #[account(init, payer = payer, space = 8 + ControllerConfig::INIT_SPACE, seeds = [CONFIG_SEED], bump)]
    pub config: Account<'info, ControllerConfig>,
    /// CHECK: validated in the handler via `load_mint` against Token-2022.
    pub mna_mint: UncheckedAccount<'info>,
    /// CHECK: validated in the handler via `load_mint` against the quote token program.
    pub quote_mint: UncheckedAccount<'info>,
    /// CHECK: created and address-pinned by the Associated Token Program CPI,
    /// then validated via `require_token_account`.
    #[account(mut)]
    pub quote_vault: UncheckedAccount<'info>,
    /// CHECK: address-checked against Token-2022 in the handler.
    pub mna_token_program: UncheckedAccount<'info>,
    /// CHECK: checked to be Token or Token-2022 in the handler.
    pub quote_token_program: UncheckedAccount<'info>,
    /// CHECK: address-checked against the Associated Token Program in the handler.
    pub associated_token_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct MintMna<'info> {
    pub user: Signer<'info>,
    #[account(seeds = [CONFIG_SEED], bump = config.bump)]
    pub config: Account<'info, ControllerConfig>,
    /// CHECK: address-checked against `config.mna_mint` in the handler.
    #[account(mut)]
    pub mna_mint: UncheckedAccount<'info>,
    /// CHECK: address-checked against `config.quote_mint` in the handler.
    pub quote_mint: UncheckedAccount<'info>,
    /// CHECK: mint/owner validated in the handler.
    #[account(mut)]
    pub user_quote_account: UncheckedAccount<'info>,
    /// CHECK: address-checked against `config.quote_vault` and validated in the handler.
    #[account(mut)]
    pub quote_vault: UncheckedAccount<'info>,
    /// CHECK: mint/owner validated in the handler.
    #[account(mut)]
    pub user_mna_account: UncheckedAccount<'info>,
    /// CHECK: address-checked against Token-2022 in the handler.
    pub mna_token_program: UncheckedAccount<'info>,
    /// CHECK: address-checked against `config.quote_token_program` in the handler.
    pub quote_token_program: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct RedeemMna<'info> {
    pub user: Signer<'info>,
    #[account(seeds = [CONFIG_SEED], bump = config.bump)]
    pub config: Account<'info, ControllerConfig>,
    /// CHECK: address-checked against `config.mna_mint` in the handler.
    #[account(mut)]
    pub mna_mint: UncheckedAccount<'info>,
    /// CHECK: address-checked against `config.quote_mint` in the handler.
    pub quote_mint: UncheckedAccount<'info>,
    /// CHECK: mint/owner validated in the handler.
    #[account(mut)]
    pub user_mna_account: UncheckedAccount<'info>,
    /// CHECK: address-checked against `config.quote_vault` and validated in the handler.
    #[account(mut)]
    pub quote_vault: UncheckedAccount<'info>,
    /// CHECK: mint/owner validated in the handler.
    #[account(mut)]
    pub user_quote_account: UncheckedAccount<'info>,
    /// CHECK: address-checked against Token-2022 in the handler.
    pub mna_token_program: UncheckedAccount<'info>,
    /// CHECK: address-checked against `config.quote_token_program` in the handler.
    pub quote_token_program: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct SetPaused<'info> {
    #[account(mut, seeds = [CONFIG_SEED], bump = config.bump, has_one = admin)]
    pub config: Account<'info, ControllerConfig>,
    pub admin: Signer<'info>,
}

#[account]
#[derive(InitSpace)]
pub struct ControllerConfig {
    pub admin: Pubkey,
    pub mna_mint: Pubkey,
    pub quote_mint: Pubkey,
    pub quote_vault: Pubkey,
    pub quote_token_program: Pubkey,
    pub mna_decimals: u8,
    pub quote_decimals: u8,
    pub rate_mna: u64,
    pub rate_quote: u64,
    pub paused: bool,
    pub bump: u8,
}

#[event]
pub struct ControllerInitialized {
    pub admin: Pubkey,
    pub mna_mint: Pubkey,
    pub quote_mint: Pubkey,
    pub quote_vault: Pubkey,
}

#[event]
pub struct MnaMinted {
    pub user: Pubkey,
    pub quote_amount: u64,
    pub mna_amount: u64,
}

#[event]
pub struct MnaRedeemed {
    pub user: Pubkey,
    pub mna_amount: u64,
    pub quote_amount: u64,
}

#[event]
pub struct PauseChanged {
    pub paused: bool,
}

#[error_code]
pub enum MannaError {
    #[msg("The MNA mint must use Token-2022")]
    InvalidMnaTokenProgram,
    #[msg("The MNA mint authority must be the controller config PDA")]
    InvalidMnaMintAuthority,
    #[msg("MNA must use 6 decimals")]
    InvalidMnaDecimals,
    #[msg("The quote token must use 6 decimals")]
    InvalidQuoteDecimals,
    #[msg("The controller is paused")]
    Paused,
    #[msg("Amount must be greater than zero")]
    ZeroAmount,
    #[msg("Quote amount cannot be represented exactly at the fixed rate")]
    InexactConversion,
    #[msg("Arithmetic overflow")]
    ArithmeticOverflow,
    #[msg("The reserve vault does not have enough quote tokens")]
    InsufficientReserve,
    #[msg("Token account or mint failed validation")]
    InvalidTokenAccount,
    #[msg("Unsupported token program")]
    InvalidTokenProgram,
}
