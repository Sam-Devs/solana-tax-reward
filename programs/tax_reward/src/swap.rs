//! Swap logic to convert collected tokens into SOL rewards via DEX adapters with fallback
use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    pubkey::Pubkey,
    msg,
    program::{invoke_signed},
    instruction::{AccountMeta, Instruction},
    program_error::ProgramError,
};
use spl_token::ID as TOKEN_PROGRAM_ID;

/// Swap collected tokens for SOL using external DEX (primary: Jupiter, fallback: Serum)
pub fn swap_tokens_for_sol(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    token_amount: u64,
    min_amount_out: u64,
) -> ProgramResult {
    // Try primary DEX (Jupiter)
    if let Err(err) = dex_swap_jupiter(program_id, accounts, token_amount) {
        msg!("Primary DEX (Jupiter) swap failed: {:?}", err);
        msg!("Falling back to Serum DEX");
        serum_swap(program_id, accounts, token_amount)?;
    }
    Ok(())
}

fn dex_swap_jupiter(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    _token_amount: u64,
) -> ProgramResult {
    if accounts.len() < 12 {
        msg!("Not enough accounts provided for Jupiter swap");
        return Err(ProgramError::NotEnoughAccountKeys);
    }
    // Stub instruction data for Jupiter swap
    let data = vec![1u8];
    let metas = build_swap_metas(accounts)?;
    let ix = Instruction {
        program_id: *accounts[9].key,
        accounts: metas,
        data,
    };
    invoke_signed(&ix, accounts, &[])?;
    Ok(())
}

fn serum_swap(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    _token_amount: u64,
    min_amount_out: u64,
) -> ProgramResult {
    if accounts.len() < 12 {
        msg!("Not enough accounts provided for Serum swap");
        return Err(ProgramError::NotEnoughAccountKeys);
    }
    // Stub instruction data for Serum swap
    let data = vec![9u8];
    let metas = build_swap_metas(accounts)?;
    let ix = Instruction {
        program_id: *accounts[9].key,
        accounts: metas,
        data,
    };
    invoke_signed(&ix, accounts, &[])?;
    Ok(())
}

fn build_swap_metas(accounts: &[AccountInfo]) -> Result<Vec<AccountMeta>, ProgramError> {
    Ok(vec![
        AccountMeta::new(*accounts[0].key, false),
        AccountMeta::new(*accounts[1].key, false),
        AccountMeta::new(*accounts[2].key, false),
        AccountMeta::new(*accounts[3].key, false),
        AccountMeta::new(*accounts[4].key, false),
        AccountMeta::new(*accounts[5].key, false),
        AccountMeta::new(*accounts[6].key, false),
        AccountMeta::new(*accounts[7].key, false),
        AccountMeta::new_readonly(*accounts[8].key, false),
        AccountMeta::new_readonly(*accounts[9].key, false),
        AccountMeta::new_readonly(*accounts[10].key, false),
        AccountMeta::new_readonly(*accounts[11].key, true),
    ])
}