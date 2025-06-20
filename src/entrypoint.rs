//! Program entrypoint for solana_tax_reward

use solana_program::{
    account_info::AccountInfo,
    entrypoint,
    entrypoint::ProgramResult,
    pubkey::Pubkey,
    msg,
    program_error::ProgramError,
};

use crate::processor::process_instruction;

entrypoint!(process_instruction);

/// Entrypoint of the program
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    msg!("solana_tax_reward: process_instruction called");
    crate::processor::process(program_id, accounts, instruction_data)
}