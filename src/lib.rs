
mod linear;

pub mod util;
pub mod v2;

pub use v2::*;
pub use linear::{CoordinateRange, Scale};

/// Re-exported from [`svg`] for convenience
pub use svg::{Document, node::{element::{Group, self}, Node, self, Value}};