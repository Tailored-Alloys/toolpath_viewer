//! Value Objects
//!
//! Immutable domain primitives that represent values without identity.

mod point;
mod color;
mod bounds;

pub use point::*;
pub use color::*;
pub use bounds::*;
