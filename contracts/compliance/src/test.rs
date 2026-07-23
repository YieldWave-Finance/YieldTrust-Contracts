#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, Address, Env};

fn setup() -> (Env, ComplianceContractClient<'static>, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();

    let officer = Address::generate(&env);
    let target = Address::generate(&env);

    let contract_id = env.register(ComplianceContract, ());
    let client = ComplianceContractClient::new(&env, &contract_id);
    client.init(&officer);

    (env, client, officer, target)
}

#[test]
fn test_sanction_and_unsanction() {
    let (_env, client, _officer, target) = setup();

    assert!(!client.is_sanctioned(&target));
    client.sanction(&target);
    assert!(client.is_sanctioned(&target));
    client.unsanction(&target);
    assert!(!client.is_sanctioned(&target));
}

#[test]
fn test_flag_and_unflag() {
    let (_env, client, _officer, target) = setup();

    assert!(!client.is_flagged(&target));
    client.flag_address(&target);
    assert!(client.is_flagged(&target));
    client.unflag_address(&target);
    assert!(!client.is_flagged(&target));
}

#[test]
fn test_double_init_fails() {
    let (env, _client, officer, _target) = setup();

    // Second contract with double-init attempt
    let contract_id2 = env.register(ComplianceContract, ());
    let client2 = ComplianceContractClient::new(&env, &contract_id2);
    client2.init(&officer);
    let result = client2.try_init(&officer);
    assert_eq!(result, Err(Ok(ComplianceError::AlreadyInitialized)));
}

#[test]
fn test_sanction_and_flag_are_independent() {
    let (_env, client, _officer, target) = setup();

    client.sanction(&target);
    // Sanctioned but not flagged
    assert!(client.is_sanctioned(&target));
    assert!(!client.is_flagged(&target));

    client.flag_address(&target);
    // Now both
    assert!(client.is_sanctioned(&target));
    assert!(client.is_flagged(&target));

    client.unsanction(&target);
    // Unsanctioned but still flagged
    assert!(!client.is_sanctioned(&target));
    assert!(client.is_flagged(&target));
}
