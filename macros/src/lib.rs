//! Macros for near-sdk-contract-tools

use darling::{FromDeriveInput, FromMeta};
use proc_macro::TokenStream;
use syn::{parse_macro_input, AttributeArgs, DeriveInput, Item};

mod approval;
mod migrate;
mod owner;
mod pause;
mod rbac;
mod rename;
mod standard;
mod upgrade;

fn default_crate_name() -> syn::Path {
    syn::parse_str("::near_sdk_contract_tools").unwrap()
}

fn default_macros() -> syn::Path {
    syn::parse_str("::near_sdk_contract_tools").unwrap()
}

fn default_near_sdk() -> syn::Path {
    syn::parse_str("::near_sdk").unwrap()
}

fn default_serde() -> syn::Path {
    syn::parse_str("::serde").unwrap()
}

fn make_derive<T>(
    input: TokenStream,
    expand: fn(T) -> Result<proc_macro2::TokenStream, darling::Error>,
) -> TokenStream
where
    T: FromDeriveInput,
{
    let input = parse_macro_input!(input as DeriveInput);

    FromDeriveInput::from_derive_input(&input)
        .and_then(expand)
        .map(Into::into)
        .unwrap_or_else(|e| e.write_errors().into())
}

/// Use on a struct to emit NEP-297 event strings.
///
/// Specify event standard parameters: `#[nep297(standard = "...", version = "...")]`
///
/// Optional: `#[nep297(name = "...")]`
///
/// Rename strategy for all variants (default: unchanged): `#[event(rename = "<strategy>")]`
/// Options for `<strategy>`:
/// - `UpperCamelCase`
/// - `lowerCamelCase`
/// - `snake_case`
/// - `kebab-case`
/// - `SHOUTY_SNAKE_CASE`
/// - `SHOUTY-KEBAB-CASE`
/// - `Title Case`
#[proc_macro_derive(Nep297, attributes(nep297))]
pub fn derive_nep297(input: TokenStream) -> TokenStream {
    make_derive(input, standard::nep297::expand)
}

/// Creates a managed, lazily-loaded `Owner` implementation for the targeted
/// `#[near_bindgen]` struct.
///
/// The storage key prefix for the fields can be optionally specified (default:
/// `"~o"`) using `#[owner(storage_key = "<expression>")]`.
#[proc_macro_derive(Owner, attributes(owner))]
pub fn derive_owner(input: TokenStream) -> TokenStream {
    make_derive(input, owner::expand)
}

/// Makes a contract pausable. Provides an implementation of the `Pause` trait.
///
/// The storage key prefix for the fields can be optionally specified (default:
/// `"~p"`) using `#[pause(storage_key = "<expression>")]`.
#[proc_macro_derive(Pause, attributes(pause))]
pub fn derive_pause(input: TokenStream) -> TokenStream {
    make_derive(input, pause::expand)
}

/// Adds role-based access control. No external methods are exposed.
///
/// The roles prefix must be specify a type using #[rbac(roles = "MyRoles")].
/// Typically "MyRoles" is an enum and it's variants are the different role
/// names.
///
/// The storage key prefix for the fields can be optionally specified (default:
/// `"~r"`) using `#[rbac(storage_key = "<expression>")]`.
#[proc_macro_derive(Rbac, attributes(rbac))]
pub fn derive_rbac(input: TokenStream) -> TokenStream {
    make_derive(input, rbac::expand)
}

/// Adds NEP-141 fungible token core functionality to a contract. Exposes
/// `ft_*` functions to the public blockchain, implements internal controller
/// and receiver functionality (see: `near_sdk_contract_tools::standard::nep141`).
///
/// The storage key prefix for the fields can be optionally specified (default:
/// `"~$141"`) using `#[nep141(storage_key = "<expression>")]`.
#[proc_macro_derive(Nep141, attributes(nep141))]
pub fn derive_nep141(input: TokenStream) -> TokenStream {
    make_derive(input, standard::nep141::expand)
}

/// Adds NEP-148 fungible token metadata functionality to a contract. Metadata
/// is hardcoded into the contract code, and is therefore not stored in storage.
///
/// Specify metadata using the `#[nep148(...)]` attribute.
///
/// Fields:
///  - `name`
///  - `symbol`
///  - `decimals`
///  - `spec` (optional)
///  - `icon` (optional)
///  - `reference` (optional)
///  - `reference_hash` (optional)
#[proc_macro_derive(Nep148, attributes(nep148))]
pub fn derive_nep148(input: TokenStream) -> TokenStream {
    make_derive(input, standard::nep148::expand)
}

/// Implements NEP-141 and NEP-148 functionality, like
/// `#[derive(Nep141, Nep148)]`.
///
/// Attributes are the union of those for the constituent derive macros.
/// Specify attributes with `#[fungible_token(...)]`.
#[proc_macro_derive(FungibleToken, attributes(fungible_token))]
pub fn derive_fungible_token(input: TokenStream) -> TokenStream {
    make_derive(input, standard::fungible_token::expand)
}

/// Migrate a contract's default struct from one schema to another.
///
/// Fields may be specified in the `#[migrate(...)]` attribute.
///
/// Fields include:
///  - `from` Old default struct type to convert from. (required)
///  - `to` New default struct type to convert into. (optional, default: `Self`)
///  - `convert` Identifier of a function that converts from the old schema to
///     the new schema. Mutually exclusive with `convert_with_args`. (optional,
///     default: `<Self::NewSchema as From<Self::OldSchema>>::from`)
///  - `convert_with_args` Identifier of a function that converts from the old
///     schema to the new schema and accepts a single `String` argument.
///     Mutually exclusive with `convert`. (optional)
///  - `allow` Expression to evaluate before allowing
#[proc_macro_derive(Migrate, attributes(migrate))]
pub fn derive_migrate(input: TokenStream) -> TokenStream {
    make_derive(input, migrate::expand)
}

/// Create a simple multisig component. Does not expose any functions to the
/// blockchain. Creates implementations for `ApprovalManager` and
/// `AccountApprover` for the target contract struct.
///
/// Fields may be specified in the `#[simple_multisig(...)]` attribute.
///
/// Fields include:
///  - `storage_key` Storage prefix for multisig data (optional, default: `b"~sm"`)
///  - `action` What sort of approval `Action` can be approved by the multisig
///     component?
///  - `role` Approving accounts are required to have this `Rbac` role.
#[proc_macro_derive(SimpleMultisig, attributes(simple_multisig))]
pub fn derive_simple_multisig(input: TokenStream) -> TokenStream {
    make_derive(input, approval::simple_multisig::expand)
}

/// Smart `#[event]` macro
#[proc_macro_attribute]
pub fn event(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr = parse_macro_input!(attr as AttributeArgs);
    let item = parse_macro_input!(item as Item);

    standard::event::EventAttributeMeta::from_list(&attr)
        .and_then(|meta| standard::event::event_attribute(meta, item))
        .map(Into::into)
        .unwrap_or_else(|e| e.write_errors().into())
}

/// Create an upgrade component. Does not expose any functions to the
/// blockchain.
///
/// Fields may be specified in the `#[upgrade(...)]` attribute.
///
/// Fields include:
///  - `hook` - If included, provides an implementation of `UpgradeHook`. An implementation must be explicity provided otherwise. Options include:
///     - `"none"` - Empty upgrade hook.
///     - `"owner"` - The upgrade function may only be called by the owner of the contract as specified by an `Owner` implementation.
///     - `"role(r)"` - The upgrade function may only be called by an account that has been assigned the role `r` as determined by an `Rbac` implementation.
///  - `serializer` - `"borsh"` or `"jsonbase64"` (default). Indicates the serialization format of code the `upgrade` function will accept.
///  - `migrate_method_name` - The name of the method to call after the upgrade. Default `"migrate"`.
///  - `migrate_method_args` - The input to send to the migrate function. Default empty vector.
///  - `migrate_minimum_gas` - How much gas to guarantee the migrate function, otherwise reject. Default 15T.
#[proc_macro_derive(Upgrade, attributes(upgrade))]
pub fn derive_upgrade(input: TokenStream) -> TokenStream {
    make_derive(input, upgrade::expand)
}
