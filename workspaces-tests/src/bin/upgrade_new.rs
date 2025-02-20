#![allow(missing_docs)]

use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    near_bindgen, PanicOnDefault,
};
use near_sdk_contract_tools::{
    migrate::{MigrateExternal, MigrateHook},
    Migrate,
};

pub fn main() {} // Ignore

#[derive(BorshDeserialize)]
pub struct ContractOld {
    pub foo: u32,
}

#[derive(BorshSerialize, BorshDeserialize, PanicOnDefault, Migrate)]
#[migrate(from = "ContractOld")]
#[near_bindgen]
pub struct ContractNew {
    pub bar: u64,
}

impl MigrateHook for ContractNew {
    fn on_migrate(old_schema: ContractOld) -> Self {
        Self {
            bar: old_schema.foo as u64,
        }
    }
}

#[near_bindgen]
impl ContractNew {
    #[init]
    pub fn new() -> Self {
        Self { bar: 0 }
    }

    pub fn get_bar(&self) -> u64 {
        self.bar
    }
}
