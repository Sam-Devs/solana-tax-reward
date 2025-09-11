//! Utility functions for solana_tax_reward

use solana_program::program_error::ProgramError;

/// Safe multiplication and division to avoid overflow, returns Result
pub fn safe_mul_div(a: u64, b: u64, divisor: u64) -> Result<u64, ProgramError> {
    if divisor == 0 {
        return Err(ProgramError::InvalidInstructionData);
    }
    a.checked_mul(b)
        .and_then(|mul| mul.checked_div(divisor))
        .ok_or(ProgramError::InvalidInstructionData)
}
// PDA utilities for Account & PDA Layout
use solana_program::pubkey::Pubkey;

/// Get Config PDA: ["config", program_id, mint]
pub fn get_config_pda(mint: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"config", program_id.as_ref(), mint.as_ref()], program_id)
}

/// Get GlobalState PDA: ["global", program_id, mint]
pub fn get_global_state_pda(mint: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"global", program_id.as_ref(), mint.as_ref()], program_id)
}

/// Get TokenVault PDA: ["token_vault", program_id, mint]
pub fn get_token_vault_pda(mint: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"token_vault", program_id.as_ref(), mint.as_ref()], program_id)
}

/// Get RewardVault PDA: ["reward_vault", program_id, mint]
pub fn get_reward_vault_pda(mint: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"reward_vault", program_id.as_ref(), mint.as_ref()], program_id)
}

/// Get UserInfo PDA: ["user", program_id, user_pubkey, mint]
pub fn get_user_info_pda(user: &Pubkey, mint: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"user", program_id.as_ref(), user.as_ref(), mint.as_ref()],
        program_id,
    )
}