use anchor_lang::prelude::*;
use anchor_lang::solana_program::program_option::COption;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{
    burn, mint_to, transfer_checked, Burn, Mint, MintTo, TokenAccount, TokenInterface,
    TransferChecked,
};

declare_id!("G5g6sekcjmuxNriHN2K6kca2tFVXXqL1C6yd5uLpvoTj");

const CONFIG_SEED: &[u8] = b"config";
const TOKEN_2022_PROGRAM_ID: Pubkey = pubkey!("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");
const MNA_DECIMALS: u8 = 6;
const QUOTE_DECIMALS: u8 = 6;
const RATE_MNA: u64 = 1;
const RATE_QUOTE: u64 = 2;

#[program]
pub mod manna_controller {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let config = &mut ctx.accounts.config;
        let mna_mint = &ctx.accounts.mna_mint;
        let quote_mint = &ctx.accounts.quote_mint;

        require_keys_eq!(
            ctx.accounts.mna_token_program.key(),
            TOKEN_2022_PROGRAM_ID,
            MannaError::InvalidMnaTokenProgram
        );
        require!(
            mna_mint.mint_authority == COption::Some(config.key()),
            MannaError::InvalidMnaMintAuthority
        );
        require!(mna_mint.decimals == MNA_DECIMALS, MannaError::InvalidMnaDecimals);
        require!(
            quote_mint.decimals == QUOTE_DECIMALS,
            MannaError::InvalidQuoteDecimals
        );

        config.admin = ctx.accounts.payer.key();
        config.mna_mint = mna_mint.key();
        config.quote_mint = quote_mint.key();
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

        let mna_amount = quote_amount_to_mna(quote_amount)?;

        let quote_transfer_accounts = TransferChecked {
            from: ctx.accounts.user_quote_account.to_account_info(),
            mint: ctx.accounts.quote_mint.to_account_info(),
            to: ctx.accounts.quote_vault.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        };
        transfer_checked(
            CpiContext::new(
                ctx.accounts.quote_token_program.to_account_info(),
                quote_transfer_accounts,
            ),
            quote_amount,
            config.quote_decimals,
        )?;

        let signer_seeds: &[&[&[u8]]] = &[&[CONFIG_SEED, &[config.bump]]];
        let mint_accounts = MintTo {
            mint: ctx.accounts.mna_mint.to_account_info(),
            to: ctx.accounts.user_mna_account.to_account_info(),
            authority: ctx.accounts.config.to_account_info(),
        };
        mint_to(
            CpiContext::new_with_signer(
                ctx.accounts.mna_token_program.to_account_info(),
                mint_accounts,
                signer_seeds,
            ),
            mna_amount,
        )?;

        emit!(MnaMinted {
            user: ctx.accounts.user.key(),
            quote_amount,
            mna_amount,
        });
        Ok(())
    }

    pub fn redeem_mna(ctx: Context<RedeemMna>, mna_amount: u64) -> Result<()> {
        let config = &ctx.accounts.config;
        require!(!config.paused, MannaError::Paused);
        require!(mna_amount > 0, MannaError::ZeroAmount);

        let quote_amount = mna_to_quote_amount(mna_amount)?;
        require!(
            ctx.accounts.quote_vault.amount >= quote_amount,
            MannaError::InsufficientReserve
        );

        let burn_accounts = Burn {
            mint: ctx.accounts.mna_mint.to_account_info(),
            from: ctx.accounts.user_mna_account.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        };
        burn(
            CpiContext::new(
                ctx.accounts.mna_token_program.to_account_info(),
                burn_accounts,
            ),
            mna_amount,
        )?;

        let signer_seeds: &[&[&[u8]]] = &[&[CONFIG_SEED, &[config.bump]]];
        let quote_transfer_accounts = TransferChecked {
            from: ctx.accounts.quote_vault.to_account_info(),
            mint: ctx.accounts.quote_mint.to_account_info(),
            to: ctx.accounts.user_quote_account.to_account_info(),
            authority: ctx.accounts.config.to_account_info(),
        };
        transfer_checked(
            CpiContext::new_with_signer(
                ctx.accounts.quote_token_program.to_account_info(),
                quote_transfer_accounts,
                signer_seeds,
            ),
            quote_amount,
            config.quote_decimals,
        )?;

        emit!(MnaRedeemed {
            user: ctx.accounts.user.key(),
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
    #[account(init, payer = payer, space = 8 + ControllerConfig::INIT_SPACE, seeds = [CONFIG_SEED], bump)]
    pub config: Account<'info, ControllerConfig>,
    pub mna_mint: InterfaceAccount<'info, Mint>,
    pub quote_mint: InterfaceAccount<'info, Mint>,
    #[account(
        init,
        payer = payer,
        associated_token::mint = quote_mint,
        associated_token::authority = config,
        associated_token::token_program = quote_token_program
    )]
    pub quote_vault: InterfaceAccount<'info, TokenAccount>,
    pub mna_token_program: Interface<'info, TokenInterface>,
    pub quote_token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct MintMna<'info> {
    pub user: Signer<'info>,
    #[account(
        seeds = [CONFIG_SEED],
        bump = config.bump,
        constraint = !config.paused @ MannaError::Paused
    )]
    pub config: Account<'info, ControllerConfig>,
    #[account(address = config.mna_mint)]
    pub mna_mint: InterfaceAccount<'info, Mint>,
    #[account(address = config.quote_mint)]
    pub quote_mint: InterfaceAccount<'info, Mint>,
    #[account(
        mut,
        token::mint = quote_mint,
        token::authority = user,
        token::token_program = quote_token_program
    )]
    pub user_quote_account: InterfaceAccount<'info, TokenAccount>,
    #[account(
        mut,
        address = config.quote_vault,
        token::mint = quote_mint,
        token::authority = config,
        token::token_program = quote_token_program
    )]
    pub quote_vault: InterfaceAccount<'info, TokenAccount>,
    #[account(
        mut,
        token::mint = mna_mint,
        token::authority = user,
        token::token_program = mna_token_program
    )]
    pub user_mna_account: InterfaceAccount<'info, TokenAccount>,
    #[account(address = TOKEN_2022_PROGRAM_ID)]
    pub mna_token_program: Interface<'info, TokenInterface>,
    #[account(address = config.quote_token_program)]
    pub quote_token_program: Interface<'info, TokenInterface>,
}

#[derive(Accounts)]
pub struct RedeemMna<'info> {
    pub user: Signer<'info>,
    #[account(
        seeds = [CONFIG_SEED],
        bump = config.bump,
        constraint = !config.paused @ MannaError::Paused
    )]
    pub config: Account<'info, ControllerConfig>,
    #[account(address = config.mna_mint)]
    pub mna_mint: InterfaceAccount<'info, Mint>,
    #[account(address = config.quote_mint)]
    pub quote_mint: InterfaceAccount<'info, Mint>,
    #[account(
        mut,
        token::mint = mna_mint,
        token::authority = user,
        token::token_program = mna_token_program
    )]
    pub user_mna_account: InterfaceAccount<'info, TokenAccount>,
    #[account(
        mut,
        address = config.quote_vault,
        token::mint = quote_mint,
        token::authority = config,
        token::token_program = quote_token_program
    )]
    pub quote_vault: InterfaceAccount<'info, TokenAccount>,
    #[account(
        mut,
        token::mint = quote_mint,
        token::authority = user,
        token::token_program = quote_token_program
    )]
    pub user_quote_account: InterfaceAccount<'info, TokenAccount>,
    #[account(address = TOKEN_2022_PROGRAM_ID)]
    pub mna_token_program: Interface<'info, TokenInterface>,
    #[account(address = config.quote_token_program)]
    pub quote_token_program: Interface<'info, TokenInterface>,
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
}
