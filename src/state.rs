//! State definitions for solana_tax_reward

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

#[derive(BorshSerialize, BorshDeserialize, Debug, Default)]
pub struct FeePool {
    pub collected_tokens: u64,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Default)]
pub struct RewardPool {
    pub sol_balance: u64,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Default)]
:start_line:17
pub struct HolderInfo {
    pub owner: Pubkey,
    pub token_balance: u64,
    pub pending_rewards: u64,

    // Total rewards received historically by holder (for tracking)
    pub total_claimed_rewards: u64,
:start_line:21
}
#[derive(BorshSerialize, BorshDeserialize, Debug, Default)]
pub struct Snapshot {
    pub snapshot_id: u64,
    pub holder_balances: Vec<(Pubkey, u64)>, // List of (holder, token_balance) at snapshot
}