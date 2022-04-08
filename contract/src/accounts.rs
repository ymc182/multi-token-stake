use super::*;
use near_sdk::{ext_contract, Promise};
const STORAGE_COST: u128 = 10000000000000000000000;
#[ext_contract(ext_factory_contract)]
pub trait Factory {
    fn storage_deposit(
        &mut self,
        account_id: Option<AccountId>,
        registration_only: Option<bool>,
    ) -> StorageBalance;
}
#[ext_contract(ext_self)]
pub trait NekoStakeCallBack {
    fn setup_account_call_back(&mut self);
}
#[near_bindgen]
impl Contract {
    pub fn setup_account(&mut self) -> Promise {
        assert!(
            env::attached_deposit() > STORAGE_COST * 2,
            "Insufficient deposit"
        );
        ext_factory_contract::storage_deposit(
            Some(env::signer_account_id()),
            Some(true),
            self.factory_id.clone(),
            STORAGE_COST,
            env::prepaid_gas(),
        )
        .then(ext_self::setup_account_call_back(
            env::current_account_id(),
            STORAGE_COST,
            env::prepaid_gas(),
        ))
    }
    #[private]
    pub fn setup_account_call_back(&mut self) {
        self.token
            .storage_deposit(Some(env::signer_account_id()), Some(true));
    }
}
