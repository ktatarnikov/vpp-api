
#[cfg(feature = "version_25_10")]
#[path = "../gen/25_10/src/mod.rs"]
pub mod client;

#[cfg(feature = "version_25_10")]
pub use client::*;