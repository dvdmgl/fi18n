/*!

builders to simplify the configuration and generation of [`FluentMachine`](crate::FluentMachine)

*/

mod inheritance;
mod machine_build;

pub use inheritance::{FluentMachineInheritanceBuilder, InheritanceSyntaxErrorHandling};
pub use machine_build::FluentMachineBuilder;
