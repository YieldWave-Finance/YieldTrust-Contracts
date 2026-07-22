#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env,
};

fn set_timestamp(env: &Env, timestamp: u64) {
    env.ledger().with_mut(|li| {
        li.timestamp = timestamp;
    });
}

/// Returns `(env, client, grantor, beneficiary, token_addr)`.
/// Grantor starts with 10_000 tokens.
fn setup() -> (Env, VestingContractClient<'static>, Address, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();

    let grantor = Address::generate(&env);
    let beneficiary = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_sac = env.register_stellar_asset_contract_v2(token_admin);
    let token_addr_inner = token_sac.address();
    let stellar_asset = token::StellarAssetClient::new(&env, &token_addr_inner);
    stellar_asset.mint(&grantor, &10_000);

    let contract_id = env.register(VestingContract, ());
    let client = VestingContractClient::new(&env, &contract_id);

    (env, client, grantor, beneficiary, token_addr_inner)
}

#[test]
fn test_full_vesting_after_end() {
    let (env, client, grantor, beneficiary, token_addr) = setup();

    // start=50, cliff=100, end=200, total=1000
    set_timestamp(&env, 50);
    client.init(&grantor, &beneficiary, &token_addr, &1000, &100, &200);

    // before cliff: nothing claimable
    assert_eq!(client.claimable(), 0);

    // at end: fully vested
    set_timestamp(&env, 200);
    assert_eq!(client.claimable(), 1000);

    let claimed = client.claim();
    assert_eq!(claimed, 1000);

    let tok = token::Client::new(&env, &token_addr);
    assert_eq!(tok.balance(&beneficiary), 1000);
    assert_eq!(client.total_claimed(), 1000);
}

#[test]
fn test_linear_midpoint() {
    let (env, client, grantor, beneficiary, token_addr) = setup();

    // cliff=100, end=300, total=1000 — midpoint t=200 => 500 vested
    set_timestamp(&env, 50);
    client.init(&grantor, &beneficiary, &token_addr, &1000, &100, &300);

    set_timestamp(&env, 200);
    assert_eq!(client.claimable(), 500);
    client.claim();

    // at full end: remaining 500
    set_timestamp(&env, 300);
    assert_eq!(client.claimable(), 500);
    client.claim();

    let tok = token::Client::new(&env, &token_addr);
    assert_eq!(tok.balance(&beneficiary), 1000);
}

#[test]
fn test_cliff_not_reached_error() {
    let (env, client, grantor, beneficiary, token_addr) = setup();

    set_timestamp(&env, 10);
    client.init(&grantor, &beneficiary, &token_addr, &1000, &100, &200);

    set_timestamp(&env, 50);
    let result = client.try_claim();
    assert_eq!(result, Err(Ok(VestingError::CliffNotReached)));
}

#[test]
fn test_nothing_to_claim_error() {
    let (env, client, grantor, beneficiary, token_addr) = setup();

    set_timestamp(&env, 10);
    client.init(&grantor, &beneficiary, &token_addr, &1000, &100, &200);

    set_timestamp(&env, 200);
    client.claim();

    let result = client.try_claim();
    assert_eq!(result, Err(Ok(VestingError::NothingToClaim)));
}

#[test]
fn test_double_init_fails() {
    let (env, client, grantor, beneficiary, token_addr) = setup();

    set_timestamp(&env, 10);
    client.init(&grantor, &beneficiary, &token_addr, &500, &100, &200);

    let result = client.try_init(&grantor, &beneficiary, &token_addr, &500, &100, &200);
    assert_eq!(result, Err(Ok(VestingError::AlreadyInitialized)));
}
