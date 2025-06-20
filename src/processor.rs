//! Processor module for solana_tax_reward

use solana_program::{
    account_info::{AccountInfo, next_account_info},
    entrypoint::ProgramResult,
    pubkey::Pubkey,
    msg,
    program_error::ProgramError,
};

use crate::error::TaxRewardError;
use crate::instructions::TaxRewardInstruction;

/// Entry point for processing instructions
pub fn process(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    msg!("Processor: instruction data length {}", instruction_data.len());

    // Helper functions to load and save FeePool state
    fn load_fee_pool(account: &AccountInfo) -> Result<crate::state::FeePool, ProgramError> {
        let data = &account.data.borrow();
        crate::state::FeePool::try_from_slice(data).map_err(|_| ProgramError::InvalidAccountData)
    }

    fn save_fee_pool(account: &AccountInfo, fee_pool: &crate::state::FeePool) -> ProgramResult {
        let mut data = account.data.borrow_mut();
        fee_pool.serialize(&mut *data).map_err(|_| ProgramError::AccountDataTooSmall)
    }

   :start_line:33
    let instruction = TaxRewardInstruction::unpack(instruction_data)?;
   
    // Helper to create or update snapshot of holder balances
    fn create_or_update_snapshot(
        snapshot_account: &AccountInfo,
        holder_account: &AccountInfo,
        current_snapshot_id: u64,
    ) -> ProgramResult {
        use solana_program::{program_error::ProgramError};
        let mut snapshot = crate::state::Snapshot::try_from_slice(&snapshot_account.data.borrow())
            .unwrap_or_default();
   
        // If snapshot id differs, create new snapshot with cleared balances
        if snapshot.snapshot_id != current_snapshot_id {
            snapshot.snapshot_id = current_snapshot_id;
            snapshot.holder_balances.clear();
        }
   
        // Load current holder balances
        let holder_info = crate::state::HolderInfo::try_from_slice(&holder_account.data.borrow())
            .map_err(|_| ProgramError::InvalidAccountData)?;
   
        // Update or insert the holder balance in snapshot
        let pos = snapshot.holder_balances.iter()
            .position(|(key, _)| key == holder_info.owner);
        if let Some(idx) = pos {
            snapshot.holder_balances[idx] = (holder_info.owner, holder_info.token_balance);
        } else {
            snapshot.holder_balances.push((holder_info.owner, holder_info.token_balance));
        }
   
        snapshot.serialize(&mut *snapshot_account.data.borrow_mut())?;
        Ok(())
    }

    // Swap threshold constant - trigger swap if collected tokens exceed this
   :start_line:36
    const SWAP_THRESHOLD: u64 = 1_000_000; // Example threshold, can adjust per requirements
   
    const CURRENT_SNAPSHOT_ID: u64 = 1; // In real case, this should be advanced per epoch or timing
   
    match instruction {
    	TaxRewardInstruction::Buy { amount } => {
            msg!("Processing Buy instruction; amount: {}", amount);
            let account_info_iter = &mut accounts.iter();
            let fee_pool_account = next_account_info(account_info_iter)?;

            // Load FeePool state
            let mut fee_pool = load_fee_pool(fee_pool_account)?;

            // Calculate 5% tax
            let tax = amount.checked_mul(5).ok_or(TaxRewardError::Overflow)?
                .checked_div(100).ok_or(TaxRewardError::Overflow)?;

            // Calculate net tokens after tax
            let net_amount = amount.checked_sub(tax).ok_or(TaxRewardError::Overflow)?;

            // Accumulate tax to fee pool
            fee_pool.collected_tokens = fee_pool.collected_tokens.checked_add(tax)
                .ok_or(TaxRewardError::Overflow)?;

            // Save updated fee pool
            save_fee_pool(fee_pool_account, &fee_pool)?;

            // Trigger swap if threshold exceeded
            if fee_pool.collected_tokens >= SWAP_THRESHOLD {
                solana_tax_reward::swap::swap_tokens_for_sol(program_id, accounts, fee_pool.collected_tokens)?;

                // Reset fee pool after swapping
                fee_pool.collected_tokens = 0;
                save_fee_pool(fee_pool_account, &fee_pool)?;
            }

            // Perform actual token transfer from buyer to recipient minus tax
            {
                use solana_program::{
                    program::{invoke_signed},
                    system_instruction,
                };
                use spl_token::instruction::transfer as spl_transfer;

                // Next accounts expected:
                // [0] Fee pool account (already consumed)
                // [1] Buyer token account (source)
                // [2] Recipient token account (destination)
                // [3] Token program account
                // [4] Signer (buyer)
                let buyer_token_account = next_account_info(account_info_iter)?;
                let recipient_token_account = next_account_info(account_info_iter)?;
                let token_program_account = next_account_info(account_info_iter)?;
                let signer_account = next_account_info(account_info_iter)?;

                // Transfer net_amount tokens from buyer to recipient
                let ix = spl_transfer(
                    token_program_account.key,
                    buyer_token_account.key,
                    recipient_token_account.key,
                    signer_account.key,
                    &[],
                    net_amount,
                )?;

                invoke_signed(
                    &ix,
                    &[
                        buyer_token_account.clone(),
                        recipient_token_account.clone(),
                        signer_account.clone(),
                        token_program_account.clone(),
                    ],
                    &[],
                )?;
            }

            msg!("Buy processed: net amount: {}, tax collected: {}", net_amount, tax);

         :start_line:112
            Ok(())
           }
           TaxRewardInstruction::Sell { amount } => {
            msg!("Processing Sell instruction; amount: {}", amount);
            let account_info_iter = &mut accounts.iter();
            let fee_pool_account = next_account_info(account_info_iter)?;
         
            // Next accounts expected for snapshot:
            // [.. existing ..]
            // [X] Snapshot state account (to read/write snapshot)
            let snapshot_account = next_account_info(account_info_iter)?;
         
            // Update snapshot with updated holder balance
            create_or_update_snapshot(snapshot_account, fee_pool_account, CURRENT_SNAPSHOT_ID)?;
         

            // Load FeePool state
            let mut fee_pool = load_fee_pool(fee_pool_account)?;

            // Calculate 5% tax
            let tax = amount.checked_mul(5).ok_or(TaxRewardError::Overflow)?
                .checked_div(100).ok_or(TaxRewardError::Overflow)?;

            // Calculate net tokens after tax
            let net_amount = amount.checked_sub(tax).ok_or(TaxRewardError::Overflow)?;

            // Accumulate tax to fee pool
            fee_pool.collected_tokens = fee_pool.collected_tokens.checked_add(tax)
                .ok_or(TaxRewardError::Overflow)?;

            // Save updated fee pool
            save_fee_pool(fee_pool_account, &fee_pool)?;

            // Trigger swap if threshold exceeded
            if fee_pool.collected_tokens >= SWAP_THRESHOLD {
                solana_tax_reward::swap::swap_tokens_for_sol(program_id, accounts, fee_pool.collected_tokens)?;

                // Reset fee pool after swapping
                fee_pool.collected_tokens = 0;
                save_fee_pool(fee_pool_account, &fee_pool)?;
            }

            // Perform actual token transfer from seller to recipient minus tax
            {
                use solana_program::{
                    program::{invoke_signed},
                    system_instruction,
                };
                use spl_token::instruction::transfer as spl_transfer;

                // Next accounts expected:
                // [0] Fee pool account (already consumed)
                // [1] Seller token account (source)
                // [2] Recipient token account (destination)
                // [3] Token program account
                // [4] Signer (seller)
                let seller_token_account = next_account_info(account_info_iter)?;
                let recipient_token_account = next_account_info(account_info_iter)?;
                let token_program_account = next_account_info(account_info_iter)?;
                let signer_account = next_account_info(account_info_iter)?;

                // Transfer net_amount tokens from seller to recipient
                let ix = spl_transfer(
                    token_program_account.key,
                    seller_token_account.key,
                    recipient_token_account.key,
                    signer_account.key,
                    &[],
                    net_amount,
                )?;

                invoke_signed(
                    &ix,
                    &[
                        seller_token_account.clone(),
                        recipient_token_account.clone(),
                        signer_account.clone(),
                        token_program_account.clone(),
                    ],
                    &[],
                )?;
            }

            msg!("Sell processed: net amount: {}, tax collected: {}", net_amount, tax);

            Ok(())
        }
        TaxRewardInstruction::ClaimRewards => {
            msg!("Processing ClaimRewards instruction");
            // Implement rewards claim logic:
            {
                use solana_program::{
                    account_info::AccountInfo, program_error::ProgramError,
                    program::{invoke_signed},
                    pubkey::Pubkey,
                };
                use spl_token::instruction::transfer as spl_transfer;

                // Accounts expected for ClaimRewards:
                // [0] Reward pool account (to debit SOL from)
                // [1] Holder account (owner of tokens, to receive rewards)
                // [2] Token program account
                // [3] Signer (holder)
                let account_info_iter = &mut accounts.iter();
                let reward_pool_account = next_account_info(account_info_iter)?;
                let holder_account = next_account_info(account_info_iter)?;
                let token_program_account = next_account_info(account_info_iter)?;
                let signer_account = next_account_info(account_info_iter)?;

                // Load RewardPool state
                let mut reward_pool = crate::state::RewardPool::try_from_slice(&reward_pool_account.data.borrow())
                    .map_err(|_| ProgramError::InvalidAccountData)?;

                // Load HolderInfo state
                let mut holder_info = crate::state::HolderInfo::try_from_slice(&holder_account.data.borrow())
                    .map_err(|_| ProgramError::InvalidAccountData)?;

                let pending_rewards = holder_info.pending_rewards;
                if pending_rewards == 0 {
                    msg!("No rewards to claim");
                    return Ok(());
                }

                if reward_pool.sol_balance < pending_rewards {
                    msg!("Not enough SOL in reward pool");
                    return Err(ProgramError::InsufficientFunds);
                }

                // Transfer SOL rewards from reward pool account to holder (pseudo transfer, actual SOL transfer may require system instructions)
                **reward_pool_account.try_borrow_mut_lamports()? -= pending_rewards;
                **holder_account.try_borrow_mut_lamports()? += pending_rewards;

                // Reset holder pending_rewards
                holder_info.pending_rewards = 0;
                holder_info.serialize(&mut *holder_account.data.borrow_mut())?;

                // Update reward pool
                reward_pool.sol_balance -= pending_rewards;
                reward_pool.serialize(&mut *reward_pool_account.data.borrow_mut())?;

                msg!("Rewards of {} claimed by holder {}", pending_rewards, holder_account.key);
            }
            Ok(())
        }
    }
}