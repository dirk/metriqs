//! Receivers are how metrics come into the agent.
//!
//! There are two paradigms for how agents collect metrics:
//!   - Push
//!   - Pull

pub mod push;
pub mod pull;