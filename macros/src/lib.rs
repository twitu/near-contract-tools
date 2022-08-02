use darling::FromDeriveInput;
use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

mod event;
mod integration;
mod migrate;
mod owner;
mod pause;
mod rbac;
mod rename;

fn make_derive<T>(
    input: TokenStream,
    expand: fn(T) -> Result<TokenStream, darling::Error>,
) -> TokenStream
where
    T: FromDeriveInput,
{
    let input = parse_macro_input!(input as DeriveInput);

    FromDeriveInput::from_derive_input(&input)
        .and_then(expand)
        .unwrap_or_else(|e| e.write_errors().into())
}

/// Derives an NEP-297-compatible event emitting implementation of `Event`.
///
/// Specify event standard parameters: `#[event(standard = "...", version = "...")]`
///
/// Rename strategy for all variants (default: unchanged): `#[event(..., rename_all = "<strategy>")]`
/// Options for `<strategy>`:
/// - `UpperCamelCase`
/// - `lowerCamelCase`
/// - `snake_case`
/// - `kebab-case`
/// - `SHOUTY_SNAKE_CASE`
/// - `SHOUTY-KEBAB-CASE`
/// - `Title Case`
#[proc_macro_derive(Event, attributes(event))]
pub fn derive_event(input: TokenStream) -> TokenStream {
    make_derive(input, event::expand)
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
/// The storage key prefix for the fields can be optionally specified (default:
/// `"~r"`) using `#[rbac(storage_key = "<expression>")]`.
#[proc_macro_derive(Rbac, attributes(rbac))]
pub fn derive_rbac(input: TokenStream) -> TokenStream {
    make_derive(input, rbac::expand)
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
