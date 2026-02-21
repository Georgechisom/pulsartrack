#![cfg(test)]
use super::*;
use soroban_sdk::{testutils::Address as _, Address, Env, IntoVal};

#[test]
fn test_initialize() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, GovernanceDaoContract);
    let client = GovernanceDaoContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    client.initialize(&admin, &token, &3600u32, &1000i128, &5000u32, &100i128);
}

#[test]
#[should_panic(expected = "already initialized")]
fn test_initialize_twice() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, GovernanceDaoContract);
    let client = GovernanceDaoContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    client.initialize(&admin, &token, &3600u32, &1000i128, &5000u32, &100i128);
    client.initialize(&admin, &token, &3600u32, &1000i128, &5000u32, &100i128);
}

#[test]
#[should_panic]
fn test_initialize_non_admin_fails() {
    let env = Env::default();
    
    let contract_id = env.register_contract(None, GovernanceDaoContract);
    let client = GovernanceDaoContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    // This should panic because admin didn't authorize it and we haven't mocked it
    client.initialize(&admin, &token, &3600u32, &1000i128, &5000u32, &100i128);
}
