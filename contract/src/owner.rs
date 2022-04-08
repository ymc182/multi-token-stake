use super::*;
#[near_bindgen]
impl Contract {
    pub fn assert_owner_signer(&self) {
        require!(
            self.owner_id == env::signer_account_id(),
            "Assert owner failed"
        );
    }

    pub fn set_reward_rate(&mut self, rate: u8) {
        self.assert_owner_signer();
        self.cookie_reward_rate = rate;
    }

    pub fn set_bake_fee(&mut self, rate: u8) {
        self.assert_owner_signer();
        self.fee_percent = rate;
    }
}
