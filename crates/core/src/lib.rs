//! Core shared types, configuration, error handling, and database utilities
//! for the LegalMCP platform.
// Suppress doctests: this crate is named `core`, which shadows Rust's built-in
// `core` crate in rustdoc contexts and causes spurious resolution errors.
#![doc(test(attr(allow(unused_imports))))]

pub mod config;
pub mod db;
pub mod errors;
pub mod models;
