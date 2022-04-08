use super::*;

#[near_bindgen]
impl Contract {
    pub fn get_reward_rate(&self) -> u8 {
        self.cookie_reward_rate
    }
    pub fn get_fee_rate(&self) -> u8 {
        self.fee_percent
    }
    //Get Stake Data
    pub fn get_stake_by_id(&self, id: AccountId) -> Stake {
        self.stake
            .get(&id)
            .unwrap_or_else(|| panic!("No stake data found for this account"))
    }
}
