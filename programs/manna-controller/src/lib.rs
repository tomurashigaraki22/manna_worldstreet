//! Fixed-rate MNA mint/redemption controller.
//!
//! Written directly against `solana-program`. Everything that crosses the wire
//! is kept byte-compatible with the previous Anchor build:
//!
//!   * instruction discriminators are `sha256("global:<name>")[..8]`
//!   * the config account is `sha256("account:ControllerConfig")[..8]` followed
//!     by the same borsh field order
//!   * events are logged via `sol_log_data` as
//!     `sha256("event:<Name>")[..8] ++ borsh(fields)`
//!   * error codes keep Anchor's 6000-based numbering
//!
//! so the existing TypeScript client needs no changes.

use solana_program::{
    account_info::AccountInfo,
    declare_id,
    entrypoint,
    entrypoint::ProgramResult,
    instruction::{AccountMeta, Instruction},
    log::sol_log_data,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
    sysvar::Sysvar,
};

declare_id!("7QZLtciaBCeLy5uyooZERzBNbPLNuQ6Ppc4iA2kqEsyR");

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const CONFIG_SEED: &[u8] = b"config";

const TOKEN_PROGRAM_ID: Pubkey =
    solana_program::pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
const TOKEN_2022_PROGRAM_ID: Pubkey =
    solana_program::pubkey!("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");
const ASSOCIATED_TOKEN_PROGRAM_ID: Pubkey =
    solana_program::pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");

const MNA_DECIMALS: u8 = 6;
const QUOTE_DECIMALS: u8 = 6;
const RATE_MNA: u64 = 1;
const RATE_QUOTE: u64 = 2;

// sha256("global:<name>")[..8]
const IX_INITIALIZE: [u8; 8] = [175, 175, 109, 31, 13, 152, 155, 237];
const IX_MINT_MNA: [u8; 8] = [119, 88, 201, 132, 36, 17, 173, 104];
const IX_REDEEM_MNA: [u8; 8] = [114, 226, 202, 25, 135, 31, 138, 70];
const IX_SET_PAUSED: [u8; 8] = [91, 60, 125, 192, 176, 225, 166, 218];

// sha256("account:ControllerConfig")[..8]
const CONFIG_DISCRIMINATOR: [u8; 8] = [185, 239, 115, 13, 26, 3, 55, 72];

// sha256("event:<Name>")[..8]
const EV_INITIALIZED: [u8; 8] = [37, 65, 63, 234, 176, 123, 36, 86];
const EV_MINTED: [u8; 8] = [123, 117, 72, 145, 150, 92, 153, 233];
const EV_REDEEMED: [u8; 8] = [180, 69, 217, 28, 217, 166, 34, 34];
const EV_PAUSE_CHANGED: [u8; 8] = [238, 188, 213, 78, 134, 209, 178, 218];

// ---------------------------------------------------------------------------
// Errors — Anchor numbered these from 6000 in declaration order; preserved so
// client-side error matching keeps working.
// ---------------------------------------------------------------------------

#[repr(u32)]
enum MannaError {
    InvalidMnaTokenProgram = 6000,
    InvalidMnaMintAuthority = 6001,
    InvalidMnaDecimals = 6002,
    InvalidQuoteDecimals = 6003,
    Paused = 6004,
    ZeroAmount = 6005,
    InexactConversion = 6006,
    ArithmeticOverflow = 6007,
    InsufficientReserve = 6008,
    InvalidTokenAccount = 6009,
    InvalidTokenProgram = 6010,
}

impl From<MannaError> for ProgramError {
    fn from(e: MannaError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

/// `require!(cond, Err)` without pulling in formatting machinery.
macro_rules! require {
    ($cond:expr, $err:expr) => {
        if !$cond {
            return Err($err.into());
        }
    };
}

// ---------------------------------------------------------------------------
// Config account
//
// Layout (188 bytes): discriminator[8], admin[32], mna_mint[32],
// quote_mint[32], quote_vault[32], quote_token_program[32], mna_decimals u8,
// quote_decimals u8, rate_mna u64le, rate_quote u64le, paused u8, bump u8
// ---------------------------------------------------------------------------

const CONFIG_LEN: usize = 188;
const O_ADMIN: usize = 8;
const O_MNA_MINT: usize = 40;
const O_QUOTE_MINT: usize = 72;
const O_QUOTE_VAULT: usize = 104;
const O_QUOTE_TOKEN_PROGRAM: usize = 136;
const O_MNA_DECIMALS: usize = 168;
const O_QUOTE_DECIMALS: usize = 169;
const O_RATE_MNA: usize = 170;
const O_RATE_QUOTE: usize = 178;
const O_PAUSED: usize = 186;
const O_BUMP: usize = 187;

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

/// Validate that `account` is this program's initialized config PDA.
fn load_config<'a>(account: &AccountInfo<'a>) -> Result<(), ProgramError> {
    require!(account.owner == &crate::ID, MannaError::InvalidTokenAccount);
    let data = account.try_borrow_data()?;
    require!(data.len() >= CONFIG_LEN, MannaError::InvalidTokenAccount);
    require!(
        data[..8] == CONFIG_DISCRIMINATOR,
        MannaError::InvalidTokenAccount
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Raw SPL Token / Token-2022 account access.
//
// Token-2022 keeps the legacy layouts as its base, so one set of offsets reads
// both programs' accounts:
//
//   Mint  (82):  mint_authority COption<Pubkey> [0..36], supply [36..44],
//                decimals [44], is_initialized [45], freeze_authority [46..82]
//   Token (165): mint [0..32], owner [32..64], amount [64..72],
//                delegate [72..108], state [108]
//
// A Token-2022 account carrying extensions is longer than its base and stores
// a type byte at offset 165 (1 = Mint, 2 = Account). That byte is what keeps an
// extended mint from being mistaken for a token account, since an extended mint
// can otherwise reach exactly the 165-byte token account size.
// ---------------------------------------------------------------------------

const MINT_LEN: usize = 82;
const TOKEN_LEN: usize = 165;
const ACCOUNT_TYPE_OFFSET: usize = 165;
const ACCOUNT_TYPE_MINT: u8 = 1;
const ACCOUNT_TYPE_TOKEN: u8 = 2;

struct MintView {
    decimals: u8,
    mint_authority: Option<Pubkey>,
}

fn load_mint(account: &AccountInfo, token_program: &Pubkey) -> Result<MintView, ProgramError> {
    require!(account.owner == token_program, MannaError::InvalidTokenAccount);
    let data = account.try_borrow_data()?;
    let len = data.len();
    require!(len >= MINT_LEN, MannaError::InvalidTokenAccount);
    if len > MINT_LEN {
        require!(
            len > ACCOUNT_TYPE_OFFSET && data[ACCOUNT_TYPE_OFFSET] == ACCOUNT_TYPE_MINT,
            MannaError::InvalidTokenAccount
        );
    }
    require!(data[45] == 1, MannaError::InvalidTokenAccount);

    let mint_authority = if data[0..4] == [1, 0, 0, 0] {
        Some(read_pubkey(&data, 4))
    } else {
        None
    };
    Ok(MintView {
        decimals: data[44],
        mint_authority,
    })
}

struct TokenView {
    mint: Pubkey,
    owner: Pubkey,
    amount: u64,
}

fn load_token_account(
    account: &AccountInfo,
    token_program: &Pubkey,
) -> Result<TokenView, ProgramError> {
    require!(account.owner == token_program, MannaError::InvalidTokenAccount);
    let data = account.try_borrow_data()?;
    let len = data.len();
    require!(len >= TOKEN_LEN, MannaError::InvalidTokenAccount);
    if len > TOKEN_LEN {
        require!(
            len > ACCOUNT_TYPE_OFFSET && data[ACCOUNT_TYPE_OFFSET] == ACCOUNT_TYPE_TOKEN,
            MannaError::InvalidTokenAccount
        );
    }
    // state: 0 = uninitialized, 1 = initialized, 2 = frozen
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
) -> Result<TokenView, ProgramError> {
    let view = load_token_account(account, token_program)?;
    require!(&view.mint == mint, MannaError::InvalidTokenAccount);
    require!(&view.owner == authority, MannaError::InvalidTokenAccount);
    Ok(view)
}

fn require_signer(account: &AccountInfo) -> ProgramResult {
    require!(account.is_signer, ProgramError::MissingRequiredSignature);
    Ok(())
}

fn require_key(account: &AccountInfo, expected: &Pubkey) -> ProgramResult {
    require!(account.key == expected, MannaError::InvalidTokenAccount);
    Ok(())
}

// ---------------------------------------------------------------------------
// Token CPIs. Instruction tags are stable across Token and Token-2022.
// ---------------------------------------------------------------------------

const TAG_MINT_TO: u8 = 7;
const TAG_BURN: u8 = 8;
const TAG_TRANSFER_CHECKED: u8 = 12;

fn amount_data(tag: u8, amount: u64) -> [u8; 9] {
    let mut data = [0u8; 9];
    data[0] = tag;
    data[1..9].copy_from_slice(&amount.to_le_bytes());
    data
}

#[allow(clippy::too_many_arguments)]
fn transfer_checked<'a>(
    token_program: &AccountInfo<'a>,
    from: &AccountInfo<'a>,
    mint: &AccountInfo<'a>,
    to: &AccountInfo<'a>,
    authority: &AccountInfo<'a>,
    amount: u64,
    decimals: u8,
    signer_seeds: Option<&[&[&[u8]]]>,
) -> ProgramResult {
    let mut data = [0u8; 10];
    data[0] = TAG_TRANSFER_CHECKED;
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
        Some(seeds) => invoke_signed(&ix, &infos, seeds),
        None => invoke(&ix, &infos),
    }
}

fn mint_to<'a>(
    token_program: &AccountInfo<'a>,
    mint: &AccountInfo<'a>,
    to: &AccountInfo<'a>,
    authority: &AccountInfo<'a>,
    amount: u64,
    signer_seeds: &[&[&[u8]]],
) -> ProgramResult {
    let ix = Instruction {
        program_id: *token_program.key,
        accounts: vec![
            AccountMeta::new(*mint.key, false),
            AccountMeta::new(*to.key, false),
            AccountMeta::new_readonly(*authority.key, true),
        ],
        data: amount_data(TAG_MINT_TO, amount).to_vec(),
    };
    invoke_signed(
        &ix,
        &[
            mint.clone(),
            to.clone(),
            authority.clone(),
            token_program.clone(),
        ],
        signer_seeds,
    )
}

fn burn<'a>(
    token_program: &AccountInfo<'a>,
    from: &AccountInfo<'a>,
    mint: &AccountInfo<'a>,
    authority: &AccountInfo<'a>,
    amount: u64,
) -> ProgramResult {
    let ix = Instruction {
        program_id: *token_program.key,
        accounts: vec![
            AccountMeta::new(*from.key, false),
            AccountMeta::new(*mint.key, false),
            AccountMeta::new_readonly(*authority.key, true),
        ],
        data: amount_data(TAG_BURN, amount).to_vec(),
    };
    invoke(
        &ix,
        &[
            from.clone(),
            mint.clone(),
            authority.clone(),
            token_program.clone(),
        ],
    )
}

/// Associated Token Program `Create` (tag 0): fails if the ATA already exists,
/// matching the old `init` constraint. The ATA program derives the address
/// itself and rejects a mismatch, which is what pins the vault to the canonical
/// ATA of (config, quote_mint).
#[allow(clippy::too_many_arguments)]
fn create_associated_token_account<'a>(
    ata_program: &AccountInfo<'a>,
    payer: &AccountInfo<'a>,
    ata: &AccountInfo<'a>,
    owner: &AccountInfo<'a>,
    mint: &AccountInfo<'a>,
    system_program: &AccountInfo<'a>,
    token_program: &AccountInfo<'a>,
) -> ProgramResult {
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
    )
}

// ---------------------------------------------------------------------------
// Rate conversion
// ---------------------------------------------------------------------------

fn quote_amount_to_mna(quote_amount: u64) -> Result<u64, ProgramError> {
    require!(
        quote_amount % RATE_QUOTE == 0,
        MannaError::InexactConversion
    );
    let scaled = quote_amount
        .checked_mul(RATE_MNA)
        .ok_or(MannaError::ArithmeticOverflow)?;
    Ok(scaled / RATE_QUOTE)
}

fn mna_to_quote_amount(mna_amount: u64) -> Result<u64, ProgramError> {
    mna_amount
        .checked_mul(RATE_QUOTE)
        .ok_or(MannaError::ArithmeticOverflow.into())
}

// ---------------------------------------------------------------------------
// Entrypoint
// ---------------------------------------------------------------------------

entrypoint!(process_instruction);

fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: &[u8],
) -> ProgramResult {
    require!(program_id == &crate::ID, ProgramError::IncorrectProgramId);
    require!(data.len() >= 8, ProgramError::InvalidInstructionData);
    let (tag, args) = data.split_at(8);

    match *<&[u8; 8]>::try_from(tag).map_err(|_| ProgramError::InvalidInstructionData)? {
        IX_INITIALIZE => initialize(accounts),
        IX_MINT_MNA => {
            require!(args.len() >= 8, ProgramError::InvalidInstructionData);
            mint_mna(accounts, read_u64(args, 0))
        }
        IX_REDEEM_MNA => {
            require!(args.len() >= 8, ProgramError::InvalidInstructionData);
            redeem_mna(accounts, read_u64(args, 0))
        }
        IX_SET_PAUSED => {
            require!(!args.is_empty(), ProgramError::InvalidInstructionData);
            set_paused(accounts, args[0] != 0)
        }
        _ => Err(ProgramError::InvalidInstructionData),
    }
}

// ---------------------------------------------------------------------------
// initialize
// ---------------------------------------------------------------------------

fn initialize(accounts: &[AccountInfo]) -> ProgramResult {
    let [payer, admin, config, mna_mint, quote_mint, quote_vault, mna_token_program, quote_token_program, associated_token_program, system_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    require_signer(payer)?;
    require_signer(admin)?;
    require_key(mna_token_program, &TOKEN_2022_PROGRAM_ID)
        .map_err(|_| ProgramError::from(MannaError::InvalidMnaTokenProgram))?;
    require_key(associated_token_program, &ASSOCIATED_TOKEN_PROGRAM_ID)
        .map_err(|_| ProgramError::from(MannaError::InvalidTokenProgram))?;
    require!(
        quote_token_program.key == &TOKEN_PROGRAM_ID
            || quote_token_program.key == &TOKEN_2022_PROGRAM_ID,
        MannaError::InvalidTokenProgram
    );
    require_key(system_program, &solana_program::system_program::ID)?;

    // Derive and verify the config PDA.
    let (config_key, bump) = Pubkey::find_program_address(&[CONFIG_SEED], &crate::ID);
    require_key(config, &config_key)?;
    // `init` semantics: refuse to re-initialize.
    require!(
        config.data_is_empty() && config.owner == &solana_program::system_program::ID,
        MannaError::InvalidTokenAccount
    );

    let mna = load_mint(mna_mint, &TOKEN_2022_PROGRAM_ID)?;
    require!(
        mna.mint_authority == Some(config_key),
        MannaError::InvalidMnaMintAuthority
    );
    require!(mna.decimals == MNA_DECIMALS, MannaError::InvalidMnaDecimals);

    let quote = load_mint(quote_mint, quote_token_program.key)?;
    require!(
        quote.decimals == QUOTE_DECIMALS,
        MannaError::InvalidQuoteDecimals
    );

    // Create the config PDA.
    let lamports = Rent::get()?.minimum_balance(CONFIG_LEN);
    let seeds: &[&[u8]] = &[CONFIG_SEED, &[bump]];
    invoke_signed(
        &system_instruction::create_account(
            payer.key,
            &config_key,
            lamports,
            CONFIG_LEN as u64,
            &crate::ID,
        ),
        &[payer.clone(), config.clone(), system_program.clone()],
        &[seeds],
    )?;

    create_associated_token_account(
        associated_token_program,
        payer,
        quote_vault,
        config,
        quote_mint,
        system_program,
        quote_token_program,
    )?;
    require_token_account(
        quote_vault,
        quote_token_program.key,
        quote_mint.key,
        &config_key,
    )?;

    {
        let mut data = config.try_borrow_mut_data()?;
        data[..8].copy_from_slice(&CONFIG_DISCRIMINATOR);
        data[O_ADMIN..O_ADMIN + 32].copy_from_slice(admin.key.as_ref());
        data[O_MNA_MINT..O_MNA_MINT + 32].copy_from_slice(mna_mint.key.as_ref());
        data[O_QUOTE_MINT..O_QUOTE_MINT + 32].copy_from_slice(quote_mint.key.as_ref());
        data[O_QUOTE_VAULT..O_QUOTE_VAULT + 32].copy_from_slice(quote_vault.key.as_ref());
        data[O_QUOTE_TOKEN_PROGRAM..O_QUOTE_TOKEN_PROGRAM + 32]
            .copy_from_slice(quote_token_program.key.as_ref());
        data[O_MNA_DECIMALS] = MNA_DECIMALS;
        data[O_QUOTE_DECIMALS] = QUOTE_DECIMALS;
        data[O_RATE_MNA..O_RATE_MNA + 8].copy_from_slice(&RATE_MNA.to_le_bytes());
        data[O_RATE_QUOTE..O_RATE_QUOTE + 8].copy_from_slice(&RATE_QUOTE.to_le_bytes());
        data[O_PAUSED] = 0;
        data[O_BUMP] = bump;
    }

    let mut event = [0u8; 136];
    event[..8].copy_from_slice(&EV_INITIALIZED);
    event[8..40].copy_from_slice(admin.key.as_ref());
    event[40..72].copy_from_slice(mna_mint.key.as_ref());
    event[72..104].copy_from_slice(quote_mint.key.as_ref());
    event[104..136].copy_from_slice(quote_vault.key.as_ref());
    sol_log_data(&[&event]);

    Ok(())
}

// ---------------------------------------------------------------------------
// mint_mna
// ---------------------------------------------------------------------------

fn mint_mna(accounts: &[AccountInfo], quote_amount: u64) -> ProgramResult {
    let [user, config, mna_mint, quote_mint, user_quote_account, quote_vault, user_mna_account, mna_token_program, quote_token_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    require_signer(user)?;
    load_config(config)?;

    let (cfg_mna_mint, cfg_quote_mint, cfg_vault, cfg_quote_program, quote_decimals, bump) = {
        let data = config.try_borrow_data()?;
        require!(data[O_PAUSED] == 0, MannaError::Paused);
        (
            read_pubkey(&data, O_MNA_MINT),
            read_pubkey(&data, O_QUOTE_MINT),
            read_pubkey(&data, O_QUOTE_VAULT),
            read_pubkey(&data, O_QUOTE_TOKEN_PROGRAM),
            data[O_QUOTE_DECIMALS],
            data[O_BUMP],
        )
    };

    require!(quote_amount > 0, MannaError::ZeroAmount);

    require_key(mna_token_program, &TOKEN_2022_PROGRAM_ID)
        .map_err(|_| ProgramError::from(MannaError::InvalidMnaTokenProgram))?;
    require_key(quote_token_program, &cfg_quote_program)
        .map_err(|_| ProgramError::from(MannaError::InvalidTokenProgram))?;
    require_key(mna_mint, &cfg_mna_mint)?;
    require_key(quote_mint, &cfg_quote_mint)?;
    require_key(quote_vault, &cfg_vault)?;

    require_token_account(
        user_quote_account,
        quote_token_program.key,
        &cfg_quote_mint,
        user.key,
    )?;
    require_token_account(
        quote_vault,
        quote_token_program.key,
        &cfg_quote_mint,
        config.key,
    )?;
    require_token_account(
        user_mna_account,
        &TOKEN_2022_PROGRAM_ID,
        &cfg_mna_mint,
        user.key,
    )?;

    let mna_amount = quote_amount_to_mna(quote_amount)?;

    transfer_checked(
        quote_token_program,
        user_quote_account,
        quote_mint,
        quote_vault,
        user,
        quote_amount,
        quote_decimals,
        None,
    )?;

    let seeds: &[&[u8]] = &[CONFIG_SEED, &[bump]];
    mint_to(
        mna_token_program,
        mna_mint,
        user_mna_account,
        config,
        mna_amount,
        &[seeds],
    )?;

    let mut event = [0u8; 56];
    event[..8].copy_from_slice(&EV_MINTED);
    event[8..40].copy_from_slice(user.key.as_ref());
    event[40..48].copy_from_slice(&quote_amount.to_le_bytes());
    event[48..56].copy_from_slice(&mna_amount.to_le_bytes());
    sol_log_data(&[&event]);

    Ok(())
}

// ---------------------------------------------------------------------------
// redeem_mna
// ---------------------------------------------------------------------------

fn redeem_mna(accounts: &[AccountInfo], mna_amount: u64) -> ProgramResult {
    let [user, config, mna_mint, quote_mint, user_mna_account, quote_vault, user_quote_account, mna_token_program, quote_token_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    require_signer(user)?;
    load_config(config)?;

    let (cfg_mna_mint, cfg_quote_mint, cfg_vault, cfg_quote_program, quote_decimals, bump) = {
        let data = config.try_borrow_data()?;
        require!(data[O_PAUSED] == 0, MannaError::Paused);
        (
            read_pubkey(&data, O_MNA_MINT),
            read_pubkey(&data, O_QUOTE_MINT),
            read_pubkey(&data, O_QUOTE_VAULT),
            read_pubkey(&data, O_QUOTE_TOKEN_PROGRAM),
            data[O_QUOTE_DECIMALS],
            data[O_BUMP],
        )
    };

    require!(mna_amount > 0, MannaError::ZeroAmount);

    require_key(mna_token_program, &TOKEN_2022_PROGRAM_ID)
        .map_err(|_| ProgramError::from(MannaError::InvalidMnaTokenProgram))?;
    require_key(quote_token_program, &cfg_quote_program)
        .map_err(|_| ProgramError::from(MannaError::InvalidTokenProgram))?;
    require_key(mna_mint, &cfg_mna_mint)?;
    require_key(quote_mint, &cfg_quote_mint)?;
    require_key(quote_vault, &cfg_vault)?;

    require_token_account(
        user_mna_account,
        &TOKEN_2022_PROGRAM_ID,
        &cfg_mna_mint,
        user.key,
    )?;
    let vault = require_token_account(
        quote_vault,
        quote_token_program.key,
        &cfg_quote_mint,
        config.key,
    )?;
    require_token_account(
        user_quote_account,
        quote_token_program.key,
        &cfg_quote_mint,
        user.key,
    )?;

    let quote_amount = mna_to_quote_amount(mna_amount)?;
    require!(
        vault.amount >= quote_amount,
        MannaError::InsufficientReserve
    );

    burn(
        mna_token_program,
        user_mna_account,
        mna_mint,
        user,
        mna_amount,
    )?;

    let seeds: &[&[u8]] = &[CONFIG_SEED, &[bump]];
    transfer_checked(
        quote_token_program,
        quote_vault,
        quote_mint,
        user_quote_account,
        config,
        quote_amount,
        quote_decimals,
        Some(&[seeds]),
    )?;

    let mut event = [0u8; 56];
    event[..8].copy_from_slice(&EV_REDEEMED);
    event[8..40].copy_from_slice(user.key.as_ref());
    event[40..48].copy_from_slice(&mna_amount.to_le_bytes());
    event[48..56].copy_from_slice(&quote_amount.to_le_bytes());
    sol_log_data(&[&event]);

    Ok(())
}

// ---------------------------------------------------------------------------
// set_paused
// ---------------------------------------------------------------------------

fn set_paused(accounts: &[AccountInfo], paused: bool) -> ProgramResult {
    let [config, admin] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    require_signer(admin)?;
    load_config(config)?;

    let mut data = config.try_borrow_mut_data()?;
    require!(
        read_pubkey(&data, O_ADMIN) == *admin.key,
        MannaError::InvalidTokenAccount
    );
    data[O_PAUSED] = paused as u8;
    drop(data);

    let mut event = [0u8; 9];
    event[..8].copy_from_slice(&EV_PAUSE_CHANGED);
    event[8] = paused as u8;
    sol_log_data(&[&event]);

    Ok(())
}
