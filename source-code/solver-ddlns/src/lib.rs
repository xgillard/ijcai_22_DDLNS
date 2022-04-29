//! Lib.rs is the place where I declare the modules exposed by the library.
//! main.rs then imports the lib and uses it to create a plain old executable.
//!
//! Splitting it in lib + main vs putting all modules in the main is nothing
//! but a matter of taste. I prefer to have a clear separation but this is not
//! mandatory.

mod basics;
mod lns;
mod simple_mdd;
mod puredp;
mod utils;

pub use basics::*;
pub use lns::*;
pub use simple_mdd::*;
pub use puredp::*;
pub use utils::*;
