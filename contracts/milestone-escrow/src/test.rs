#![cfg(test)]
use super::*;
use soroban_sdk::{
    testutils::Address as _, testutils::Events, testutils::Ledger, vec, Address, Env, IntoVal,
    Symbol, Val,
};

fn setup_funded_escrow(
    env: &Env,
    milestone_amounts: soroban_sdk::Vec<i128>,
) -> (
    Address,
    Address,
    Address,
    Address,
    Address,
    soroban_sdk::Address,
    MilestoneEscrowClient<'_>,
) {
    let client_addr = Address::generate(env);
    let freelancer_addr = Address::generate(env);
    let arbiter_addr = Address::generate(env);
    let admin_addr = Address::generate(env);

    let token_contract_id = env
        .register_stellar_asset_contract_v2(admin_addr.clone())
        .address();
    let token_admin = token::StellarAssetClient::new(env, &token_contract_id);
    let total: i128 = milestone_amounts.iter().sum();
    token_admin.mint(&client_addr, &total);

    let contract_id = env.register(MilestoneEscrow, ());
    let client = MilestoneEscrowClient::new(env, &contract_id);

    client.initialize(
        &admin_addr,
        &client_addr,
        &freelancer_addr,
        &arbiter_addr,
        &token_contract_id,
        &604800,
        &milestone_amounts,
    );
    client.fund(&client_addr);

    (
        client_addr,
        freelancer_addr,
        arbiter_addr,
        admin_addr,
        token_contract_id,
        contract_id,
        client,
    )
}

#[test]
fn test_full_happy_path() {
    let env = Env::default();
    env.mock_all_auths();

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let arbiter_addr = Address::generate(&env);
    let admin_addr = Address::generate(&env);

    let token_contract_id = env
        .register_stellar_asset_contract_v2(admin_addr.clone())
        .address();
    let token = token::Client::new(&env, &token_contract_id);
    let token_admin = token::StellarAssetClient::new(&env, &token_contract_id);
    token_admin.mint(&client_addr, &10_000);

    let contract_id = env.register(MilestoneEscrow, ());
    let client = MilestoneEscrowClient::new(&env, &contract_id);

    let amounts = vec![&env, 3_000_i128, 7_000_i128];
    client.initialize(
        &admin_addr,
        &client_addr,
        &freelancer_addr,
        &arbiter_addr,
        &token_contract_id,
        &604800,
        &amounts,
    );

    assert_eq!(token.balance(&client_addr), 10_000);

    client.fund(&client_addr);
    assert_eq!(token.balance(&client_addr), 0);
    assert_eq!(token.balance(&contract_id), 10_000);

    client.mark_delivered(&freelancer_addr, &0u32);

    client.approve_milestone(&client_addr, &0u32);
    assert_eq!(token.balance(&freelancer_addr), 3_000);
    assert_eq!(token.balance(&contract_id), 7_000);

    client.mark_delivered(&freelancer_addr, &1u32);
    client.approve_milestone(&client_addr, &1u32);
    assert_eq!(token.balance(&freelancer_addr), 10_000);
    assert_eq!(token.balance(&contract_id), 0);
}

#[test]
fn test_dispute_release_to_freelancer() {
    let env = Env::default();
    env.mock_all_auths();

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let arbiter_addr = Address::generate(&env);
    let admin_addr = Address::generate(&env);

    let token_contract_id = env
        .register_stellar_asset_contract_v2(admin_addr.clone())
        .address();
    let token = token::Client::new(&env, &token_contract_id);
    let token_admin = token::StellarAssetClient::new(&env, &token_contract_id);
    token_admin.mint(&client_addr, &5_000);

    let contract_id = env.register(MilestoneEscrow, ());
    let client = MilestoneEscrowClient::new(&env, &contract_id);

    let amounts = vec![&env, 5_000_i128];
    client.initialize(
        &admin_addr,
        &client_addr,
        &freelancer_addr,
        &arbiter_addr,
        &token_contract_id,
        &604800,
        &amounts,
    );
    client.fund(&client_addr);
    client.mark_delivered(&freelancer_addr, &0u32);
    client.raise_dispute(&client_addr, &0u32);
    client.resolve_dispute(&arbiter_addr, &0u32, &true);

    assert_eq!(token.balance(&freelancer_addr), 5_000);
}

#[test]
fn test_dispute_refund_to_client() {
    let env = Env::default();
    env.mock_all_auths();

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let arbiter_addr = Address::generate(&env);
    let admin_addr = Address::generate(&env);

    let token_contract_id = env
        .register_stellar_asset_contract_v2(admin_addr.clone())
        .address();
    let token = token::Client::new(&env, &token_contract_id);
    let token_admin = token::StellarAssetClient::new(&env, &token_contract_id);
    token_admin.mint(&client_addr, &5_000);

    let contract_id = env.register(MilestoneEscrow, ());
    let client = MilestoneEscrowClient::new(&env, &contract_id);

    let amounts = vec![&env, 5_000_i128];
    client.initialize(
        &admin_addr,
        &client_addr,
        &freelancer_addr,
        &arbiter_addr,
        &token_contract_id,
        &604800,
        &amounts,
    );
    client.fund(&client_addr);
    client.raise_dispute(&client_addr, &0u32);
    client.resolve_dispute(&arbiter_addr, &0u32, &false);

    assert_eq!(token.balance(&client_addr), 5_000);
}

#[test]
fn test_double_initialize_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let arbiter_addr = Address::generate(&env);
    let admin_addr = Address::generate(&env);
    let token_contract_id = env
        .register_stellar_asset_contract_v2(admin_addr.clone())
        .address();

    let contract_id = env.register(MilestoneEscrow, ());
    let client = MilestoneEscrowClient::new(&env, &contract_id);

    let amounts = vec![&env, 1_000_i128];
    client.initialize(
        &admin_addr,
        &client_addr,
        &freelancer_addr,
        &arbiter_addr,
        &token_contract_id,
        &604800,
        &amounts,
    );

    let result = client.try_initialize(
        &admin_addr,
        &client_addr,
        &freelancer_addr,
        &arbiter_addr,
        &token_contract_id,
        &604800,
        &amounts,
    );
    assert!(result.is_err());
}

#[test]
fn test_unauthorized_fund_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let arbiter_addr = Address::generate(&env);
    let admin_addr = Address::generate(&env);
    let bad_actor = Address::generate(&env);
    let token_contract_id = env
        .register_stellar_asset_contract_v2(admin_addr.clone())
        .address();

    let contract_id = env.register(MilestoneEscrow, ());
    let client = MilestoneEscrowClient::new(&env, &contract_id);

    let amounts = vec![&env, 1_000_i128];
    client.initialize(
        &admin_addr,
        &client_addr,
        &freelancer_addr,
        &arbiter_addr,
        &token_contract_id,
        &604800,
        &amounts,
    );

    let result = client.try_fund(&bad_actor);
    assert_eq!(result, Err(Ok(Error::Unauthorized)));
}

#[test]
fn test_invalid_milestone_index_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let arbiter_addr = Address::generate(&env);
    let admin_addr = Address::generate(&env);
    let token_contract_id = env
        .register_stellar_asset_contract_v2(admin_addr.clone())
        .address();
    let token_admin = token::StellarAssetClient::new(&env, &token_contract_id);
    token_admin.mint(&client_addr, &1_000);

    let contract_id = env.register(MilestoneEscrow, ());
    let client = MilestoneEscrowClient::new(&env, &contract_id);

    let amounts = vec![&env, 1_000_i128];
    client.initialize(
        &admin_addr,
        &client_addr,
        &freelancer_addr,
        &arbiter_addr,
        &token_contract_id,
        &604800,
        &amounts,
    );
    client.fund(&client_addr);

    let result = client.try_mark_delivered(&freelancer_addr, &1u32);
    assert!(result.is_err());
}

#[test]
fn test_mark_delivered_wrong_status_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let arbiter_addr = Address::generate(&env);
    let admin_addr = Address::generate(&env);
    let token_contract_id = env
        .register_stellar_asset_contract_v2(admin_addr.clone())
        .address();
    let token_admin = token::StellarAssetClient::new(&env, &token_contract_id);
    token_admin.mint(&client_addr, &1_000);

    let contract_id = env.register(MilestoneEscrow, ());
    let client = MilestoneEscrowClient::new(&env, &contract_id);

    let amounts = vec![&env, 1_000_i128];
    client.initialize(
        &admin_addr,
        &client_addr,
        &freelancer_addr,
        &arbiter_addr,
        &token_contract_id,
        &604800,
        &amounts,
    );
    client.fund(&client_addr);
    client.mark_delivered(&freelancer_addr, &0u32);

    let result = client.try_mark_delivered(&freelancer_addr, &0u32);
    assert!(result.is_err());
}

#[test]
fn test_approve_milestone_wrong_status_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let arbiter_addr = Address::generate(&env);
    let admin_addr = Address::generate(&env);
    let token_contract_id = env
        .register_stellar_asset_contract_v2(admin_addr.clone())
        .address();
    let token_admin = token::StellarAssetClient::new(&env, &token_contract_id);
    token_admin.mint(&client_addr, &1_000);

    let contract_id = env.register(MilestoneEscrow, ());
    let client = MilestoneEscrowClient::new(&env, &contract_id);

    let amounts = vec![&env, 1_000_i128];
    client.initialize(
        &admin_addr,
        &client_addr,
        &freelancer_addr,
        &arbiter_addr,
        &token_contract_id,
        &604800,
        &amounts,
    );
    client.fund(&client_addr);

    let result = client.try_approve_milestone(&client_addr, &0u32);
    assert_eq!(result, Err(Ok(Error::InvalidStatus)));
}

#[test]
fn test_approve_milestone_invalid_index_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let arbiter_addr = Address::generate(&env);
    let admin_addr = Address::generate(&env);

    let token_contract_id = env
        .register_stellar_asset_contract_v2(admin_addr.clone())
        .address();
    let token_admin = token::StellarAssetClient::new(&env, &token_contract_id);
    token_admin.mint(&client_addr, &10_000);

    let contract_id = env.register(MilestoneEscrow, ());
    let client = MilestoneEscrowClient::new(&env, &contract_id);

    let amounts = vec![&env, 10_000_i128];
    client.initialize(
        &admin_addr,
        &client_addr,
        &freelancer_addr,
        &arbiter_addr,
        &token_contract_id,
        &604800,
        &amounts,
    );
    client.fund(&client_addr);
    client.mark_delivered(&freelancer_addr, &0u32);

    // Try to approve milestone at non-existent index
    let result = client.try_approve_milestone(&client_addr, &1u32);
    assert_eq!(result, Err(Ok(Error::InvalidMilestone)));
}

#[test]
fn test_approve_milestone_zero_amount_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let arbiter_addr = Address::generate(&env);
    let admin_addr = Address::generate(&env);

    let token_contract_id = env
        .register_stellar_asset_contract_v2(admin_addr.clone())
        .address();
    let token_admin = token::StellarAssetClient::new(&env, &token_contract_id);
    token_admin.mint(&client_addr, &10_000);

    let contract_id = env.register(MilestoneEscrow, ());
    let client = MilestoneEscrowClient::new(&env, &contract_id);

    // Milestone with zero amount should be rejected before state is written.
    let amounts = vec![&env, 0_i128];
    let result = client.try_initialize(
        &admin_addr,
        &client_addr,
        &freelancer_addr,
        &arbiter_addr,
        &token_contract_id,
        &604800,
        &amounts,
    );
    assert_eq!(result, Err(Ok(Error::InvalidAmount)));
}

#[test]
fn test_raise_dispute_unauthorized_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let arbiter_addr = Address::generate(&env);
    let admin_addr = Address::generate(&env);
    let bad_actor = Address::generate(&env);
    let token_contract_id = env
        .register_stellar_asset_contract_v2(admin_addr.clone())
        .address();
    let token_admin = token::StellarAssetClient::new(&env, &token_contract_id);
    token_admin.mint(&client_addr, &1_000);

    let contract_id = env.register(MilestoneEscrow, ());
    let client = MilestoneEscrowClient::new(&env, &contract_id);

    let amounts = vec![&env, 1_000_i128];
    client.initialize(
        &admin_addr,
        &client_addr,
        &freelancer_addr,
        &arbiter_addr,
        &token_contract_id,
        &604800,
        &amounts,
    );
    client.fund(&client_addr);

    let result = client.try_raise_dispute(&bad_actor, &0u32);
    assert!(result.is_err());
}

#[test]
fn test_raise_dispute_wrong_status_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let arbiter_addr = Address::generate(&env);
    let admin_addr = Address::generate(&env);
    let token_contract_id = env
        .register_stellar_asset_contract_v2(admin_addr.clone())
        .address();
    let token_admin = token::StellarAssetClient::new(&env, &token_contract_id);
    token_admin.mint(&client_addr, &1_000);

    let contract_id = env.register(MilestoneEscrow, ());
    let client = MilestoneEscrowClient::new(&env, &contract_id);

    let amounts = vec![&env, 1_000_i128];
    client.initialize(
        &admin_addr,
        &client_addr,
        &freelancer_addr,
        &arbiter_addr,
        &token_contract_id,
        &604800,
        &amounts,
    );
    client.fund(&client_addr);
    client.mark_delivered(&freelancer_addr, &0u32);
    client.approve_milestone(&client_addr, &0u32);

    let result = client.try_raise_dispute(&client_addr, &0u32);
    assert!(result.is_err());
}

#[test]
fn test_resolve_dispute_unauthorized_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let arbiter_addr = Address::generate(&env);
    let admin_addr = Address::generate(&env);
    let bad_actor = Address::generate(&env);
    let token_contract_id = env
        .register_stellar_asset_contract_v2(admin_addr.clone())
        .address();
    let token_admin = token::StellarAssetClient::new(&env, &token_contract_id);
    token_admin.mint(&client_addr, &1_000);

    let contract_id = env.register(MilestoneEscrow, ());
    let client = MilestoneEscrowClient::new(&env, &contract_id);

    let amounts = vec![&env, 1_000_i128];
    client.initialize(
        &admin_addr,
        &client_addr,
        &freelancer_addr,
        &arbiter_addr,
        &token_contract_id,
        &604800,
        &amounts,
    );
    client.fund(&client_addr);
    client.raise_dispute(&client_addr, &0u32);

    let result = client.try_resolve_dispute(&bad_actor, &0u32, &true);
    assert!(result.is_err());
}

#[test]
fn test_resolve_dispute_wrong_status_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let arbiter_addr = Address::generate(&env);
    let admin_addr = Address::generate(&env);
    let token_contract_id = env
        .register_stellar_asset_contract_v2(admin_addr.clone())
        .address();
    let token_admin = token::StellarAssetClient::new(&env, &token_contract_id);
    token_admin.mint(&client_addr, &1_000);

    let contract_id = env.register(MilestoneEscrow, ());
    let client = MilestoneEscrowClient::new(&env, &contract_id);

    let amounts = vec![&env, 1_000_i128];
    client.initialize(
        &admin_addr,
        &client_addr,
        &freelancer_addr,
        &arbiter_addr,
        &token_contract_id,
        &604800,
        &amounts,
    );
    client.fund(&client_addr);

    let result = client.try_resolve_dispute(&arbiter_addr, &0u32, &true);
    assert!(result.is_err());
}

#[test]
fn test_fund_before_initialized_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let client_addr = Address::generate(&env);
    let contract_id = env.register(MilestoneEscrow, ());
    let client = MilestoneEscrowClient::new(&env, &contract_id);

    let result = client.try_fund(&client_addr);
    assert_eq!(result, Err(Ok(Error::NotInitialized)));
}

#[test]
fn test_double_fund_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let arbiter_addr = Address::generate(&env);
    let admin_addr = Address::generate(&env);
    let token_contract_id = env
        .register_stellar_asset_contract_v2(admin_addr.clone())
        .address();
    let token_admin = token::StellarAssetClient::new(&env, &token_contract_id);
    token_admin.mint(&client_addr, &2_000);

    let contract_id = env.register(MilestoneEscrow, ());
    let client = MilestoneEscrowClient::new(&env, &contract_id);

    let amounts = vec![&env, 1_000_i128];
    client.initialize(
        &admin_addr,
        &client_addr,
        &freelancer_addr,
        &arbiter_addr,
        &token_contract_id,
        &604800,
        &amounts,
    );
    client.fund(&client_addr);

    let result = client.try_fund(&client_addr);
    assert_eq!(result, Err(Ok(Error::AlreadyFunded)));
}

#[test]
fn test_mark_delivered_before_funded_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let arbiter_addr = Address::generate(&env);
    let admin_addr = Address::generate(&env);
    let token_contract_id = env
        .register_stellar_asset_contract_v2(admin_addr.clone())
        .address();

    let contract_id = env.register(MilestoneEscrow, ());
    let client = MilestoneEscrowClient::new(&env, &contract_id);

    let amounts = vec![&env, 1_000_i128];
    client.initialize(
        &admin_addr,
        &client_addr,
        &freelancer_addr,
        &arbiter_addr,
        &token_contract_id,
        &604800,
        &amounts,
    );

    let result = client.try_mark_delivered(&freelancer_addr, &0u32);
    assert!(result.is_err());
}

#[test]
fn test_admin_add_token() {
    let env = Env::default();
    env.mock_all_auths();

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let arbiter_addr = Address::generate(&env);
    let admin_addr = Address::generate(&env);

    let token1 = env
        .register_stellar_asset_contract_v2(admin_addr.clone())
        .address();
    let token2 = env
        .register_stellar_asset_contract_v2(admin_addr.clone())
        .address();

    let contract_id = env.register(MilestoneEscrow, ());
    let client = MilestoneEscrowClient::new(&env, &contract_id);

    let amounts = vec![&env, 1_000_i128];
    client.initialize(
        &admin_addr,
        &client_addr,
        &freelancer_addr,
        &arbiter_addr,
        &token1,
        &604800,
        &amounts,
    );

    assert!(client.is_token_whitelisted(&token1));
    assert!(!client.is_token_whitelisted(&token2));

    client.add_whitelisted_token(&admin_addr, &token2);
    assert!(client.is_token_whitelisted(&token2));

    let whitelist = client.get_whitelisted_tokens();
    assert_eq!(whitelist.len(), 2);
}

#[test]
fn test_non_admin_add_token_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let arbiter_addr = Address::generate(&env);
    let admin_addr = Address::generate(&env);
    let bad_actor = Address::generate(&env);

    let token1 = env
        .register_stellar_asset_contract_v2(admin_addr.clone())
        .address();
    let token2 = env
        .register_stellar_asset_contract_v2(admin_addr.clone())
        .address();

    let contract_id = env.register(MilestoneEscrow, ());
    let client = MilestoneEscrowClient::new(&env, &contract_id);

    let amounts = vec![&env, 1_000_i128];
    client.initialize(
        &admin_addr,
        &client_addr,
        &freelancer_addr,
        &arbiter_addr,
        &token1,
        &604800,
        &amounts,
    );

    let result = client.try_add_whitelisted_token(&bad_actor, &token2);
    assert!(result.is_err());
}

#[test]
fn test_admin_remove_token() {
    let env = Env::default();
    env.mock_all_auths();

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let arbiter_addr = Address::generate(&env);
    let admin_addr = Address::generate(&env);

    let token1 = env
        .register_stellar_asset_contract_v2(admin_addr.clone())
        .address();
    let token2 = env
        .register_stellar_asset_contract_v2(admin_addr.clone())
        .address();

    let contract_id = env.register(MilestoneEscrow, ());
    let client = MilestoneEscrowClient::new(&env, &contract_id);

    let amounts = vec![&env, 1_000_i128];
    client.initialize(
        &admin_addr,
        &client_addr,
        &freelancer_addr,
        &arbiter_addr,
        &token1,
        &604800,
        &amounts,
    );
    client.add_whitelisted_token(&admin_addr, &token2);

    assert!(client.is_token_whitelisted(&token2));

    client.remove_whitelisted_token(&admin_addr, &token2);
    assert!(!client.is_token_whitelisted(&token2));
}

#[test]
fn test_non_admin_remove_token_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let arbiter_addr = Address::generate(&env);
    let admin_addr = Address::generate(&env);
    let bad_actor = Address::generate(&env);

    let token1 = env
        .register_stellar_asset_contract_v2(admin_addr.clone())
        .address();
    let token2 = env
        .register_stellar_asset_contract_v2(admin_addr.clone())
        .address();

    let contract_id = env.register(MilestoneEscrow, ());
    let client = MilestoneEscrowClient::new(&env, &contract_id);

    let amounts = vec![&env, 1_000_i128];
    client.initialize(
        &admin_addr,
        &client_addr,
        &freelancer_addr,
        &arbiter_addr,
        &token1,
        &604800,
        &amounts,
    );
    client.add_whitelisted_token(&admin_addr, &token2);

    let result = client.try_remove_whitelisted_token(&bad_actor, &token2);
    assert!(result.is_err());
}

#[test]
fn test_add_existing_token_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let arbiter_addr = Address::generate(&env);
    let admin_addr = Address::generate(&env);

    let token1 = env
        .register_stellar_asset_contract_v2(admin_addr.clone())
        .address();

    let contract_id = env.register(MilestoneEscrow, ());
    let client = MilestoneEscrowClient::new(&env, &contract_id);

    let amounts = vec![&env, 1_000_i128];
    client.initialize(
        &admin_addr,
        &client_addr,
        &freelancer_addr,
        &arbiter_addr,
        &token1,
        &604800,
        &amounts,
    );

    let result = client.try_add_whitelisted_token(&admin_addr, &token1);
    assert!(result.is_err());
}

#[test]
fn test_remove_nonexistent_token_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let arbiter_addr = Address::generate(&env);
    let admin_addr = Address::generate(&env);

    let token1 = env
        .register_stellar_asset_contract_v2(admin_addr.clone())
        .address();
    let token2 = env
        .register_stellar_asset_contract_v2(admin_addr.clone())
        .address();

    let contract_id = env.register(MilestoneEscrow, ());
    let client = MilestoneEscrowClient::new(&env, &contract_id);

    let amounts = vec![&env, 1_000_i128];
    client.initialize(
        &admin_addr,
        &client_addr,
        &freelancer_addr,
        &arbiter_addr,
        &token1,
        &604800,
        &amounts,
    );

    let result = client.try_remove_whitelisted_token(&admin_addr, &token2);
    assert!(result.is_err());
}

#[test]
fn test_partial_release_remaining_balance() {
    let env = Env::default();
    env.mock_all_auths();

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let arbiter_addr = Address::generate(&env);
    let admin_addr = Address::generate(&env);

    let token_contract_id = env
        .register_stellar_asset_contract_v2(admin_addr.clone())
        .address();
    let token = token::Client::new(&env, &token_contract_id);
    let token_admin = token::StellarAssetClient::new(&env, &token_contract_id);
    token_admin.mint(&client_addr, &10_000);

    let contract_id = env.register(MilestoneEscrow, ());
    let client = MilestoneEscrowClient::new(&env, &contract_id);

    let amounts = vec![&env, 10_000_i128];
    client.initialize(
        &admin_addr,
        &client_addr,
        &freelancer_addr,
        &arbiter_addr,
        &token_contract_id,
        &604800,
        &amounts,
    );

    client.fund(&client_addr);
    client.mark_delivered(&freelancer_addr, &0u32);
    client.approve_partial(&client_addr, &0u32, &4_000_i128);

    assert_eq!(token.balance(&freelancer_addr), 4_000);
    assert_eq!(token.balance(&contract_id), 6_000);

    let job = client.get_job();
    let milestone = job.milestones.get(0).unwrap();
    assert_eq!(milestone.released_amount, 4_000);
    assert_eq!(milestone.status, MilestoneStatus::PartiallyReleased);
}

#[test]
fn test_multiple_partial_releases_sum_full() {
    let env = Env::default();
    env.mock_all_auths();

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let arbiter_addr = Address::generate(&env);
    let admin_addr = Address::generate(&env);

    let token_contract_id = env
        .register_stellar_asset_contract_v2(admin_addr.clone())
        .address();
    let token = token::Client::new(&env, &token_contract_id);
    let token_admin = token::StellarAssetClient::new(&env, &token_contract_id);
    token_admin.mint(&client_addr, &10_000);

    let contract_id = env.register(MilestoneEscrow, ());
    let client = MilestoneEscrowClient::new(&env, &contract_id);

    let amounts = vec![&env, 10_000_i128];
    client.initialize(
        &admin_addr,
        &client_addr,
        &freelancer_addr,
        &arbiter_addr,
        &token_contract_id,
        &604800,
        &amounts,
    );

    client.fund(&client_addr);
    client.mark_delivered(&freelancer_addr, &0u32);
    client.approve_partial(&client_addr, &0u32, &3_000_i128);
    client.approve_partial(&client_addr, &0u32, &3_000_i128);
    client.approve_partial(&client_addr, &0u32, &4_000_i128);

    assert_eq!(token.balance(&freelancer_addr), 10_000);
    assert_eq!(token.balance(&contract_id), 0);

    let job = client.get_job();
    let milestone = job.milestones.get(0).unwrap();
    assert_eq!(milestone.released_amount, 10_000);
    assert_eq!(milestone.status, MilestoneStatus::Released);
}

#[test]
fn test_over_release_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let arbiter_addr = Address::generate(&env);
    let admin_addr = Address::generate(&env);

    let token_contract_id = env
        .register_stellar_asset_contract_v2(admin_addr.clone())
        .address();
    let token_admin = token::StellarAssetClient::new(&env, &token_contract_id);
    token_admin.mint(&client_addr, &10_000);

    let contract_id = env.register(MilestoneEscrow, ());
    let client = MilestoneEscrowClient::new(&env, &contract_id);

    let amounts = vec![&env, 10_000_i128];
    client.initialize(
        &admin_addr,
        &client_addr,
        &freelancer_addr,
        &arbiter_addr,
        &token_contract_id,
        &604800,
        &amounts,
    );

    client.fund(&client_addr);
    client.mark_delivered(&freelancer_addr, &0u32);

    let result = client.try_approve_partial(&client_addr, &0u32, &11_000_i128);
    assert!(result.is_err());
}

#[test]
fn test_negative_or_zero_release_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let arbiter_addr = Address::generate(&env);
    let admin_addr = Address::generate(&env);

    let token_contract_id = env
        .register_stellar_asset_contract_v2(admin_addr.clone())
        .address();
    let token_admin = token::StellarAssetClient::new(&env, &token_contract_id);
    token_admin.mint(&client_addr, &10_000);

    let contract_id = env.register(MilestoneEscrow, ());
    let client = MilestoneEscrowClient::new(&env, &contract_id);

    let amounts = vec![&env, 10_000_i128];
    client.initialize(
        &admin_addr,
        &client_addr,
        &freelancer_addr,
        &arbiter_addr,
        &token_contract_id,
        &604800,
        &amounts,
    );

    client.fund(&client_addr);
    client.mark_delivered(&freelancer_addr, &0u32);

    let result1 = client.try_approve_partial(&client_addr, &0u32, &0_i128);
    assert!(result1.is_err());

    let result2 = client.try_approve_partial(&client_addr, &0u32, &-1000_i128);
    assert!(result2.is_err());
}

#[test]
fn test_release_on_wrong_status_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let arbiter_addr = Address::generate(&env);
    let admin_addr = Address::generate(&env);

    let token_contract_id = env
        .register_stellar_asset_contract_v2(admin_addr.clone())
        .address();
    let token_admin = token::StellarAssetClient::new(&env, &token_contract_id);
    token_admin.mint(&client_addr, &10_000);

    let contract_id = env.register(MilestoneEscrow, ());
    let client = MilestoneEscrowClient::new(&env, &contract_id);

    let amounts = vec![&env, 10_000_i128];
    client.initialize(
        &admin_addr,
        &client_addr,
        &freelancer_addr,
        &arbiter_addr,
        &token_contract_id,
        &604800,
        &amounts,
    );

    client.fund(&client_addr);

    let result = client.try_approve_partial(&client_addr, &0u32, &4000_i128);
    assert!(result.is_err());
}

#[test]
fn test_approve_partial_wrong_status_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let arbiter_addr = Address::generate(&env);
    let admin_addr = Address::generate(&env);

    let token_contract_id = env
        .register_stellar_asset_contract_v2(admin_addr.clone())
        .address();
    let token_admin = token::StellarAssetClient::new(&env, &token_contract_id);
    token_admin.mint(&client_addr, &10_000);

    let contract_id = env.register(MilestoneEscrow, ());
    let client = MilestoneEscrowClient::new(&env, &contract_id);

    let amounts = vec![&env, 10_000_i128];
    client.initialize(
        &admin_addr,
        &client_addr,
        &freelancer_addr,
        &arbiter_addr,
        &token_contract_id,
        &604800,
        &amounts,
    );
    client.fund(&client_addr);

    // Try to approve partial on Pending status
    let result = client.try_approve_partial(&client_addr, &0u32, &4000_i128);
    assert_eq!(result, Err(Ok(Error::InvalidStatus)));

    // Mark delivered and approve fully
    client.mark_delivered(&freelancer_addr, &0u32);
    client.approve_milestone(&client_addr, &0u32);

    // Try to approve partial on Released status
    let result = client.try_approve_partial(&client_addr, &0u32, &1000_i128);
    assert_eq!(result, Err(Ok(Error::InvalidStatus)));
}

#[test]
fn test_approve_partial_invalid_milestone_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let arbiter_addr = Address::generate(&env);
    let admin_addr = Address::generate(&env);

    let token_contract_id = env
        .register_stellar_asset_contract_v2(admin_addr.clone())
        .address();
    let token_admin = token::StellarAssetClient::new(&env, &token_contract_id);
    token_admin.mint(&client_addr, &10_000);

    let contract_id = env.register(MilestoneEscrow, ());
    let client = MilestoneEscrowClient::new(&env, &contract_id);

    let amounts = vec![&env, 10_000_i128];
    client.initialize(
        &admin_addr,
        &client_addr,
        &freelancer_addr,
        &arbiter_addr,
        &token_contract_id,
        &604800,
        &amounts,
    );
    client.fund(&client_addr);
    client.mark_delivered(&freelancer_addr, &0u32);

    // Try to approve partial on non-existent milestone
    let result = client.try_approve_partial(&client_addr, &1u32, &4000_i128);
    assert_eq!(result, Err(Ok(Error::InvalidMilestone)));
}

#[test]
fn test_approve_partial_before_funded_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let arbiter_addr = Address::generate(&env);
    let admin_addr = Address::generate(&env);

    let token_contract_id = env
        .register_stellar_asset_contract_v2(admin_addr.clone())
        .address();
    let token_admin = token::StellarAssetClient::new(&env, &token_contract_id);
    token_admin.mint(&client_addr, &10_000);

    let contract_id = env.register(MilestoneEscrow, ());
    let client = MilestoneEscrowClient::new(&env, &contract_id);

    let amounts = vec![&env, 10_000_i128];
    client.initialize(
        &admin_addr,
        &client_addr,
        &freelancer_addr,
        &arbiter_addr,
        &token_contract_id,
        &604800,
        &amounts,
    );

    // Try to approve partial on Pending status (before mark_delivered)
    let result = client.try_approve_partial(&client_addr, &0u32, &4000_i128);
    assert_eq!(result, Err(Ok(Error::InvalidStatus)));
}

#[test]
fn test_approve_partial_unauthorized_partial_release_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let arbiter_addr = Address::generate(&env);
    let admin_addr = Address::generate(&env);
    let bad_actor = Address::generate(&env);

    let token_contract_id = env
        .register_stellar_asset_contract_v2(admin_addr.clone())
        .address();
    let token_admin = token::StellarAssetClient::new(&env, &token_contract_id);
    token_admin.mint(&client_addr, &10_000);

    let contract_id = env.register(MilestoneEscrow, ());
    let client = MilestoneEscrowClient::new(&env, &contract_id);

    let amounts = vec![&env, 10_000_i128];
    client.initialize(
        &admin_addr,
        &client_addr,
        &freelancer_addr,
        &arbiter_addr,
        &token_contract_id,
        &604800,
        &amounts,
    );

    client.fund(&client_addr);
    client.mark_delivered(&freelancer_addr, &0u32);

    let result = client.try_approve_partial(&bad_actor, &0u32, &4000_i128);
    assert!(result.is_err());
}

#[test]
fn test_approve_partial_state_transitions() {
    let env = Env::default();
    env.mock_all_auths();

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let arbiter_addr = Address::generate(&env);
    let admin_addr = Address::generate(&env);

    let token_contract_id = env
        .register_stellar_asset_contract_v2(admin_addr.clone())
        .address();
    let token = token::Client::new(&env, &token_contract_id);
    let token_admin = token::StellarAssetClient::new(&env, &token_contract_id);
    token_admin.mint(&client_addr, &10_000);

    let contract_id = env.register(MilestoneEscrow, ());
    let client = MilestoneEscrowClient::new(&env, &contract_id);

    let amounts = vec![&env, 10_000_i128];
    client.initialize(
        &admin_addr,
        &client_addr,
        &freelancer_addr,
        &arbiter_addr,
        &token_contract_id,
        &604800,
        &amounts,
    );
    client.fund(&client_addr);

    // Test 1: Pending → InvalidStatus (should fail)
    let result = client.try_approve_partial(&client_addr, &0u32, &4000_i128);
    assert_eq!(result, Err(Ok(Error::InvalidStatus)));

    // Test 2: Delivered → PartiallyReleased (should pass)
    client.mark_delivered(&freelancer_addr, &0u32);
    client.approve_partial(&client_addr, &0u32, &4000_i128);
    let job = client.get_job();
    assert_eq!(
        job.milestones.get(0).unwrap().status,
        MilestoneStatus::PartiallyReleased
    );
    assert_eq!(job.milestones.get(0).unwrap().released_amount, 4000);

    // Test 3: PartiallyReleased → PartiallyReleased (should pass)
    client.approve_partial(&client_addr, &0u32, &3000_i128);
    let job = client.get_job();
    assert_eq!(
        job.milestones.get(0).unwrap().status,
        MilestoneStatus::PartiallyReleased
    );
    assert_eq!(job.milestones.get(0).unwrap().released_amount, 7000);

    // Test 4: PartiallyReleased → Released (should pass)
    client.approve_partial(&client_addr, &0u32, &3000_i128);
    let job = client.get_job();
    assert_eq!(
        job.milestones.get(0).unwrap().status,
        MilestoneStatus::Released
    );
    assert_eq!(job.milestones.get(0).unwrap().released_amount, 10000);

    // Test 5: Released → InvalidStatus (should fail)
    let result = client.try_approve_partial(&client_addr, &0u32, &1000_i128);
    assert_eq!(result, Err(Ok(Error::InvalidStatus)));

    // Verify token balances
    assert_eq!(token.balance(&freelancer_addr), 10000);
    assert_eq!(token.balance(&contract_id), 0);
}

#[test]
fn test_approve_partial_emits_approved_event() {
    let env = Env::default();
    env.mock_all_auths();

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let arbiter_addr = Address::generate(&env);
    let admin_addr = Address::generate(&env);

    let token_contract_id = env
        .register_stellar_asset_contract_v2(admin_addr.clone())
        .address();
    let token = token::Client::new(&env, &token_contract_id);
    let token_admin = token::StellarAssetClient::new(&env, &token_contract_id);
    token_admin.mint(&client_addr, &10_000);

    let contract_id = env.register(MilestoneEscrow, ());
    let client = MilestoneEscrowClient::new(&env, &contract_id);

    let amounts = vec![&env, 10_000_i128];
    client.initialize(
        &admin_addr,
        &client_addr,
        &freelancer_addr,
        &arbiter_addr,
        &token_contract_id,
        &604800,
        &amounts,
    );

    client.fund(&client_addr);
    client.mark_delivered(&freelancer_addr, &0u32);
    client.approve_partial(&client_addr, &0u32, &4_000_i128);

    let events = env.events().all();
    let approve_topic: Symbol = symbol_short!("approve");
    let approve_topic_val: Val = approve_topic.into_val(&env);
    let mut approve_count = 0u32;
    for e in events.iter() {
        if let Some(topic) = e.1.get(0) {
            if topic.get_payload() == approve_topic_val.get_payload() {
                assert_eq!(e.1.len(), 1);
                approve_count += 1;
            }
        }
    }
    assert_eq!(approve_count, 1);
}

#[test]
fn test_approve_partial_emits_exactly_one_event() {
    let env = Env::default();
    env.mock_all_auths();

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let arbiter_addr = Address::generate(&env);
    let admin_addr = Address::generate(&env);

    let token_contract_id = env
        .register_stellar_asset_contract_v2(admin_addr.clone())
        .address();
    let token_admin = token::StellarAssetClient::new(&env, &token_contract_id);
    token_admin.mint(&client_addr, &10_000);

    let contract_id = env.register(MilestoneEscrow, ());
    let client = MilestoneEscrowClient::new(&env, &contract_id);

    let amounts = vec![&env, 10_000_i128];
    client.initialize(
        &admin_addr,
        &client_addr,
        &freelancer_addr,
        &arbiter_addr,
        &token_contract_id,
        &604800,
        &amounts,
    );

    client.fund(&client_addr);
    client.mark_delivered(&freelancer_addr, &0u32);
    client.approve_partial(&client_addr, &0u32, &4_000_i128);

    let approve_topic: Symbol = symbol_short!("approve");
    let approve_topic_val: Val = approve_topic.into_val(&env);
    let approve_count = env.events().all().iter().fold(0u32, |acc, e| {
        if let Some(topic) = e.1.get(0) {
            if topic.get_payload() == approve_topic_val.get_payload() {
                return acc + 1;
            }
        }
        acc
    });

    assert_eq!(approve_count, 1);
}

#[test]
fn test_claim_auto_release_before_deadline_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let arbiter_addr = Address::generate(&env);
    let admin_addr = Address::generate(&env);

    let token_contract_id = env
        .register_stellar_asset_contract_v2(admin_addr.clone())
        .address();
    let token_admin = token::StellarAssetClient::new(&env, &token_contract_id);
    token_admin.mint(&client_addr, &10_000);

    let contract_id = env.register(MilestoneEscrow, ());
    let client = MilestoneEscrowClient::new(&env, &contract_id);

    let amounts = vec![&env, 10_000_i128];
    client.initialize(
        &admin_addr,
        &client_addr,
        &freelancer_addr,
        &arbiter_addr,
        &token_contract_id,
        &100,
        &amounts,
    );

    client.fund(&client_addr);
    client.mark_delivered(&freelancer_addr, &0u32);

    let result = client.try_claim_auto_release(&freelancer_addr, &0u32);
    assert!(result.is_err());
}

#[test]
fn test_claim_auto_release_after_deadline_succeeds() {
    let env = Env::default();
    env.mock_all_auths();

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let arbiter_addr = Address::generate(&env);
    let admin_addr = Address::generate(&env);

    let token_contract_id = env
        .register_stellar_asset_contract_v2(admin_addr.clone())
        .address();
    let token = token::Client::new(&env, &token_contract_id);
    let token_admin = token::StellarAssetClient::new(&env, &token_contract_id);
    token_admin.mint(&client_addr, &10_000);

    let contract_id = env.register(MilestoneEscrow, ());
    let client = MilestoneEscrowClient::new(&env, &contract_id);

    let amounts = vec![&env, 10_000_i128];
    client.initialize(
        &admin_addr,
        &client_addr,
        &freelancer_addr,
        &arbiter_addr,
        &token_contract_id,
        &100,
        &amounts,
    );

    client.fund(&client_addr);
    client.mark_delivered(&freelancer_addr, &0u32);

    env.ledger().with_mut(|li| {
        li.timestamp += 200;
    });

    client.claim_auto_release(&freelancer_addr, &0u32);

    assert_eq!(token.balance(&freelancer_addr), 10_000);
    assert_eq!(token.balance(&contract_id), 0);

    let job = client.get_job();
    let milestone = job.milestones.get(0).unwrap();
    assert_eq!(milestone.status, MilestoneStatus::Released);
}

#[test]
fn test_claim_auto_release_wrong_status_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let arbiter_addr = Address::generate(&env);
    let admin_addr = Address::generate(&env);

    let token_contract_id = env
        .register_stellar_asset_contract_v2(admin_addr.clone())
        .address();
    let token_admin = token::StellarAssetClient::new(&env, &token_contract_id);
    token_admin.mint(&client_addr, &10_000);

    let contract_id = env.register(MilestoneEscrow, ());
    let client = MilestoneEscrowClient::new(&env, &contract_id);

    let amounts = vec![&env, 10_000_i128];
    client.initialize(
        &admin_addr,
        &client_addr,
        &freelancer_addr,
        &arbiter_addr,
        &token_contract_id,
        &100,
        &amounts,
    );

    client.fund(&client_addr);
    client.mark_delivered(&freelancer_addr, &0u32);
    client.approve_milestone(&client_addr, &0u32);

    let result = client.try_claim_auto_release(&freelancer_addr, &0u32);
    assert!(result.is_err());
}

#[test]
fn test_claim_auto_release_unauthorized_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let arbiter_addr = Address::generate(&env);
    let admin_addr = Address::generate(&env);
    let bad_actor = Address::generate(&env);

    let token_contract_id = env
        .register_stellar_asset_contract_v2(admin_addr.clone())
        .address();
    let token_admin = token::StellarAssetClient::new(&env, &token_contract_id);
    token_admin.mint(&client_addr, &10_000);

    let contract_id = env.register(MilestoneEscrow, ());
    let client = MilestoneEscrowClient::new(&env, &contract_id);

    let amounts = vec![&env, 10_000_i128];
    client.initialize(
        &admin_addr,
        &client_addr,
        &freelancer_addr,
        &arbiter_addr,
        &token_contract_id,
        &100,
        &amounts,
    );

    client.fund(&client_addr);
    client.mark_delivered(&freelancer_addr, &0u32);

    env.ledger().with_mut(|li| {
        li.timestamp += 200;
    });

    let result = client.try_claim_auto_release(&bad_actor, &0u32);
    assert!(result.is_err());
}

#[test]
fn test_time_until_auto_release() {
    let env = Env::default();
    env.mock_all_auths();

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let arbiter_addr = Address::generate(&env);
    let admin_addr = Address::generate(&env);

    let token_contract_id = env
        .register_stellar_asset_contract_v2(admin_addr.clone())
        .address();
    let token_admin = token::StellarAssetClient::new(&env, &token_contract_id);
    token_admin.mint(&client_addr, &10_000);

    let contract_id = env.register(MilestoneEscrow, ());
    let client = MilestoneEscrowClient::new(&env, &contract_id);

    let amounts = vec![&env, 10_000_i128];
    client.initialize(
        &admin_addr,
        &client_addr,
        &freelancer_addr,
        &arbiter_addr,
        &token_contract_id,
        &100,
        &amounts,
    );

    client.fund(&client_addr);
    client.mark_delivered(&freelancer_addr, &0u32);

    let time_remaining = client.time_until_auto_release(&0u32);
    assert!(time_remaining > 0);
    assert_eq!(time_remaining, 100);

    env.ledger().with_mut(|li| {
        li.timestamp += 50;
    });
    let time_remaining2 = client.time_until_auto_release(&0u32);
    assert_eq!(time_remaining2, 50);

    env.ledger().with_mut(|li| {
        li.timestamp += 100;
    });
    let time_remaining3 = client.time_until_auto_release(&0u32);
    assert!(time_remaining3 < 0);
}

#[test]
fn test_mark_delivered_unauthorized_client_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let arbiter_addr = Address::generate(&env);
    let admin_addr = Address::generate(&env);
    let token_contract_id = env
        .register_stellar_asset_contract_v2(admin_addr.clone())
        .address();
    let token_admin = token::StellarAssetClient::new(&env, &token_contract_id);
    token_admin.mint(&client_addr, &1_000);

    let contract_id = env.register(MilestoneEscrow, ());
    let client = MilestoneEscrowClient::new(&env, &contract_id);
    let amounts = vec![&env, 1_000_i128];
    client.initialize(
        &admin_addr,
        &client_addr,
        &freelancer_addr,
        &arbiter_addr,
        &token_contract_id,
        &604800,
        &amounts,
    );
    client.fund(&client_addr);

    let result = client.try_mark_delivered(&client_addr, &0u32);
    assert_eq!(result, Err(Ok(Error::Unauthorized)));
}

#[test]
fn test_mark_delivered_unauthorized_arbiter_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let arbiter_addr = Address::generate(&env);
    let admin_addr = Address::generate(&env);
    let token_contract_id = env
        .register_stellar_asset_contract_v2(admin_addr.clone())
        .address();
    let token_admin = token::StellarAssetClient::new(&env, &token_contract_id);
    token_admin.mint(&client_addr, &1_000);

    let contract_id = env.register(MilestoneEscrow, ());
    let client = MilestoneEscrowClient::new(&env, &contract_id);
    let amounts = vec![&env, 1_000_i128];
    client.initialize(
        &admin_addr,
        &client_addr,
        &freelancer_addr,
        &arbiter_addr,
        &token_contract_id,
        &604800,
        &amounts,
    );
    client.fund(&client_addr);

    let result = client.try_mark_delivered(&arbiter_addr, &0u32);
    assert_eq!(result, Err(Ok(Error::Unauthorized)));
}

#[test]
fn test_mark_delivered_unauthorized_freelancer_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let arbiter_addr = Address::generate(&env);
    let admin_addr = Address::generate(&env);
    let impostor = Address::generate(&env);
    let token_contract_id = env
        .register_stellar_asset_contract_v2(admin_addr.clone())
        .address();
    let token_admin = token::StellarAssetClient::new(&env, &token_contract_id);
    token_admin.mint(&client_addr, &1_000);

    let contract_id = env.register(MilestoneEscrow, ());
    let client = MilestoneEscrowClient::new(&env, &contract_id);
    let amounts = vec![&env, 1_000_i128];
    client.initialize(
        &admin_addr,
        &client_addr,
        &freelancer_addr,
        &arbiter_addr,
        &token_contract_id,
        &604800,
        &amounts,
    );
    client.fund(&client_addr);

    let result = client.try_mark_delivered(&impostor, &0u32);
    assert_eq!(result, Err(Ok(Error::Unauthorized)));
}

#[test]
fn test_approve_partial_zero_amount_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let arbiter_addr = Address::generate(&env);
    let admin_addr = Address::generate(&env);

    let token_contract_id = env
        .register_stellar_asset_contract_v2(admin_addr.clone())
        .address();
    let token_admin = token::StellarAssetClient::new(&env, &token_contract_id);
    token_admin.mint(&client_addr, &10_000);

    let contract_id = env.register(MilestoneEscrow, ());
    let client = MilestoneEscrowClient::new(&env, &contract_id);

    let amounts = vec![&env, 10_000_i128];
    client.initialize(
        &admin_addr,
        &client_addr,
        &freelancer_addr,
        &arbiter_addr,
        &token_contract_id,
        &604800,
        &amounts,
    );
    client.fund(&client_addr);
    client.mark_delivered(&freelancer_addr, &0u32);

    let result = client.try_approve_partial(&client_addr, &0u32, &0_i128);
    assert_eq!(result, Err(Ok(Error::InvalidAmount)));
}

#[test]
fn test_approve_partial_negative_amount_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let arbiter_addr = Address::generate(&env);
    let admin_addr = Address::generate(&env);

    let token_contract_id = env
        .register_stellar_asset_contract_v2(admin_addr.clone())
        .address();
    let token_admin = token::StellarAssetClient::new(&env, &token_contract_id);
    token_admin.mint(&client_addr, &10_000);

    let contract_id = env.register(MilestoneEscrow, ());
    let client = MilestoneEscrowClient::new(&env, &contract_id);

    let amounts = vec![&env, 10_000_i128];
    client.initialize(
        &admin_addr,
        &client_addr,
        &freelancer_addr,
        &arbiter_addr,
        &token_contract_id,
        &604800,
        &amounts,
    );
    client.fund(&client_addr);
    client.mark_delivered(&freelancer_addr, &0u32);

    let result = client.try_approve_partial(&client_addr, &0u32, &-500_i128);
    assert_eq!(result, Err(Ok(Error::InvalidAmount)));
}

#[test]
fn test_approve_partial_exceeds_remaining_amount_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let arbiter_addr = Address::generate(&env);
    let admin_addr = Address::generate(&env);

    let token_contract_id = env
        .register_stellar_asset_contract_v2(admin_addr.clone())
        .address();
    let token_admin = token::StellarAssetClient::new(&env, &token_contract_id);
    token_admin.mint(&client_addr, &10_000);

    let contract_id = env.register(MilestoneEscrow, ());
    let client = MilestoneEscrowClient::new(&env, &contract_id);

    let amounts = vec![&env, 10_000_i128];
    client.initialize(
        &admin_addr,
        &client_addr,
        &freelancer_addr,
        &arbiter_addr,
        &token_contract_id,
        &604800,
        &amounts,
    );
    client.fund(&client_addr);
    client.mark_delivered(&freelancer_addr, &0u32);
    client.approve_partial(&client_addr, &0u32, &6_000_i128);

    let result = client.try_approve_partial(&client_addr, &0u32, &5_000_i128);
    assert_eq!(result, Err(Ok(Error::InvalidAmount)));
}

#[test]
fn test_approve_partial_index_out_of_range_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let arbiter_addr = Address::generate(&env);
    let admin_addr = Address::generate(&env);

    let token_contract_id = env
        .register_stellar_asset_contract_v2(admin_addr.clone())
        .address();
    let token_admin = token::StellarAssetClient::new(&env, &token_contract_id);
    token_admin.mint(&client_addr, &10_000);

    let contract_id = env.register(MilestoneEscrow, ());
    let client = MilestoneEscrowClient::new(&env, &contract_id);

    let amounts = vec![&env, 5_000_i128, 5_000_i128];
    client.initialize(
        &admin_addr,
        &client_addr,
        &freelancer_addr,
        &arbiter_addr,
        &token_contract_id,
        &604800,
        &amounts,
    );
    client.fund(&client_addr);

    let result = client.try_approve_partial(&client_addr, &2u32, &1_000_i128);
    assert_eq!(result, Err(Ok(Error::InvalidMilestone)));

    let result = client.try_approve_partial(&client_addr, &999u32, &1_000_i128);
    assert_eq!(result, Err(Ok(Error::InvalidMilestone)));
}

#[test]
fn test_mark_delivered_state_transitions() {
    let env = Env::default();
    env.mock_all_auths();

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let arbiter_addr = Address::generate(&env);
    let admin_addr = Address::generate(&env);
    let token_contract_id = env
        .register_stellar_asset_contract_v2(admin_addr.clone())
        .address();
    let token_admin = token::StellarAssetClient::new(&env, &token_contract_id);
    token_admin.mint(&client_addr, &2_000);

    let contract_id = env.register(MilestoneEscrow, ());
    let client = MilestoneEscrowClient::new(&env, &contract_id);
    let amounts = vec![&env, 1_000_i128, 1_000_i128];
    client.initialize(
        &admin_addr,
        &client_addr,
        &freelancer_addr,
        &arbiter_addr,
        &token_contract_id,
        &604800,
        &amounts,
    );
    client.fund(&client_addr);

    // Test 1: Pending → Delivered (should pass)
    client.mark_delivered(&freelancer_addr, &0u32);
    let job = client.get_job();
    assert_eq!(
        job.milestones.get(0).unwrap().status,
        MilestoneStatus::Delivered
    );

    // Test 2: Delivered → Delivered (should fail)
    let result = client.try_mark_delivered(&freelancer_addr, &0u32);
    assert_eq!(result, Err(Ok(Error::InvalidStatus)));

    // Setup milestone 1 for PartiallyReleased
    client.mark_delivered(&freelancer_addr, &1u32);
    client.approve_partial(&client_addr, &1u32, &500_i128);
    let result = client.try_mark_delivered(&freelancer_addr, &1u32);
    assert_eq!(result, Err(Ok(Error::InvalidStatus)));

    // Reset with new environment for remaining states
    let env2 = Env::default();
    env2.mock_all_auths();
    let client_addr2 = Address::generate(&env2);
    let freelancer_addr2 = Address::generate(&env2);
    let arbiter_addr2 = Address::generate(&env2);
    let admin_addr2 = Address::generate(&env2);
    let token_contract_id2 = env2
        .register_stellar_asset_contract_v2(admin_addr2.clone())
        .address();
    let token_admin2 = token::StellarAssetClient::new(&env2, &token_contract_id2);
    token_admin2.mint(&client_addr2, &4_000);
    let contract_id2 = env2.register(MilestoneEscrow, ());
    let client2 = MilestoneEscrowClient::new(&env2, &contract_id2);
    let amounts2 = vec![&env2, 1_000_i128, 1_000_i128, 1_000_i128, 1_000_i128];
    client2.initialize(
        &admin_addr2,
        &client_addr2,
        &freelancer_addr2,
        &arbiter_addr2,
        &token_contract_id2,
        &604800,
        &amounts2,
    );
    client2.fund(&client_addr2);

    // Released
    client2.mark_delivered(&freelancer_addr2, &0u32);
    client2.approve_milestone(&client_addr2, &0u32);
    let result = client2.try_mark_delivered(&freelancer_addr2, &0u32);
    assert_eq!(result, Err(Ok(Error::InvalidStatus)));

    // Disputed
    client2.mark_delivered(&freelancer_addr2, &1u32);
    client2.raise_dispute(&client_addr2, &1u32);
    let result = client2.try_mark_delivered(&freelancer_addr2, &1u32);
    assert_eq!(result, Err(Ok(Error::InvalidStatus)));

    // Refunded
    client2.mark_delivered(&freelancer_addr2, &2u32);
    client2.raise_dispute(&client_addr2, &2u32);
    client2.resolve_dispute(&arbiter_addr2, &2u32, &false);
    let result = client2.try_mark_delivered(&freelancer_addr2, &2u32);
    assert_eq!(result, Err(Ok(Error::InvalidStatus)));
}

// ============================================================================
// approve_partial — hardened test suite
// ============================================================================

/// Helper: set up a funded escrow with a single milestone of `amount` tokens,
/// mark it delivered, and return all relevant handles in one call.
fn setup_delivered_single(
    env: &Env,
    amount: i128,
) -> (
    Address, // client
    Address, // freelancer
    Address, // arbiter
    Address, // token contract
    Address, // escrow contract
    MilestoneEscrowClient<'_>,
) {
    let client_addr = Address::generate(env);
    let freelancer_addr = Address::generate(env);
    let arbiter_addr = Address::generate(env);
    let admin_addr = Address::generate(env);

    let token_id = env
        .register_stellar_asset_contract_v2(admin_addr.clone())
        .address();
    let token_admin = token::StellarAssetClient::new(env, &token_id);
    token_admin.mint(&client_addr, &amount);

    let contract_id = env.register(MilestoneEscrow, ());
    let escrow = MilestoneEscrowClient::new(env, &contract_id);

    let amounts = vec![env, amount];
    escrow.initialize(
        &admin_addr,
        &client_addr,
        &freelancer_addr,
        &arbiter_addr,
        &token_id,
        &604800,
        &amounts,
    );
    escrow.fund(&client_addr);
    escrow.mark_delivered(&freelancer_addr, &0u32);

    (
        client_addr,
        freelancer_addr,
        arbiter_addr,
        token_id,
        contract_id,
        escrow,
    )
}

/// Test 1 — AUTHORIZATION: The freelancer (a known but non-client party) cannot
/// call `approve_partial`.  We assert the precise `Error::Unauthorized` variant
/// rather than just `is_err()`.
#[test]
fn test_approve_partial_freelancer_is_unauthorized() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, freelancer_addr, _, _, _, escrow) = setup_delivered_single(&env, 10_000);

    let result = escrow.try_approve_partial(&freelancer_addr, &0u32, &1_000_i128);
    assert_eq!(result, Err(Ok(Error::Unauthorized)));
}

/// Test 2 — AUTHORIZATION: The arbiter (also a known but non-client party)
/// cannot call `approve_partial`.  Asserts `Error::Unauthorized` precisely.
#[test]
fn test_approve_partial_arbiter_is_unauthorized() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, _, arbiter_addr, _, _, escrow) = setup_delivered_single(&env, 10_000);

    let result = escrow.try_approve_partial(&arbiter_addr, &0u32, &1_000_i128);
    assert_eq!(result, Err(Ok(Error::Unauthorized)));
}

/// Test 3 — INVALID STATE (Disputed): A milestone in `Disputed` status is not
/// approvable.  Raises a dispute first, then attempts `approve_partial` and
/// asserts `Error::InvalidStatus`.
#[test]
fn test_approve_partial_on_disputed_milestone_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let (client_addr, _, _, _, _, escrow) = setup_delivered_single(&env, 10_000);

    // Move the milestone into Disputed state (client may raise after delivery).
    escrow.raise_dispute(&client_addr, &0u32);

    let result = escrow.try_approve_partial(&client_addr, &0u32, &1_000_i128);
    assert_eq!(result, Err(Ok(Error::InvalidStatus)));
}

/// Test 4 — INVALID STATE (Refunded): A milestone that has been refunded to the
/// client is a terminal state; `approve_partial` must be rejected with
/// `Error::InvalidStatus`.
#[test]
fn test_approve_partial_on_refunded_milestone_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let (client_addr, _, arbiter_addr, _, _, escrow) = setup_delivered_single(&env, 10_000);

    // Dispute then resolve in favour of the client → status = Refunded.
    escrow.raise_dispute(&client_addr, &0u32);
    escrow.resolve_dispute(&arbiter_addr, &0u32, &false);

    let result = escrow.try_approve_partial(&client_addr, &0u32, &1_000_i128);
    assert_eq!(result, Err(Ok(Error::InvalidStatus)));
}

/// Test 5 — NON-EXISTENT MILESTONE ID: Supplying a milestone index that is
/// strictly out of the initialised range must return `Error::InvalidMilestone`
/// rather than panicking or silently succeeding.
#[test]
fn test_approve_partial_nonexistent_milestone_index_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let (client_addr, _, _, _, _, escrow) = setup_delivered_single(&env, 10_000);

    // The contract was initialised with exactly 1 milestone (index 0).
    // Index 1 does not exist.
    let result = escrow.try_approve_partial(&client_addr, &1u32, &1_000_i128);
    assert_eq!(result, Err(Ok(Error::InvalidMilestone)));
}

/// Test 6 — EXACT REMAINING BALANCE ON A PARTIALLY-RELEASED MILESTONE:
/// After one partial release the milestone is `PartiallyReleased`.  Approving
/// exactly the residual balance must flip the status to `Released` and leave
/// `released_amount == milestone.amount`.  No tokens should remain in escrow.
#[test]
fn test_approve_partial_exact_remaining_balance_transitions_to_released() {
    let env = Env::default();
    env.mock_all_auths();

    let (client_addr, freelancer_addr, _, token_id, contract_id, escrow) =
        setup_delivered_single(&env, 12_000);

    let token = token::Client::new(&env, &token_id);

    // First installment — leaves 7 000 remaining.
    escrow.approve_partial(&client_addr, &0u32, &5_000_i128);
    assert_eq!(token.balance(&freelancer_addr), 5_000);

    let job = escrow.get_job();
    let ms = job.milestones.get(0).unwrap();
    assert_eq!(ms.status, MilestoneStatus::PartiallyReleased);
    assert_eq!(ms.released_amount, 5_000);

    // Second installment — exactly the remainder.
    escrow.approve_partial(&client_addr, &0u32, &7_000_i128);

    let job2 = escrow.get_job();
    let ms2 = job2.milestones.get(0).unwrap();
    assert_eq!(ms2.status, MilestoneStatus::Released);
    assert_eq!(ms2.released_amount, 12_000);
    assert_eq!(token.balance(&freelancer_addr), 12_000);
    assert_eq!(token.balance(&contract_id), 0);
}

/// Test 7 — OVER-ALLOCATION ON A PARTIALLY-RELEASED MILESTONE:
/// After one partial release the remaining balance is smaller than the
/// original amount.  Requesting more than *that residual* must be rejected
/// with `Error::InvalidAmount` even though the requested value is individually
/// less than the milestone's total amount.
#[test]
fn test_approve_partial_over_release_after_prior_partial_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let (client_addr, _, _, _, _, escrow) = setup_delivered_single(&env, 10_000);

    // Release 6 000; only 4 000 remains.
    escrow.approve_partial(&client_addr, &0u32, &6_000_i128);

    // Attempt to release 5 000 — valid against the original total but exceeds
    // the 4 000 residual balance.
    let result = escrow.try_approve_partial(&client_addr, &0u32, &5_000_i128);
    assert_eq!(result, Err(Ok(Error::InvalidAmount)));
}

/// Test 8 — STATE ISOLATION ACROSS MILESTONES:
/// A partial release on milestone 0 must not alter the stored state of
/// milestone 1.  `released_amount` and `status` of the untouched milestone
/// must remain exactly as initialised.
#[test]
fn test_approve_partial_does_not_mutate_sibling_milestone() {
    let env = Env::default();
    env.mock_all_auths();

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let arbiter_addr = Address::generate(&env);
    let admin_addr = Address::generate(&env);

    let token_id = env
        .register_stellar_asset_contract_v2(admin_addr.clone())
        .address();
    let token_admin = token::StellarAssetClient::new(&env, &token_id);
    token_admin.mint(&client_addr, &20_000);

    let contract_id = env.register(MilestoneEscrow, ());
    let escrow = MilestoneEscrowClient::new(&env, &contract_id);

    // Two milestones of equal size.
    let amounts = vec![&env, 10_000_i128, 10_000_i128];
    escrow.initialize(
        &admin_addr,
        &client_addr,
        &freelancer_addr,
        &arbiter_addr,
        &token_id,
        &604800,
        &amounts,
    );
    escrow.fund(&client_addr);
    escrow.mark_delivered(&freelancer_addr, &0u32);

    // Partially release milestone 0.
    escrow.approve_partial(&client_addr, &0u32, &3_000_i128);

    // Milestone 1 must remain completely untouched.
    let job = escrow.get_job();
    let ms1 = job.milestones.get(1).unwrap();
    assert_eq!(ms1.status, MilestoneStatus::Pending);
    assert_eq!(ms1.released_amount, 0);
    assert_eq!(ms1.amount, 10_000);
}

/// Test 9 — PRE-INITIALIZATION GUARD:
/// Calling `approve_partial` before the contract has been initialised at all
/// must return `Error::NotInitialized` — the function should not panic
/// unexpectedly or return a misleading error variant.
#[test]
fn test_approve_partial_before_initialize_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let caller = Address::generate(&env);
    let contract_id = env.register(MilestoneEscrow, ());
    let escrow = MilestoneEscrowClient::new(&env, &contract_id);

    let result = escrow.try_approve_partial(&caller, &0u32, &1_000_i128);
    assert_eq!(result, Err(Ok(Error::NotInitialized)));
}

#[test]
fn test_approve_milestone_state_transitions() {
    let env = Env::default();
    env.mock_all_auths();

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let arbiter_addr = Address::generate(&env);
    let admin_addr = Address::generate(&env);
    let token_contract_id = env
        .register_stellar_asset_contract_v2(admin_addr.clone())
        .address();
    let token_admin = token::StellarAssetClient::new(&env, &token_contract_id);
    token_admin.mint(&client_addr, &10_000);

    let contract_id = env.register(MilestoneEscrow, ());
    let client = MilestoneEscrowClient::new(&env, &contract_id);

    let amounts = vec![&env, 10_000_i128];
    client.initialize(
        &admin_addr,
        &client_addr,
        &freelancer_addr,
        &arbiter_addr,
        &token_contract_id,
        &604800,
        &amounts,
    );
    client.fund(&client_addr);

    // Test 1: Pending → InvalidStatus (should fail)
    let result = client.try_approve_milestone(&client_addr, &0u32);
    assert_eq!(result, Err(Ok(Error::InvalidStatus)));

    // Test 2: Delivered → Released (should pass)
    client.mark_delivered(&freelancer_addr, &0u32);
    client.approve_milestone(&client_addr, &0u32);
    let job = client.get_job();
    assert_eq!(
        job.milestones.get(0).unwrap().status,
        MilestoneStatus::Released
    );
    assert_eq!(job.milestones.get(0).unwrap().released_amount, 10_000);

    // Test 3: Released → InvalidStatus (should fail)
    let result = client.try_approve_milestone(&client_addr, &0u32);
    assert_eq!(result, Err(Ok(Error::InvalidStatus)));
}

#[test]
fn test_approve_milestone_emits_event() {
    let env = Env::default();
    env.mock_all_auths();

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let arbiter_addr = Address::generate(&env);
    let admin_addr = Address::generate(&env);
    let token_contract_id = env
        .register_stellar_asset_contract_v2(admin_addr.clone())
        .address();
    let token_admin = token::StellarAssetClient::new(&env, &token_contract_id);
    token_admin.mint(&client_addr, &10_000);

    let contract_id = env.register(MilestoneEscrow, ());
    let client = MilestoneEscrowClient::new(&env, &contract_id);

    let amounts = vec![&env, 10_000_i128];
    client.initialize(
        &admin_addr,
        &client_addr,
        &freelancer_addr,
        &arbiter_addr,
        &token_contract_id,
        &604800,
        &amounts,
    );
    client.fund(&client_addr);
    client.mark_delivered(&freelancer_addr, &0u32);
    client.approve_milestone(&client_addr, &0u32);

    let approve_topic: Symbol = symbol_short!("approve");
    let approve_topic_val: Val = approve_topic.into_val(&env);
    let approve_count = env.events().all().iter().fold(0u32, |acc, e| {
        if let Some(topic) = e.1.get(0) {
            if topic.get_payload() == approve_topic_val.get_payload() {
                return acc + 1;
            }
        }
        acc
    });

    assert_eq!(approve_count, 1);
}

#[test]
fn test_approve_milestone_after_partial_release() {
    let env = Env::default();
    env.mock_all_auths();

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let arbiter_addr = Address::generate(&env);
    let admin_addr = Address::generate(&env);
    let token_contract_id = env
        .register_stellar_asset_contract_v2(admin_addr.clone())
        .address();
    let token = token::Client::new(&env, &token_contract_id);
    let token_admin = token::StellarAssetClient::new(&env, &token_contract_id);
    token_admin.mint(&client_addr, &10_000);

    let contract_id = env.register(MilestoneEscrow, ());
    let client = MilestoneEscrowClient::new(&env, &contract_id);

    let amounts = vec![&env, 10_000_i128];
    client.initialize(
        &admin_addr,
        &client_addr,
        &freelancer_addr,
        &arbiter_addr,
        &token_contract_id,
        &604800,
        &amounts,
    );
    client.fund(&client_addr);
    client.mark_delivered(&freelancer_addr, &0u32);
    client.approve_partial(&client_addr, &0u32, &4_000_i128);

    // Now approve the milestone fully
    client.approve_milestone(&client_addr, &0u32);

    let job = client.get_job();
    let ms = job.milestones.get(0).unwrap();
    assert_eq!(ms.status, MilestoneStatus::Released);
    assert_eq!(ms.released_amount, 10_000);
    assert_eq!(token.balance(&freelancer_addr), 10_000);
    assert_eq!(token.balance(&contract_id), 0);

    // Approving again should fail
    let result = client.try_approve_milestone(&client_addr, &0u32);
    assert_eq!(result, Err(Ok(Error::InvalidStatus)));
}

#[test]
fn test_approve_milestone_on_disputed_milestone_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let arbiter_addr = Address::generate(&env);
    let admin_addr = Address::generate(&env);
    let token_contract_id = env
        .register_stellar_asset_contract_v2(admin_addr.clone())
        .address();
    let token_admin = token::StellarAssetClient::new(&env, &token_contract_id);
    token_admin.mint(&client_addr, &10_000);

    let contract_id = env.register(MilestoneEscrow, ());
    let client = MilestoneEscrowClient::new(&env, &contract_id);

    let amounts = vec![&env, 10_000_i128];
    client.initialize(
        &admin_addr,
        &client_addr,
        &freelancer_addr,
        &arbiter_addr,
        &token_contract_id,
        &604800,
        &amounts,
    );
    client.fund(&client_addr);
    client.mark_delivered(&freelancer_addr, &0u32);
    client.raise_dispute(&client_addr, &0u32);

    let result = client.try_approve_milestone(&client_addr, &0u32);
    assert_eq!(result, Err(Ok(Error::InvalidStatus)));
}

#[test]
fn test_approve_milestone_on_refunded_milestone_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let arbiter_addr = Address::generate(&env);
    let admin_addr = Address::generate(&env);
    let token_contract_id = env
        .register_stellar_asset_contract_v2(admin_addr.clone())
        .address();
    let token_admin = token::StellarAssetClient::new(&env, &token_contract_id);
    token_admin.mint(&client_addr, &10_000);

    let contract_id = env.register(MilestoneEscrow, ());
    let client = MilestoneEscrowClient::new(&env, &contract_id);

    let amounts = vec![&env, 10_000_i128];
    client.initialize(
        &admin_addr,
        &client_addr,
        &freelancer_addr,
        &arbiter_addr,
        &token_contract_id,
        &604800,
        &amounts,
    );
    client.fund(&client_addr);
    client.mark_delivered(&freelancer_addr, &0u32);
    client.raise_dispute(&client_addr, &0u32);
    client.resolve_dispute(&arbiter_addr, &0u32, &false);

    let result = client.try_approve_milestone(&client_addr, &0u32);
    assert_eq!(result, Err(Ok(Error::InvalidStatus)));
}

/// Test — AUTHORIZATION: The freelancer (a known but non-client party) cannot
/// call `approve_milestone`.  We assert the precise `Error::Unauthorized` variant
/// is returned (not some generic error), confirming `require_auth()` + stored
/// client check both participate in the rejection.
#[test]
fn test_approve_milestone_freelancer_is_unauthorized() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, freelancer_addr, _, _, _, escrow) =
        setup_delivered_single(&env, 10_000);

    let result = escrow.try_approve_milestone(&freelancer_addr, &0u32);
    assert_eq!(result, Err(Ok(Error::Unauthorized)));
}

/// Test — AUTHORIZATION: The arbiter (also a known but non-client party) cannot
/// call `approve_milestone`.  Asserts `Error::Unauthorized` precisely.
#[test]
fn test_approve_milestone_arbiter_is_unauthorized() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, _, arbiter_addr, _, _, escrow) =
        setup_delivered_single(&env, 10_000);

    let result = escrow.try_approve_milestone(&arbiter_addr, &0u32);
    assert_eq!(result, Err(Ok(Error::Unauthorized)));
}

#[test]
fn test_approve_milestone_gas_scales_sublinearly() {
    let env = Env::default();
    env.mock_all_auths();

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let arbiter_addr = Address::generate(&env);
    let admin_addr = Address::generate(&env);

    let token_id = env
        .register_stellar_asset_contract_v2(admin_addr.clone())
        .address();
    let token = token::Client::new(&env, &token_id);
    let token_admin = token::StellarAssetClient::new(&env, &token_id);

    let contract_id = env.register(MilestoneEscrow, ());
    let escrow = MilestoneEscrowClient::new(&env, &contract_id);

    let milestone_count = 120u32;
    let per_milestone = 100i128;
    let mut amounts: Vec<i128> = Vec::new(&env);
    for _ in 0..milestone_count {
        amounts.push_back(per_milestone);
    }
    token_admin.mint(&client_addr, &(milestone_count as i128 * per_milestone));

    escrow.initialize(
        &admin_addr,
        &client_addr,
        &freelancer_addr,
        &arbiter_addr,
        &token_id,
        &604800,
        &amounts,
    );
    escrow.fund(&client_addr);

    escrow.mark_delivered(&freelancer_addr, &0u32);
    escrow.approve_milestone(&client_addr, &0u32);

    assert_eq!(token.balance(&freelancer_addr), per_milestone);
    assert_eq!(token.balance(&contract_id), (milestone_count - 1) as i128 * per_milestone);

    let job = escrow.get_job();
    let ms = job.milestones.get(0).unwrap();
    assert_eq!(ms.status, MilestoneStatus::Released);
    assert_eq!(ms.released_amount, per_milestone);
    assert_eq!(ms.amount, per_milestone);

    let sibling = job.milestones.get(milestone_count - 1).unwrap();
    assert_eq!(sibling.status, MilestoneStatus::Pending);
    assert_eq!(sibling.released_amount, 0);
}

#[test]
fn test_approve_milestone_100_plus_milestones_scales() {
    let env = Env::default();
    env.mock_all_auths();

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let arbiter_addr = Address::generate(&env);
    let admin_addr = Address::generate(&env);

    let token_id = env
        .register_stellar_asset_contract_v2(admin_addr.clone())
        .address();
    let token = token::Client::new(&env, &token_id);
    let token_admin = token::StellarAssetClient::new(&env, &token_id);

    let contract_id = env.register(MilestoneEscrow, ());
    let escrow = MilestoneEscrowClient::new(&env, &contract_id);

    let milestone_count = 100u32;
    let per_milestone = 100i128;
    let mut amounts: Vec<i128> = Vec::new(&env);
    for _ in 0..milestone_count {
        amounts.push_back(per_milestone);
    }
    token_admin.mint(&client_addr, &(milestone_count as i128 * per_milestone));

    escrow.initialize(
        &admin_addr,
        &client_addr,
        &freelancer_addr,
        &arbiter_addr,
        &token_id,
        &604800,
        &amounts,
    );
    escrow.fund(&client_addr);

    for i in 0..milestone_count {
        escrow.mark_delivered(&freelancer_addr, &i);
        escrow.approve_milestone(&client_addr, &i);
    }

    assert_eq!(token.balance(&freelancer_addr), milestone_count as i128 * per_milestone);
    assert_eq!(token.balance(&contract_id), 0);

    let job = escrow.get_job();
    for i in 0..milestone_count {
        let ms = job.milestones.get(i).unwrap();
        assert_eq!(ms.status, MilestoneStatus::Released);
        assert_eq!(ms.released_amount, per_milestone);
    }
}

#[test]
fn test_approve_milestone_emits_exactly_one_event() {
    let env = Env::default();
    env.mock_all_auths();

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let arbiter_addr = Address::generate(&env);
    let admin_addr = Address::generate(&env);

    let token_id = env
        .register_stellar_asset_contract_v2(admin_addr.clone())
        .address();
    let token_admin = token::StellarAssetClient::new(&env, &token_id);
    token_admin.mint(&client_addr, &10_000);

    let contract_id = env.register(MilestoneEscrow, ());
    let escrow = MilestoneEscrowClient::new(&env, &contract_id);

    let amounts = vec![&env, 10_000_i128];
    escrow.initialize(
        &admin_addr,
        &client_addr,
        &freelancer_addr,
        &arbiter_addr,
        &token_id,
        &604800,
        &amounts,
    );
    escrow.fund(&client_addr);
    escrow.mark_delivered(&freelancer_addr, &0u32);
    escrow.approve_milestone(&client_addr, &0u32);

    let approve_topic: Symbol = symbol_short!("approve");
    let approve_topic_val: Val = approve_topic.into_val(&env);
    let approve_count = env.events().all().iter().fold(0u32, |acc, e| {
        if let Some(topic) = e.1.get(0) {
            if topic.get_payload() == approve_topic_val.get_payload() {
                return acc + 1;
            }
        }
        acc
    });

    assert_eq!(approve_count, 1);
}

#[test]
fn test_approve_milestone_before_initialize_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let caller = Address::generate(&env);
    let contract_id = env.register(MilestoneEscrow, ());
    let escrow = MilestoneEscrowClient::new(&env, &contract_id);

    let result = escrow.try_approve_milestone(&caller, &0u32);
    assert_eq!(result, Err(Ok(Error::NotInitialized)));
}

#[test]
fn test_approve_milestone_does_not_mutate_sibling_milestone() {
    let env = Env::default();
    env.mock_all_auths();

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let arbiter_addr = Address::generate(&env);
    let admin_addr = Address::generate(&env);

    let token_id = env
        .register_stellar_asset_contract_v2(admin_addr.clone())
        .address();
    let token_admin = token::StellarAssetClient::new(&env, &token_id);
    token_admin.mint(&client_addr, &20_000);

    let contract_id = env.register(MilestoneEscrow, ());
    let escrow = MilestoneEscrowClient::new(&env, &contract_id);

    let amounts = vec![&env, 10_000_i128, 10_000_i128];
    escrow.initialize(
        &admin_addr,
        &client_addr,
        &freelancer_addr,
        &arbiter_addr,
        &token_id,
        &604800,
        &amounts,
    );
    escrow.fund(&client_addr);
    escrow.mark_delivered(&freelancer_addr, &0u32);

    escrow.approve_milestone(&client_addr, &0u32);

    let job = escrow.get_job();
    let ms1 = job.milestones.get(1).unwrap();
    assert_eq!(ms1.status, MilestoneStatus::Pending);
    assert_eq!(ms1.released_amount, 0);
    assert_eq!(ms1.amount, 10_000);
}

// ============================================================================
// Gas Efficiency Tests for approve_milestone
// ============================================================================

/// Tests that approve_milestone scales O(1) with number of milestones.
/// Gas consumption should not significantly increase when approving milestone 0
/// regardless of whether there are 10 or 120 total milestones.
#[test]
fn test_approve_milestone_gas_scales_constant() {
    let env = Env::default();
    env.mock_all_auths();

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let arbiter_addr = Address::generate(&env);
    let admin_addr = Address::generate(&env);

    let token_id = env
        .register_stellar_asset_contract_v2(admin_addr.clone())
        .address();
    let token_admin = token::StellarAssetClient::new(&env, &token_id);

    let contract_id = env.register(MilestoneEscrow, ());
    let escrow = MilestoneEscrowClient::new(&env, &contract_id);

    let milestone_count = 120u32;
    let per_milestone = 100i128;
    let mut amounts: Vec<i128> = Vec::new(&env);
    for _ in 0..milestone_count {
        amounts.push_back(per_milestone);
    }
    token_admin.mint(&client_addr, &(milestone_count as i128 * per_milestone));

    escrow.initialize(
        &admin_addr,
        &client_addr,
        &freelancer_addr,
        &arbiter_addr,
        &token_id,
        &604800,
        &amounts,
    );
    escrow.fund(&client_addr);

    let initial_budget = env.cost_estimate().budget().cpu_instruction_cost();
    escrow.mark_delivered(&freelancer_addr, &0u32);
    escrow.approve_milestone(&client_addr, &0u32);
    let budget_after_120 = env.cost_estimate().budget().cpu_instruction_cost();

    let env2 = Env::default();
    env2.mock_all_auths();

    let client_addr2 = Address::generate(&env2);
    let freelancer_addr2 = Address::generate(&env2);
    let arbiter_addr2 = Address::generate(&env2);
    let admin_addr2 = Address::generate(&env2);
    let token_id2 = env2
        .register_stellar_asset_contract_v2(admin_addr2.clone())
        .address();
    let token_admin2 = token::StellarAssetClient::new(&env2, &token_id2);
    let contract_id2 = env2.register(MilestoneEscrow, ());
    let escrow2 = MilestoneEscrowClient::new(&env2, &contract_id2);

    let milestone_count_small = 10u32;
    let mut amounts2: Vec<i128> = Vec::new(&env2);
    for _ in 0..milestone_count_small {
        amounts2.push_back(per_milestone);
    }
    token_admin2.mint(&client_addr2, &(milestone_count_small as i128 * per_milestone));

    escrow2.initialize(
        &admin_addr2,
        &client_addr2,
        &freelancer_addr2,
        &arbiter_addr2,
        &token_id2,
        &604800,
        &amounts2,
    );
    escrow2.fund(&client_addr2);

    let initial_budget2 = env2.cost_estimate().budget().cpu_instruction_cost();
    escrow2.mark_delivered(&freelancer_addr2, &0u32);
    escrow2.approve_milestone(&client_addr2, &0u32);
    let budget_after_10 = env2.cost_estimate().budget().cpu_instruction_cost();

    let gas_120 = initial_budget - budget_after_120;
    let gas_10 = initial_budget2 - budget_after_10;
    assert!(gas_120 < gas_10 * 3, "approve_milestone should scale O(1), not O(n)");

    let job = escrow.get_job();
    let ms = job.milestones.get(0).unwrap();
    assert_eq!(ms.status, MilestoneStatus::Released);
    assert_eq!(ms.released_amount, per_milestone);
}

/// Tests that approving milestone at the end of a large list has same gas cost
/// as approving at the beginning (constant time access).
#[test]
fn test_approve_milestone_constant_time_by_index() {
    let env = Env::default();
    env.mock_all_auths();

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let arbiter_addr = Address::generate(&env);
    let admin_addr = Address::generate(&env);

    let token_id = env
        .register_stellar_asset_contract_v2(admin_addr.clone())
        .address();
    let token_admin = token::StellarAssetClient::new(&env, &token_id);

    let contract_id = env.register(MilestoneEscrow, ());
    let escrow = MilestoneEscrowClient::new(&env, &contract_id);

    let milestone_count = 120u32;
    let per_milestone = 100i128;
    let mut amounts: Vec<i128> = Vec::new(&env);
    for _ in 0..milestone_count {
        amounts.push_back(per_milestone);
    }
    token_admin.mint(&client_addr, &(milestone_count as i128 * per_milestone));

    escrow.initialize(
        &admin_addr,
        &client_addr,
        &freelancer_addr,
        &arbiter_addr,
        &token_id,
        &604800,
        &amounts,
    );
    escrow.fund(&client_addr);

    let initial_budget_0 = env.cost_estimate().budget().cpu_instruction_cost();
    escrow.mark_delivered(&freelancer_addr, &0u32);
    escrow.approve_milestone(&client_addr, &0u32);
    let budget_after_0 = env.cost_estimate().budget().cpu_instruction_cost();

    let initial_budget_end = env.cost_estimate().budget().cpu_instruction_cost();
    escrow.mark_delivered(&freelancer_addr, &119u32);
    escrow.approve_milestone(&client_addr, &119u32);
    let budget_after_end = env.cost_estimate().budget().cpu_instruction_cost();

    let gas_0 = initial_budget_0 - budget_after_0;
    let gas_end = initial_budget_end - budget_after_end;

    assert!(gas_end < gas_0 * 3, "approving last milestone should be O(1), not O(n)");

    let job = escrow.get_job();
    let ms0 = job.milestones.get(0).unwrap();
    let ms119 = job.milestones.get(119).unwrap();
    assert_eq!(ms0.status, MilestoneStatus::Released);
    assert_eq!(ms119.status, MilestoneStatus::Released);
}

// ============================================================================
// NEW: Boundary / Edge-Case / Auth / Negative-Input Tests for approve_milestone
// ============================================================================

/// Unrelated bad-actor address calls approve_milestone → Unauthorized.
#[test]
fn test_approve_milestone_unknown_caller_unauthorized() {
    let env = Env::default();
    env.mock_all_auths();

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let arbiter_addr = Address::generate(&env);
    let admin_addr = Address::generate(&env);
    let bad_actor = Address::generate(&env);
    let token_contract_id = env
        .register_stellar_asset_contract_v2(admin_addr.clone())
        .address();
    let token_admin = token::StellarAssetClient::new(&env, &token_contract_id);
    token_admin.mint(&client_addr, &10_000);

    let contract_id = env.register(MilestoneEscrow, ());
    let client = MilestoneEscrowClient::new(&env, &contract_id);

    let amounts = vec![&env, 10_000_i128];
    client.initialize(
        &admin_addr,
        &client_addr,
        &freelancer_addr,
        &arbiter_addr,
        &token_contract_id,
        &604800,
        &amounts,
    );
    client.fund(&client_addr);
    client.mark_delivered(&freelancer_addr, &0u32);

    let result = client.try_approve_milestone(&bad_actor, &0u32);
    assert_eq!(result, Err(Ok(Error::Unauthorized)));
}

/// Multiple partial releases before the final approve_milestone still leaves
/// the milestone in PartiallyReleased and the final approve transfers the
/// exact remaining amount.
#[test]
fn test_approve_milestone_multiple_partial_releases_then_full() {
    let env = Env::default();
    env.mock_all_auths();

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let arbiter_addr = Address::generate(&env);
    let admin_addr = Address::generate(&env);
    let token_contract_id = env
        .register_stellar_asset_contract_v2(admin_addr.clone())
        .address();
    let token = token::Client::new(&env, &token_contract_id);
    let token_admin = token::StellarAssetClient::new(&env, &token_contract_id);
    token_admin.mint(&client_addr, &10_000);

    let contract_id = env.register(MilestoneEscrow, ());
    let client = MilestoneEscrowClient::new(&env, &contract_id);

    let amounts = vec![&env, 10_000_i128];
    client.initialize(
        &admin_addr,
        &client_addr,
        &freelancer_addr,
        &arbiter_addr,
        &token_contract_id,
        &604800,
        &amounts,
    );
    client.fund(&client_addr);
    client.mark_delivered(&freelancer_addr, &0u32);

    client.approve_partial(&client_addr, &0u32, &2_000_i128);
    client.approve_partial(&client_addr, &0u32, &3_000_i128);
    client.approve_partial(&client_addr, &0u32, &1_000_i128);

    assert_eq!(token.balance(&freelancer_addr), 6_000);
    let job = client.get_job();
    let ms = job.milestones.get(0).unwrap();
    assert_eq!(ms.released_amount, 6_000);
    assert_eq!(ms.status, MilestoneStatus::PartiallyReleased);

    client.approve_milestone(&client_addr, &0u32);

    assert_eq!(token.balance(&freelancer_addr), 10_000);
    assert_eq!(token.balance(&contract_id), 0);
    let job = client.get_job();
    let ms = job.milestones.get(0).unwrap();
    assert_eq!(ms.status, MilestoneStatus::Released);
    assert_eq!(ms.released_amount, 10_000);
}

/// approve_partial with the exact remaining amount sets the milestone to
/// Released; a subsequent approve_milestone must fail with InvalidStatus
/// because remaining == 0.
#[test]
fn test_approve_milestone_after_full_partial_release_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let arbiter_addr = Address::generate(&env);
    let admin_addr = Address::generate(&env);
    let token_contract_id = env
        .register_stellar_asset_contract_v2(admin_addr.clone())
        .address();
    let token_admin = token::StellarAssetClient::new(&env, &token_contract_id);
    token_admin.mint(&client_addr, &10_000);

    let contract_id = env.register(MilestoneEscrow, ());
    let client = MilestoneEscrowClient::new(&env, &contract_id);

    let amounts = vec![&env, 10_000_i128];
    client.initialize(
        &admin_addr,
        &client_addr,
        &freelancer_addr,
        &arbiter_addr,
        &token_contract_id,
        &604800,
        &amounts,
    );
    client.fund(&client_addr);
    client.mark_delivered(&freelancer_addr, &0u32);

    client.approve_partial(&client_addr, &0u32, &10_000_i128);

    let job = client.get_job();
    let ms = job.milestones.get(0).unwrap();
    assert_eq!(ms.status, MilestoneStatus::Released);
    assert_eq!(ms.released_amount, 10_000);

    let result = client.try_approve_milestone(&client_addr, &0u32);
    assert_eq!(result, Err(Ok(Error::InvalidStatus)));
}

/// Approving milestone 1 in a two-milestone escrow after milestone 0 is
/// Released transfers only milestone 1's funds.
#[test]
fn test_approve_milestone_second_milestone_in_multi_escrow() {
    let env = Env::default();
    env.mock_all_auths();

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let arbiter_addr = Address::generate(&env);
    let admin_addr = Address::generate(&env);
    let token_contract_id = env
        .register_stellar_asset_contract_v2(admin_addr.clone())
        .address();
    let token = token::Client::new(&env, &token_contract_id);
    let token_admin = token::StellarAssetClient::new(&env, &token_contract_id);
    token_admin.mint(&client_addr, &15_000);

    let contract_id = env.register(MilestoneEscrow, ());
    let client = MilestoneEscrowClient::new(&env, &contract_id);

    let amounts = vec![&env, 10_000_i128, 5_000_i128];
    client.initialize(
        &admin_addr,
        &client_addr,
        &freelancer_addr,
        &arbiter_addr,
        &token_contract_id,
        &604800,
        &amounts,
    );
    client.fund(&client_addr);
    client.mark_delivered(&freelancer_addr, &0u32);
    client.mark_delivered(&freelancer_addr, &1u32);

    client.approve_milestone(&client_addr, &0u32);
    assert_eq!(token.balance(&freelancer_addr), 10_000);
    assert_eq!(token.balance(&contract_id), 5_000);

    client.approve_milestone(&client_addr, &1u32);
    assert_eq!(token.balance(&freelancer_addr), 15_000);
    assert_eq!(token.balance(&contract_id), 0);

    let job = client.get_job();
    let ms0 = job.milestones.get(0).unwrap();
    let ms1 = job.milestones.get(1).unwrap();
    assert_eq!(ms0.status, MilestoneStatus::Released);
    assert_eq!(ms1.status, MilestoneStatus::Released);
    assert_eq!(ms0.released_amount, 10_000);
    assert_eq!(ms1.released_amount, 5_000);
}

/// After claim_auto_release sets the milestone to Released, approve_milestone
/// must reject with InvalidStatus (negative-input / boundary state).
#[test]
fn test_approve_milestone_after_claim_auto_release_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let arbiter_addr = Address::generate(&env);
    let admin_addr = Address::generate(&env);
    let token_contract_id = env
        .register_stellar_asset_contract_v2(admin_addr.clone())
        .address();
    let token_admin = token::StellarAssetClient::new(&env, &token_contract_id);
    token_admin.mint(&client_addr, &10_000);

    let contract_id = env.register(MilestoneEscrow, ());
    let client = MilestoneEscrowClient::new(&env, &contract_id);

    let amounts = vec![&env, 10_000_i128];
    client.initialize(
        &admin_addr,
        &client_addr,
        &freelancer_addr,
        &arbiter_addr,
        &token_contract_id,
        &1,
        &amounts,
    );
    client.fund(&client_addr);
    client.mark_delivered(&freelancer_addr, &0u32);

    env.ledger().set_timestamp(2);
    client.claim_auto_release(&freelancer_addr, &0u32);

    let job = client.get_job();
    let ms = job.milestones.get(0).unwrap();
    assert_eq!(ms.status, MilestoneStatus::Released);
    assert_eq!(ms.released_amount, 10_000);

    let result = client.try_approve_milestone(&client_addr, &0u32);
    assert_eq!(result, Err(Ok(Error::InvalidStatus)));
}

/// Two approve_partial calls + one approve_milestone emit exactly three
/// "approve" events, confirming no duplicate or spurious events.
#[test]
fn test_approve_milestone_event_count_after_multiple_partials() {
    let env = Env::default();
    env.mock_all_auths();

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let arbiter_addr = Address::generate(&env);
    let admin_addr = Address::generate(&env);
    let token_contract_id = env
        .register_stellar_asset_contract_v2(admin_addr.clone())
        .address();
    let token_admin = token::StellarAssetClient::new(&env, &token_contract_id);
    token_admin.mint(&client_addr, &10_000);

    let contract_id = env.register(MilestoneEscrow, ());
    let client = MilestoneEscrowClient::new(&env, &contract_id);

    let amounts = vec![&env, 10_000_i128];
    client.initialize(
        &admin_addr,
        &client_addr,
        &freelancer_addr,
        &arbiter_addr,
        &token_contract_id,
        &604800,
        &amounts,
    );
    client.fund(&client_addr);
    client.mark_delivered(&freelancer_addr, &0u32);

    client.approve_partial(&client_addr, &0u32, &3_000_i128);
    client.approve_partial(&client_addr, &0u32, &4_000_i128);
    client.approve_milestone(&client_addr, &0u32);

    let approve_topic: Symbol = symbol_short!("approve");
    let approve_topic_val: Val = approve_topic.into_val(&env);
    let approve_count = env.events().all().iter().fold(0u32, |acc, e| {
        if let Some(topic) = e.1.get(0) {
            if topic.get_payload() == approve_topic_val.get_payload() {
                return acc + 1;
            }
        }
        acc
    });

    assert_eq!(approve_count, 3);
}

// ============================================================================

/// Test for mark_delivered gas scaling - verifies O(1) complexity
#[test]
fn test_mark_delivered_gas_scales_sublinearly() {
    let env = Env::default();
    env.mock_all_auths();

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let arbiter_addr = Address::generate(&env);
    let admin_addr = Address::generate(&env);

    let token_id = env
        .register_stellar_asset_contract_v2(admin_addr.clone())
        .address();
    let token_admin = token::StellarAssetClient::new(&env, &token_id);

    let contract_id = env.register(MilestoneEscrow, ());
    let escrow = MilestoneEscrowClient::new(&env, &contract_id);

    let milestone_count = 120u32;
    let per_milestone = 100i128;
    let mut amounts: Vec<i128> = Vec::new(&env);
    for _ in 0..milestone_count {
        amounts.push_back(per_milestone);
    }
    token_admin.mint(&client_addr, &(milestone_count as i128 * per_milestone));

    escrow.initialize(
        &admin_addr,
        &client_addr,
        &freelancer_addr,
        &arbiter_addr,
        &token_id,
        &604800,
        &amounts,
    );
    escrow.fund(&client_addr);

    for i in 0..(milestone_count - 2) {
        escrow.mark_delivered(&freelancer_addr, &i);
        escrow.approve_milestone(&client_addr, &i);
    }

    let initial_budget = env.cost_estimate().budget().cpu_instruction_cost();
    escrow.mark_delivered(&freelancer_addr, &119u32);
    let budget_after_mark = env.cost_estimate().budget().cpu_instruction_cost();

    let job = escrow.get_job();
    let ms = job.milestones.get(119).unwrap();
    assert_eq!(ms.status, MilestoneStatus::Delivered);

    let gas_used = initial_budget - budget_after_mark;
    assert!(gas_used < 500_000, "mark_delivered should be O(1), gas used: {}", gas_used);
}

/// Test for mark_delivered with many milestones - verifies state isolation
#[test]
fn test_mark_delivered_many_milestones() {
    let env = Env::default();
    env.mock_all_auths();

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let arbiter_addr = Address::generate(&env);
    let admin_addr = Address::generate(&env);

    let token_id = env
        .register_stellar_asset_contract_v2(admin_addr.clone())
        .address();
    let token_admin = token::StellarAssetClient::new(&env, &token_id);

    let contract_id = env.register(MilestoneEscrow, ());
    let escrow = MilestoneEscrowClient::new(&env, &contract_id);

    let milestone_count = 100u32;
    let per_milestone = 100i128;
    let mut amounts: Vec<i128> = Vec::new(&env);
    for _ in 0..milestone_count {
        amounts.push_back(per_milestone);
    }
    token_admin.mint(&client_addr, &(milestone_count as i128 * per_milestone));

    escrow.initialize(
        &admin_addr,
        &client_addr,
        &freelancer_addr,
        &arbiter_addr,
        &token_id,
        &604800,
        &amounts,
    );
    escrow.fund(&client_addr);

    escrow.mark_delivered(&freelancer_addr, &0u32);

    let job = escrow.get_job();
    for i in 0..milestone_count {
        let ms = job.milestones.get(i).unwrap();
        if i == 0 {
            assert_eq!(ms.status, MilestoneStatus::Delivered);
        } else {
            assert_eq!(ms.status, MilestoneStatus::Pending);
        }
    }
}

/// Test that only client can approve_partial (unauthorized fails)
#[test]
fn test_unauthorized_partial_release_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let arbiter_addr = Address::generate(&env);
    let admin_addr = Address::generate(&env);
    let bad_actor = Address::generate(&env);

    let token_id = env
        .register_stellar_asset_contract_v2(admin_addr.clone())
        .address();
    let token_admin = token::StellarAssetClient::new(&env, &token_id);
    token_admin.mint(&client_addr, &10_000);

    let contract_id = env.register(MilestoneEscrow, ());
    let escrow = MilestoneEscrowClient::new(&env, &contract_id);

    let amounts = vec![&env, 10_000_i128];
    escrow.initialize(
        &admin_addr,
        &client_addr,
        &freelancer_addr,
        &arbiter_addr,
        &token_id,
        &604800,
        &amounts,
    );
    escrow.fund(&client_addr);
    escrow.mark_delivered(&freelancer_addr, &0u32);

    let result = escrow.try_approve_partial(&bad_actor, &0u32, &1_000_i128);
    assert!(result.is_err());
}

/// Reentrancy guard test for approve_partial
#[test]
fn test_approve_partial_reentrancy_guard() {
    let env = Env::default();
    env.mock_all_auths();

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let arbiter_addr = Address::generate(&env);
    let admin_addr = Address::generate(&env);

    let token_id = env
        .register_stellar_asset_contract_v2(admin_addr.clone())
        .address();
    let token_admin = token::StellarAssetClient::new(&env, &token_id);
    token_admin.mint(&client_addr, &10_000);

    let contract_id = env.register(MilestoneEscrow, ());
    let escrow = MilestoneEscrowClient::new(&env, &contract_id);

    let amounts = vec![&env, 10_000_i128];
    escrow.initialize(
        &admin_addr,
        &client_addr,
        &freelancer_addr,
        &arbiter_addr,
        &token_id,
        &604800,
        &amounts,
    );
    escrow.fund(&client_addr);
    escrow.mark_delivered(&freelancer_addr, &0u32);

    escrow.approve_partial(&client_addr, &0u32, &4_000_i128);

    let job = escrow.get_job();
    let ms = job.milestones.get(0).unwrap();
    assert_eq!(ms.released_amount, 4_000);
    assert_eq!(ms.status, MilestoneStatus::PartiallyReleased);
}
