use proptest::prelude::*;
use solana_tax_reward::{
    state::{Config, GlobalState, UserInfo},
    error::TaxRewardError,
};
use anchor_lang::prelude::*;

/// Property-based tests for tax and reward calculations
mod tax_reward_properties {
    use super::*;

    /// Generate valid tax rates (0-10000 bps)
    fn valid_tax_rate() -> impl Strategy<Value = u16> {
        0u16..=10_000u16
    }

    /// Generate valid token amounts (avoid overflow)
    fn valid_token_amount() -> impl Strategy<Value = u64> {
        1u64..1_000_000_000u64
    }

    /// Generate valid reward scale values
    fn reward_scale_values() -> impl Strategy<Value = u128> {
        0u128..1_000_000_000_000_000_000u128 // Up to 1e18
    }

    proptest! {
        /// Property: Tax should never exceed the input amount
        #[test]
        fn tax_never_exceeds_amount(
            amount in valid_token_amount(),
            tax_rate_bps in valid_tax_rate()
        ) {
            let tax = calculate_tax(amount, tax_rate_bps);
            prop_assert!(tax <= amount, "Tax {} should not exceed amount {}", tax, amount);
        }

        /// Property: Tax should be zero when tax rate is zero
        #[test]
        fn zero_tax_rate_gives_zero_tax(
            amount in valid_token_amount()
        ) {
            let tax = calculate_tax(amount, 0);
            prop_assert_eq!(tax, 0, "Zero tax rate should give zero tax");
        }

        /// Property: 100% tax rate should equal the input amount
        #[test]
        fn full_tax_rate_gives_full_amount(
            amount in valid_token_amount()
        ) {
            let tax = calculate_tax(amount, 10_000);
            prop_assert_eq!(tax, amount, "100% tax rate should equal input amount");
        }

        /// Property: Tax should be proportional to tax rate
        #[test]
        fn tax_proportional_to_rate(
            amount in valid_token_amount(),
            tax_rate_bps in 1u16..5_000u16
        ) {
            let tax1 = calculate_tax(amount, tax_rate_bps);
            let tax2 = calculate_tax(amount, tax_rate_bps * 2);
            
            // Double tax rate should give approximately double tax (within rounding)
            if tax1 > 0 {
                prop_assert!(tax2 >= tax1, "Double tax rate should give at least same tax");
                prop_assert!(tax2 <= tax1 * 2 + 1, "Double tax rate should not give more than double + 1 (rounding)");
            }
        }

        /// Property: Reward calculation should be deterministic
        #[test]
        fn reward_calculation_deterministic(
            balance in valid_token_amount(),
            global_cum in reward_scale_values(),
            user_cum in reward_scale_values()
        ) {
            if global_cum >= user_cum {
                let reward1 = calculate_reward(balance, global_cum, user_cum);
                let reward2 = calculate_reward(balance, global_cum, user_cum);
                prop_assert_eq!(reward1, reward2, "Reward calculation should be deterministic");
            }
        }

        /// Property: Rewards should be zero when global_cum equals user_cum
        #[test]
        fn no_new_rewards_gives_zero(
            balance in valid_token_amount(),
            cum_value in reward_scale_values()
        ) {
            let reward = calculate_reward(balance, cum_value, cum_value);
            prop_assert_eq!(reward, 0, "Same cum values should give zero rewards");
        }

        /// Property: Rewards should be zero when balance is zero
        #[test]
        fn zero_balance_gives_zero_rewards(
            global_cum in reward_scale_values(),
            user_cum in 0u128..global_cum
        ) {
            let reward = calculate_reward(0, global_cum, user_cum);
            prop_assert_eq!(reward, 0, "Zero balance should give zero rewards");
        }

        /// Property: Reward should increase with balance
        #[test]
        fn reward_increases_with_balance(
            balance in 1u64..1_000_000u64,
            global_cum in reward_scale_values(),
            user_cum in 0u128..global_cum
        ) {
            if global_cum > user_cum {
                let reward1 = calculate_reward(balance, global_cum, user_cum);
                let reward2 = calculate_reward(balance * 2, global_cum, user_cum);
                prop_assert!(reward2 >= reward1, "Double balance should give at least same rewards");
            }
        }

        /// Property: Account size calculations should be consistent
        #[test]
        fn account_sizes_consistent(
            tax_rate in valid_tax_rate(),
            total_supply in 1u64..u64::MAX,
            cum_reward in reward_scale_values(),
            balance_snapshot in valid_token_amount(),
            last_cum in reward_scale_values()
        ) {
            // Test Config size consistency
            let config = Config {
                tax_rate_bps: tax_rate,
                owner: Pubkey::new_unique(),
                dex_program: Pubkey::new_unique(),
                paused: false,
            };
            
            let serialized_size = config.try_to_vec().unwrap().len();
            prop_assert_eq!(serialized_size, Config::LEN, "Config serialized size should match LEN constant");

            // Test GlobalState size consistency
            let global_state = GlobalState {
                total_supply,
                cum_reward_per_token: cum_reward,
            };
            
            let serialized_size = global_state.try_to_vec().unwrap().len();
            prop_assert_eq!(serialized_size, GlobalState::LEN, "GlobalState serialized size should match LEN constant");

            // Test UserInfo size consistency
            let user_info = UserInfo {
                last_cum,
                balance_snapshot,
            };
            
            let serialized_size = user_info.try_to_vec().unwrap().len();
            prop_assert_eq!(serialized_size, UserInfo::LEN, "UserInfo serialized size should match LEN constant");
        }

        /// Property: PDA derivation should be consistent
        #[test]
        fn pda_derivation_consistent(
            program_seed in 0u8..255,
            mint_seed in 0u8..255,
            user_seed in 0u8..255
        ) {
            let program_id = Pubkey::new_from_array([program_seed; 32]);
            let mint = Pubkey::new_from_array([mint_seed; 32]);
            let user = Pubkey::new_from_array([user_seed; 32]);
            
            // Test that same inputs give same outputs
            let (config1, _) = Pubkey::find_program_address(
                &[b"config", program_id.as_ref(), mint.as_ref()],
                &program_id,
            );
            let (config2, _) = Pubkey::find_program_address(
                &[b"config", program_id.as_ref(), mint.as_ref()],
                &program_id,
            );
            prop_assert_eq!(config1, config2, "PDA derivation should be deterministic");
            
            // Test that different inputs give different outputs (with high probability)
            let different_mint = Pubkey::new_from_array([mint_seed.wrapping_add(1); 32]);
            let (config3, _) = Pubkey::find_program_address(
                &[b"config", program_id.as_ref(), different_mint.as_ref()],
                &program_id,
            );
            
            if mint != different_mint {
                prop_assert_ne!(config1, config3, "Different mints should give different PDAs");
            }
        }
    }

    /// Helper function to calculate tax (mirrors program logic)
    fn calculate_tax(amount: u64, tax_rate_bps: u16) -> u64 {
        amount
            .checked_mul(tax_rate_bps as u64)
            .unwrap_or(0)
            .checked_div(10_000)
            .unwrap_or(0)
    }

    /// Helper function to calculate reward (mirrors program logic)
    fn calculate_reward(balance: u64, global_cum: u128, user_cum: u128) -> u64 {
        const SCALE: u128 = 1_000_000_000_000_000_000;
        
        if global_cum < user_cum {
            return 0;
        }
        
        let diff = global_cum - user_cum;
        (balance as u128)
            .checked_mul(diff)
            .unwrap_or(0)
            .checked_div(SCALE)
            .unwrap_or(0) as u64
    }
}

/// Edge case tests for boundary conditions
mod edge_case_tests {
    use super::*;

    #[test]
    fn test_maximum_values() {
        // Test with maximum safe values
        let max_safe_amount = u64::MAX / 10_000; // Avoid overflow in tax calculation
        let tax = max_safe_amount
            .checked_mul(10_000)
            .unwrap()
            .checked_div(10_000)
            .unwrap();
        assert_eq!(tax, max_safe_amount);

        // Test reward calculation with maximum values
        let scale = 1_000_000_000_000_000_000u128;
        let max_balance = u64::MAX;
        let max_cum = u128::MAX / (max_balance as u128 + 1); // Avoid overflow
        
        if let Some(product) = (max_balance as u128).checked_mul(max_cum) {
            let reward = product.checked_div(scale).unwrap_or(0);
            assert!(reward <= max_balance as u128);
        }
    }

    #[test]
    fn test_minimum_values() {
        // Test with minimum values
        assert_eq!(calculate_tax_safe(0, 0), 0);
        assert_eq!(calculate_tax_safe(0, 10_000), 0);
        assert_eq!(calculate_tax_safe(1, 0), 0);
        assert_eq!(calculate_tax_safe(1, 1), 0); // 0.01% of 1 rounds to 0

        // Test minimum reward calculation
        assert_eq!(calculate_reward_safe(0, 1_000_000_000_000_000_000, 0), 0);
        assert_eq!(calculate_reward_safe(1, 0, 0), 0);
    }

    #[test]
    fn test_rounding_behavior() {
        // Test that rounding behaves consistently
        assert_eq!(calculate_tax_safe(999, 1), 0); // 0.9999 rounds to 0
        assert_eq!(calculate_tax_safe(1000, 1), 0); // 1.0 rounds to 0  
        assert_eq!(calculate_tax_safe(10_000, 1), 1); // 10.0 rounds to 1

        // Test reward rounding
        let scale = 1_000_000_000_000_000_000u128;
        let small_reward = scale / 2; // 0.5 units
        assert_eq!(calculate_reward_safe(1, small_reward, 0), 0); // Rounds down
        
        let exact_reward = scale; // 1.0 units
        assert_eq!(calculate_reward_safe(1, exact_reward, 0), 1); // Exact
    }

    #[test]
    fn test_overflow_protection() {
        // Test that overflow conditions are handled gracefully
        let result = u64::MAX.checked_mul(10_000);
        assert!(result.is_none(), "Should detect overflow");

        let result = u128::MAX.checked_mul(u128::MAX);
        assert!(result.is_none(), "Should detect overflow");

        // Test that safe operations work
        let safe_result = 1000u64.checked_mul(500).unwrap().checked_div(10_000).unwrap();
        assert_eq!(safe_result, 50);
    }

    #[test]
    fn test_state_transitions() {
        // Test valid state transitions
        let mut global_state = GlobalState {
            total_supply: 1_000_000,
            cum_reward_per_token: 0,
        };

        // Simulate reward distribution
        let reward_distribution = 1_000_000_000_000_000_000u128; // 1 SOL per token
        global_state.cum_reward_per_token += reward_distribution;
        assert_eq!(global_state.cum_reward_per_token, reward_distribution);

        // Test user state transitions
        let mut user_info = UserInfo {
            last_cum: 0,
            balance_snapshot: 1000,
        };

        // User claims rewards
        user_info.last_cum = global_state.cum_reward_per_token;
        assert_eq!(user_info.last_cum, reward_distribution);

        // User balance changes
        user_info.balance_snapshot = 1500;
        assert_eq!(user_info.balance_snapshot, 1500);
    }

    /// Helper functions with overflow protection
    fn calculate_tax_safe(amount: u64, tax_rate_bps: u16) -> u64 {
        amount
            .checked_mul(tax_rate_bps as u64)
            .and_then(|x| x.checked_div(10_000))
            .unwrap_or(0)
    }

    fn calculate_reward_safe(balance: u64, global_cum: u128, user_cum: u128) -> u64 {
        const SCALE: u128 = 1_000_000_000_000_000_000;
        
        global_cum
            .checked_sub(user_cum)
            .and_then(|diff| (balance as u128).checked_mul(diff))
            .and_then(|product| product.checked_div(SCALE))
            .unwrap_or(0) as u64
    }
}

/// Integration-style tests that combine multiple components
mod integration_tests {
    use super::*;

    #[test]
    fn test_complete_tax_and_reward_flow() {
        // Simulate a complete flow through the system
        
        // 1. Initial setup
        let mut global_state = GlobalState {
            total_supply: 1_000_000,
            cum_reward_per_token: 0,
        };
        
        let mut user_info = UserInfo {
            last_cum: 0,
            balance_snapshot: 1000,
        };
        
        // 2. User performs a taxed swap
        let swap_amount = 1000u64;
        let tax_rate_bps = 500u16; // 5%
        let tax_collected = calculate_tax_safe(swap_amount, tax_rate_bps);
        assert_eq!(tax_collected, 50);
        
        // 3. Tax is swapped to SOL and distributed
        let sol_received = tax_collected * 2; // Assume 2:1 swap rate
        let reward_per_token = (sol_received as u128 * 1_000_000_000_000_000_000u128) / global_state.total_supply as u128;
        global_state.cum_reward_per_token += reward_per_token;
        
        // 4. User claims rewards
        let owed = calculate_reward_safe(
            user_info.balance_snapshot,
            global_state.cum_reward_per_token,
            user_info.last_cum,
        );
        
        assert!(owed > 0, "User should have earned some rewards");
        
        // Update user state
        user_info.last_cum = global_state.cum_reward_per_token;
        
        // 5. Second swap should not give immediate rewards (already claimed)
        let owed_again = calculate_reward_safe(
            user_info.balance_snapshot,
            global_state.cum_reward_per_token,
            user_info.last_cum,
        );
        
        assert_eq!(owed_again, 0, "No new rewards should be available immediately");
    }

    #[test]
    fn test_multiple_users_scenario() {
        // Test scenario with multiple users
        let mut global_state = GlobalState {
            total_supply: 10_000,
            cum_reward_per_token: 0,
        };
        
        // User 1: 1000 tokens
        let mut user1 = UserInfo {
            last_cum: 0,
            balance_snapshot: 1000,
        };
        
        // User 2: 4000 tokens  
        let mut user2 = UserInfo {
            last_cum: 0,
            balance_snapshot: 4000,
        };
        
        // Distribute 1000 lamports as rewards
        let total_rewards = 1000u64;
        let reward_per_token = (total_rewards as u128 * 1_000_000_000_000_000_000u128) / global_state.total_supply as u128;
        global_state.cum_reward_per_token += reward_per_token;
        
        // Calculate rewards for each user
        let user1_reward = calculate_reward_safe(
            user1.balance_snapshot,
            global_state.cum_reward_per_token,
            user1.last_cum,
        );
        
        let user2_reward = calculate_reward_safe(
            user2.balance_snapshot,
            global_state.cum_reward_per_token,
            user2.last_cum,
        );
        
        // User2 should get 4x more rewards (4x more tokens)
        assert!(user2_reward >= user1_reward * 3, "User2 should get proportionally more rewards");
        assert!(user2_reward <= user1_reward * 5, "User2 rewards should be reasonable proportion");
        
        // Total rewards should not exceed distributed amount (with rounding tolerance)
        assert!(user1_reward + user2_reward <= total_rewards + 2, "Total claimed should not exceed distributed");
    }

    #[test]
    fn test_config_validation() {
        // Test valid configurations
        let valid_configs = vec![
            (0, true),      // 0% tax, paused
            (500, false),   // 5% tax, active  
            (10_000, true), // 100% tax, paused
        ];
        
        for (tax_rate_bps, paused) in valid_configs {
            let config = Config {
                tax_rate_bps,
                owner: Pubkey::new_unique(),
                dex_program: Pubkey::new_unique(),
                paused,
            };
            
            assert!(config.tax_rate_bps <= 10_000, "Tax rate should be valid");
            
            // Test serialization
            let serialized = config.try_to_vec().unwrap();
            let deserialized = Config::try_from_slice(&serialized).unwrap();
            assert_eq!(config.tax_rate_bps, deserialized.tax_rate_bps);
            assert_eq!(config.paused, deserialized.paused);
        }
    }

    /// Helper function with overflow protection
    fn calculate_tax_safe(amount: u64, tax_rate_bps: u16) -> u64 {
        amount
            .checked_mul(tax_rate_bps as u64)
            .and_then(|x| x.checked_div(10_000))
            .unwrap_or(0)
    }

    fn calculate_reward_safe(balance: u64, global_cum: u128, user_cum: u128) -> u64 {
        const SCALE: u128 = 1_000_000_000_000_000_000;
        
        global_cum
            .checked_sub(user_cum)
            .and_then(|diff| (balance as u128).checked_mul(diff))
            .and_then(|product| product.checked_div(SCALE))
            .unwrap_or(0) as u64
    }
}

/// Stress tests for performance and limits
mod stress_tests {
    use super::*;

    #[test]
    fn test_large_scale_calculations() {
        // Test with large but realistic values
        let large_supply = 1_000_000_000_000u64; // 1 trillion tokens
        let large_balance = 1_000_000_000u64;    // 1 billion tokens
        let large_cum_reward = 1_000_000_000_000_000_000u128; // 1 SOL per token
        
        let reward = calculate_reward_safe(large_balance, large_cum_reward, 0);
        assert!(reward > 0, "Should calculate rewards for large values");
        assert!(reward <= large_balance, "Reward should not exceed balance");
    }

    #[test]
    fn test_precision_limits() {
        // Test precision at the limits of our scale
        let scale = 1_000_000_000_000_000_000u128;
        
        // Test smallest possible reward
        let smallest_reward = calculate_reward_safe(1, 1, 0);
        assert_eq!(smallest_reward, 0, "Smallest reward should round to 0");
        
        // Test just above precision limit  
        let just_above_limit = calculate_reward_safe(1, scale, 0);
        assert_eq!(just_above_limit, 1, "Should give exactly 1 unit of reward");
        
        // Test precision with large balances
        let large_balance = u64::MAX / 1000; // Avoid overflow
        let small_cum_diff = scale / (large_balance as u128); // Tiny per-token reward
        let reward = calculate_reward_safe(large_balance, small_cum_diff, 0);
        assert!(reward <= 1, "Small per-token reward should give small total");
    }

    fn calculate_reward_safe(balance: u64, global_cum: u128, user_cum: u128) -> u64 {
        const SCALE: u128 = 1_000_000_000_000_000_000;
        
        global_cum
            .checked_sub(user_cum)
            .and_then(|diff| (balance as u128).checked_mul(diff))
            .and_then(|product| product.checked_div(SCALE))
            .unwrap_or(0) as u64
    }
}
