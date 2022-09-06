#![allow(missing_docs)]

// Ignore
pub fn main() {}

use std::fmt::Display;

use near_contract_tools::{
    approval::{
        self,
        simple_multisig::{AccountAuthorizer, ApprovalState, Configuration},
        ApprovalManager,
    },
    rbac::Rbac,
    slot::Slot,
    Rbac,
};
use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    env, near_bindgen, BorshStorageKey, PanicOnDefault,
};

#[derive(BorshSerialize, BorshStorageKey)]
enum StorageKey {
    SimpleMultisig,
}

#[derive(BorshSerialize, BorshDeserialize)]
enum MyAction {
    SayHello,
    SayGoodbye,
}

impl approval::Action for MyAction {
    type Output = &'static str;

    fn execute(self) -> Self::Output {
        match self {
            Self::SayHello => "hello",
            Self::SayGoodbye => "goodbye",
        }
    }
}

#[derive(BorshSerialize, BorshStorageKey)]
enum Role {
    Multisig,
}

#[derive(PanicOnDefault, BorshSerialize, BorshDeserialize, Rbac)]
#[rbac(roles = "Role")]
#[near_bindgen]
pub struct Contract {}

// This single function implementation completely implements simple multisig on
// the contract
impl ApprovalManager<MyAction, ApprovalState, Configuration<Self>> for Contract {
    fn root() -> Slot<()> {
        Slot::new(StorageKey::SimpleMultisig)
    }
}

#[derive(Clone, Copy, Debug)]
pub enum ApproverError {
    UnauthorizedAccount,
}

impl Display for ApproverError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Unauthorized account")
    }
}

// We don't have to check env::predecessor_account_id or anything like that
// SimpleMultisig handles it all for us
impl AccountAuthorizer for Contract {
    fn is_account_authorized(account_id: &near_sdk::AccountId) -> bool {
        Contract::has_role(account_id, &Role::Multisig)
    }
}

#[near_bindgen]
impl Contract {
    const APPROVAL_THRESHOLD: u8 = 2;
    const VALIDITY_PERIOD: u64 = 1000000 * 1000 * 60 * 60 * 24 * 7;

    #[init]
    pub fn new() -> Self {
        <Self as ApprovalManager<_, _, _>>::init(Configuration::new(
            Self::APPROVAL_THRESHOLD,
            Self::VALIDITY_PERIOD,
        ));

        Self {}
    }

    pub fn obtain_multisig_permission(&mut self) {
        self.add_role(&env::predecessor_account_id(), &Role::Multisig);
    }

    pub fn request(&mut self, action: String) -> u32 {
        let action = match &action[..] {
            "hello" => MyAction::SayHello,
            "goodbye" => MyAction::SayGoodbye,
            _ => env::panic_str("action must be \"hello\" or \"goodbye\""),
        };

        let request_id = self.create_request(action, ApprovalState::new()).unwrap();

        near_sdk::log!(format!("Request ID: {request_id}"));

        request_id
    }

    pub fn approve(&mut self, request_id: u32) {
        self.approve_request(request_id).unwrap();
    }

    pub fn is_approved(&self, request_id: u32) -> bool {
        <Contract as ApprovalManager<_, _, _>>::is_approved(request_id)
    }

    pub fn execute(&mut self, request_id: u32) -> String {
        self.execute_request(request_id).unwrap().to_string()
    }
}
