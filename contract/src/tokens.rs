use crate::constants::MAX_SUPPLY;

use super::*;
#[near_bindgen]
impl Contract {
    pub(crate) fn ft_internal_mint(&mut self, to: &AccountId, amount: Balance) {
        if self.token.total_supply + amount > MAX_SUPPLY {
            panic!("MAX SUPPLY EXCEEDED");
        } else {
            if self.token.accounts.get(to).is_some() {
                self.token.internal_deposit(to, amount);
            } else {
                self.token.internal_register_account(&to);
                self.token.internal_deposit(to, amount);
            }
        }
    }
    pub fn ft_mint(&mut self, to: AccountId, amount: Balance) {
        self.ft_internal_mint(&to, amount);
    }
}
