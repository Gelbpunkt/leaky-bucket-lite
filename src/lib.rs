//! # leaky-bucket-lite
//!
//! [![docs badge][]][docs link]
//! [![crates badge][]][crates link]
//! [![actions badge][]][actions link]
//!
//! A token-based rate limiter based on the [leaky bucket] algorithm, mainly a lazy reimplementation of [udoprog's leaky-bucket] with less dependencies and overhead.
//!
//! If the tokens are already available, the acquisition will be instant through
//! a fast path, and the acquired number of tokens will be added to the bucket.
//!
//! If they aren't, the task that tried to acquire the tokens will be suspended
//! until the required number of tokens has been added.
//!
//! ## Usage
//!
//! Add the following to your `Cargo.toml`:
//!
//! ```toml
//! leaky-bucket-lite = "0.5"
//! ```
//!
//! ## Features
//!
//! leaky-bucket-lite provides 3 implementations:
//!   * [`LeakyBucket`] (thread-safe, available via the `tokio` feature, which is on by default)
//!   * [`sync_threadsafe::LeakyBucket`] (thread-safe, available via the `sync-threadsafe` feature)
//!   * [`sync::LeakyBucket`] (not thread-safe, available via the `sync` feature).
//!
//! For potential performance increase with `sync-threadsafe` or `tokio` using [`parking_lot`]'s locking objects, enable the `parking_lot` feature.
//!
//! ## Example
//!
//! ```rust
//! use leaky_bucket_lite::LeakyBucket;
//! use std::time::Duration;
//!
//! #[tokio::main]
//! async fn main() {
//!     let rate_limiter = LeakyBucket::builder()
//!         .max(5)
//!         .tokens(0)
//!         .refill_interval(Duration::from_secs(1))
//!         .refill_amount(1)
//!         .build();
//!
//!     println!("Waiting for permit...");
//!     // should take about 5 seconds to acquire.
//!     rate_limiter.acquire(5).await;
//!     println!("I made it!");
//! }
//! ```
//!
//! [actions badge]: https://github.com/Gelbpunkt/leaky-bucket-lite/workflows/Rust/badge.svg
//! [actions link]: https://github.com/Gelbpunkt/leaky-bucket-lite/actions
//! [crates badge]: https://img.shields.io/crates/v/leaky-bucket-lite.svg
//! [crates link]: https://crates.io/crates/leaky-bucket-lite
//! [docs badge]: https://docs.rs/leaky-bucket-lite/badge.svg
//! [docs link]: https://docs.rs/leaky-bucket-lite
//! [leaky bucket]: https://en.wikipedia.org/wiki/Leaky_bucket
//! [udoprog's leaky-bucket]: https://github.com/udoprog/leaky-bucket

#![deny(
    clippy::all,
    clippy::missing_const_for_fn,
    clippy::pedantic,
    future_incompatible,
    missing_docs,
    nonstandard_style,
    rust_2018_idioms,
    rustdoc::broken_intra_doc_links,
    unsafe_code,
    unused,
    warnings
)]

#[cfg(feature = "tokio")]
mod tokio;
#[cfg(feature = "tokio")]
pub use crate::tokio::*;
#[cfg(feature = "sync")]
pub mod sync;
#[cfg(feature = "sync-threadsafe")]
pub mod sync_threadsafe;
