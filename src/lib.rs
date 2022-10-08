#![doc = include_str!("../README.md")]

pub mod builders;
pub mod error;
pub mod fkey;
pub mod loaders;
pub mod machine;

#[cfg(feature = "actix-web4")]
mod actix;

// re exports
pub use fluent_bundle::{FluentArgs, FluentError, FluentResource, FluentValue};
pub use fluent_langneg::NegotiationStrategy;
pub use unic_langid::LanguageIdentifier;

pub use error::Error;
pub use fkey::Fkey;
pub use machine::{FluentMachine, FluentMachineLoader, MachineBundle};

/// A helper macro to simplify creation of FluentArgs.
///
/// # Example
///
/// ```
/// use fi18n::f_args;
///
/// let mut args = f_args![
///     "name" => "John",
///     "emailCount" => 5,
/// ];
#[macro_export]
macro_rules! f_args {
    ( $($key:expr => $value:expr),* $(,)? ) => {
        {
            let mut args: $crate::FluentArgs = $crate::FluentArgs::new();
            $(
                args.set($key, $value);
            )*
            args
        }
    };
}
