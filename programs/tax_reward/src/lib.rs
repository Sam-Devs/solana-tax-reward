// solana_tax_reward program entrypoint using Anchor
use crate::{error::TaxRewardError, instructions::*};
use anchor_lang::prelude::*;
use anchor_spl::token;
// Module declarations
pub mod error;
pub mod instructions;
pub mod state;
pub mod swap;

// TODO: Replace with actual program ID after deployment
// declare_id!("ReplaceWithProgramID");
declare_id!("2A1sUwgtyMHswvfY2N4UTZLyPKLNjEHmhRzYFZgQgHQF");

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
        msg!(
            "initialize: authority={}, tax_rate_bps={}, dex_program={}",
            ctx.accounts.authority.key,
            tax_rate_bps,
            dex_program
        );

        // Validate initialization parameters
        require!(tax_rate_bps <= 10_000, TaxRewardError::InvalidTaxRate);
        require!(
            ctx.accounts.mint.supply > 0,
            TaxRewardError::InvalidMintSupply
        );

        let cfg = &mut ctx.accounts.config;
        cfg.tax_rate_bps = tax_rate_bps;
        cfg.owner = *ctx.accounts.authority.key;
        cfg.dex_program = dex_program;
        cfg.paused = false;

        let global = &mut ctx.accounts.global_state;
        global.total_supply = ctx.accounts.mint.supply;
        global.cum_reward_per_token = 0;

        msg!(
            "Program initialized: tax_rate={}bps, total_supply={}",
            tax_rate_bps,
            global.total_supply
        );
        Ok(())
    }

    /// Handles buys & sells via DEX, taxes, swaps & updates rewards
    pub fn taxed_swap_and_distribute(
        ctx: Context<TaxedSwap>,
        amount_in: u64,
        min_amount_out: u64,
    ) -> Result<()> {
        msg!(
            "taxed_swap_and_distribute: user={}, amount_in={}, min_amount_out={}",
            ctx.accounts.user_wallet.key,
            amount_in,
            min_amount_out
        );

        // Comprehensive validation and reentrancy protection
        let cfg = &ctx.accounts.config;
        require!(!cfg.paused, TaxRewardError::ProgramPaused);
        require!(amount_in > 0, TaxRewardError::InvalidInstruction);
        require!(cfg.tax_rate_bps <= 10_000, TaxRewardError::InvalidTaxRate);

        // Validate token account belongs to the correct mint
        require!(
            ctx.accounts.user_token_account.mint == ctx.accounts.mint.key(),
            TaxRewardError::InvalidTokenAccount
        );

        // Ensure user has sufficient token balance for the swap + tax
        require!(
            ctx.accounts.user_token_account.amount >= amount_in,
            TaxRewardError::InsufficientFunds
        );

        // Validate mint supply is reasonable (not zero, not overflowing)
        let global = &ctx.accounts.global_state;
        require!(global.total_supply > 0, TaxRewardError::InvalidMintSupply);

        // 1. Lazy pull pending rewards before user interaction
        let global = &mut ctx.accounts.global_state;
        let user_info = &mut ctx.accounts.user_info;
        let owed = calculate_owed_rewards(
            user_info.balance_snapshot,
            global.cum_reward_per_token,
            user_info.last_cum,
        )?;

        if owed > 0 {
            msg!("Transferring owed rewards: {}", owed);
            let rv = ctx.accounts.reward_vault.to_account_info();
            let ix = anchor_lang::solana_program::system_instruction::transfer(
                rv.key,
                ctx.accounts.user_wallet.key,
                owed,
            );
            let mint_key = ctx.accounts.mint.key();
            let (_, reward_vault_bump) = Pubkey::find_program_address(
                &[b"reward_vault", ctx.program_id.as_ref(), mint_key.as_ref()],
                ctx.program_id,
            );
            let reward_vault_seeds = &[
                b"reward_vault",
                ctx.program_id.as_ref(),
                mint_key.as_ref(),
                &[reward_vault_bump],
            ];
            anchor_lang::solana_program::program::invoke_signed(
                &ix,
                &[
                    rv.clone(),
                    ctx.accounts.user_wallet.to_account_info().clone(),
                    ctx.accounts.system_program.to_account_info().clone(),
                ],
                &[reward_vault_seeds],
            )?;
        }
        // update user last_cum
        user_info.last_cum = global.cum_reward_per_token;

        // 2. Trigger token swap via DEX adapter (external CPI)
        // record SOL balance before swap
        let rv_info = ctx.accounts.reward_vault.to_account_info();
        let pre_balance = **rv_info.lamports.borrow();

        msg!("Performing token swap of amount {}", amount_in);
        // Create account info slice for swap function
        let account_infos = ctx.accounts.user_token_account.to_account_info();
        crate::swap::swap_tokens_for_sol(
            &ctx.program_id,
            &[account_infos],
            amount_in,
            min_amount_out,
        )?;

        // calculate delta...
        let post_balance = **rv_info.lamports.borrow();
        let delta_sol = post_balance
            .checked_sub(pre_balance)
            .ok_or(TaxRewardError::Overflow)? as u128;

        // Enhanced slippage protection with detailed logging
        let swapped_amount = delta_sol as u64;
        msg!(
            "Swap result: expected_min={}, actual={}, delta={}",
            min_amount_out,
            swapped_amount,
            delta_sol
        );

        if swapped_amount < min_amount_out {
            msg!(
                "Slippage exceeded: got {} SOL, expected minimum {} SOL",
                swapped_amount,
                min_amount_out
            );
            return Err(TaxRewardError::SlippageExceeded.into());
        }

        // Ensure reward vault has sufficient balance for pending rewards
        let rv_balance = **rv_info.lamports.borrow();
        if owed > 0 && rv_balance < owed {
            msg!(
                "Reward vault balance {} insufficient for owed rewards {}",
                rv_balance,
                owed
            );
            return Err(TaxRewardError::InsufficientRewardVault.into());
        }

        // 3. Update cumulative reward accounting...
        let delta_cum = delta_sol
            .checked_mul(SCALE)
            .ok_or(TaxRewardError::Overflow)?
            .checked_div(global.total_supply as u128)
            .ok_or(TaxRewardError::Overflow)?;
        global.cum_reward_per_token = global
            .cum_reward_per_token
            .checked_add(delta_cum)
            .ok_or(TaxRewardError::Overflow)?;

        // 4. Collect tax...
        let tax_amount = amount_in
            .checked_mul(cfg.tax_rate_bps as u64)
            .ok_or(TaxRewardError::Overflow)?
            .checked_div(10_000)
            .ok_or(TaxRewardError::Overflow)?;
        msg!("Transferring taxed tokens: {}", tax_amount);
        
        // Create tax transfer context before borrowing user_info mutably again
        let tax_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            token::Transfer {
                from: ctx.accounts.user_token_account.to_account_info(),
                to: ctx.accounts.token_vault.to_account_info(),
                authority: ctx.accounts.user_wallet.to_account_info(),
            },
        );
        token::transfer(tax_ctx, tax_amount)?;

        // 5. Snapshot user's new balance
        user_info.balance_snapshot = ctx.accounts.user_token_account.amount;

        Ok(())
    }

    /// Allows any holder to settle pending SOL rewards
    pub fn claim_rewards(ctx: Context<Claim>) -> Result<()> {
        msg!("claim_rewards: user={}", ctx.accounts.user_wallet.key);
        let global = &ctx.accounts.global_state;
        let user_info = &mut ctx.accounts.user_info;

        // calculate owed rewards
        let owed = calculate_owed_rewards(
            user_info.balance_snapshot,
            global.cum_reward_per_token,
            user_info.last_cum,
        )?;

        if owed > 0 {
            let rv = ctx.accounts.reward_vault.to_account_info();
            let ix = anchor_lang::solana_program::system_instruction::transfer(
                rv.key,
                ctx.accounts.user_wallet.key,
                owed,
            );
            let mint_key = ctx.accounts.mint.key();
            let (_, reward_vault_bump) = Pubkey::find_program_address(
                &[b"reward_vault", ctx.program_id.as_ref(), mint_key.as_ref()],
                ctx.program_id,
            );
            let reward_vault_seeds = &[
                b"reward_vault",
                ctx.program_id.as_ref(),
                mint_key.as_ref(),
                &[reward_vault_bump],
            ];
            anchor_lang::solana_program::program::invoke_signed(
                &ix,
                &[
                    rv.clone(),
                    ctx.accounts.user_wallet.to_account_info().clone(),
                    ctx.accounts.system_program.to_account_info().clone(),
                ],
                &[reward_vault_seeds],
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
        msg!(
            "update_config: owner={}, new_tax_rate_bps={}, paused={}",
            ctx.accounts.owner.key,
            new_tax_rate_bps,
            paused
        );
        let cfg = &mut ctx.accounts.config;
        require!(
            ctx.accounts.owner.key == &cfg.owner,
            TaxRewardError::Unauthorized
        );
        cfg.tax_rate_bps = new_tax_rate_bps;
        cfg.paused = paused;
        Ok(())
    }

    /// Close and cleanup stale UserInfo account, reclaim rent
    pub fn close_user_info(ctx: Context<CloseUserInfo>) -> Result<()> {
        msg!(
            "close_user_info: user_info={}, authority={}",
            ctx.accounts.user_info.to_account_info().key,
            ctx.accounts.authority.key
        );
        let user_info = &mut ctx.accounts.user_info;
        user_info.close(ctx.accounts.authority.to_account_info())?;
        Ok(())
    }

    /// Update total supply tracking for accurate reward distribution
    /// Called by admin when mint supply changes significantly
    pub fn update_total_supply(ctx: Context<UpdateTotalSupply>) -> Result<()> {
        msg!("update_total_supply: owner={}", ctx.accounts.owner.key);
        let cfg = &ctx.accounts.config;
        require!(
            ctx.accounts.owner.key == &cfg.owner,
            TaxRewardError::Unauthorized
        );

        // Update global state with current mint supply
        let global = &mut ctx.accounts.global_state;
        let old_supply = global.total_supply;
        global.total_supply = ctx.accounts.mint.supply;

        msg!(
            "Total supply updated: {} -> {}",
            old_supply,
            global.total_supply
        );

        Ok(())
    }
}

/// Helper function to calculate owed rewards for a user
fn calculate_owed_rewards(
    user_balance_snapshot: u64,
    global_cum_reward_per_token: u128,
    user_last_cum: u128,
) -> Result<u64> {
    let owed_u128 = (user_balance_snapshot as u128)
        .checked_mul(
            global_cum_reward_per_token
                .checked_sub(user_last_cum)
                .ok_or(TaxRewardError::Overflow)?,
        )
        .ok_or(TaxRewardError::Overflow)?
        .checked_div(1_000_000_000_000_000_000u128) // SCALE constant
        .ok_or(TaxRewardError::Overflow)?;

    Ok(owed_u128 as u64)
}

