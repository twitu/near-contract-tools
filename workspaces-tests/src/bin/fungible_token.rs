#![allow(missing_docs)]

// Ignore
pub fn main() {}

use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    env,
    json_types::U128,
    near_bindgen, PanicOnDefault,
};
use near_sdk_contract_tools::{standard::nep141::*, FungibleToken};

#[derive(PanicOnDefault, BorshSerialize, BorshDeserialize, FungibleToken)]
#[fungible_token(name = "My Fungible Token", symbol = "MYFT", decimals = 18, no_hooks)]
#[near_bindgen]
pub struct Contract {}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new() -> Self {
        Self {}
    }

    pub fn mint(&mut self, amount: U128) {
        self.deposit_unchecked(&env::predecessor_account_id(), amount.into());
    }
}
