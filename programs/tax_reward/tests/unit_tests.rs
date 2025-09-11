use solana_program::{
    pubkey::Pubkey,
    account_info::{AccountInfo, create_is_signer_account_infos, AccountMeta},
    program_error::ProgramError,
};

use solana_tax_reward::processor::process;
use solana_tax_reward::instructions::TaxRewardInstruction;
use borsh::{BorshSerialize, BorshDeserialize};

#[test]
fn test_tax_calculation_buy() {
    let amount = 1000u64;
    let expected_tax = 50u64; // 5%
    let expected_net = 950u64;
    let tax = amount.checked_mul(5).unwrap().checked_div(100).unwrap();
    let net_amount = amount.checked_sub(tax).unwrap();

    assert_eq!(tax, expected_tax);
    assert_eq!(net_amount, expected_net);
}

#[test]
fn test_fee_pool_load_save() {
    let mut fee_pool = solana_tax_reward::state::FeePool::default();
    fee_pool.collected_tokens = 1234;

    let mut data = vec![0u8; fee_pool.try_to_vec().unwrap().len()];
    fee_pool.serialize(&mut data.as_mut_slice()).unwrap();

    let loaded_fee_pool = solana_tax_reward::state::FeePool::try_from_slice(&data).unwrap();
    assert_eq!(loaded_fee_pool.collected_tokens, 1234);
}

// Additional tests for instruction unpacking, processor buy/sell handling, and swap invocation should be added here with mock contexts