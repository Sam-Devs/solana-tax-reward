use anchor_lang::prelude::*;

/// Holds tax rates, owner, DEX config, paused flag.
#[account]
pub struct Config {
    pub tax_rate_bps: u16,
    pub owner: Pubkey,
    pub dex_program: Pubkey,
    pub paused: bool,
}

impl Config {
    // u16 + Pubkey + Pubkey + bool
    pub const LEN: usize = 2 + 32 + 32 + 1;
}

/// Tracks total supply and cumulative rewards per token (scaled by 1e18).
#[account]
pub struct GlobalState {
    pub total_supply: u64,
    pub cum_reward_per_token: u128,
}

impl GlobalState {
    // u64 + u128
    pub const LEN: usize = 8 + 16;
}

/// User-specific info for reward pulls.
#[account]
pub struct UserInfo {
    pub last_cum: u128,
    pub balance_snapshot: u64,
}

impl UserInfo {
    // u128 + u64
    pub const LEN: usize = 16 + 8;
}