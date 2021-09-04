#![deny(missing_docs, clippy::pedantic)]
//! [![Documentation](https://docs.rs/leaky-bucket-lite/badge.svg)](https://docs.rs/leaky-bucket-lite)
//! [![Crates](https://img.shields.io/crates/v/leaky-bucket-lite.svg)](https://crates.io/crates/leaky-bucket-lite)
//! [![Actions Status](https://github.com/Gelbpunkt/leaky-bucket-lite/workflows/Rust/badge.svg)](https://github.com/Gelbpunkt/leaky-bucket-lite/actions)
//!
//! A token-based rate limiter based on the [leaky bucket] algorithm.
//!
//! The implementation is fair: Whoever acquires first will be served first.
//!
//! If the tokens are already available, the acquisition will be instant through
//! a fast path, and the acquired number of tokens will be added to the bucket.
//!
//! If they are not available, it will wait until enough tokens are available.
//!
//! ## Usage
//!
//! Add the following to your `Cargo.toml`:
//!
//! ```toml
//! leaky-bucket-lite = "0.1.0"
//! ```
//!
//! ## Example
//!
//! ```no_run
//! use leaky_bucket_lite::LeakyBucket;
//! use std::{error::Error, time::Duration};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn Error>> {
//!     let rate_limiter = LeakyBucket::builder()
//!         .max(5.0)
//!         .tokens(0.0)
//!         .refill_interval(Duration::from_secs(1))
//!         .refill_amount(1.0)
//!         .build();
//!
//!     println!("Waiting for permit...");
//!     // should take about 5 seconds to acquire.
//!     rate_limiter.acquire(5.0).await?;
//!     println!("I made it!");
//!     Ok(())
//! }
//! ```
//!
//! [leaky bucket]: https://en.wikipedia.org/wiki/Leaky_bucket
#[cfg(feature = "tokio")]
mod tokio;
#[cfg(feature = "tokio")]
pub use crate::tokio::*;
#[cfg(feature = "sync")]
pub mod sync;
