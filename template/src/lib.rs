/*!
A different string template parser.

The shape of templates accepted by this crate are different from `std::fmt`.
It doesn't have any direct understanding of formatting flags.
Instead, it just parses field expressions between braces in a string and
leaves it up to a consumer to decide what to do with them.
*/

#[macro_use]
extern crate quote;

pub mod ct;
mod rt;

#[doc(hidden)]
pub mod __private {
    pub use super::rt::{build, Part, Template};
}

pub use self::rt::{Context, Template};
