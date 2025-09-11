use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount};
use solana_program::system_program;
use solana_program_test::*;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
    system_instruction,
};

use solana_tax_reward::{
    state::{Config, GlobalState, UserInfo},
    error::TaxRewardError,
};

/// Test environment setup helper
pub struct TestEnvironment {
    pub context: ProgramTestContext,
    pub program_id: Pubkey,
    pub mint: Keypair,
    pub mint_authority: Keypair,
}

impl TestEnvironment {
    /// Initialize a complete test environment with SPL token support
    pub async fn new() -> Self {
        let program_id = solana_tax_reward::id();
        let mut program_test = ProgramTest::new(
            "solana_tax_reward",
            program_id,
            processor!(solana_tax_reward::entry),
        );
        
        // Add SPL Token program
        program_test.add_program(
            "spl_token",
            spl_token::id(),
            processor!(spl_token::processor::Processor::process),
        );
        
        let context = program_test.start_with_context().await;
        let mint = Keypair::new();
        let mint_authority = Keypair::new();
        
        Self {
            context,
            program_id,
            mint,
            mint_authority,
        }
    }
    
    /// Create a mint account for testing
    pub async fn create_mint(&mut self, decimals: u8) -> Result<(), Box<dyn std::error::Error>> {
        let rent = self.context.banks_client.get_rent().await?;
        let mint_rent = rent.minimum_balance(Mint::LEN);
        
        let create_mint_ix = system_instruction::create_account(
            &self.context.payer.pubkey(),
            &self.mint.pubkey(),
            mint_rent,
            Mint::LEN as u64,
            &spl_token::id(),
        );
        
        let init_mint_ix = spl_token::instruction::initialize_mint(
            &spl_token::id(),
            &self.mint.pubkey(),
            &self.mint_authority.pubkey(),
            None,
            decimals,
        )?;
        
        let recent_blockhash = self.context.banks_client.get_recent_blockhash().await?;
        let transaction = Transaction::new_signed_with_payer(
            &[create_mint_ix, init_mint_ix],
            Some(&self.context.payer.pubkey()),
            &[&self.context.payer, &self.mint],
            recent_blockhash,
        );
        
        self.context.banks_client.process_transaction(transaction).await?;
        Ok(())
    }
    
    /// Create a token account for a user
    pub async fn create_token_account(
        &mut self,
        owner: &Pubkey,
    ) -> Result<Pubkey, Box<dyn std::error::Error>> {
        let token_account = Keypair::new();
        let rent = self.context.banks_client.get_rent().await?;
        let account_rent = rent.minimum_balance(TokenAccount::LEN);
        
        let create_account_ix = system_instruction::create_account(
            &self.context.payer.pubkey(),
            &token_account.pubkey(),
            account_rent,
            TokenAccount::LEN as u64,
            &spl_token::id(),
        );
        
        let init_account_ix = spl_token::instruction::initialize_account(
            &spl_token::id(),
            &token_account.pubkey(),
            &self.mint.pubkey(),
            owner,
        )?;
        
        let recent_blockhash = self.context.banks_client.get_recent_blockhash().await?;
        let transaction = Transaction::new_signed_with_payer(
            &[create_account_ix, init_account_ix],
            Some(&self.context.payer.pubkey()),
            &[&self.context.payer, &token_account],
            recent_blockhash,
        );
        
        self.context.banks_client.process_transaction(transaction).await?;
        Ok(token_account.pubkey())
    }
    
    /// Mint tokens to an account
    pub async fn mint_to(
        &mut self,
        token_account: &Pubkey,
        amount: u64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mint_to_ix = spl_token::instruction::mint_to(
            &spl_token::id(),
            &self.mint.pubkey(),
            token_account,
            &self.mint_authority.pubkey(),
            &[],
            amount,
        )?;
        
        let recent_blockhash = self.context.banks_client.get_recent_blockhash().await?;
        let transaction = Transaction::new_signed_with_payer(
            &[mint_to_ix],
            Some(&self.context.payer.pubkey()),
            &[&self.context.payer, &self.mint_authority],
            recent_blockhash,
        );
        
        self.context.banks_client.process_transaction(transaction).await?;
        Ok(())
    }
    
    /// Fund an account with SOL for fees
    pub async fn fund_account(
        &mut self,
        account: &Pubkey,
        lamports: u64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let transfer_ix = system_instruction::transfer(
            &self.context.payer.pubkey(),
            account,
            lamports,
        );
        
        let recent_blockhash = self.context.banks_client.get_recent_blockhash().await?;
        let transaction = Transaction::new_signed_with_payer(
            &[transfer_ix],
            Some(&self.context.payer.pubkey()),
            &[&self.context.payer],
            recent_blockhash,
        );
        
        self.context.banks_client.process_transaction(transaction).await?;
        Ok(())
    }
}

/// Helper to derive all program PDAs
pub fn derive_pdas(program_id: &Pubkey, mint: &Pubkey) -> ProgramPdas {
    let (config, config_bump) = Pubkey::find_program_address(
        &[b"config", program_id.as_ref(), mint.as_ref()],
        program_id,
    );
    
    let (global_state, global_bump) = Pubkey::find_program_address(
        &[b"global", program_id.as_ref(), mint.as_ref()],
        program_id,
    );
    
    let (token_vault, vault_bump) = Pubkey::find_program_address(
        &[b"token_vault", program_id.as_ref(), mint.as_ref()],
        program_id,
    );
    
    let (vault_authority, auth_bump) = Pubkey::find_program_address(
        &[b"vault_authority", program_id.as_ref(), mint.as_ref()],
        program_id,
    );
    
    let (reward_vault, reward_bump) = Pubkey::find_program_address(
        &[b"reward_vault", program_id.as_ref(), mint.as_ref()],
        program_id,
    );
    
    ProgramPdas {
        config,
        config_bump,
        global_state,
        global_bump,
        token_vault,
        vault_bump,
        vault_authority,
        auth_bump,
        reward_vault,
        reward_bump,
    }
}

/// Helper to derive user PDA
pub fn derive_user_pda(program_id: &Pubkey, user: &Pubkey, mint: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"user", program_id.as_ref(), user.as_ref(), mint.as_ref()],
        program_id,
    )
}

/// Struct containing all program PDAs with bumps
pub struct ProgramPdas {
    pub config: Pubkey,
    pub config_bump: u8,
    pub global_state: Pubkey,
    pub global_bump: u8,
    pub token_vault: Pubkey,
    pub vault_bump: u8,
    pub vault_authority: Pubkey,
    pub auth_bump: u8,
    pub reward_vault: Pubkey,
    pub reward_bump: u8,
}

/// Test data generator for property-based testing
pub struct TestDataGenerator;

impl TestDataGenerator {
    /// Generate valid tax rates (0-10000 bps)
    pub fn valid_tax_rates() -> Vec<u16> {
        vec![0, 100, 500, 1000, 2500, 5000, 10000]
    }
    
    /// Generate invalid tax rates (>10000 bps)
    pub fn invalid_tax_rates() -> Vec<u16> {
        vec![10001, 15000, u16::MAX]
    }
    
    /// Generate test token amounts
    pub fn token_amounts() -> Vec<u64> {
        vec![0, 1, 100, 1000, 10000, 1_000_000, u64::MAX / 10000]
    }
    
    /// Generate test reward scenarios
    pub fn reward_scenarios() -> Vec<RewardTestCase> {
        vec![
            RewardTestCase {
                user_balance: 1000,
                global_cum: 2_000_000_000_000_000_000u128, // 2 SOL per token
                user_last_cum: 1_000_000_000_000_000_000u128, // 1 SOL per token
                expected_reward: 1000, // 1000 tokens * 1 SOL/token
                description: "Standard reward case",
            },
            RewardTestCase {
                user_balance: 0,
                global_cum: 1_000_000_000_000_000_000u128,
                user_last_cum: 0,
                expected_reward: 0,
                description: "Zero balance case",
            },
            RewardTestCase {
                user_balance: 1000,
                global_cum: 1_000_000_000_000_000_000u128,
                user_last_cum: 1_000_000_000_000_000_000u128,
                expected_reward: 0,
                description: "No new rewards case",
            },
        ]
    }
}

/// Test case for reward calculations
pub struct RewardTestCase {
    pub user_balance: u64,
    pub global_cum: u128,
    pub user_last_cum: u128,
    pub expected_reward: u64,
    pub description: &'static str,
}

/// Mock swap result for testing
pub struct MockSwapResult {
    pub tokens_in: u64,
    pub sol_out: u64,
    pub success: bool,
}

impl MockSwapResult {
    /// Create a successful mock swap
    pub fn success(tokens_in: u64, sol_out: u64) -> Self {
        Self {
            tokens_in,
            sol_out,
            success: true,
        }
    }
    
    /// Create a failed mock swap
    pub fn failure(tokens_in: u64) -> Self {
        Self {
            tokens_in,
            sol_out: 0,
            success: false,
        }
    }
    
    /// Simulate a 2:1 token to SOL swap rate
    pub fn simulate_swap(tokens_in: u64, min_out: u64) -> Self {
        let sol_out = tokens_in / 2; // 2 tokens per 1 SOL
        let success = sol_out >= min_out;
        
        Self {
            tokens_in,
            sol_out,
            success,
        }
    }
}

/// Assertion helpers for tests
pub mod assertions {
    use super::*;
    
    /// Assert tax calculation is correct
    pub fn assert_tax_calculation(amount: u64, tax_rate_bps: u16, expected: u64) {
        let actual = amount
            .checked_mul(tax_rate_bps as u64)
            .unwrap()
            .checked_div(10_000)
            .unwrap();
        
        assert_eq!(
            actual, expected,
            "Tax calculation failed: {} tokens at {}bps should be {} tax, got {}",
            amount, tax_rate_bps, expected, actual
        );
    }
    
    /// Assert reward calculation is correct
    pub fn assert_reward_calculation(
        balance: u64,
        global_cum: u128,
        user_cum: u128,
        expected: u64,
    ) {
        let scale = 1_000_000_000_000_000_000u128;
        let actual = (balance as u128)
            .checked_mul(global_cum.checked_sub(user_cum).unwrap())
            .unwrap()
            .checked_div(scale)
            .unwrap() as u64;
        
        assert_eq!(
            actual, expected,
            "Reward calculation failed: {} balance with {}->{} cum should be {} reward, got {}",
            balance, user_cum, global_cum, expected, actual
        );
    }
    
    /// Assert PDA derivation is consistent
    pub fn assert_pda_consistency(program_id: &Pubkey, mint: &Pubkey) {
        let pdas1 = derive_pdas(program_id, mint);
        let pdas2 = derive_pdas(program_id, mint);
        
        assert_eq!(pdas1.config, pdas2.config);
        assert_eq!(pdas1.global_state, pdas2.global_state);
        assert_eq!(pdas1.token_vault, pdas2.token_vault);
        assert_eq!(pdas1.vault_authority, pdas2.vault_authority);
        assert_eq!(pdas1.reward_vault, pdas2.reward_vault);
    }
}
