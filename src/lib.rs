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
// Document required features when the `docsrs` cfg set
#![cfg_attr(docsrs, feature(doc_auto_cfg))]

#[cfg(feature = "tokio")]
mod tokio;

#[cfg(feature = "tokio")]
pub use crate::tokio::*;
#[cfg(feature = "sync")]
pub mod sync;
#[cfg(feature = "sync-threadsafe")]
pub mod sync_threadsafe;

use std::time::Instant;

/// Error returned from the `try_acquire` functions when the operation can't be completed immediately.
#[derive(Debug)]
pub struct TryAcquireError {
    kind: TryAcquireErrorKind,
}

#[derive(Debug)]
enum TryAcquireErrorKind {
    /// The lock can't be acquired without waiting.
    Locked,
    /// Not enough tokens are available.
    InsufficientTokens(Instant),
}

impl std::error::Error for TryAcquireError {}

impl std::fmt::Display for TryAcquireError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.kind {
            TryAcquireErrorKind::Locked => write!(f, "operation would block"),
            TryAcquireErrorKind::InsufficientTokens(_i) => {
                write!(f, "insufficient tokens available to fulfill the request")
            }
        }
    }
}

impl TryAcquireError {
    const fn new_locked() -> Self {
        Self {
            kind: TryAcquireErrorKind::Locked,
        }
    }

    const fn new_insufficient_tokens(target_time: Instant) -> Self {
        Self {
            kind: TryAcquireErrorKind::InsufficientTokens(target_time),
        }
    }

    /// Returns `true` if the call failed because the internal lock couldn't be acquired without waiting.
    #[must_use]
    pub const fn is_locked(&self) -> bool {
        matches!(self.kind, TryAcquireErrorKind::Locked)
    }

    /// Returns `true` if the call failed because there were not enough tokens available without waiting.
    #[must_use]
    pub const fn is_insufficient_tokens(&self) -> bool {
        matches!(self.kind, TryAcquireErrorKind::InsufficientTokens(_))
    }

    /// Return the time at which enough tokens would be available to return without waiting.
    ///
    /// Depending on the cause of this error, this value might be unknown.
    ///
    /// This only applies, if no other token is requested in the meantime.
    #[must_use]
    pub const fn target_time(&self) -> Option<Instant> {
        match self.kind {
            TryAcquireErrorKind::Locked => None,
            TryAcquireErrorKind::InsufficientTokens(target_time) => Some(target_time),
        }
    }
}
