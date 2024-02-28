//! Trait for formatting all kind of values into `Screen`

use std::error::Error;

use database::Database;
use gears::types::context::context::Context;
use proto_messages::cosmos::tx::v1beta1::screen::Screen;
use store::StoreKey;

/// Render primitive type into content for `Screen`.
/// Use for formatting simple primitive `Copy` types that doesn't require error handling
pub trait PrimitiveValueRenderer<V> {
    /// Get string representation of some `V`
    fn format(value: V) -> String;

    /// Try format specific value
    fn format_try(value: V) -> Result<String, Box<dyn Error>>;
}

/// The notion of "value renderer" is defined in ADR-050.
pub trait ValueRenderer<SK: StoreKey, DB: Database> {
    /// Format renders the Protobuf value to a list of Screens.
    fn format(&self, ctx: &Context<'_, '_, DB, SK>) -> Result<Vec<Screen>, Box<dyn Error>>;
}

/// Static structure which implement trait for formatting primitive types
/// like `i64` or `bool` and made for using in `gears`
pub struct DefaultPrimitiveRenderer;

/// Static structure which implement trait for formatting messages
/// like `Coin` or `Tx<M : Message>`
pub struct DefaultValueRenderer;