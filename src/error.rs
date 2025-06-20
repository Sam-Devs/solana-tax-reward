//! Custom error definitions for solana_tax_reward

use thiserror::Error;
use solana_program::program_error::ProgramError;

#[derive(Error, Debug, Copy, Clone)]
pub enum TaxRewardError {
    #[error("Invalid Instruction")]
    InvalidInstruction,

    #[error("Insufficient Funds")]
    InsufficientFunds,

    #[error("Unauthorized Action")]
    Unauthorized,

    #[error("Calculation Overflow")]
    Overflow,
}

impl From<TaxRewardError> for ProgramError {
    fn from(e: TaxRewardError) -> Self {
        ProgramError::Custom(e as u32)
    }
}