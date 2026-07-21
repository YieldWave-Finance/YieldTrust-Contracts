#![cfg(test)]

use super::*;
use soroban_sdk::token;
use soroban_sdk::{testutils::Address as _, Address, Env};

fn setup() -> (Env, ArbitrationContractClient<'static>, Address, Address, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let funder = Address::generate(&env);
    let grantee = Address::generate(&env);
    let arbitrator = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_addr = env.register_stellar_asset_contract_v2(token_admin).address();
    let token_client = token::StellarAssetClient::new(&env, &token_addr);
    token_client.mint(&funder, &1000);

    let contract_id = env.register(ArbitrationContract, ());
    let client = ArbitrationContractClient::new(&env, &contract_id);
    client.init(&admin, &token_addr);

    (env, client, funder, grantee, arbitrator, token_addr)
}

#[test]
fn test_raise_and_resolve_dispute() {
    let (env, client, funder, grantee, arbitrator, token_addr) = setup();

    let dispute_id = client.raise_dispute(&1, &funder, &grantee, &1000, &arbitrator);
    assert_eq!(dispute_id, 1);
    assert_eq!(client.dispute_count(), 1);

    // Resolve with equal split
    client.resolve_dispute(&dispute_id, &500, &500);

    let real_token = token::Client::new(&env, &token_addr);
    assert_eq!(real_token.balance(&funder), 500);
    assert_eq!(real_token.balance(&grantee), 500);
}

#[test]
fn test_accept_dispute_state_transition() {
    let (_env, client, funder, grantee, arbitrator, _token_addr) = setup();

    let dispute_id = client.raise_dispute(&2, &funder, &grantee, &500, &arbitrator);

    // After raising, dispute should be Pending
    let d = client.get_dispute(&dispute_id);
    assert_eq!(d.status, DisputeStatus::Pending);

    // Arbitrator accepts -> InArbitration
    client.accept_dispute(&dispute_id);
    let d2 = client.get_dispute(&dispute_id);
    assert_eq!(d2.status, DisputeStatus::InArbitration);
}

#[test]
fn test_full_award_to_grantee() {
    let (env, client, funder, grantee, arbitrator, token_addr) = setup();

    let dispute_id = client.raise_dispute(&3, &funder, &grantee, &800, &arbitrator);
    client.resolve_dispute(&dispute_id, &0, &800);

    let real_token = token::Client::new(&env, &token_addr);
    assert_eq!(real_token.balance(&grantee), 800);
    assert_eq!(real_token.balance(&funder), 200); // had 1000, escrowed 800
}

#[test]
#[should_panic(expected = "Already resolved")]
fn test_double_resolution_panics() {
    let (_env, client, funder, grantee, arbitrator, _token_addr) = setup();

    let dispute_id = client.raise_dispute(&4, &funder, &grantee, &100, &arbitrator);
    client.resolve_dispute(&dispute_id, &50, &50);
    // Second call must panic
    client.resolve_dispute(&dispute_id, &0, &0);
}

#[test]
#[should_panic(expected = "Awards exceed escrowed amount")]
fn test_over_award_panics() {
    let (_env, client, funder, grantee, arbitrator, _token_addr) = setup();

    let dispute_id = client.raise_dispute(&5, &funder, &grantee, &100, &arbitrator);
    client.resolve_dispute(&dispute_id, &101, &0);
}
