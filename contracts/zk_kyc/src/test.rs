#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, Address, Env};

fn setup() -> (Env, ZKKYCContractClient<'static>, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();

    let verifier = Address::generate(&env);
    let user = Address::generate(&env);

    let contract_id = env.register(ZKKYCContract, ());
    let client = ZKKYCContractClient::new(&env, &contract_id);
    client.init(&verifier);

    (env, client, verifier, user)
}

#[test]
fn test_verify_and_revoke() {
    let (_env, client, _verifier, user) = setup();

    assert!(!client.is_verified(&user));
    client.verify_user(&user);
    assert!(client.is_verified(&user));
    client.revoke_user(&user);
    assert!(!client.is_verified(&user));
}

#[test]
fn test_double_init_fails() {
    let (env, _client, verifier, _user) = setup();

    let contract_id2 = env.register(ZKKYCContract, ());
    let client2 = ZKKYCContractClient::new(&env, &contract_id2);
    client2.init(&verifier);

    let result = client2.try_init(&verifier);
    assert_eq!(result, Err(Ok(KycError::AlreadyInitialized)));
}

#[test]
fn test_unverified_user_returns_false() {
    let (_env, client, _verifier, user) = setup();
    // Fresh user — never verified
    assert!(!client.is_verified(&user));
}

#[test]
fn test_multiple_users_independent() {
    let (env, client, _verifier, user1) = setup();
    let user2 = Address::generate(&env);

    client.verify_user(&user1);
    assert!(client.is_verified(&user1));
    assert!(!client.is_verified(&user2));

    client.verify_user(&user2);
    assert!(client.is_verified(&user1));
    assert!(client.is_verified(&user2));

    client.revoke_user(&user1);
    assert!(!client.is_verified(&user1));
    assert!(client.is_verified(&user2));
}
