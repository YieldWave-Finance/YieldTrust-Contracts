//! # ⚠️ WIP — DO NOT DEPLOY TO PRODUCTION ⚠️
//!
//! **Status: Work In Progress**
//!
//! This contract is named `zk_kyc` but currently contains **no zero-knowledge
//! proof verification logic whatsoever**. It is a simple verifier-managed
//! allow-list, not a ZK system. The following are required before this
//! contract can be considered production-ready:
//!
//! - **ZK proof verification**: integrate an on-chain verifier (e.g. a Groth16
//!   or PLONK verifier) that accepts a proof and public inputs. The `verify_user`
//!   function must validate the proof rather than simply trusting the caller to
//!   be the designated verifier.
//! - **Nullifier tracking**: to prevent proof replay attacks, each proof
//!   nullifier must be stored and checked for uniqueness.
//! - **Credential expiry**: KYC attestations should carry an expiry timestamp;
//!   `is_verified` must check freshness.
//! - **Revocation list**: support a signed revocation list alongside individual
//!   `revoke_user` calls.
//! - **Verifier key rotation**: the trusted verification key (currently the
//!   single `verifier` address) must be rotatable by governance without
//!   disrupting existing attestations.
//! - **Privacy guarantees**: the current design stores `KycStatus(address)` in
//!   persistent storage, which is publicly readable on-chain. A true ZK-KYC
//!   system must not leak the identity of verified users.
//!
//! Until all of the above are addressed, this contract provides **no
//! meaningful privacy or ZK security properties** and **must not** be
//! deployed to Mainnet.

#![no_std]
use soroban_sdk::{contract, contractimpl, contracterror, contracttype, Address, Env};

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[contracterror]
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
#[repr(u32)]
pub enum KycError {
    NotInitialized = 1,
    AlreadyInitialized = 2,
    /// Placeholder — will be used once real ZK proof verification is wired up.
    InvalidProof = 3,
    /// Proof nullifier has already been used (replay protection — WIP).
    NullifierReused = 4,
    /// Attestation has expired (WIP — expiry not yet enforced).
    AttestationExpired = 5,
}

// ---------------------------------------------------------------------------
// Storage keys
// ---------------------------------------------------------------------------

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    /// The trusted verifier / proof-authority address.
    Verifier,
    /// Whether a given address has passed KYC.
    ///
    /// # WIP — privacy risk
    /// This key is publicly readable. A production ZK-KYC contract must not
    /// expose user identity on-chain.
    KycStatus(Address),
}

// ---------------------------------------------------------------------------
// Contract
// ---------------------------------------------------------------------------

#[contract]
pub struct ZKKYCContract;

#[contractimpl]
impl ZKKYCContract {
    /// Initialise the contract with a trusted verifier address.
    /// May only be called once.
    pub fn init(env: Env, verifier: Address) -> Result<(), KycError> {
        if env.storage().instance().has(&DataKey::Verifier) {
            return Err(KycError::AlreadyInitialized);
        }
        env.storage().instance().set(&DataKey::Verifier, &verifier);
        Ok(())
    }

    /// Mark `user` as KYC-verified.
    ///
    /// # ⚠️ WIP — Not a real ZK proof check
    /// This function does **not** verify any zero-knowledge proof. It simply
    /// requires the trusted `verifier` address to sign the transaction.
    /// This is a centralised allow-list, not a privacy-preserving ZK system.
    ///
    /// A production implementation must:
    /// 1. Accept a serialised ZK proof and public inputs as parameters.
    /// 2. Run the verifier circuit on-chain and reject if invalid.
    /// 3. Record the proof nullifier to prevent replay.
    pub fn verify_user(env: Env, user: Address) -> Result<(), KycError> {
        let verifier: Address = env
            .storage()
            .instance()
            .get(&DataKey::Verifier)
            .ok_or(KycError::NotInitialized)?;
        verifier.require_auth();
        env.storage()
            .persistent()
            .set(&DataKey::KycStatus(user), &true);
        Ok(())
    }

    /// Revoke KYC status for `user`.
    ///
    /// # WIP
    /// Does not emit an event or add the user to a revocation list.
    pub fn revoke_user(env: Env, user: Address) -> Result<(), KycError> {
        let verifier: Address = env
            .storage()
            .instance()
            .get(&DataKey::Verifier)
            .ok_or(KycError::NotInitialized)?;
        verifier.require_auth();
        env.storage()
            .persistent()
            .remove(&DataKey::KycStatus(user));
        Ok(())
    }

    /// Returns `true` if `user` has an active KYC attestation.
    ///
    /// # ⚠️ WIP — No expiry enforced
    /// Attestations currently never expire. A production system must check
    /// an expiry timestamp stored alongside the KYC status.
    pub fn is_verified(env: Env, user: Address) -> bool {
        env.storage()
            .persistent()
            .get(&DataKey::KycStatus(user))
            .unwrap_or(false)
    }
}
