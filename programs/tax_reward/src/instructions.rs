// Instruction context definitions using Anchor

use anchor_lang::prelude::*;
use anchor_spl::token::{TokenAccount, Token, Mint};
use crate::state::{Config, GlobalState, UserInfo};

#[derive(Accounts)]
#[instruction(tax_rate_bps: u16, dex_program: Pubkey)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    pub mint: Account<'info, Mint>,
    #[account(
        init,
        payer = authority,
        space = Config::LEN,
        seeds = [b"config", program_id.as_ref(), mint.key().as_ref()],
        bump
    )]
    pub config: Account<'info, Config>,
    #[account(
        init,
        payer = authority,
        space = GlobalState::LEN + 8,
        seeds = [b"global", program_id.as_ref(), mint.key().as_ref()],
        bump
    )]
    pub global_state: Account<'info, GlobalState>,
    #[account(
        init,
        payer = authority,
        token::mint = mint,
        token::authority = program_id,
        seeds = [b"token_vault", program_id.as_ref(), mint.key().as_ref()],
        bump
    )]
    pub token_vault: Account<'info, TokenAccount>,
    #[account(
        init,
        payer = authority,
        seeds = [b"reward_vault", program_id.as_ref(), mint.key().as_ref()],
        bump
    )]
    /// CHECK: SOL vault for reward distribution
    pub reward_vault: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct TaxedSwap<'info> {
    #[account(mut, seeds = [b"config", program_id.as_ref(), mint.key().as_ref()], bump)]
    pub config: Account<'info, Config>,
    #[account(mut, seeds = [b"global", program_id.as_ref(), mint.key().as_ref()], bump)]
    pub global_state: Account<'info, GlobalState>,
    #[account(mut, seeds = [b"token_vault", program_id.as_ref(), mint.key().as_ref()], bump)]
    pub token_vault: Account<'info, TokenAccount>,
    #[account(mut, seeds = [b"reward_vault", program_id.as_ref(), mint.key().as_ref()], bump)]
    /// CHECK: SOL vault for distribution
    pub reward_vault: AccountInfo<'info>,
    #[account(
        mut,
        seeds = [b"user", program_id.as_ref(), user_wallet.key().as_ref(), mint.key().as_ref()],
        bump,
        init_if_needed,
        payer = user_wallet,
        space = UserInfo::LEN
    )]
    pub user_info: Account<'info, UserInfo>,
    #[account(mut)]
    pub user_wallet: Signer<'info>,
    pub mint: Account<'info, Mint>,
    #[account(mut)]
    pub user_token_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Claim<'info> {
    #[account(mut, seeds = [b"global", program_id.as_ref(), mint.key().as_ref()], bump)]
    pub global_state: Account<'info, GlobalState>,
    #[account(mut, seeds = [b"reward_vault", program_id.as_ref(), mint.key().as_ref()], bump)]
    /// CHECK: SOL vault for distribution
    pub reward_vault: AccountInfo<'info>,
    #[account(mut, seeds = [b"user", program_id.as_ref(), user_wallet.key().as_ref(), mint.key().as_ref()], bump)]
    pub user_info: Account<'info, UserInfo>,
    #[account(mut)]
    pub user_wallet: Signer<'info>,
    pub mint: Account<'info, Mint>,
    #[account(mut)]
    pub user_token_account: Account<'info, TokenAccount>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateConfig<'info> {
    #[account(
        mut,
        seeds = [b"config", program_id.as_ref(), mint.key().as_ref()],
        bump,
        has_one = owner
    )]
    pub config: Account<'info, Config>,
    /// CHECK: authority must match config.owner
    #[account(signer)]
    pub authority: AccountInfo<'info>,
    pub mint: Account<'info, Mint>,
}

#[derive(Accounts)]
pub struct CloseUserInfo<'info> {
    #[account(
        mut,
        seeds = [b"user", program_id.as_ref(), user_wallet.key().as_ref(), mint.key().as_ref()],
        bump,
        close = authority
    )]
    pub user_info: Account<'info, UserInfo>,
    pub mint: Account<'info, Mint>,
    #[account(mut)]
    pub user_wallet: Signer<'info>,
    pub authority: Signer<'info>,
}