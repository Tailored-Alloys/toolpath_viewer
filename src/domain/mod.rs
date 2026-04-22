//! Domain Layer - Core business entities and logic
//!
//! This layer contains:
//! - Entities: Core data structures (Vector, Layer, Slice, etc.)
//! - Value Objects: Immutable domain primitives
//! - Domain Services: Pure business logic
//!
//! This layer has NO dependencies on external frameworks.

pub mod entities;
pub mod value_objects;
pub mod services;

