//! # ⚠️ WIP — DO NOT DEPLOY TO PRODUCTION ⚠️
//!
//! **Status: Work In Progress**
//!
//! This contract provides cliff + linear vesting schedules for team and
//! investor token allocations. The following features are **not yet
//! implemented** and MUST be completed before any production deployment:
//!
//! - Multi-beneficiary support: currently only a single beneficiary per
//!   deployed instance is supported.
//! - Revocable vesting: an admin/grantor cannot cancel an in-progress schedule
//!   and recover unvested tokens.
//! - Cliff grace-period enforcement: the contract relies on `env.ledger().timestamp()`
//!   which may be manipulated in testing; real time-lock auditing is needed.
//! - Token allowance model: the contract currently requires a pre-funded escrow;
//!   a `transferFrom`-style approval flow is not yet supported.
//! - Schedule amendment: modifying a schedule after it starts is not supported.
//! - Events / audit log: no structured events are emitted on claim or schedule
//!   creation.
//!
//! Until these items are addressed this contract **must not** be included in a
//! production build or deployed on Mainnet.

#![no_std]
use soroban_sdk::{contract, contractimpl, contracterror, contracttype, token, Address, Env};

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[contracterror]
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
#[repr(u32)]
pub enum VestingError {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    CliffNotReached = 3,
    NothingToClaim = 4,
    InvalidSchedule = 5,
}

// ---------------------------------------------------------------------------
// Storage keys
// ---------------------------------------------------------------------------

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Schedule,
    Claimed,
}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A cliff + linear vesting schedule.
///
/// All timestamps are Unix seconds (from `env.ledger().timestamp()`).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VestingSchedule {
    /// Address that will receive vested tokens.
    pub beneficiary: Address,
    /// Token contract address.
    pub token: Address,
    /// Total tokens to vest over the full duration.
    pub total_amount: i128,
    /// Timestamp at which the cliff ends and linear vesting begins.
    pub cliff_timestamp: u64,
    /// Timestamp at which all tokens are fully vested.
    pub end_timestamp: u64,
    /// Timestamp at which vesting started (for linear calculation).
    pub start_timestamp: u64,
}

// ---------------------------------------------------------------------------
// Contract
// ---------------------------------------------------------------------------

#[contract]
pub struct VestingContract;

#[contractimpl]
impl VestingContract {
    /// Initialise a vesting schedule and deposit `total_amount` tokens into
    /// the contract's escrow.
    ///
    /// `grantor` must have pre-approved the contract to transfer the tokens
    /// via `token::Client::approve`.
    ///
    /// # WIP limitations
    /// - Only a single beneficiary / schedule per contract instance is
    ///   supported.
    /// - Schedule cannot be amended or revoked after creation.
    pub fn init(
        env: Env,
        grantor: Address,
        beneficiary: Address,
        token: Address,
        total_amount: i128,
        cliff_timestamp: u64,
        end_timestamp: u64,
    ) -> Result<(), VestingError> {
        grantor.require_auth();

        if env.storage().instance().has(&DataKey::Schedule) {
            return Err(VestingError::AlreadyInitialized);
        }

        let now = env.ledger().timestamp();
        if cliff_timestamp < now || end_timestamp <= cliff_timestamp || total_amount <= 0 {
            return Err(VestingError::InvalidSchedule);
        }

        // Pull tokens into escrow
        let token_client = token::Client::new(&env, &token);
        token_client.transfer(&grantor, &env.current_contract_address(), &total_amount);

        let schedule = VestingSchedule {
            beneficiary,
            token,
            total_amount,
            cliff_timestamp,
            end_timestamp,
            start_timestamp: now,
        };

        env.storage().instance().set(&DataKey::Schedule, &schedule);
        env.storage().instance().set(&DataKey::Claimed, &0i128);
        Ok(())
    }

    /// Returns the amount of tokens that have vested up to `now` but have
    /// not yet been claimed.
    pub fn claimable(env: Env) -> Result<i128, VestingError> {
        let schedule: VestingSchedule = env
            .storage()
            .instance()
            .get(&DataKey::Schedule)
            .ok_or(VestingError::NotInitialized)?;
        let claimed: i128 = env
            .storage()
            .instance()
            .get(&DataKey::Claimed)
            .unwrap_or(0);

        let now = env.ledger().timestamp();
        let vested = Self::vested_amount(&schedule, now);
        Ok(vested.saturating_sub(claimed))
    }

    /// Claim all currently vested but unclaimed tokens.
    ///
    /// Only the beneficiary may call this.
    pub fn claim(env: Env) -> Result<i128, VestingError> {
        let schedule: VestingSchedule = env
            .storage()
            .instance()
            .get(&DataKey::Schedule)
            .ok_or(VestingError::NotInitialized)?;
        schedule.beneficiary.require_auth();

        let now = env.ledger().timestamp();

        if now < schedule.cliff_timestamp {
            return Err(VestingError::CliffNotReached);
        }

        let claimed: i128 = env
            .storage()
            .instance()
            .get(&DataKey::Claimed)
            .unwrap_or(0);

        let vested = Self::vested_amount(&schedule, now);
        let claimable = vested.saturating_sub(claimed);

        if claimable == 0 {
            return Err(VestingError::NothingToClaim);
        }

        env.storage()
            .instance()
            .set(&DataKey::Claimed, &(claimed + claimable));

        let token_client = token::Client::new(&env, &schedule.token);
        token_client.transfer(
            &env.current_contract_address(),
            &schedule.beneficiary,
            &claimable,
        );

        Ok(claimable)
    }

    /// Read the vesting schedule.
    pub fn get_schedule(env: Env) -> Result<VestingSchedule, VestingError> {
        env.storage()
            .instance()
            .get(&DataKey::Schedule)
            .ok_or(VestingError::NotInitialized)
    }

    /// Total tokens claimed so far.
    pub fn total_claimed(env: Env) -> i128 {
        env.storage()
            .instance()
            .get(&DataKey::Claimed)
            .unwrap_or(0)
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    /// Compute total vested amount at timestamp `now` using cliff + linear
    /// vesting.
    ///
    /// - Before `cliff_timestamp`: 0.
    /// - Between `cliff_timestamp` and `end_timestamp`: linear proportion.
    /// - After `end_timestamp`: `total_amount`.
    fn vested_amount(schedule: &VestingSchedule, now: u64) -> i128 {
        if now < schedule.cliff_timestamp {
            return 0;
        }
        if now >= schedule.end_timestamp {
            return schedule.total_amount;
        }
        // Linear interpolation between cliff and end
        let elapsed = (now - schedule.cliff_timestamp) as i128;
        let duration = (schedule.end_timestamp - schedule.cliff_timestamp) as i128;
        // Use u128 intermediate to avoid overflow for large amounts
        ((schedule.total_amount as u128)
            .saturating_mul(elapsed as u128)
            / (duration as u128)) as i128
    }
}

mod test;
