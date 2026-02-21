//! PulsarTrack - Multisig Treasury (Soroban)
//! Multi-signature treasury for platform fund management on Stellar.

#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short,
    token, Address, Env, String, Vec,
};

#[contracttype]
#[derive(Clone, PartialEq)]
pub enum TxStatus {
    Pending,
    Approved,
    Executed,
    Rejected,
    Expired,
}

#[contracttype]
#[derive(Clone)]
pub struct TreasuryTx {
    pub tx_id: u64,
    pub proposer: Address,
    pub recipient: Address,
    pub token: Address,
    pub amount: i128,
    pub description: String,
    pub status: TxStatus,
    pub approvals: u32,
    pub rejections: u32,
    pub required_approvals: u32,
    pub created_at: u64,
    pub expires_at: u64,
    pub executed_at: Option<u64>,
}

#[contracttype]
pub enum DataKey {
    Admin,
    Signers,
    RequiredSigners,
    TxCounter,
    Tx(u64),
    TxApproval(u64, Address),
}

#[contract]
pub struct MultisigTreasuryContract;

#[contractimpl]
impl MultisigTreasuryContract {
    pub fn initialize(env: Env, admin: Address, initial_signers: Vec<Address>, required: u32) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("already initialized");
        }
        admin.require_auth();

        if required == 0 || required > initial_signers.len() as u32 {
            panic!("invalid required signers");
        }

        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Signers, &initial_signers);
        env.storage().instance().set(&DataKey::RequiredSigners, &required);
        env.storage().instance().set(&DataKey::TxCounter, &0u64);
    }

    pub fn propose_transaction(
        env: Env,
        proposer: Address,
        recipient: Address,
        token: Address,
        amount: i128,
        description: String,
        expires_in: u64,
    ) -> u64 {
        proposer.require_auth();

        let signers: Vec<Address> = env.storage().instance().get(&DataKey::Signers).unwrap();
        let is_signer = signers.contains(&proposer);

        if !is_signer {
            panic!("not a signer");
        }

        if amount <= 0 {
            panic!("invalid amount");
        }

        let counter: u64 = env.storage().instance().get(&DataKey::TxCounter).unwrap_or(0);
        let tx_id = counter + 1;
        let required: u32 = env.storage().instance().get(&DataKey::RequiredSigners).unwrap();

        let tx = TreasuryTx {
            tx_id,
            proposer: proposer.clone(),
            recipient,
            token,
            amount,
            description,
            status: TxStatus::Pending,
            approvals: 0,
            rejections: 0,
            required_approvals: required,
            created_at: env.ledger().timestamp(),
            expires_at: env.ledger().timestamp() + expires_in,
            executed_at: None,
        };

        env.storage().persistent().set(&DataKey::Tx(tx_id), &tx);
        env.storage().instance().set(&DataKey::TxCounter, &tx_id);

        env.events().publish(
            (symbol_short!("treasury"), symbol_short!("proposed")),
            (tx_id, proposer),
        );

        tx_id
    }

    pub fn approve_transaction(env: Env, signer: Address, tx_id: u64) {
        signer.require_auth();

        let signers: Vec<Address> = env.storage().instance().get(&DataKey::Signers).unwrap();
        if !signers.contains(&signer) {
            panic!("not a signer");
        }

        if env.storage().persistent().has(&DataKey::TxApproval(tx_id, signer.clone())) {
            panic!("already voted");
        }

        let mut tx: TreasuryTx = env
            .storage()
            .persistent()
            .get(&DataKey::Tx(tx_id))
            .expect("tx not found");

        if tx.status != TxStatus::Pending {
            panic!("tx not pending");
        }

        if env.ledger().timestamp() > tx.expires_at {
            tx.status = TxStatus::Expired;
            env.storage().persistent().set(&DataKey::Tx(tx_id), &tx);
            panic!("tx expired");
        }

        tx.approvals += 1;
        env.storage().persistent().set(&DataKey::TxApproval(tx_id, signer), &true);

        if tx.approvals >= tx.required_approvals {
            tx.status = TxStatus::Approved;
        }

        env.storage().persistent().set(&DataKey::Tx(tx_id), &tx);
    }

    pub fn execute_transaction(env: Env, caller: Address, tx_id: u64) {
        caller.require_auth();

        let signers: Vec<Address> = env.storage().instance().get(&DataKey::Signers).unwrap();
        if !signers.contains(&caller) {
            panic!("not a signer");
        }

        let mut tx: TreasuryTx = env
            .storage()
            .persistent()
            .get(&DataKey::Tx(tx_id))
            .expect("tx not found");

        if tx.status != TxStatus::Approved {
            panic!("tx not approved");
        }

        let token_client = token::Client::new(&env, &tx.token);
        token_client.transfer(
            &env.current_contract_address(),
            &tx.recipient,
            &tx.amount,
        );

        tx.status = TxStatus::Executed;
        tx.executed_at = Some(env.ledger().timestamp());
        env.storage().persistent().set(&DataKey::Tx(tx_id), &tx);

        env.events().publish(
            (symbol_short!("treasury"), symbol_short!("executed")),
            (tx_id, tx.amount),
        );
    }

    pub fn reject_transaction(env: Env, signer: Address, tx_id: u64) {
        signer.require_auth();

        let signers: Vec<Address> = env.storage().instance().get(&DataKey::Signers).unwrap();
        if !signers.contains(&signer) {
            panic!("not a signer");
        }

        let mut tx: TreasuryTx = env
            .storage()
            .persistent()
            .get(&DataKey::Tx(tx_id))
            .expect("tx not found");

        if tx.status != TxStatus::Pending {
            panic!("tx not pending");
        }

        tx.rejections += 1;

        let total_signers = signers.len() as u32;
        let required: u32 = env.storage().instance().get(&DataKey::RequiredSigners).unwrap();
        let max_possible_approvals = total_signers - tx.rejections;

        if max_possible_approvals < required {
            tx.status = TxStatus::Rejected;
        }

        env.storage().persistent().set(&DataKey::Tx(tx_id), &tx);
    }

    pub fn get_transaction(env: Env, tx_id: u64) -> Option<TreasuryTx> {
        env.storage().persistent().get(&DataKey::Tx(tx_id))
    }

    pub fn get_signers(env: Env) -> Vec<Address> {
        env.storage().instance().get(&DataKey::Signers).unwrap()
    }
}

mod test;
