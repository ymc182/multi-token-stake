/*
 * This is an example of a Rust smart contract with two simple, symmetric functions:
 *
 * 1. set_greeting: accepts a greeting, such as "howdy", and records it for the user (account_id)
 *    who sent the request
 * 2. get_greeting: accepts an account_id and returns the greeting saved for it, defaulting to
 *    "Hello"
 *
 * Learn more about writing NEAR smart contracts with Rust:
 * https://github.com/near/near-sdk-rs
 *
 */

// To conserve gas, efficient serialization is achieved through Borsh (http://borsh.io/)
use crate::constants::*;
use near_contract_standards::fungible_token::metadata::{
    FungibleTokenMetadata, FungibleTokenMetadataProvider, FT_METADATA_SPEC,
};
use near_contract_standards::fungible_token::FungibleToken;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LazyOption, UnorderedMap};
use near_sdk::json_types::Base64VecU8;
use near_sdk::json_types::U128;
use near_sdk::Promise;
use near_sdk::{
    env, log, near_bindgen, require, AccountId, Balance, BorshStorageKey, PanicOnDefault,
    PromiseOrValue,
};
use stake::Stake;
mod accounts;
mod constants;
mod owner;
mod stake;
mod tokens;
mod view;
/* const contract_version: &str = "0.0.1"; */
#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {
    token: FungibleToken,
    metadata: LazyOption<FungibleTokenMetadata>,
    owner_id: AccountId,
    stake: UnorderedMap<AccountId, Stake>,

    //config
    factory_id: AccountId,
    vault_id: AccountId,
    fee_percent: u8,
    cookie_reward_rate: u8,
}
#[derive(BorshSerialize, BorshStorageKey)]
enum StorageKey {
    FungibleToken,
    Metadata,
    StakeData,
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new_default_meta(
        owner_id: AccountId,
        vault_id: AccountId,
        factory_id: AccountId,
        fee_percent: u8,
        cookie_reward_rate: u8,
    ) -> Self {
        Self::new(
            owner_id,
            FungibleTokenMetadata {
                spec: FT_METADATA_SPEC.to_string(),
                name: "NEKO".to_string(),
                symbol: "NEK".to_string(),
                icon: Some(DATA_IMAGE_SVG_NEAR_ICON.to_string()),
                reference: None,
                reference_hash: None,
                decimals: 0,
            },
            vault_id,
            factory_id,
            fee_percent,
            cookie_reward_rate,
        )
    }
    #[init]
    pub fn new(
        owner_id: AccountId,
        metadata: FungibleTokenMetadata,
        vault_id: AccountId,
        factory_id: AccountId,
        fee_percent: u8,
        cookie_reward_rate: u8,
    ) -> Self {
        require!(!env::state_exists(), "Already initialized");
        metadata.assert_valid();
        let mut this = Self {
            owner_id: owner_id.clone(),
            token: FungibleToken::new(StorageKey::FungibleToken),
            metadata: LazyOption::new(StorageKey::Metadata, Some(&metadata)),
            stake: UnorderedMap::new(StorageKey::StakeData.try_to_vec().unwrap()),
            factory_id: factory_id.clone(),
            vault_id: vault_id.clone(),
            fee_percent,
            cookie_reward_rate,
        };
        this.token.vault = vault_id.clone();
        this.token.internal_register_account(&owner_id);
        this.token.internal_register_account(&vault_id.clone());
        this.token.internal_register_account(&factory_id);
        if this.owner_id != env::current_account_id() {
            this.token
                .internal_register_account(&env::current_account_id());
        }

        this
    }

    pub fn clean(keys: Vec<Base64VecU8>) {
        for key in keys.iter() {
            env::storage_remove(&key.0);
        }
    }

    fn on_account_closed(&mut self, account_id: AccountId, balance: Balance) {
        log!("Closed @{} with {}", account_id, balance);
    }

    fn on_tokens_burned(&mut self, account_id: AccountId, amount: Balance) {
        log!("Account @{} burned {}", account_id, amount);
    }
    pub fn update_vault(&mut self, vault_id: AccountId) {
        self.assert_owner(env::signer_account_id());
        self.token.vault = vault_id.clone();
        self.token.internal_register_account(&vault_id.clone());
        env::log_str("update vault");
    }
    pub fn assert_owner(&self, account_id: AccountId) {
        assert_eq!(self.owner_id, account_id, "Assert owner failed");
    }
    pub fn register_account(&mut self, account_id: AccountId) {
        self.token.internal_register_account(&account_id);
    }
}
near_contract_standards::impl_fungible_token_core!(Contract, token, on_tokens_burned);
near_contract_standards::impl_fungible_token_storage!(Contract, token, on_account_closed);
#[near_bindgen]
impl FungibleTokenMetadataProvider for Contract {
    fn ft_metadata(&self) -> FungibleTokenMetadata {
        self.metadata.get().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::convert::TryInto;

    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::{testing_env, Gas, ONE_NEAR};

    // mock the context for testing, notice "signer_account_id" that was accessed above from env::
    fn get_context(predecessor_account_id: AccountId) -> VMContextBuilder {
        let mut builder = VMContextBuilder::new();
        builder
            .current_account_id(accounts(0))
            .signer_account_id(predecessor_account_id.clone())
            .predecessor_account_id(predecessor_account_id);
        builder
    }

    #[test]
    fn mint_and_transfer_test() {
        let mut context = get_context(accounts(0));
        testing_env!(context
            .storage_usage(env::storage_usage())
            .attached_deposit(1)
            .predecessor_account_id(accounts(0))
            .build());
        let mut contract = Contract::new_default_meta(
            accounts(0),
            "vault.testnet".parse().unwrap(),
            "factory.testnet".parse().unwrap(),
            5,
            5,
        );
        contract.ft_mint(accounts(0), 10_000_000_000);
        contract.ft_mint(accounts(1), 0);

        contract.ft_transfer(accounts(1), U128(10_000_000_000), None);
        let a = contract.ft_balance_of(accounts(1));
        assert_eq!(a.0, 9500000000);
    }
    #[test]
    fn test_views() {
        let mut context = get_context(accounts(0));
        testing_env!(context
            .storage_usage(env::storage_usage())
            .attached_deposit(0)
            .predecessor_account_id(accounts(0))
            .build());
        let contract = Contract::new_default_meta(
            accounts(0),
            "vault.testnet".parse().unwrap(),
            "factory.testnet".parse().unwrap(),
            5,
            5,
        );
        println!("Fee rate:{:?}", contract.get_fee_rate());
        assert!(contract.get_fee_rate() == 5);
        println!("Cookie reward rate:{:?}", contract.get_reward_rate());
        assert!(contract.get_reward_rate() == 5);
    }
    #[test]
    fn test_owner_methods() {
        let mut context = get_context(accounts(0));
        testing_env!(context
            .storage_usage(env::storage_usage())
            .attached_deposit(0)
            .predecessor_account_id(accounts(0))
            .build());
        let mut contract = Contract::new_default_meta(
            accounts(0),
            "vault.testnet".parse().unwrap(),
            "factory.testnet".parse().unwrap(),
            5,
            5,
        );
        assert!(contract.get_fee_rate() == 5);
        assert!(contract.get_reward_rate() == 5);
        contract.set_bake_fee(10);
        contract.set_reward_rate(10);
        println!("Fee rate:{:?}", contract.get_fee_rate());
        assert!(contract.get_fee_rate() == 10);
        println!("Cookie reward rate:{:?}", contract.get_reward_rate());
        assert!(contract.get_reward_rate() == 10);
    }
    #[test]
    #[should_panic(expected = "Assert owner failed")]
    fn test_owner_access() {
        let mut context = get_context(accounts(0));
        testing_env!(context
            .storage_usage(env::storage_usage())
            .attached_deposit(0)
            .predecessor_account_id(accounts(0))
            .build());
        let mut contract = Contract::new_default_meta(
            accounts(1),
            "vault.testnet".parse().unwrap(),
            "factory.testnet".parse().unwrap(),
            5,
            5,
        );

        contract.set_reward_rate(10);
    }
}
