#![cfg(test)]
use super::*;
use soroban_sdk::{testutils::Address as _, Address, Env, vec, String};

#[test]
fn test_initialize() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, PerformanceOracleContract);
    let client = PerformanceOracleContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let min_attesters = 1u32;

    client.initialize(&admin, &min_attesters);
}

#[test]
#[should_panic(expected = "already initialized")]
fn test_initialize_twice() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, PerformanceOracleContract);
    let client = PerformanceOracleContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let min_attesters = 1u32;

    client.initialize(&admin, &min_attesters);
    client.initialize(&admin, &min_attesters);
}

#[test]
#[should_panic]
fn test_initialize_non_admin_fails() {
    let env = Env::default();
    
    let contract_id = env.register_contract(None, PerformanceOracleContract);
    let client = PerformanceOracleContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let min_attesters = 1u32;

    // This should panic because admin didn't authorize it and we haven't mocked it
    client.initialize(&admin, &min_attesters);
}
