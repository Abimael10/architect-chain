//! Configuration management
//!
//! This module handles basic configuration settings for the blockchain node,
//! including network addresses and mining settings.
//!
//! Simplified to focus on essential blockchain configuration only.

pub mod settings;

pub use settings::{Config, GLOBAL_CONFIG};
