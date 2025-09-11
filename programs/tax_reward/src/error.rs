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
    
    #[msg("Invalid Tax Rate - must be <= 10000 bps (100%)")]
    InvalidTaxRate,
    
    #[msg("Invalid Token Account - wrong mint or authority")]
    InvalidTokenAccount,
    
    #[msg("Reward Vault Insufficient Balance")]
    InsufficientRewardVault,
    
    #[msg("DEX Swap Failed")]
    SwapFailed,
    
    #[msg("Program is Paused")]
    ProgramPaused,
    
    #[msg("Invalid Mint Supply")]
    InvalidMintSupply,
}
