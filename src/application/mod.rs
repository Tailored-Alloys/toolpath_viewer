//! Application Layer - Use Cases and Application Services
//!
//! This layer contains:
//! - Ports: Interfaces (traits) for external dependencies
//! - Use Cases: Application-specific business rules
//! - DTOs: Data transfer objects for crossing boundaries
//!
//! This layer depends only on the Domain layer.

pub mod ports;
pub mod use_cases;
pub mod dto;
