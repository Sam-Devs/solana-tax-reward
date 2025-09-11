use anchor_lang::prelude::*;

/// Custom errors for solana_tax_reward
#[error_code]
pub enum TaxRewardError {
    #[msg("Invalid Instruction")]
    InvalidInstruction,

    #[msg("Insufficient Funds")]
    InsufficientFunds,

    #[msg("Unauthorized Action")]
    Unauthorized,

    #[msg("Calculation Overflow")]
    Overflow,
    
    #[msg("Slippage Exceeded")]
    SlippageExceeded,
}