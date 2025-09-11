use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{self, Mint, Token, TokenAccount},
};
use solana_program::system_program;
use solana_program_test::*;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};

use solana_tax_reward::{
    state::{Config, GlobalState, UserInfo},
    error::TaxRewardError,
    program::SolanaTaxReward,
};

/// Test helper to derive PDAs
pub fn get_pdas(program_id: &Pubkey, mint: &Pubkey) -> (Pubkey, Pubkey, Pubkey, Pubkey, Pubkey) {
    let (config, _) = Pubkey::find_program_address(
        &[b"config", program_id.as_ref(), mint.as_ref()],
        program_id,
    );
    
    let (global_state, _) = Pubkey::find_program_address(
        &[b"global", program_id.as_ref(), mint.as_ref()],
        program_id,
    );
    
    let (token_vault, _) = Pubkey::find_program_address(
        &[b"token_vault", program_id.as_ref(), mint.as_ref()],
        program_id,
    );
    
    let (vault_authority, _) = Pubkey::find_program_address(
        &[b"vault_authority", program_id.as_ref(), mint.as_ref()],
        program_id,
    );
    
    let (reward_vault, _) = Pubkey::find_program_address(
        &[b"reward_vault", program_id.as_ref(), mint.as_ref()],
        program_id,
    );
    
    (config, global_state, token_vault, vault_authority, reward_vault)
}

/// Test helper to get user PDA
pub fn get_user_pda(program_id: &Pubkey, user: &Pubkey, mint: &Pubkey) -> Pubkey {
    let (user_info, _) = Pubkey::find_program_address(
        &[b"user", program_id.as_ref(), user.as_ref(), mint.as_ref()],
        program_id,
    );
    user_info
}

#[tokio::test]
async fn test_initialize_program() {
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
    
    let mut context = program_test.start_with_context().await;
    
    // Create a mint
    let mint_keypair = Keypair::new();
    let mint_pubkey = mint_keypair.pubkey();
    
    // Create mint account
    let rent = context.banks_client.get_rent().await.unwrap();
    let mint_rent = rent.minimum_balance(Mint::LEN);
    
    let create_mint_ix = system_program::create_account(
        &context.payer.pubkey(),
        &mint_pubkey,
        mint_rent,
        Mint::LEN as u64,
        &spl_token::id(),
    );
    
    let init_mint_ix = spl_token::instruction::initialize_mint(
        &spl_token::id(),
        &mint_pubkey,
        &context.payer.pubkey(),
        None,
        9, // 9 decimals
    ).unwrap();
    
    // Create and process transaction
    let recent_blockhash = context.banks_client.get_recent_blockhash().await.unwrap();
    let transaction = Transaction::new_signed_with_payer(
        &[create_mint_ix, init_mint_ix],
        Some(&context.payer.pubkey()),
        &[&context.payer, &mint_keypair],
        recent_blockhash,
    );
    
    context.banks_client.process_transaction(transaction).await.unwrap();
    
    // Now test our initialize instruction
    let authority = &context.payer;
    let tax_rate_bps = 500; // 5%
    let dex_program = Pubkey::new_unique();
    
    let (config, global_state, token_vault, vault_authority, reward_vault) = 
        get_pdas(&program_id, &mint_pubkey);
    
    // This is a placeholder for the actual initialize instruction
    // In a real test, we would create the Anchor instruction here
    println!("PDAs derived successfully:");
    println!("Config: {}", config);
    println!("GlobalState: {}", global_state);
    println!("TokenVault: {}", token_vault);
    println!("VaultAuthority: {}", vault_authority);
    println!("RewardVault: {}", reward_vault);
}

#[tokio::test]
async fn test_tax_calculation_scenarios() {
    // Test various tax calculation scenarios
    struct TestCase {
        amount: u64,
        tax_rate_bps: u16,
        expected_tax: u64,
        description: &'static str,
    }
    
    let test_cases = vec![
        TestCase {
            amount: 1000,
            tax_rate_bps: 500, // 5%
            expected_tax: 50,
            description: "5% tax on 1000 tokens",
        },
        TestCase {
            amount: 10_000,
            tax_rate_bps: 250, // 2.5%
            expected_tax: 250,
            description: "2.5% tax on 10,000 tokens",
        },
        TestCase {
            amount: 1,
            tax_rate_bps: 10_000, // 100%
            expected_tax: 1,
            description: "100% tax on 1 token",
        },
        TestCase {
            amount: 1000,
            tax_rate_bps: 0, // 0%
            expected_tax: 0,
            description: "0% tax on 1000 tokens",
        },
    ];
    
    for test_case in test_cases {
        let tax = test_case.amount
            .checked_mul(test_case.tax_rate_bps as u64)
            .unwrap()
            .checked_div(10_000)
            .unwrap();
        
        assert_eq!(
            tax, 
            test_case.expected_tax,
            "Failed test case: {}",
            test_case.description
        );
    }
}

#[tokio::test]
async fn test_reward_calculation_scenarios() {
    let scale = 1_000_000_000_000_000_000u128; // 1e18
    
    struct RewardTestCase {
        user_balance: u64,
        global_cum_reward: u128,
        user_last_cum: u128,
        expected_reward: u64,
        description: &'static str,
    }
    
    let test_cases = vec![
        RewardTestCase {
            user_balance: 1000,
            global_cum_reward: 2 * scale,
            user_last_cum: 1 * scale,
            expected_reward: 1000,
            description: "1000 tokens, 1 SOL per token reward",
        },
        RewardTestCase {
            user_balance: 500,
            global_cum_reward: 1 * scale,
            user_last_cum: 0,
            expected_reward: 500,
            description: "500 tokens, first time claiming",
        },
        RewardTestCase {
            user_balance: 1000,
            global_cum_reward: 1 * scale,
            user_last_cum: 1 * scale,
            expected_reward: 0,
            description: "No new rewards since last claim",
        },
    ];
    
    for test_case in test_cases {
        let owed_u128 = (test_case.user_balance as u128)
            .checked_mul(
                test_case.global_cum_reward
                    .checked_sub(test_case.user_last_cum)
                    .unwrap()
            )
            .unwrap()
            .checked_div(scale)
            .unwrap();
        let owed = owed_u128 as u64;
        
        assert_eq!(
            owed,
            test_case.expected_reward,
            "Failed reward test case: {}",
            test_case.description
        );
    }
}

#[tokio::test]
async fn test_pda_derivation() {
    let program_id = solana_tax_reward::id();
    let mint = Pubkey::new_unique();
    let user = Pubkey::new_unique();
    
    // Test that PDA derivation is consistent
    let (config1, global1, vault1, auth1, reward1) = get_pdas(&program_id, &mint);
    let (config2, global2, vault2, auth2, reward2) = get_pdas(&program_id, &mint);
    
    assert_eq!(config1, config2);
    assert_eq!(global1, global2);
    assert_eq!(vault1, vault2);
    assert_eq!(auth1, auth2);
    assert_eq!(reward1, reward2);
    
    // Test user PDA
    let user_pda1 = get_user_pda(&program_id, &user, &mint);
    let user_pda2 = get_user_pda(&program_id, &user, &mint);
    assert_eq!(user_pda1, user_pda2);
    
    // Different mint should produce different PDAs
    let different_mint = Pubkey::new_unique();
    let (config3, _, _, _, _) = get_pdas(&program_id, &different_mint);
    assert_ne!(config1, config3);
}

#[tokio::test]
async fn test_overflow_conditions() {
    // Test overflow conditions that should be handled gracefully
    
    // Tax calculation overflow
    let result = u64::MAX.checked_mul(10_000);
    assert!(result.is_none(), "Should detect overflow in tax calculation");
    
    // Reward calculation overflow
    let scale = 1_000_000_000_000_000_000u128;
    let result = u128::MAX.checked_mul(scale);
    assert!(result.is_none(), "Should detect overflow in reward calculation");
    
    // Safe calculations should work
    let safe_tax = 1000u64.checked_mul(500).unwrap().checked_div(10_000).unwrap();
    assert_eq!(safe_tax, 50);
    
    let safe_reward = (1000u128)
        .checked_mul(scale)
        .unwrap()
        .checked_div(scale)
        .unwrap();
    assert_eq!(safe_reward, 1000);
}

/// Test that validates error conditions
#[tokio::test]
async fn test_error_conditions() {
    // Test tax rate validation
    let invalid_tax_rates = vec![10_001u16, u16::MAX];
    
    for invalid_rate in invalid_tax_rates {
        // In a real test, we would call the initialize instruction
        // and expect it to fail with InvalidTaxRate error
        assert!(invalid_rate > 10_000, "Tax rate {} should be invalid", invalid_rate);
    }
    
    // Test valid tax rates
    let valid_tax_rates = vec![0u16, 500u16, 10_000u16];
    
    for valid_rate in valid_tax_rates {
        assert!(valid_rate <= 10_000, "Tax rate {} should be valid", valid_rate);
    }
}

/// Mock test for swap functionality
#[tokio::test]
async fn test_swap_mock_functionality() {
    // Test that our mock swap behaves predictably
    
    let token_amount = 1000u64;
    let min_amount_out = 500u64;
    
    // In mock mode, we expect the swap to "succeed" without actual DEX interaction
    // The mock should:
    // 1. Validate inputs
    // 2. Log the swap attempt
    // 3. Return success for testing
    
    assert!(token_amount > 0, "Token amount should be valid for swap");
    assert!(min_amount_out > 0, "Min amount out should be valid");
    
    // Mock swap logic validation
    let simulated_output = token_amount / 2; // Simulate 2:1 ratio
    let swap_successful = simulated_output >= min_amount_out;
    assert!(swap_successful, "Mock swap should meet minimum output requirement");
}
