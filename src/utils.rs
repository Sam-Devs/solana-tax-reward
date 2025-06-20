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