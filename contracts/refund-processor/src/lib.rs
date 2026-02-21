//! PulsarTrack - Refund Processor (Soroban)
//! Campaign refund processing and dispute resolution on Stellar.

#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short,
    token, Address, Env, String,
};

#[contracttype]
#[derive(Clone, PartialEq)]
pub enum RefundStatus {
    Requested,
    UnderReview,
    Approved,
    Rejected,
    Processed,
}

#[contracttype]
#[derive(Clone)]
pub struct RefundRequest {
    pub refund_id: u64,
    pub requester: Address,
    pub campaign_id: u64,
    pub token: Address,
    pub amount_requested: i128,
    pub amount_approved: i128,
    pub reason: String,
    pub status: RefundStatus,
    pub submitted_at: u64,
    pub resolved_at: Option<u64>,
}

#[contracttype]
pub enum DataKey {
    Admin,
    TokenAddress,
    RefundCounter,
    AutoRefundPeriod,
    Refund(u64),
}

#[contract]
pub struct RefundProcessorContract;

#[contractimpl]
impl RefundProcessorContract {
    pub fn initialize(env: Env, admin: Address, token: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("already initialized");
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::TokenAddress, &token);
        env.storage().instance().set(&DataKey::RefundCounter, &0u64);
        env.storage().instance().set(&DataKey::AutoRefundPeriod, &604_800u64); // 7 days
    }

    pub fn request_refund(
        env: Env,
        requester: Address,
        campaign_id: u64,
        amount: i128,
        reason: String,
    ) -> u64 {
        requester.require_auth();

        if amount <= 0 {
            panic!("invalid amount");
        }

        let counter: u64 = env.storage().instance().get(&DataKey::RefundCounter).unwrap_or(0);
        let refund_id = counter + 1;

        let token_addr: Address = env.storage().instance().get(&DataKey::TokenAddress).unwrap();

        let refund = RefundRequest {
            refund_id,
            requester: requester.clone(),
            campaign_id,
            token: token_addr,
            amount_requested: amount,
            amount_approved: 0,
            reason,
            status: RefundStatus::Requested,
            submitted_at: env.ledger().timestamp(),
            resolved_at: None,
        };

        env.storage().persistent().set(&DataKey::Refund(refund_id), &refund);
        env.storage().instance().set(&DataKey::RefundCounter, &refund_id);

        refund_id
    }

    pub fn approve_refund(env: Env, admin: Address, refund_id: u64, approved_amount: i128) {
        admin.require_auth();
        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        if admin != stored_admin {
            panic!("unauthorized");
        }

        let mut refund: RefundRequest = env
            .storage()
            .persistent()
            .get(&DataKey::Refund(refund_id))
            .expect("refund not found");

        if refund.status != RefundStatus::Requested && refund.status != RefundStatus::UnderReview {
            panic!("invalid status");
        }

        refund.amount_approved = approved_amount.min(refund.amount_requested);
        refund.status = RefundStatus::Approved;
        refund.resolved_at = Some(env.ledger().timestamp());

        env.storage().persistent().set(&DataKey::Refund(refund_id), &refund);
    }

    pub fn reject_refund(env: Env, admin: Address, refund_id: u64) {
        admin.require_auth();
        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        if admin != stored_admin {
            panic!("unauthorized");
        }

        let mut refund: RefundRequest = env
            .storage()
            .persistent()
            .get(&DataKey::Refund(refund_id))
            .expect("refund not found");

        refund.status = RefundStatus::Rejected;
        refund.resolved_at = Some(env.ledger().timestamp());

        env.storage().persistent().set(&DataKey::Refund(refund_id), &refund);
    }

    pub fn process_refund(env: Env, refund_id: u64) {
        let mut refund: RefundRequest = env
            .storage()
            .persistent()
            .get(&DataKey::Refund(refund_id))
            .expect("refund not found");

        if refund.status != RefundStatus::Approved {
            panic!("refund not approved");
        }

        let token_client = token::Client::new(&env, &refund.token);
        token_client.transfer(
            &env.current_contract_address(),
            &refund.requester,
            &refund.amount_approved,
        );

        refund.status = RefundStatus::Processed;
        env.storage().persistent().set(&DataKey::Refund(refund_id), &refund);

        env.events().publish(
            (symbol_short!("refund"), symbol_short!("processed")),
            (refund_id, refund.amount_approved),
        );
    }

    pub fn get_refund(env: Env, refund_id: u64) -> Option<RefundRequest> {
        env.storage().persistent().get(&DataKey::Refund(refund_id))
    }
}

mod test;
