#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, token, Address, Env};

fn setup() -> (Env, ArbitrationContractClient<'static>, Address, Address, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let funder = Address::generate(&env);
    let grantee = Address::generate(&env);
    let arbitrator = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_sac = env.register_stellar_asset_contract_v2(token_admin);
    let token_addr_inner = token_sac.address();
    let stellar_asset = token::StellarAssetClient::new(&env, &token_addr_inner);
    stellar_asset.mint(&funder, &1000);

    let contract_id = env.register(ArbitrationContract, ());
    let client = ArbitrationContractClient::new(&env, &contract_id);
    client.init(&admin, &token_addr_inner);

    (env, client, funder, grantee, arbitrator, token_addr_inner)
}

#[test]
fn test_raise_and_resolve_dispute() {
    let (env, client, funder, grantee, arbitrator, token_addr) = setup();

    let dispute_id = client.raise_dispute(&1, &funder, &grantee, &1000, &arbitrator);
    assert_eq!(dispute_id, 1);
    assert_eq!(client.dispute_count(), 1);

    client.resolve_dispute(&dispute_id, &500, &500);

    let tok = token::Client::new(&env, &token_addr);
    assert_eq!(tok.balance(&funder), 500);
    assert_eq!(tok.balance(&grantee), 500);
}

#[test]
fn test_accept_dispute_state_transition() {
    let (_env, client, funder, grantee, arbitrator, _token_addr) = setup();

    let dispute_id = client.raise_dispute(&2, &funder, &grantee, &500, &arbitrator);

    let d = client.get_dispute(&dispute_id);
    assert_eq!(d.status, DisputeStatus::Pending);

    client.accept_dispute(&dispute_id);
    let d2 = client.get_dispute(&dispute_id);
    assert_eq!(d2.status, DisputeStatus::InArbitration);
}

#[test]
fn test_full_award_to_grantee() {
    let (env, client, funder, grantee, arbitrator, token_addr) = setup();

    let dispute_id = client.raise_dispute(&3, &funder, &grantee, &800, &arbitrator);
    client.resolve_dispute(&dispute_id, &0, &800);

    let tok = token::Client::new(&env, &token_addr);
    assert_eq!(tok.balance(&grantee), 800);
    assert_eq!(tok.balance(&funder), 200);
}

#[test]
#[should_panic(expected = "Already resolved")]
fn test_double_resolution_panics() {
    let (_env, client, funder, grantee, arbitrator, _token_addr) = setup();

    let dispute_id = client.raise_dispute(&4, &funder, &grantee, &100, &arbitrator);
    client.resolve_dispute(&dispute_id, &50, &50);
    client.resolve_dispute(&dispute_id, &0, &0);
}

#[test]
#[should_panic(expected = "Awards exceed escrowed amount")]
fn test_over_award_panics() {
    let (_env, client, funder, grantee, arbitrator, _token_addr) = setup();

    let dispute_id = client.raise_dispute(&5, &funder, &grantee, &100, &arbitrator);
    client.resolve_dispute(&dispute_id, &101, &0);
}
