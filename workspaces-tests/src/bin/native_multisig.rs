// Ignore
pub fn main() {}

use std::fmt::Display;

use near_contract_tools::{
    approval::{
        native_transaction_action::{self, NativeTransactionAction},
        simple_multisig::{AccountApprover, ApprovalState, Configuration},
        ApprovalManager,
    },
    rbac::Rbac,
    slot::Slot,
    Rbac,
};
use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    env, near_bindgen, AccountId, BorshStorageKey, PanicOnDefault, Promise,
};

#[derive(BorshSerialize, BorshStorageKey)]
enum StorageKey {
    SimpleMultisig,
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
impl ApprovalManager<NativeTransactionAction, ApprovalState, Configuration<Self>> for Contract {
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
impl AccountApprover for Contract {
    type Error = ApproverError;

    fn approve_account(account_id: &AccountId) -> Result<(), ApproverError> {
        if Contract::has_role(account_id, &Role::Multisig) {
            Ok(())
        } else {
            Err(ApproverError::UnauthorizedAccount)
        }
    }
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new() -> Self {
        <Self as ApprovalManager<_, _, _>>::init(Configuration::new(2));

        Self {}
    }

    pub fn obtain_multisig_permission(&mut self) {
        self.add_role(&env::predecessor_account_id(), &Role::Multisig);
    }

    pub fn request(
        &mut self,
        receiver_id: AccountId,
        actions: Vec<native_transaction_action::PromiseAction>,
    ) -> u32 {
        self.require_role(&Role::Multisig);

        let request_id = self.add_request(
            native_transaction_action::NativeTransactionAction {
                receiver_id,
                actions,
            },
            Default::default(),
        );

        near_sdk::log!(format!("Request ID: {request_id}"));

        request_id
    }

    pub fn approve(&mut self, request_id: u32) {
        self.approve_request(request_id, None);
    }

    pub fn is_approved(&self, request_id: u32) -> bool {
        <Contract as ApprovalManager<_, _, _>>::is_approved(request_id)
    }

    pub fn execute(&mut self, request_id: u32) -> Promise {
        self.execute_request(request_id)
    }
}
