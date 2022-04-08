use super::*;
/* use near_contract_standards::fungible_token::events::FtStake; */
use near_sdk::{assert_one_yocto, ext_contract, Promise, ONE_YOCTO};
use serde::Serialize;

#[ext_contract(ext_factory_contract)]
pub trait Factory {
    fn checked_bake(&mut self, to: AccountId, amount: Balance, fee: Balance);
    fn ft_transfer(&mut self, receiver_id: AccountId, amount: U128, memo: Option<String>);
    fn checked_exchange(&mut self, amount: Balance) -> Balance;
}
#[ext_contract(ext_self)]
pub trait NekoStakeCallBack {
    fn neko_stake_call_back(&mut self, to: AccountId, amount: Balance, fee: Balance);
    fn claim_reward_call_back(&mut self);
    fn cookie_exchange_call_back(&mut self);
}

#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault, Serialize, Debug)]
pub struct Stake {
    pub total_stake: Balance,
    pub acc_reward: u128,
    pub last_update_time: u64,
}
impl Stake {
    pub fn cal_reward(&self, reward_rate: u128) -> Balance {
        let time_diff: u128 = (env::block_timestamp() - self.last_update_time) as u128;
        let time_since_last_update_in_min: u128 = (time_diff)
            .checked_div(1000000000 * 60)
            .unwrap_or_else(|| panic!("Reward Time Calculation Overflow")); // 60 seconds
        let reward_mul: u128 = (self.total_stake * reward_rate) as u128;
        let reward_per_mins = (reward_mul)
            .checked_div(100)
            .unwrap_or_else(|| panic!("Reward Per Time Calculation Overflow"));

        let reward_add: u128 = reward_per_mins * time_since_last_update_in_min;
        reward_add
    }
}
//Call Methods
#[near_bindgen]
impl Contract {
    #[payable]
    pub fn stake(&mut self, amount: u128) -> Promise {
        assert_eq!(env::attached_deposit(), ONE_YOCTO * 2);
        assert_eq!(
            amount <= self.token.ft_balance_of(env::signer_account_id()).0,
            true,
            "Insufficient NEKO balance for staking"
        );
        assert_eq!(amount >= 100, true, "Minimum stake is 100 NEKO");
        //process fee
        let fee = (amount * self.fee_percent as u128)
            .checked_div(100)
            .unwrap();

        let amount_after_fee = amount - fee;

        ext_factory_contract::checked_bake(
            env::signer_account_id(),
            amount_after_fee,
            fee,
            self.factory_id.clone(),
            ONE_YOCTO,
            env::prepaid_gas() / 3,
        )
        .then(ext_self::neko_stake_call_back(
            env::signer_account_id(),
            amount,
            fee,
            env::current_account_id(),
            ONE_YOCTO,
            env::prepaid_gas() / 3,
        ))
    }
    #[payable]
    pub fn claim_cookie(&mut self) -> Promise {
        assert_one_yocto();
        let claim_reward_fee = self.fee_percent;

        let reward_before_fee = self
            .stake
            .get(&env::signer_account_id())
            .unwrap_or_else(|| panic!("No stake record found for this account"))
            .acc_reward;
        let total_fee = (reward_before_fee * (claim_reward_fee as u128))
            .checked_div(100)
            .unwrap_or_else(|| panic!("Divide overflow check fail"));
        let reward_after_fee = reward_before_fee - total_fee;

        //external transfer cookie
        ext_factory_contract::ft_transfer(
            env::signer_account_id(),
            U128(reward_after_fee as u128),
            None,
            self.factory_id.clone(),
            ONE_YOCTO,
            env::prepaid_gas() / 3,
        )
        .then(ext_self::claim_reward_call_back(
            env::current_account_id(),
            0,
            env::prepaid_gas() / 3,
        ))
    }
    #[payable]
    pub fn unstake(&mut self, amount: u128) -> Promise {
        assert_one_yocto();
        /* assert!(
            self.token.ft_balance_of(env::current_account_id()).0 >= amount,
            "Insufficient NEKO balance in contract"
        ); */

        ext_factory_contract::checked_exchange(
            amount,
            self.factory_id.clone(),
            ONE_YOCTO,
            env::prepaid_gas() / 3,
        )
        .then(ext_self::cookie_exchange_call_back(
            env::current_account_id(),
            ONE_YOCTO,
            env::prepaid_gas() / 3,
        ))
    }
    #[private]
    pub fn claim_reward_call_back(&mut self) {
        let mut stake_data = self
            .stake
            .get(&env::signer_account_id())
            .unwrap_or_else(|| panic!("No stake record found for this account"));
        stake_data.acc_reward = 0;
        self.stake.insert(&env::signer_account_id(), &stake_data);
    }
    #[private]
    #[payable]
    pub fn neko_stake_call_back(
        &mut self,
        to: AccountId,
        amount: Balance,
        fee: Balance,
        #[callback] val: Balance,
    ) {
        assert_eq!(to, env::signer_account_id(), "Error signer");

        let amount_after_fee = val;
        self.token.internal_transfer(
            &env::signer_account_id(),
            &env::current_account_id(),
            amount,
            None,
        );
        //Burn the fee ( in NEKO) which will be Minted in Cookie
        self.token
            .internal_withdraw(&env::current_account_id(), fee);
        //update Account Stake Data and Increase Total Stake
        if let Some(_stake_data) = self.stake.get(&env::signer_account_id()) {
            self.update_stake_increase(&env::signer_account_id(), amount_after_fee)
        } else {
            let stake_data = Stake {
                total_stake: amount_after_fee,
                acc_reward: 0,
                last_update_time: env::block_timestamp(),
            };
            self.stake.insert(&env::signer_account_id(), &stake_data);
        }
    }
    #[private]
    #[payable]
    pub fn cookie_exchange_call_back(&mut self, #[callback] exchange_amount: Balance) {
        if self.token.ft_balance_of(env::current_account_id()).0 < exchange_amount {
            self.token
                .internal_deposit(&env::current_account_id(), exchange_amount)
        }
        self.token.internal_transfer(
            &env::current_account_id(),
            &env::signer_account_id(),
            exchange_amount,
            None,
        );
        if let Some(stake_data) = self.stake.get(&env::signer_account_id()) {
            self.update_stake_decrease(
                &env::signer_account_id(),
                std::cmp::min(exchange_amount, stake_data.total_stake),
            );
        }
    }
}

#[near_bindgen]
impl Contract {
    pub fn update_stake_data(&mut self, account_id: &AccountId) {
        if let Some(mut stake_data) = self.stake.get(account_id) {
            let reward_add = stake_data.cal_reward(self.cookie_reward_rate as u128);
            env::log_str(format!("reward added:{}", reward_add).as_str());
            if reward_add > 0 {
                stake_data.acc_reward += reward_add;
                stake_data.last_update_time = env::block_timestamp();
                self.stake.insert(account_id, &stake_data);
            }
        } else {
            panic!("None stake data found for this account");
        }
    }
    fn update_stake_increase(&mut self, account_id: &AccountId, amount: Balance) {
        if let Some(mut stake_data) = self.stake.get(account_id) {
            let reward_add = stake_data.cal_reward(self.cookie_reward_rate as u128);
            env::log_str(format!("reward added:{}", reward_add).as_str());
            /*    if reward_add > 0 { */
            stake_data.acc_reward += reward_add;
            stake_data.last_update_time = env::block_timestamp();
            /*    } */
            stake_data.total_stake += amount;
            self.stake.insert(account_id, &stake_data);
        } else {
            panic!("None stake data found for this account");
        }
    }
    fn update_stake_decrease(&mut self, account_id: &AccountId, amount: Balance) {
        if let Some(mut stake_data) = self.stake.get(account_id) {
            let reward_add = stake_data.cal_reward(self.cookie_reward_rate as u128);

            /*   if reward_add > 0 { */
            stake_data.acc_reward += reward_add;
            stake_data.last_update_time = env::block_timestamp();
            /*   } */
            stake_data.total_stake -= amount;
            self.stake.insert(account_id, &stake_data);
        } else {
            env::log_str(format!("Convert Without Stake added:{}", amount).as_str());
        }
    }
}
