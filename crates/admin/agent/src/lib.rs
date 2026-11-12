//! Fail-closed admin agent: pinned policy, verification, durable stores and outbound channel.
pub mod channel;
pub mod config;
pub mod policy;
pub mod store;
pub mod verifier;

#[cfg(test)]
mod tests;
