//! NEP-141 fungible token core implementation
//! <https://github.com/near/NEPs/blob/master/neps/nep-0141.md>
#![allow(missing_docs)] // ext_contract doesn't play nice with #![warn(missing_docs)]

use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    env, ext_contract,
    json_types::U128,
    require, AccountId, BorshStorageKey, Gas, Promise, PromiseOrValue, PromiseResult,
};
use near_sdk_contract_tools_macros::event;
use serde::{Deserialize, Serialize};

use crate::{slot::Slot, standard::nep297::*, DefaultStorageKey};

/// Gas value required for ft_resolve_transfer calls
pub const GAS_FOR_RESOLVE_TRANSFER: Gas = Gas(5_000_000_000_000);
/// Gas value required for ft_transfer_call calls (includes gas for )
pub const GAS_FOR_FT_TRANSFER_CALL: Gas = Gas(25_000_000_000_000 + GAS_FOR_RESOLVE_TRANSFER.0);

const MORE_GAS_FAIL_MESSAGE: &str = "More gas is required";

/// NEP-141 standard events for minting, burning, and transferring tokens
#[event(
    crate = "crate",
    macros = "crate",
    serde = "serde",
    standard = "nep141",
    version = "1.0.0"
)]
#[derive(Debug, Clone)]
pub enum Nep141Event {
    /// Token mint event. Emitted when tokens are created and total_supply is
    /// increased.
    FtMint(Vec<event::FtMintData>),

    /// Token transfer event. Emitted when tokens are transferred between two
    /// accounts. No change to total_supply.
    FtTransfer(Vec<event::FtTransferData>),

    /// Token burn event. Emitted when tokens are burned (removed from supply).
    /// Decrease in total_supply.
    FtBurn(Vec<event::FtBurnData>),
}

pub mod event {
    use near_sdk::{json_types::U128, AccountId};
    use serde::Serialize;

    /// Individual mint metadata
    #[derive(Serialize, Debug, Clone)]
    pub struct FtMintData {
        /// Address to which new tokens were minted
        pub owner_id: AccountId,
        /// Amount of minted tokens
        pub amount: U128,
        /// Optional note
        #[serde(skip_serializing_if = "Option::is_none")]
        pub memo: Option<String>,
    }

    /// Individual transfer metadata
    #[derive(Serialize, Debug, Clone)]
    pub struct FtTransferData {
        /// Account ID of the sender
        pub old_owner_id: AccountId,
        /// Account ID of the receiver
        pub new_owner_id: AccountId,
        /// Amount of transferred tokens
        pub amount: U128,
        /// Optional note
        #[serde(skip_serializing_if = "Option::is_none")]
        pub memo: Option<String>,
    }

    /// Individual burn metadata
    #[derive(Serialize, Debug, Clone)]
    pub struct FtBurnData {
        /// Account ID from which tokens were burned
        pub owner_id: AccountId,
        /// Amount of burned tokens
        pub amount: U128,
        /// Optional note
        #[serde(skip_serializing_if = "Option::is_none")]
        pub memo: Option<String>,
    }

    #[cfg(test)]
    mod tests {

        use super::{super::Nep141Event, *};
        use crate::standard::nep297::Event;

        #[test]
        fn mint() {
            assert_eq!(
                Nep141Event::FtMint(vec![FtMintData {
                    owner_id: "foundation.near".parse().unwrap(),
                    amount: 500u128.into(),
                    memo: None,
                }])
                .to_event_string(),
                r#"EVENT_JSON:{"standard":"nep141","version":"1.0.0","event":"ft_mint","data":[{"owner_id":"foundation.near","amount":"500"}]}"#,
            );
        }

        #[test]
        fn transfer() {
            assert_eq!(
                Nep141Event::FtTransfer(vec![
                    FtTransferData {
                        old_owner_id: "from.near".parse().unwrap(),
                        new_owner_id: "to.near".parse().unwrap(),
                        amount: 42u128.into(),
                        memo: Some("hi hello bonjour".to_string()),
                    },
                    FtTransferData {
                        old_owner_id: "user1.near".parse().unwrap(),
                        new_owner_id: "user2.near".parse().unwrap(),
                        amount: 7500u128.into(),
                        memo: None
                    },
                ])
                .to_event_string(),
                r#"EVENT_JSON:{"standard":"nep141","version":"1.0.0","event":"ft_transfer","data":[{"old_owner_id":"from.near","new_owner_id":"to.near","amount":"42","memo":"hi hello bonjour"},{"old_owner_id":"user1.near","new_owner_id":"user2.near","amount":"7500"}]}"#,
            );
        }

        #[test]
        fn burn() {
            assert_eq!(
                Nep141Event::FtBurn(vec![FtBurnData {
                    owner_id: "foundation.near".parse().unwrap(),
                    amount: 100u128.into(),
                    memo: None,
                }])
                .to_event_string(),
                r#"EVENT_JSON:{"standard":"nep141","version":"1.0.0","event":"ft_burn","data":[{"owner_id":"foundation.near","amount":"100"}]}"#,
            );
        }
    }
}

#[derive(BorshSerialize, BorshStorageKey)]
enum StorageKey {
    TotalSupply,
    Account(AccountId),
}

/// Contracts may implement this trait to inject code into NEP-141 functions.
///
/// `T` is an optional value for passing state between different lifecycle
/// hooks. This may be useful for charging callers for storage usage, for
/// example.
pub trait Nep141Hook<T: Default = ()> {
    /// Executed before a token transfer is conducted
    ///
    /// May return an optional state value which will be passed along to the
    /// following `after_transfer`.
    fn before_transfer(&mut self, _transfer: &Nep141Transfer) -> T {
        Default::default()
    }

    /// Executed after a token transfer is conducted
    ///
    /// Receives the state value returned by `before_transfer`.
    fn after_transfer(&mut self, _transfer: &Nep141Transfer, _state: T) {}
}

/// Transfer metadata generic over both types of transfer (`ft_transfer` and
/// `ft_transfer_call`).
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, PartialEq, Eq, Clone, Debug)]
pub struct Nep141Transfer {
    /// Sender's account ID
    pub sender_id: AccountId,
    /// Receiver's account ID
    pub receiver_id: AccountId,
    /// Transferred amount
    pub amount: u128,
    /// Optional memo string
    pub memo: Option<String>,
    /// Message passed to contract located at `receiver_id`
    pub msg: Option<String>,
}

impl Nep141Transfer {
    /// Returns `true` if this transfer comes from a `ft_transfer_call`
    /// call, `false` otherwise
    pub fn is_transfer_call(&self) -> bool {
        self.msg.is_some()
    }
}

/// Non-public implementations of functions for managing a fungible token.
pub trait Nep141Controller {
    /// Root storage slot
    fn root() -> Slot<()> {
        Slot::new(DefaultStorageKey::Nep141)
    }

    /// Slot for account data
    fn slot_account(account_id: &AccountId) -> Slot<u128> {
        Self::root().field(StorageKey::Account(account_id.clone()))
    }

    /// Slot for storing total supply
    fn slot_total_supply() -> Slot<u128> {
        Self::root().field(StorageKey::TotalSupply)
    }

    /// Get the balance of an account. Returns 0 if the account does not exist.
    fn balance_of(account_id: &AccountId) -> u128 {
        Self::slot_account(account_id).read().unwrap_or(0)
    }

    /// Get the total circulating supply of the token.
    fn total_supply() -> u128 {
        Self::slot_total_supply().read().unwrap_or(0)
    }

    /// Removes tokens from an account and decreases total supply. No event
    /// emission.
    ///
    /// # Panics
    ///
    /// Panics if the current balance of `account_id` is less than `amount` or
    /// if `total_supply` is less than `amount`.
    fn withdraw_unchecked(&mut self, account_id: &AccountId, amount: u128) {
        if amount != 0 {
            let balance = Self::balance_of(account_id);
            if let Some(balance) = balance.checked_sub(amount) {
                Self::slot_account(account_id).write(&balance);
            } else {
                env::panic_str("Balance underflow");
            }

            let total_supply = Self::total_supply();
            if let Some(total_supply) = total_supply.checked_sub(amount) {
                Self::slot_total_supply().write(&total_supply);
            } else {
                env::panic_str("Total supply underflow");
            }
        }
    }

    /// Increases the token balance of an account. Updates total supply. No
    /// event emission,
    ///
    /// # Panics
    ///
    /// Panics if the balance of `account_id` plus `amount` >= `u128::MAX`, or
    /// if the total supply plus `amount` >= `u128::MAX`.
    fn deposit_unchecked(&mut self, account_id: &AccountId, amount: u128) {
        if amount != 0 {
            let balance = Self::balance_of(account_id);
            if let Some(balance) = balance.checked_add(amount) {
                Self::slot_account(account_id).write(&balance);
            } else {
                env::panic_str("Balance overflow");
            }

            let total_supply = Self::total_supply();
            if let Some(total_supply) = total_supply.checked_add(amount) {
                Self::slot_total_supply().write(&total_supply);
            } else {
                env::panic_str("Total supply overflow");
            }
        }
    }

    /// Decreases the balance of `sender_account_id` by `amount` and increases
    /// the balance of `receiver_account_id` by the same. No change to total
    /// supply. No event emission.
    ///
    /// # Panics
    ///
    /// Panics if the balance of `sender_account_id` < `amount` or if the
    /// balance of `receiver_account_id` plus `amount` >= `u128::MAX`.
    fn transfer_unchecked(
        &mut self,
        sender_account_id: &AccountId,
        receiver_account_id: &AccountId,
        amount: u128,
    ) {
        let sender_balance = Self::balance_of(sender_account_id);

        if let Some(sender_balance) = sender_balance.checked_sub(amount) {
            let receiver_balance = Self::balance_of(receiver_account_id);
            if let Some(receiver_balance) = receiver_balance.checked_add(amount) {
                Self::slot_account(sender_account_id).write(&sender_balance);
                Self::slot_account(receiver_account_id).write(&receiver_balance);
            } else {
                env::panic_str("Receiver balance overflow");
            }
        } else {
            env::panic_str("Sender balance underflow");
        }
    }

    /// Performs an NEP-141 token transfer, with event emission.
    ///
    /// # Panics
    ///
    /// See: `Nep141Controller::transfer_unchecked`
    fn transfer(
        &mut self,
        sender_account_id: AccountId,
        receiver_account_id: AccountId,
        amount: u128,
        memo: Option<String>,
    ) {
        self.transfer_unchecked(&sender_account_id, &receiver_account_id, amount);

        Nep141Event::FtTransfer(vec![event::FtTransferData {
            old_owner_id: sender_account_id,
            new_owner_id: receiver_account_id,
            amount: amount.into(),
            memo,
        }])
        .emit();
    }

    /// Performs an NEP-141 token mint, with event emission.
    ///
    /// # Panics
    ///
    /// See: `Nep141Controller::deposit_unchecked`
    fn mint(&mut self, account_id: AccountId, amount: u128, memo: Option<String>) {
        self.deposit_unchecked(&account_id, amount);

        Nep141Event::FtMint(vec![event::FtMintData {
            owner_id: account_id,
            amount: amount.into(),
            memo,
        }])
        .emit();
    }

    /// Performs an NEP-141 token burn, with event emission.
    ///
    /// # Panics
    ///
    /// See: `Nep141Controller::withdraw_unchecked`
    fn burn(&mut self, account_id: AccountId, amount: u128, memo: Option<String>) {
        self.withdraw_unchecked(&account_id, amount);

        Nep141Event::FtBurn(vec![event::FtBurnData {
            owner_id: account_id,
            amount: amount.into(),
            memo,
        }])
        .emit();
    }

    /// Performs an NEP-141 token transfer call, with event emission.
    ///
    /// # Panics
    ///
    /// Panics if `gas_allowance` < `GAS_FOR_FT_TRANSFER_CALL`.
    ///
    /// See also: `Nep141Controller::transfer`
    fn transfer_call(
        &mut self,
        sender_account_id: AccountId,
        receiver_account_id: AccountId,
        amount: u128,
        memo: Option<String>,
        msg: String,
        gas_allowance: Gas,
    ) -> Promise {
        require!(
            gas_allowance >= GAS_FOR_FT_TRANSFER_CALL,
            MORE_GAS_FAIL_MESSAGE,
        );

        self.transfer(
            sender_account_id.clone(),
            receiver_account_id.clone(),
            amount,
            memo,
        );

        let receiver_gas = gas_allowance
            .0
            .checked_sub(GAS_FOR_FT_TRANSFER_CALL.0) // TODO: Double-check this math. Should this be GAS_FOR_RESOLVE_TRANSFER? If not, this checked_sub call is superfluous given the require!() at the top of this function.
            .unwrap_or_else(|| env::panic_str("Prepaid gas overflow"));

        // Initiating receiver's call and the callback
        ext_nep141_receiver::ext(receiver_account_id.clone())
            .with_static_gas(receiver_gas.into())
            .ft_on_transfer(sender_account_id.clone(), amount.into(), msg)
            .then(
                ext_nep141_resolver::ext(env::current_account_id())
                    .with_static_gas(GAS_FOR_RESOLVE_TRANSFER)
                    .ft_resolve_transfer(sender_account_id, receiver_account_id, amount.into()),
            )
    }

    /// Resolves an NEP-141 `ft_transfer_call` promise chain.
    fn resolve_transfer(
        &mut self,
        sender_id: AccountId,
        receiver_id: AccountId,
        amount: u128,
    ) -> u128 {
        let ft_on_transfer_promise_result = env::promise_result(0);

        let unused_amount = match ft_on_transfer_promise_result {
            PromiseResult::NotReady => env::abort(),
            PromiseResult::Successful(value) => {
                if let Ok(U128(unused_amount)) = serde_json::from_slice::<U128>(&value) {
                    std::cmp::min(amount, unused_amount)
                } else {
                    amount
                }
            }
            PromiseResult::Failed => amount,
        };

        let refunded_amount = if unused_amount > 0 {
            let receiver_balance = Self::balance_of(&receiver_id);
            if receiver_balance > 0 {
                let refund_amount = std::cmp::min(receiver_balance, unused_amount);
                self.transfer(receiver_id, sender_id, refund_amount, None);
                refund_amount
            } else {
                0
            }
        } else {
            0
        };

        // Used amount
        amount - refunded_amount
    }
}

/// A contract that may be the recipient of an `ft_transfer_call` function
/// call.
#[ext_contract(ext_nep141_receiver)]
pub trait Nep141Receiver {
    /// Function that is called in an `ft_transfer_call` promise chain.
    /// Returns the number of tokens "used", that is, those that will be kept
    /// in the receiving contract's account. (The contract will attempt to
    /// refund the difference from `amount` to the original sender.)
    fn ft_on_transfer(
        &mut self,
        sender_id: AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128>;
}

/// Fungible token contract callback after `ft_transfer_call` execution.
#[ext_contract(ext_nep141_resolver)]
pub trait Nep141Resolver {
    /// Callback, last in `ft_transfer_call` promise chain. Returns the amount
    /// of tokens refunded to the original sender.
    fn ft_resolve_transfer(
        &mut self,
        sender_id: AccountId,
        receiver_id: AccountId,
        amount: U128,
    ) -> U128;
}

/// Externally-accessible NEP-141-compatible fungible token interface.
#[ext_contract(ext_nep141)]
pub trait Nep141 {
    /// Performs a token transfer
    fn ft_transfer(&mut self, receiver_id: AccountId, amount: U128, memo: Option<String>);

    /// Performs a token transfer, then initiates a promise chain that calls
    /// `ft_on_transfer` on the receiving account, followed by
    /// `ft_resolve_transfer` on the original token contract (this contract).
    fn ft_transfer_call(
        &mut self,
        receiver_id: AccountId,
        amount: U128,
        memo: Option<String>,
        msg: String,
    ) -> Promise;

    /// Returns the current total amount of tokens tracked by the contract
    fn ft_total_supply(&self) -> U128;

    /// Returns the amount of tokens controlled by `account_id`
    fn ft_balance_of(&self, account_id: AccountId) -> U128;
}
