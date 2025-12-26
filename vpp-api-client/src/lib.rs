#[cfg(feature = "version_25_10")]
#[path = "../gen/25.10/src/mod.rs"]
pub mod client;

#[cfg(feature = "version_25_10")]
pub use client::*;

#[cfg(feature = "version_25_06")]
#[path = "../gen/25.06/src/mod.rs"]
pub mod client;

#[cfg(feature = "version_25_06")]
pub use client::*;
