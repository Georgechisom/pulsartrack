#![cfg(test)]
use super::*;
use soroban_sdk::{testutils::{Address as _, Events}, Address, Env, vec, IntoVal};

#[test]
fn test_initialize() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, EscrowVaultContract);
    let client = EscrowVaultContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let oracle = Address::generate(&env);

    client.initialize(&admin, &token, &oracle);
}

#[test]
#[should_panic(expected = "already initialized")]
fn test_initialize_twice() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, EscrowVaultContract);
    let client = EscrowVaultContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let oracle = Address::generate(&env);

    client.initialize(&admin, &token, &oracle);
    client.initialize(&admin, &token, &oracle);
}

#[test]
#[should_panic]
fn test_initialize_non_admin_fails() {
    let env = Env::default();
    
    let contract_id = env.register_contract(None, EscrowVaultContract);
    let client = EscrowVaultContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let oracle = Address::generate(&env);

    // This should panic because admin didn't authorize it and we haven't mocked it
    client.initialize(&admin, &token, &oracle);
}
