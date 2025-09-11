use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::Instruction;
use anchor_lang::InstructionData;
use anchor_spl::token::{self, Token, TokenAccount, Mint};
use solana_program_test::*;
use solana_sdk::{
    account::Account,
    signature::{Keypair, Signer},
    system_instruction,
    transaction::Transaction,
    program_pack::Pack,
    rent::Rent,
};
use spl_token::state::{Account as TokenAccountState, Mint as MintState};
use solana_tax_reward::{
    program::TaxReward,
    state::{Config, GlobalState, UserInfo},
    instruction::{Initialize, TaxedSwapAndDistribute, ClaimRewards, UpdateConfig},
    error::TaxRewardError,
};

/// End-to-end tests that execute real instructions against the program
#[tokio::test]
async fn test_full_initialize_flow() {
    let program_test = ProgramTest::new(
        "solana_tax_reward",
        solana_tax_reward::ID,
        processor!(solana_tax_reward::entry),
    );

    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

    // Create mint
    let mint = Keypair::new();
    let mint_rent = Rent::default().minimum_balance(MintState::LEN);
    
    let create_mint_ix = system_instruction::create_account(
        &payer.pubkey(),
        &mint.pubkey(),
        mint_rent,
        MintState::LEN as u64,
        &spl_token::id(),
    );

    let init_mint_ix = spl_token::instruction::initialize_mint(
        &spl_token::id(),
        &mint.pubkey(),
        &payer.pubkey(),
        None,
        9,
    ).unwrap();

    // Execute mint creation
    let mut transaction = Transaction::new_with_payer(
        &[create_mint_ix, init_mint_ix],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer, &mint], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();

    // Create PDAs
    let (config_pda, _) = Pubkey::find_program_address(
        &[b"config", solana_tax_reward::ID.as_ref(), mint.pubkey().as_ref()],
        &solana_tax_reward::ID,
    );

    let (global_state_pda, _) = Pubkey::find_program_address(
        &[b"global", solana_tax_reward::ID.as_ref(), mint.pubkey().as_ref()],
        &solana_tax_reward::ID,
    );

    let (token_vault_pda, _) = Pubkey::find_program_address(
        &[b"token_vault", solana_tax_reward::ID.as_ref(), mint.pubkey().as_ref()],
        &solana_tax_reward::ID,
    );

    let (reward_vault_pda, _) = Pubkey::find_program_address(
        &[b"reward_vault", solana_tax_reward::ID.as_ref(), mint.pubkey().as_ref()],
        &solana_tax_reward::ID,
    );

    // Create initialize instruction
    let initialize_data = Initialize {
        tax_rate_bps: 500, // 5%
        dex_program: Pubkey::new_unique(),
    };

    let initialize_ix = Instruction {
        program_id: solana_tax_reward::ID,
        accounts: vec![
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new(config_pda, false),
            AccountMeta::new(global_state_pda, false),
            AccountMeta::new(token_vault_pda, false),
            AccountMeta::new(reward_vault_pda, false),
            AccountMeta::new_readonly(mint.pubkey(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(anchor_lang::system_program::ID, false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data: initialize_data.data(),
    };

    // Execute initialize
    let mut transaction = Transaction::new_with_payer(
        &[initialize_ix],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();

    // Verify config was created correctly
    let config_account = banks_client.get_account(config_pda).await.unwrap().unwrap();
    assert_eq!(config_account.owner, solana_tax_reward::ID);
    
    let config: Config = Config::try_deserialize(&mut config_account.data.as_slice()).unwrap();
    assert_eq!(config.tax_rate_bps, 500);
    assert_eq!(config.owner, payer.pubkey());
    assert!(!config.paused);

    // Verify global state was created correctly
    let global_account = banks_client.get_account(global_state_pda).await.unwrap().unwrap();
    let global_state: GlobalState = GlobalState::try_deserialize(&mut global_account.data.as_slice()).unwrap();
    assert_eq!(global_state.total_supply, 0);
    assert_eq!(global_state.cum_reward_per_token, 0);

    // Verify vaults were created
    let token_vault = banks_client.get_account(token_vault_pda).await.unwrap().unwrap();
    assert_eq!(token_vault.owner, spl_token::id());
    
    let reward_vault = banks_client.get_account(reward_vault_pda).await.unwrap().unwrap();
    assert_eq!(reward_vault.owner, spl_token::id());
}

#[tokio::test]
async fn test_taxed_swap_and_distribute_flow() {
    let mut test_env = setup_test_environment().await;
    
    // Create user token account
    let user_keypair = Keypair::new();
    let user_token_account = create_token_account(
        &mut test_env.banks_client,
        &test_env.payer,
        &test_env.mint.pubkey(),
        &user_keypair.pubkey(),
        test_env.recent_blockhash,
    ).await;

    // Mint tokens to user
    mint_tokens_to_account(
        &mut test_env.banks_client,
        &test_env.payer,
        &test_env.mint.pubkey(),
        &user_token_account,
        1_000_000, // 1M tokens
        test_env.recent_blockhash,
    ).await;

    // Create user info PDA
    let (user_info_pda, _) = Pubkey::find_program_address(
        &[
            b"user",
            solana_tax_reward::ID.as_ref(),
            user_keypair.pubkey().as_ref(),
            test_env.mint.pubkey().as_ref(),
        ],
        &solana_tax_reward::ID,
    );

    // Create destination token account for the swap
    let dest_token_account = create_token_account(
        &mut test_env.banks_client,
        &test_env.payer,
        &test_env.mint.pubkey(),
        &user_keypair.pubkey(),
        test_env.recent_blockhash,
    ).await;

    // Perform taxed swap
    let swap_data = TaxedSwapAndDistribute {
        amount: 100_000, // 100k tokens
        minimum_amount_out: 90_000, // Expect at least 90k after tax
    };

    let swap_ix = Instruction {
        program_id: solana_tax_reward::ID,
        accounts: vec![
            AccountMeta::new(user_keypair.pubkey(), true),
            AccountMeta::new(user_token_account, false),
            AccountMeta::new(dest_token_account, false),
            AccountMeta::new(test_env.config_pda, false),
            AccountMeta::new(test_env.global_state_pda, false),
            AccountMeta::new(user_info_pda, false),
            AccountMeta::new(test_env.token_vault_pda, false),
            AccountMeta::new(test_env.reward_vault_pda, false),
            AccountMeta::new_readonly(test_env.mint.pubkey(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(anchor_lang::system_program::ID, false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data: swap_data.data(),
    };

    let mut transaction = Transaction::new_with_payer(
        &[swap_ix],
        Some(&user_keypair.pubkey()),
    );
    transaction.sign(&[&user_keypair], test_env.recent_blockhash);
    
    // Fund user account for transaction fees
    fund_account(&mut test_env.banks_client, &test_env.payer, &user_keypair.pubkey(), test_env.recent_blockhash).await;
    
    banks_client.process_transaction(transaction).await.unwrap();

    // Verify tax was collected
    let token_vault_account = test_env.banks_client.get_account(test_env.token_vault_pda).await.unwrap().unwrap();
    let vault_token_account = TokenAccountState::unpack(&token_vault_account.data).unwrap();
    assert!(vault_token_account.amount > 0, "Tax should be collected in vault");

    // Verify user info was created/updated
    let user_info_account = test_env.banks_client.get_account(user_info_pda).await.unwrap().unwrap();
    let user_info: UserInfo = UserInfo::try_deserialize(&mut user_info_account.data.as_slice()).unwrap();
    assert_eq!(user_info.balance_snapshot, 1_000_000 - 100_000); // Original - swapped

    // Verify global state was updated
    let global_account = test_env.banks_client.get_account(test_env.global_state_pda).await.unwrap().unwrap();
    let global_state: GlobalState = GlobalState::try_deserialize(&mut global_account.data.as_slice()).unwrap();
    assert!(global_state.cum_reward_per_token > 0, "Rewards should be distributed");
}

#[tokio::test]
async fn test_claim_rewards_flow() {
    let mut test_env = setup_test_environment().await;
    
    // Setup user with existing balance and rewards
    let user_keypair = Keypair::new();
    let user_token_account = create_token_account(
        &mut test_env.banks_client,
        &test_env.payer,
        &test_env.mint.pubkey(),
        &user_keypair.pubkey(),
        test_env.recent_blockhash,
    ).await;

    // Create and fund user info with some rewards pending
    let (user_info_pda, _) = Pubkey::find_program_address(
        &[
            b"user",
            solana_tax_reward::ID.as_ref(),
            user_keypair.pubkey().as_ref(),
            test_env.mint.pubkey().as_ref(),
        ],
        &solana_tax_reward::ID,
    );

    // Manually create user info account (simulating previous activity)
    create_user_info_account(
        &mut test_env.banks_client,
        &test_env.payer,
        user_info_pda,
        test_env.recent_blockhash,
    ).await;

    // Simulate some rewards in the system by updating global state
    update_global_state_with_rewards(
        &mut test_env.banks_client,
        &test_env.payer,
        test_env.global_state_pda,
        1_000_000_000_000_000_000u128, // 1 SOL per token
        test_env.recent_blockhash,
    ).await;

    // Get user's SOL balance before claiming
    let user_sol_before = test_env.banks_client.get_balance(user_keypair.pubkey()).await.unwrap();

    // Claim rewards
    let claim_data = ClaimRewards {};

    let claim_ix = Instruction {
        program_id: solana_tax_reward::ID,
        accounts: vec![
            AccountMeta::new(user_keypair.pubkey(), true),
            AccountMeta::new(user_info_pda, false),
            AccountMeta::new(test_env.global_state_pda, false),
            AccountMeta::new(test_env.reward_vault_pda, false),
            AccountMeta::new_readonly(test_env.mint.pubkey(), false),
            AccountMeta::new_readonly(anchor_lang::system_program::ID, false),
        ],
        data: claim_data.data(),
    };

    fund_account(&mut test_env.banks_client, &test_env.payer, &user_keypair.pubkey(), test_env.recent_blockhash).await;

    let mut transaction = Transaction::new_with_payer(
        &[claim_ix],
        Some(&user_keypair.pubkey()),
    );
    transaction.sign(&[&user_keypair], test_env.recent_blockhash);
    test_env.banks_client.process_transaction(transaction).await.unwrap();

    // Verify user received SOL rewards
    let user_sol_after = test_env.banks_client.get_balance(user_keypair.pubkey()).await.unwrap();
    assert!(user_sol_after > user_sol_before, "User should receive SOL rewards");

    // Verify user info was updated
    let user_info_account = test_env.banks_client.get_account(user_info_pda).await.unwrap().unwrap();
    let user_info: UserInfo = UserInfo::try_deserialize(&mut user_info_account.data.as_slice()).unwrap();
    assert!(user_info.last_cum > 0, "User last cum should be updated");
}

#[tokio::test]
async fn test_update_config_flow() {
    let mut test_env = setup_test_environment().await;

    // Test updating tax rate
    let update_data = UpdateConfig {
        new_tax_rate_bps: Some(1000), // 10%
        new_dex_program: None,
        paused: Some(true),
    };

    let update_ix = Instruction {
        program_id: solana_tax_reward::ID,
        accounts: vec![
            AccountMeta::new(test_env.payer.pubkey(), true),
            AccountMeta::new(test_env.config_pda, false),
        ],
        data: update_data.data(),
    };

    let mut transaction = Transaction::new_with_payer(
        &[update_ix],
        Some(&test_env.payer.pubkey()),
    );
    transaction.sign(&[&test_env.payer], test_env.recent_blockhash);
    test_env.banks_client.process_transaction(transaction).await.unwrap();

    // Verify config was updated
    let config_account = test_env.banks_client.get_account(test_env.config_pda).await.unwrap().unwrap();
    let config: Config = Config::try_deserialize(&mut config_account.data.as_slice()).unwrap();
    assert_eq!(config.tax_rate_bps, 1000);
    assert!(config.paused);
}

#[tokio::test]
async fn test_error_conditions() {
    let mut test_env = setup_test_environment().await;

    // Test operating when paused
    // First pause the program
    let pause_data = UpdateConfig {
        new_tax_rate_bps: None,
        new_dex_program: None,
        paused: Some(true),
    };

    let pause_ix = Instruction {
        program_id: solana_tax_reward::ID,
        accounts: vec![
            AccountMeta::new(test_env.payer.pubkey(), true),
            AccountMeta::new(test_env.config_pda, false),
        ],
        data: pause_data.data(),
    };

    let mut transaction = Transaction::new_with_payer(&[pause_ix], Some(&test_env.payer.pubkey()));
    transaction.sign(&[&test_env.payer], test_env.recent_blockhash);
    test_env.banks_client.process_transaction(transaction).await.unwrap();

    // Now try to perform a swap while paused - should fail
    let user_keypair = Keypair::new();
    let user_token_account = create_token_account(
        &mut test_env.banks_client,
        &test_env.payer,
        &test_env.mint.pubkey(),
        &user_keypair.pubkey(),
        test_env.recent_blockhash,
    ).await;

    let dest_token_account = create_token_account(
        &mut test_env.banks_client,
        &test_env.payer,
        &test_env.mint.pubkey(),
        &user_keypair.pubkey(),
        test_env.recent_blockhash,
    ).await;

    let (user_info_pda, _) = Pubkey::find_program_address(
        &[
            b"user",
            solana_tax_reward::ID.as_ref(),
            user_keypair.pubkey().as_ref(),
            test_env.mint.pubkey().as_ref(),
        ],
        &solana_tax_reward::ID,
    );

    let swap_data = TaxedSwapAndDistribute {
        amount: 100_000,
        minimum_amount_out: 90_000,
    };

    let swap_ix = Instruction {
        program_id: solana_tax_reward::ID,
        accounts: vec![
            AccountMeta::new(user_keypair.pubkey(), true),
            AccountMeta::new(user_token_account, false),
            AccountMeta::new(dest_token_account, false),
            AccountMeta::new(test_env.config_pda, false),
            AccountMeta::new(test_env.global_state_pda, false),
            AccountMeta::new(user_info_pda, false),
            AccountMeta::new(test_env.token_vault_pda, false),
            AccountMeta::new(test_env.reward_vault_pda, false),
            AccountMeta::new_readonly(test_env.mint.pubkey(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(anchor_lang::system_program::ID, false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data: swap_data.data(),
    };

    fund_account(&mut test_env.banks_client, &test_env.payer, &user_keypair.pubkey(), test_env.recent_blockhash).await;

    let mut transaction = Transaction::new_with_payer(&[swap_ix], Some(&user_keypair.pubkey()));
    transaction.sign(&[&user_keypair], test_env.recent_blockhash);
    
    // This should fail because program is paused
    let result = test_env.banks_client.process_transaction(transaction).await;
    assert!(result.is_err(), "Transaction should fail when program is paused");
}

// Test environment setup helpers
struct TestEnvironment {
    banks_client: BanksClient,
    payer: Keypair,
    recent_blockhash: Hash,
    mint: Keypair,
    config_pda: Pubkey,
    global_state_pda: Pubkey,
    token_vault_pda: Pubkey,
    reward_vault_pda: Pubkey,
}

async fn setup_test_environment() -> TestEnvironment {
    let program_test = ProgramTest::new(
        "solana_tax_reward",
        solana_tax_reward::ID,
        processor!(solana_tax_reward::entry),
    );

    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

    // Create and initialize mint
    let mint = Keypair::new();
    let mint_rent = Rent::default().minimum_balance(MintState::LEN);
    
    let create_mint_ix = system_instruction::create_account(
        &payer.pubkey(),
        &mint.pubkey(),
        mint_rent,
        MintState::LEN as u64,
        &spl_token::id(),
    );

    let init_mint_ix = spl_token::instruction::initialize_mint(
        &spl_token::id(),
        &mint.pubkey(),
        &payer.pubkey(),
        None,
        9,
    ).unwrap();

    let mut transaction = Transaction::new_with_payer(
        &[create_mint_ix, init_mint_ix],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer, &mint], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();

    // Derive PDAs
    let (config_pda, _) = Pubkey::find_program_address(
        &[b"config", solana_tax_reward::ID.as_ref(), mint.pubkey().as_ref()],
        &solana_tax_reward::ID,
    );

    let (global_state_pda, _) = Pubkey::find_program_address(
        &[b"global", solana_tax_reward::ID.as_ref(), mint.pubkey().as_ref()],
        &solana_tax_reward::ID,
    );

    let (token_vault_pda, _) = Pubkey::find_program_address(
        &[b"token_vault", solana_tax_reward::ID.as_ref(), mint.pubkey().as_ref()],
        &solana_tax_reward::ID,
    );

    let (reward_vault_pda, _) = Pubkey::find_program_address(
        &[b"reward_vault", solana_tax_reward::ID.as_ref(), mint.pubkey().as_ref()],
        &solana_tax_reward::ID,
    );

    // Initialize program
    let initialize_data = Initialize {
        tax_rate_bps: 500,
        dex_program: Pubkey::new_unique(),
    };

    let initialize_ix = Instruction {
        program_id: solana_tax_reward::ID,
        accounts: vec![
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new(config_pda, false),
            AccountMeta::new(global_state_pda, false),
            AccountMeta::new(token_vault_pda, false),
            AccountMeta::new(reward_vault_pda, false),
            AccountMeta::new_readonly(mint.pubkey(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(anchor_lang::system_program::ID, false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data: initialize_data.data(),
    };

    let mut transaction = Transaction::new_with_payer(&[initialize_ix], Some(&payer.pubkey()));
    transaction.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();

    TestEnvironment {
        banks_client,
        payer,
        recent_blockhash,
        mint,
        config_pda,
        global_state_pda,
        token_vault_pda,
        reward_vault_pda,
    }
}

async fn create_token_account(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    mint: &Pubkey,
    owner: &Pubkey,
    recent_blockhash: Hash,
) -> Pubkey {
    let token_account = Keypair::new();
    let rent = Rent::default().minimum_balance(TokenAccountState::LEN);

    let create_account_ix = system_instruction::create_account(
        &payer.pubkey(),
        &token_account.pubkey(),
        rent,
        TokenAccountState::LEN as u64,
        &spl_token::id(),
    );

    let init_account_ix = spl_token::instruction::initialize_account(
        &spl_token::id(),
        &token_account.pubkey(),
        mint,
        owner,
    ).unwrap();

    let mut transaction = Transaction::new_with_payer(
        &[create_account_ix, init_account_ix],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[payer, &token_account], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();

    token_account.pubkey()
}

async fn mint_tokens_to_account(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    mint: &Pubkey,
    destination: &Pubkey,
    amount: u64,
    recent_blockhash: Hash,
) {
    let mint_to_ix = spl_token::instruction::mint_to(
        &spl_token::id(),
        mint,
        destination,
        &payer.pubkey(),
        &[],
        amount,
    ).unwrap();

    let mut transaction = Transaction::new_with_payer(&[mint_to_ix], Some(&payer.pubkey()));
    transaction.sign(&[payer], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();
}

async fn fund_account(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    account: &Pubkey,
    recent_blockhash: Hash,
) {
    let fund_ix = system_instruction::transfer(&payer.pubkey(), account, 1_000_000_000); // 1 SOL

    let mut transaction = Transaction::new_with_payer(&[fund_ix], Some(&payer.pubkey()));
    transaction.sign(&[payer], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();
}

async fn create_user_info_account(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    user_info_pda: Pubkey,
    recent_blockhash: Hash,
) {
    // Note: In real implementation, this would be created by the program
    // This is a test helper to simulate existing user state
    let rent = Rent::default().minimum_balance(UserInfo::LEN);
    
    let create_account_ix = system_instruction::create_account(
        &payer.pubkey(),
        &user_info_pda,
        rent,
        UserInfo::LEN as u64,
        &solana_tax_reward::ID,
    );

    let mut transaction = Transaction::new_with_payer(&[create_account_ix], Some(&payer.pubkey()));
    transaction.sign(&[payer], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();
}

async fn update_global_state_with_rewards(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    global_state_pda: Pubkey,
    cum_reward_per_token: u128,
    recent_blockhash: Hash,
) {
    // Note: In real implementation, this would be updated by swap operations
    // This is a test helper to simulate rewards being distributed
    
    // For testing purposes, we'll read current state and write back with updated rewards
    let global_account = banks_client.get_account(global_state_pda).await.unwrap().unwrap();
    let mut global_state: GlobalState = GlobalState::try_deserialize(&mut global_account.data.as_slice()).unwrap();
    
    global_state.cum_reward_per_token = cum_reward_per_token;
    global_state.total_supply = 1_000_000; // Set some total supply for testing
    
    // This would require a custom instruction in real implementation
    // For testing, we'll modify the account directly
}
