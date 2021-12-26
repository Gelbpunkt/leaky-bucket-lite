//! Sync, thread-safe implementation of leaky-bucket for minimal applications.
//!
//! Requires enabling the `sync-threadsafe` feature.
//!
//! ## Example
//!
//! ```no_run
//! use leaky_bucket_lite::sync_threadsafe::LeakyBucket;
//! use std::{error::Error, time::Duration};
//!
//! fn main() -> Result<(), Box<dyn Error>> {
//!     let rate_limiter = LeakyBucket::builder()
//!         .max(5.0)
//!         .tokens(0.0)
//!         .refill_interval(Duration::from_secs(1))
//!         .refill_amount(1.0)
//!         .build();
//!
//!     println!("Waiting for permit...");
//!     // should take about 5 seconds to acquire.
//!     rate_limiter.acquire(5.0);
//!     println!("I made it!");
//!     Ok(())
//! }
//! ```
#[cfg(feature = "parking_lot")]
use parking_lot::{Mutex, MutexGuard};
#[cfg(not(feature = "parking_lot"))]
use std::sync::{Mutex, MutexGuard};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

/// The mutable internals of the leaky bucket.
#[derive(Debug)]
struct LeakyBucketInner {
    tokens: f64,
    last_refill: Instant,
}

/// The leaky bucket.
#[derive(Clone, Debug)]
pub struct LeakyBucket {
    max: f64,
    refill_interval: Duration,
    refill_amount: f64,
    inner: Arc<Mutex<LeakyBucketInner>>,
}

impl LeakyBucket {
    fn new(max: f64, tokens: f64, refill_interval: Duration, refill_amount: f64) -> Self {
        Self {
            max,
            refill_interval,
            refill_amount,
            inner: Arc::new(Mutex::new(LeakyBucketInner {
                tokens,
                last_refill: Instant::now(),
            })),
        }
    }

    #[inline]
    fn update_tokens(&self, inner: &mut MutexGuard<'_, LeakyBucketInner>) {
        let time_passed = Instant::now() - inner.last_refill;
        let refills_since =
            (time_passed.as_secs_f64() / self.refill_interval.as_secs_f64()).floor();
        inner.tokens += self.refill_amount * refills_since;
        inner.last_refill += self.refill_interval.mul_f64(refills_since);

        if inner.tokens > self.max {
            inner.tokens = self.max;
        }
    }

    /// Construct a new leaky bucket through a builder.
    #[must_use]
    pub const fn builder() -> Builder {
        Builder::new()
    }

    /// Get the max number of tokens this rate limiter is configured for.
    #[must_use]
    pub const fn max(&self) -> f64 {
        self.max
    }

    /// Get the current number of tokens available.
    #[must_use]
    pub fn tokens(&self) -> f64 {
        #[cfg(not(feature = "parking_lot"))]
        let mut inner = self.inner.lock().expect("Mutex poisoned");
        #[cfg(feature = "parking_lot")]
        let mut inner = self.inner.lock();
        self.update_tokens(&mut inner);
        inner.tokens
    }

    /// Acquire a single token.
    ///
    /// This is identical to [`acquire`] with an argument of `1.0`.
    ///
    /// [`acquire`]: LeakyBucket::acquire
    ///
    /// # Example
    ///
    /// ```rust
    /// use leaky_bucket_lite::sync_threadsafe::LeakyBucket;
    /// use std::{error::Error, time::Duration};
    ///
    /// fn main() -> Result<(), Box<dyn Error>> {
    ///     let rate_limiter = LeakyBucket::builder()
    ///         .max(5.0)
    ///         .tokens(0.0)
    ///         .refill_interval(Duration::from_secs(5))
    ///         .refill_amount(1.0)
    ///         .build();
    ///
    ///     println!("Waiting for permit...");
    ///     // should take about 5 seconds to acquire.
    ///     rate_limiter.acquire_one();
    ///     println!("I made it!");
    ///
    ///     Ok(())
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an `Error` when communicating with the actor fails.
    #[inline]
    pub fn acquire_one(&self) {
        self.acquire(1.0);
    }

    /// Acquire the given `amount` of tokens.
    ///
    /// # Example
    ///
    /// ```rust
    /// use leaky_bucket_lite::sync_threadsafe::LeakyBucket;
    /// use std::{error::Error, time::Duration};
    ///
    /// fn main() -> Result<(), Box<dyn Error>> {
    ///     let rate_limiter = LeakyBucket::builder()
    ///         .max(5.0)
    ///         .tokens(0.0)
    ///         .refill_interval(Duration::from_secs(5))
    ///         .refill_amount(1.0)
    ///         .build();
    ///
    ///     println!("Waiting for permit...");
    ///     // should take about 25 seconds to acquire.
    ///     rate_limiter.acquire(5.0);
    ///     println!("I made it!");
    ///
    ///     Ok(())
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an `Error` when communicating with the actor fails.
    pub fn acquire(&self, amount: f64) {
        #[cfg(not(feature = "parking_lot"))]
        let mut inner = self.inner.lock().expect("Mutex poisoned");
        #[cfg(feature = "parking_lot")]
        let mut inner = self.inner.lock();

        self.update_tokens(&mut inner);

        if inner.tokens < amount {
            let tokens_needed = amount - inner.tokens;
            let refills_needed = (tokens_needed / self.refill_amount).ceil();
            let target_time = inner.last_refill + self.refill_interval.mul_f64(refills_needed);

            std::thread::sleep(target_time - Instant::now());

            self.update_tokens(&mut inner);
        }

        inner.tokens -= amount;
    }
}

/// Builder for a leaky bucket.
#[derive(Debug)]
pub struct Builder {
    max: Option<f64>,
    tokens: Option<f64>,
    refill_interval: Option<Duration>,
    refill_amount: Option<f64>,
}

impl Builder {
    /// Create a new builder with all defaults.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            max: None,
            tokens: None,
            refill_interval: None,
            refill_amount: None,
        }
    }

    /// Set the max value for the builder.
    #[must_use]
    pub const fn max(mut self, max: f64) -> Self {
        self.max = Some(max);
        self
    }

    /// The number of tokens that the bucket should start with.
    ///
    /// If set to larger than `max` at build time, will only saturate to max.
    #[must_use]
    pub const fn tokens(mut self, tokens: f64) -> Self {
        self.tokens = Some(tokens);
        self
    }

    /// Set the max value for the builder.
    #[must_use]
    pub const fn refill_interval(mut self, refill_interval: Duration) -> Self {
        self.refill_interval = Some(refill_interval);
        self
    }

    /// Set the refill amount to use.
    #[must_use]
    pub const fn refill_amount(mut self, refill_amount: f64) -> Self {
        self.refill_amount = Some(refill_amount);
        self
    }

    /// Construct a new leaky bucket.
    #[must_use]
    pub fn build(self) -> LeakyBucket {
        const DEFAULT_MAX: f64 = 120.0;
        const DEFAULT_TOKENS: f64 = 0.0;
        const DEFAULT_REFILL_INTERVAL: Duration = Duration::from_secs(1);
        const DEFAULT_REFILL_AMOUNT: f64 = 1.0;

        let max = self.max.unwrap_or(DEFAULT_MAX);
        let tokens = self.tokens.unwrap_or(DEFAULT_TOKENS);
        let refill_interval = self.refill_interval.unwrap_or(DEFAULT_REFILL_INTERVAL);
        let refill_amount = self.refill_amount.unwrap_or(DEFAULT_REFILL_AMOUNT);

        LeakyBucket::new(max, tokens, refill_interval, refill_amount)
    }
}

impl Default for Builder {
    fn default() -> Self {
        Self::new()
    }
}
