//! # ⚠️ WIP — DO NOT DEPLOY TO PRODUCTION ⚠️
//!
//! **Status: Work In Progress**
//!
//! This contract provides a basic on-chain arbitration escrow skeleton. The
//! following features are **not yet implemented** and MUST be completed before
//! any production deployment:
//!
//! - `InArbitration` state transition: arbitrator acceptance of a dispute is
//!   not enforced; any caller can jump straight to resolution.
//! - Multi-arbitrator / quorum support.
//! - Appeal window / time-lock on resolution.
//! - Integration with the `grant_stream` contract for automatic escrow release.
//! - Comprehensive edge-case test coverage (partial awards, duplicate raises,
//!   re-entrancy guards).
//!
//! Until these items are addressed this contract **must not** be included in a
//! production build or deployed on Mainnet.

#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, token, Address, Env};

// ---------------------------------------------------------------------------
// Storage keys
// ---------------------------------------------------------------------------

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Admin,
    Token,
    DisputeCounter,
    Dispute(u32),
}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DisputeStatus {
    /// Dispute raised; awaiting arbitrator acceptance.
    Pending,
    /// Arbitrator has accepted and is actively adjudicating.
    /// NOTE: transition into this state is not yet enforced — WIP.
    InArbitration,
    /// Final award has been paid out.
    Resolved,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Dispute {
    pub grant_id: u32,
    pub funder: Address,
    pub grantee: Address,
    /// Total escrowed amount (in token stroops).
    pub amount: i128,
    pub status: DisputeStatus,
    pub arbitrator: Address,
}

// ---------------------------------------------------------------------------
// Contract
// ---------------------------------------------------------------------------

#[contract]
pub struct ArbitrationContract;

#[contractimpl]
impl ArbitrationContract {
    /// Initialise the contract. May only be called once.
    pub fn init(env: Env, admin: Address, token: Address) {
        admin.require_auth();
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("Already initialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Token, &token);
        env.storage().instance().set(&DataKey::DisputeCounter, &0u32);
    }

    /// Open a dispute and escrow `amount` tokens from the funder.
    ///
    /// Returns the new dispute ID.
    ///
    /// # WIP limitations
    /// - There is no check that `grant_id` corresponds to an active grant in
    ///   `grant_stream`.
    /// - Re-raising a dispute for the same grant is not blocked.
    pub fn raise_dispute(
        env: Env,
        grant_id: u32,
        funder: Address,
        grantee: Address,
        amount: i128,
        arbitrator: Address,
    ) -> u32 {
        funder.require_auth();

        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let token_client = token::Client::new(&env, &token_addr);
        token_client.transfer(&funder, &env.current_contract_address(), &amount);

        let mut counter: u32 = env
            .storage()
            .instance()
            .get(&DataKey::DisputeCounter)
            .unwrap();
        counter += 1;
        env.storage()
            .instance()
            .set(&DataKey::DisputeCounter, &counter);

        let dispute = Dispute {
            grant_id,
            funder,
            grantee,
            amount,
            status: DisputeStatus::Pending,
            arbitrator,
        };

        env.storage()
            .persistent()
            .set(&DataKey::Dispute(counter), &dispute);
        counter
    }

    /// Arbitrator accepts a dispute and moves it into active adjudication.
    ///
    /// # WIP
    /// This function is provided as a stub. It transitions the dispute to
    /// `InArbitration` but does **not** yet enforce a time-lock, require a
    /// bond from the arbitrator, or notify the grant_stream contract.
    pub fn accept_dispute(env: Env, dispute_id: u32) {
        let mut dispute: Dispute = env
            .storage()
            .persistent()
            .get(&DataKey::Dispute(dispute_id))
            .unwrap();
        dispute.arbitrator.require_auth();

        if dispute.status != DisputeStatus::Pending {
            panic!("Dispute is not in Pending state");
        }

        dispute.status = DisputeStatus::InArbitration;
        env.storage()
            .persistent()
            .set(&DataKey::Dispute(dispute_id), &dispute);
    }

    /// Resolve a dispute and distribute escrowed funds.
    ///
    /// Only the assigned arbitrator may call this.  Awards must not exceed the
    /// total escrowed amount; any remainder stays in the contract escrow.
    ///
    /// # WIP limitations
    /// - No appeal window is enforced.
    /// - Remainder (amount − funder_award − grantee_award) is locked forever.
    pub fn resolve_dispute(
        env: Env,
        dispute_id: u32,
        funder_award: i128,
        grantee_award: i128,
    ) {
        let mut dispute: Dispute = env
            .storage()
            .persistent()
            .get(&DataKey::Dispute(dispute_id))
            .unwrap();
        dispute.arbitrator.require_auth();

        if dispute.status == DisputeStatus::Resolved {
            panic!("Already resolved");
        }
        if funder_award + grantee_award > dispute.amount {
            panic!("Awards exceed escrowed amount");
        }

        dispute.status = DisputeStatus::Resolved;

        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let token_client = token::Client::new(&env, &token_addr);

        if funder_award > 0 {
            token_client.transfer(
                &env.current_contract_address(),
                &dispute.funder,
                &funder_award,
            );
        }
        if grantee_award > 0 {
            token_client.transfer(
                &env.current_contract_address(),
                &dispute.grantee,
                &grantee_award,
            );
        }

        env.storage()
            .persistent()
            .set(&DataKey::Dispute(dispute_id), &dispute);
    }

    /// Read a dispute record by ID.
    pub fn get_dispute(env: Env, dispute_id: u32) -> Dispute {
        env.storage()
            .persistent()
            .get(&DataKey::Dispute(dispute_id))
            .unwrap()
    }

    /// Return the current dispute counter (total disputes ever raised).
    pub fn dispute_count(env: Env) -> u32 {
        env.storage()
            .instance()
            .get(&DataKey::DisputeCounter)
            .unwrap_or(0)
    }
}
