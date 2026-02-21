#![cfg(test)]
use super::*;
use soroban_sdk::{testutils::Address as _, Address, Env, vec, String};

#[test]
fn test_initialize() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, TimelockExecutorContract);
    let client = TimelockExecutorContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let executor = Address::generate(&env);
    let min_delay_secs = 1u64;
    let max_delay_secs = 1u64;

    client.initialize(&admin, &executor, &min_delay_secs, &max_delay_secs);
}

#[test]
#[should_panic(expected = "already initialized")]
fn test_initialize_twice() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, TimelockExecutorContract);
    let client = TimelockExecutorContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let executor = Address::generate(&env);
    let min_delay_secs = 1u64;
    let max_delay_secs = 1u64;

    client.initialize(&admin, &executor, &min_delay_secs, &max_delay_secs);
    client.initialize(&admin, &executor, &min_delay_secs, &max_delay_secs);
}

#[test]
#[should_panic]
fn test_initialize_non_admin_fails() {
    let env = Env::default();
    
    let contract_id = env.register_contract(None, TimelockExecutorContract);
    let client = TimelockExecutorContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let executor = Address::generate(&env);
    let min_delay_secs = 1u64;
    let max_delay_secs = 1u64;

    // This should panic because admin didn't authorize it and we haven't mocked it
    client.initialize(&admin, &executor, &min_delay_secs, &max_delay_secs);
}
