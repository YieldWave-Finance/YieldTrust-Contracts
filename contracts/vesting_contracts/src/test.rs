#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, token, Address, Env};

/// Helper: create a funded env with a token and a deployed vesting contract.
///
/// Returns `(env, client, grantor, beneficiary, token_addr)`.
/// The grantor starts with 10_000 tokens.
fn setup(
) -> (
    Env,
    VestingContractClient<'static>,
    Address,
    Address,
    Address,
) {
    let env = Env::default();
    env.mock_all_auths();

    let grantor = Address::generate(&env);
    let beneficiary = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_addr = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();
    let stellar_asset = token::StellarAssetClient::new(&env, &token_addr);
    stellar_asset.mint(&grantor, &10_000);

    let contract_id = env.register(VestingContract, ());
    let client = VestingContractClient::new(&env, &contract_id);

    (env, client, grantor, beneficiary, token_addr)
}

#[test]
fn test_full_vesting_after_end() {
    let (mut env, client, grantor, beneficiary, token_addr) = setup();

    // Schedule: cliff at t=100, ends at t=200, total=1000
    env.ledger().set_timestamp(50);
    client.init(&grantor, &beneficiary, &token_addr, &1000, &100, &200);

    // Before cliff: nothing claimable
    assert_eq!(client.claimable(), Ok(0));

    // At end: everything vested
    env.ledger().set_timestamp(200);
    assert_eq!(client.claimable(), Ok(1000));

    // Claim all
    let claimed = client.claim();
    assert_eq!(claimed, Ok(1000));

    let tok = token::Client::new(&env, &token_addr);
    assert_eq!(tok.balance(&beneficiary), 1000);
    assert_eq!(client.total_claimed(), 1000);
}

#[test]
fn test_linear_midpoint() {
    let (mut env, client, grantor, beneficiary, token_addr) = setup();

    // Cliff at 100, end at 300, total=1000 — midpoint is t=200 => 500 vested
    env.ledger().set_timestamp(50);
    client.init(&grantor, &beneficiary, &token_addr, &1000, &100, &300);

    env.ledger().set_timestamp(200);
    assert_eq!(client.claimable(), Ok(500));

    client.claim();

    // Second claim at full end: remaining 500
    env.ledger().set_timestamp(300);
    assert_eq!(client.claimable(), Ok(500));
    client.claim();

    let tok = token::Client::new(&env, &token_addr);
    assert_eq!(tok.balance(&beneficiary), 1000);
}

#[test]
fn test_cliff_not_reached_error() {
    let (mut env, client, grantor, beneficiary, token_addr) = setup();

    env.ledger().set_timestamp(10);
    client.init(&grantor, &beneficiary, &token_addr, &1000, &100, &200);

    // Still before cliff
    env.ledger().set_timestamp(50);
    let result = client.try_claim();
    assert_eq!(result, Err(Ok(VestingError::CliffNotReached)));
}

#[test]
fn test_nothing_to_claim_error() {
    let (mut env, client, grantor, beneficiary, token_addr) = setup();

    env.ledger().set_timestamp(10);
    client.init(&grantor, &beneficiary, &token_addr, &1000, &100, &200);

    // Claim at end
    env.ledger().set_timestamp(200);
    client.claim();

    // Second claim immediately — nothing left
    let result = client.try_claim();
    assert_eq!(result, Err(Ok(VestingError::NothingToClaim)));
}

#[test]
fn test_double_init_fails() {
    let (mut env, client, grantor, beneficiary, token_addr) = setup();

    env.ledger().set_timestamp(10);
    client.init(&grantor, &beneficiary, &token_addr, &500, &100, &200);

    let result = client.try_init(&grantor, &beneficiary, &token_addr, &500, &100, &200);
    assert_eq!(result, Err(Ok(VestingError::AlreadyInitialized)));
}
