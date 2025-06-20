//! Instruction definitions and unpacking for solana_tax_reward

use solana_program::program_error::ProgramError;
use std::convert::TryInto;

/// TaxReward program instructions
#[derive(Debug)]
pub enum TaxRewardInstruction {
    /// Buy tokens with tax applied
    /// Fields: amount of tokens
    Buy {
        amount: u64,
    },

    /// Sell tokens with tax applied
    /// Fields: amount of tokens
    Sell {
        amount: u64,
    },

    /// Claim accumulated rewards
    ClaimRewards,
}

impl TaxRewardInstruction {
    /// Unpack binary instruction data into TaxRewardInstruction enum
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        let (&tag, rest) = input.split_first().ok_or(ProgramError::InvalidInstructionData)?;

        Ok(match tag {
            0 => {
                let amount = Self::unpack_amount(rest)?;
                Self::Buy { amount }
            }
            1 => {
                let amount = Self::unpack_amount(rest)?;
                Self::Sell { amount }
            }
            2 => Self::ClaimRewards,
            _ => return Err(ProgramError::InvalidInstructionData),
        })
    }

    fn unpack_amount(input: &[u8]) -> Result<u64, ProgramError> {
        if input.len() < 8 {
            return Err(ProgramError::InvalidInstructionData);
        }
        let amount = input[..8].try_into().map(u64::from_le_bytes).map_err(|_| ProgramError::InvalidInstructionData)?;
        Ok(amount)
    }
}