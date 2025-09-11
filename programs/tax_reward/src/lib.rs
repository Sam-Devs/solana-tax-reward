// solana_tax_reward program entrypoint using Anchor
use anchor_lang::prelude::*;
use anchor_spl::token::{self, TokenAccount, Token, Mint};
use crate::error::TaxRewardError;

declare_id!("ReplaceWithProgramID");

#[program]
pub mod solana_tax_reward {
    use super::*;

    // Scale factor for cumulative reward accounting (1e18)
    const SCALE: u128 = 1_000_000_000_000_000_000;

    /// Initialize the program; called once by deployer
    pub fn initialize(
        ctx: Context<Initialize>,
        tax_rate_bps: u16,
        dex_program: Pubkey,
    ) -> Result<()> {
        let cfg = &mut ctx.accounts.config;
        cfg.tax_rate_bps = tax_rate_bps;
        cfg.owner = *ctx.accounts.authority.key;
        cfg.dex_program = dex_program;
        cfg.paused = false;

        let global = &mut ctx.accounts.global_state;
        global.total_supply = ctx.accounts.mint.supply;
        global.cum_reward_per_token = 0;

        Ok(())
    }

    /// Handles buys & sells via DEX, taxes, swaps & updates rewards
    pub fn taxed_swap_and_distribute(
        ctx: Context<TaxedSwap>,
        amount_in: u64,
        min_amount_out: u64,
    ) -> Result<()> {
        let cfg = &ctx.accounts.config;
        require!(!cfg.paused, TaxRewardError::Unauthorized);

        // 1. Lazy pull pending rewards before user interaction
        let global = &mut ctx.accounts.global_state;
        let user_info = &mut ctx.accounts.user_info;
        let owed_u128 = (user_info.balance_snapshot as u128)
            .checked_mul(
                global
                    .cum_reward_per_token
                    .checked_sub(user_info.last_cum)
                    .ok_or(TaxRewardError::Overflow)?,
            )
            .ok_or(TaxRewardError::Overflow)?
            .checked_div(SCALE)
            .ok_or(TaxRewardError::Overflow)?;
        let owed = owed_u128 as u64;

        if owed > 0 {
            let rv = ctx.accounts.reward_vault.to_account_info();
            let ix = anchor_lang::solana_program::system_instruction::transfer(
                rv.key,
                ctx.accounts.user_wallet.key,
                owed,
            );
            anchor_lang::solana_program::program::invoke(
                &ix,
                &[
                    rv.clone(),
                    ctx.accounts.user_wallet.to_account_info().clone(),
                    ctx.accounts.system_program.to_account_info().clone(),
                ],
            )?;
        }
        // update user last_cum
        user_info.last_cum = global.cum_reward_per_token;

        // 2. Trigger token swap via DEX adapter (external CPI)
        // record SOL balance before swap
        let rv_info = ctx.accounts.reward_vault.to_account_info();
        let pre_balance = **rv_info.lamports.borrow();

        crate::swap::swap_tokens_for_sol(
            &ctx.program_id,
            &ctx.to_account_infos(),
            amount_in,
            min_amount_out
        )?;

        // calculate delta SOL
        let post_balance = **rv_info.lamports.borrow();
        let delta_sol = post_balance
            .checked_sub(pre_balance)
            .ok_or(TaxRewardError::Overflow)? as u128;

        // slippage protection: revert if received SOL below minimum
        let swapped_amount = delta_sol as u64;
        if swapped_amount < min_amount_out {
            return Err(TaxRewardError::SlippageExceeded.into());
        }

        // 3. Update cumulative reward accounting
        let delta_cum = delta_sol
            .checked_mul(SCALE)
            .ok_or(TaxRewardError::Overflow)?
            .checked_div(global.total_supply as u128)
            .ok_or(TaxRewardError::Overflow)?;
        global.cum_reward_per_token = global
            .cum_reward_per_token
            .checked_add(delta_cum)
            .ok_or(TaxRewardError::Overflow)?;

        // 4. Collect tax from user token account into token vault
        let tax_amount = amount_in
            .checked_mul(cfg.tax_rate_bps as u64)
            .ok_or(TaxRewardError::Overflow)?
            .checked_div(10_000)
            .ok_or(TaxRewardError::Overflow)?;
        token::transfer(ctx.accounts.into_tax_ctx(), tax_amount)?;

        // 5. Snapshot user's new balance
        user_info.balance_snapshot = ctx.accounts.user_token_account.amount;

        Ok(())
    }

    /// Allows any holder to settle pending SOL rewards
    pub fn claim_rewards(ctx: Context<Claim>) -> Result<()> {
        let global = &ctx.accounts.global_state;
        let user_info = &mut ctx.accounts.user_info;

        // calculate owed rewards
        let owed_u128 = (user_info.balance_snapshot as u128)
            .checked_mul(
                global
                    .cum_reward_per_token
                    .checked_sub(user_info.last_cum)
                    .ok_or(TaxRewardError::Overflow)?,
            )
            .ok_or(TaxRewardError::Overflow)?
            .checked_div(SCALE)
            .ok_or(TaxRewardError::Overflow)?;
        let owed = owed_u128 as u64;

        if owed > 0 {
            let rv = ctx.accounts.reward_vault.to_account_info();
            let ix = anchor_lang::solana_program::system_instruction::transfer(
                rv.key,
                ctx.accounts.user_wallet.key,
                owed,
            );
            anchor_lang::solana_program::program::invoke(
                &ix,
                &[
                    rv.clone(),
                    ctx.accounts.user_wallet.to_account_info().clone(),
                    ctx.accounts.system_program.to_account_info().clone(),
                ],
            )?;
        }
        // update snapshot points
        user_info.last_cum = global.cum_reward_per_token;
        user_info.balance_snapshot = ctx.accounts.user_token_account.amount;

        Ok(())
    }

    /// Governance admin: update tax rates, pause/unpause
    pub fn update_config(
        ctx: Context<UpdateConfig>,
        new_tax_rate_bps: u16,
        paused: bool,
    ) -> Result<()> {
        let cfg = &mut ctx.accounts.config;
        require!(
            ctx.accounts.authority.key == &cfg.owner,
            TaxRewardError::Unauthorized
        );
        cfg.tax_rate_bps = new_tax_rate_bps;
        cfg.paused = paused;
        Ok(())
    }

    /// Close and cleanup stale UserInfo account, reclaim rent
    pub fn close_user_info(ctx: Context<CloseUserInfo>) -> Result<()> {
        let user_info = &mut ctx.accounts.user_info;
        user_info.close(ctx.accounts.authority.to_account_info())?;
        Ok(())
    }
}

// Helper for tax token transfer
impl<'info> TaxedSwap<'info> {
    fn into_tax_ctx(&self) -> CpiContext<'_, '_, '_, 'info, token::Transfer<'info>> {
        CpiContext::new(
            self.token_program.to_account_info(),
            token::Transfer {
                from: self.user_token_account.to_account_info(),
                to: self.token_vault.to_account_info(),
                authority: self.user_wallet.to_account_info(),
            },
        )
    }
}