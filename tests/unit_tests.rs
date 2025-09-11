use anchor_lang::prelude::*;
use solana_tax_reward::{
    state::{Config, GlobalState, UserInfo},
    error::TaxRewardError,
};
use proptest::prelude::*;
use borsh::{BorshSerialize, BorshDeserialize};

/// Test tax calculation with various rates and amounts
#[test]
fn test_tax_calculation() {
    // Test 5% tax rate (500 bps)
    let amount = 1000u64;
    let tax_rate_bps = 500u16;
    let expected_tax = 50u64; // 5% of 1000
    let expected_net = 950u64;
    
    let tax = amount.checked_mul(tax_rate_bps as u64)
        .unwrap()
        .checked_div(10_000)
        .unwrap();
    let net_amount = amount.checked_sub(tax).unwrap();
    
    assert_eq!(tax, expected_tax);
    assert_eq!(net_amount, expected_net);
    
    // Test edge cases
    assert_eq!(calculate_tax(1000, 0), 0); // 0% tax
    assert_eq!(calculate_tax(1000, 10_000), 1000); // 100% tax
    assert_eq!(calculate_tax(0, 500), 0); // 0 amount
}

/// Helper function to calculate tax (mirrors program logic)
fn calculate_tax(amount: u64, tax_rate_bps: u16) -> u64 {
    amount.checked_mul(tax_rate_bps as u64)
        .unwrap_or(0)
        .checked_div(10_000)
        .unwrap_or(0)
}

/// Test reward calculation logic
#[test]
fn test_reward_calculation() {
    let scale = 1_000_000_000_000_000_000u128; // 1e18
    
    // Test basic reward calculation
    let user_balance = 1000u64;
    let global_cum_reward = 2 * scale; // 2 SOL per token
    let user_last_cum = 1 * scale; // User last saw 1 SOL per token
    
    let expected_reward = 1000u64; // 1000 tokens * (2-1) SOL/token = 1000 lamports
    
    let owed_u128 = (user_balance as u128)
        .checked_mul(global_cum_reward.checked_sub(user_last_cum).unwrap())
        .unwrap()
        .checked_div(scale)
        .unwrap();
    let owed = owed_u128 as u64;
    
    assert_eq!(owed, expected_reward);
    
    // Test no rewards case
    let no_reward = (user_balance as u128)
        .checked_mul(global_cum_reward.checked_sub(global_cum_reward).unwrap())
        .unwrap()
        .checked_div(scale)
        .unwrap();
    assert_eq!(no_reward as u64, 0);
}

/// Test state serialization/deserialization
#[test]
fn test_state_serialization() {
    // Test Config
    let mut config = Config {
        tax_rate_bps: 500,
        owner: Pubkey::new_unique(),
        dex_program: Pubkey::new_unique(),
        paused: false,
    };
    
    let serialized = config.try_to_vec().unwrap();
    let deserialized = Config::try_from_slice(&serialized).unwrap();
    assert_eq!(config.tax_rate_bps, deserialized.tax_rate_bps);
    assert_eq!(config.owner, deserialized.owner);
    assert_eq!(config.paused, deserialized.paused);
    
    // Test GlobalState
    let global_state = GlobalState {
        total_supply: 1_000_000,
        cum_reward_per_token: 123456789,
    };
    
    let serialized = global_state.try_to_vec().unwrap();
    let deserialized = GlobalState::try_from_slice(&serialized).unwrap();
    assert_eq!(global_state.total_supply, deserialized.total_supply);
    assert_eq!(global_state.cum_reward_per_token, deserialized.cum_reward_per_token);
    
    // Test UserInfo
    let user_info = UserInfo {
        last_cum: 987654321,
        balance_snapshot: 5000,
    };
    
    let serialized = user_info.try_to_vec().unwrap();
    let deserialized = UserInfo::try_from_slice(&serialized).unwrap();
    assert_eq!(user_info.last_cum, deserialized.last_cum);
    assert_eq!(user_info.balance_snapshot, deserialized.balance_snapshot);
}

/// Test account size calculations
#[test]
fn test_account_sizes() {
    assert_eq!(Config::LEN, 2 + 32 + 32 + 1); // u16 + Pubkey + Pubkey + bool
    assert_eq!(GlobalState::LEN, 8 + 16); // u64 + u128
    assert_eq!(UserInfo::LEN, 16 + 8); // u128 + u64
}

/// Test overflow protection in calculations
#[test]
fn test_overflow_protection() {
    // Test tax calculation overflow
    let result = u64::MAX.checked_mul(10_000);
    assert!(result.is_none(), "Should overflow");
    
    // Test reward calculation overflow
    let scale = 1_000_000_000_000_000_000u128;
    let result = u128::MAX.checked_mul(scale);
    assert!(result.is_none(), "Should overflow");
    
    // Test safe calculations
    let safe_amount = 1000u64;
    let safe_tax = safe_amount.checked_mul(500).unwrap().checked_div(10_000).unwrap();
    assert_eq!(safe_tax, 50);
}

// Property-based tests using proptest
proptest! {
    /// Property: Tax calculation should never exceed the input amount
    #[test]
    fn prop_tax_never_exceeds_amount(
        amount in 0u64..1_000_000u64,
        tax_rate_bps in 0u16..=10_000u16
    ) {
        let tax = calculate_tax(amount, tax_rate_bps);
        prop_assert!(tax <= amount);
    }
    
    /// Property: Tax should be proportional to tax rate
    #[test]
    fn prop_tax_proportional_to_rate(
        amount in 1u64..1_000_000u64,
        tax_rate_bps in 1u16..5_000u16
    ) {
        let tax1 = calculate_tax(amount, tax_rate_bps);
        let tax2 = calculate_tax(amount, tax_rate_bps * 2);
        
        // Tax with double rate should be approximately double
        // Allow for rounding differences
        if tax1 > 0 {
            prop_assert!(tax2 >= tax1);
            prop_assert!(tax2 <= tax1 * 2 + 1); // +1 for rounding
        }
    }
    
    /// Property: Reward calculation should be deterministic
    #[test]
    fn prop_reward_calculation_deterministic(
        balance in 1u64..1_000_000u64,
        global_cum in 0u128..1_000_000_000_000_000_000u128,
        user_cum in 0u128..1_000_000_000_000_000_000u128
    ) {
        let scale = 1_000_000_000_000_000_000u128;
        
        if global_cum >= user_cum {
            let diff = global_cum - user_cum;
            if let Some(product) = (balance as u128).checked_mul(diff) {
                if let Some(reward) = product.checked_div(scale) {
                    // Same calculation should yield same result
                    let reward2 = (balance as u128)
                        .checked_mul(diff)
                        .unwrap()
                        .checked_div(scale)
                        .unwrap();
                    prop_assert_eq!(reward, reward2);
                }
            }
        }
    }
}

/// Test error conditions
#[test]
fn test_error_types() {
    // Test that error types have proper messages
    let error = TaxRewardError::InvalidTaxRate;
    assert_eq!(error.to_string(), "Invalid Tax Rate - must be <= 10000 bps (100%)");
    
    let error = TaxRewardError::SlippageExceeded;
    assert_eq!(error.to_string(), "Slippage Exceeded");
    
    let error = TaxRewardError::ProgramPaused;
    assert_eq!(error.to_string(), "Program is Paused");
}
