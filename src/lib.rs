#![warn(missing_docs)]
//! NTE Patcher SDK
//!
//! A high-performance, concurrent downloading and patching SDK for NTE game assets.

pub mod cas;
pub mod config;
pub mod crypto;
pub mod download;
pub mod error;
pub mod manager;
pub mod mmap;
pub mod model;
pub mod parser;
pub mod retry;
pub mod unzip;
pub mod verify;
