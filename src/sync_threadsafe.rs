//! Sync, thread-safe implementation of leaky-bucket for minimal applications.
//!
//! Requires enabling the `sync-threadsafe` feature.
//!
//! ## Example
//!
//! ```no_run
//! use leaky_bucket_lite::sync_threadsafe::LeakyBucket;
//! use std::time::Duration;
//!
//! let rate_limiter = LeakyBucket::builder()
//!     .max(5)
//!     .tokens(0)
//!     .refill_interval(Duration::from_secs(1))
//!     .refill_amount(1)
//!     .build();
//!
//! println!("Waiting for permit...");
//! // should take about 5 seconds to acquire.
//! rate_limiter.acquire(5);
//! println!("I made it!");
//! ```

use crate::TryAcquireError;
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
    tokens: u32,
    last_refill: Instant,
}

/// The leaky bucket.
#[derive(Clone, Debug)]
pub struct LeakyBucket {
    max: u32,
    refill_interval: Duration,
    refill_amount: u32,
    inner: Arc<Mutex<LeakyBucketInner>>,
}

impl LeakyBucket {
    fn new(max: u32, tokens: u32, refill_interval: Duration, refill_amount: u32) -> Self {
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

        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let refills_since =
            (time_passed.as_secs_f64() / self.refill_interval.as_secs_f64()).floor() as u32;

        inner.tokens += self.refill_amount * refills_since;
        inner.last_refill += self.refill_interval * refills_since;

        inner.tokens = inner.tokens.min(self.max);
    }

    /// Construct a new leaky bucket through a builder.
    #[must_use]
    pub const fn builder() -> Builder {
        Builder::new()
    }

    /// Get the max number of tokens this rate limiter is configured for.
    #[must_use]
    pub const fn max(&self) -> u32 {
        self.max
    }

    /// Get the current number of tokens available.
    #[must_use]
    pub fn tokens(&self) -> u32 {
        #[cfg(not(feature = "parking_lot"))]
        let mut inner = self.inner.lock().expect("Mutex poisoned");
        #[cfg(feature = "parking_lot")]
        let mut inner = self.inner.lock();
        self.update_tokens(&mut inner);
        inner.tokens
    }

    /// Get the next time at which the tokens will be refilled.
    #[must_use]
    pub fn next_refill(&self) -> Instant {
        #[cfg(not(feature = "parking_lot"))]
        let mut inner = self.inner.lock().expect("Mutex poisoned");
        #[cfg(feature = "parking_lot")]
        let mut inner = self.inner.lock();
        self.update_tokens(&mut inner);

        inner.last_refill + self.refill_interval
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
    /// use std::time::Duration;
    ///
    /// let rate_limiter = LeakyBucket::builder()
    ///     .max(5)
    ///     .tokens(0)
    ///     .refill_interval(Duration::from_secs(5))
    ///     .refill_amount(1)
    ///     .build();
    ///
    /// println!("Waiting for permit...");
    /// // should take about 5 seconds to acquire.
    /// rate_limiter.acquire_one();
    /// println!("I made it!");
    /// ```
    #[inline]
    pub fn acquire_one(&self) {
        self.acquire(1);
    }

    /// Acquire the given `amount` of tokens.
    ///
    /// # Example
    ///
    /// ```rust
    /// use leaky_bucket_lite::sync_threadsafe::LeakyBucket;
    /// use std::time::Duration;
    ///
    /// let rate_limiter = LeakyBucket::builder()
    ///     .max(5)
    ///     .tokens(0)
    ///     .refill_interval(Duration::from_secs(5))
    ///     .refill_amount(1)
    ///     .build();
    ///
    /// println!("Waiting for permit...");
    /// // should take about 25 seconds to acquire.
    /// rate_limiter.acquire(5);
    /// println!("I made it!");
    /// ```
    ///
    /// # Panics
    ///
    /// This method will panic when acquiring more tokens than the configured maximum.
    pub fn acquire(&self, amount: u32) {
        #[cfg(not(feature = "parking_lot"))]
        let mut inner = self.inner.lock().expect("Mutex poisoned");
        #[cfg(feature = "parking_lot")]
        let mut inner = self.inner.lock();

        if let Err(target_time) = self.try_acquire_locked(amount, &mut inner) {
            std::thread::sleep(target_time - Instant::now());

            self.update_tokens(&mut inner);
            inner.tokens -= amount;
        }
    }

    /// Try to acquire one token.
    ///
    /// This will not wait for the token to become ready.
    ///
    /// # Errors
    ///
    /// If less tokens are currently available, returns a [`TryAcquireError`] error.
    ///
    /// # Example
    ///
    /// ```rust
    /// use leaky_bucket_lite::sync_threadsafe::LeakyBucket;
    /// use std::time::Duration;
    ///
    /// let mut rate_limiter = LeakyBucket::builder()
    ///     .max(5)
    ///     .tokens(1)
    ///     .refill_interval(Duration::from_secs(1))
    ///     .build();
    ///
    /// assert!(matches!(rate_limiter.try_acquire_one(), Ok(())));
    /// assert!(matches!(rate_limiter.try_acquire_one(), Err(_)));
    /// ```
    ///
    /// # Panics
    ///
    /// This method will panic when acquiring more tokens than the configured maximum.
    #[inline]
    pub fn try_acquire_one(&self) -> Result<(), TryAcquireError> {
        self.try_acquire(1)
    }

    /// Try to acquire the given `amount` of tokens.
    ///
    /// This will not wait for the tokens to become ready.
    ///
    /// # Errors
    ///
    /// If less tokens are currently available, returns a [`TryAcquireError`] error.
    ///
    /// # Example
    ///
    /// ```rust
    /// use leaky_bucket_lite::sync_threadsafe::LeakyBucket;
    /// use std::time::Duration;
    ///
    /// let mut rate_limiter = LeakyBucket::builder()
    ///     .max(5)
    ///     .tokens(1)
    ///     .refill_interval(Duration::from_secs(1))
    ///     .build();
    ///
    /// assert!(matches!(rate_limiter.try_acquire(5), Err(_)));
    /// assert!(matches!(rate_limiter.try_acquire(1), Ok(())));
    /// assert!(matches!(rate_limiter.try_acquire(1), Err(_)));
    /// ```
    ///
    /// # Panics
    ///
    /// This method will panic when acquiring more tokens than the configured maximum.
    pub fn try_acquire(&self, amount: u32) -> Result<(), TryAcquireError> {
        #[cfg(not(feature = "parking_lot"))]
        let mut inner = match self.inner.try_lock() {
            Err(std::sync::TryLockError::WouldBlock) => return Err(TryAcquireError::new_locked(a)),
            Err(std::sync::TryLockError::Poisoned(_)) => panic!("Mutex poisoned"),
            Ok(inner) => inner,
        };
        #[cfg(feature = "parking_lot")]
        let mut inner = self.inner.try_lock().ok_or(TryAcquireError::new_locked())?;

        self.try_acquire_locked(amount, &mut inner)
            .map_err(TryAcquireError::new_insufficient_tokens)
    }

    fn try_acquire_locked(
        &self,
        amount: u32,
        inner: &mut MutexGuard<'_, LeakyBucketInner>,
    ) -> Result<(), Instant> {
        assert!(
            amount <= self.max(),
            "Acquiring more tokens than the configured maximum is not possible"
        );

        self.update_tokens(inner);

        if inner.tokens < amount {
            let tokens_needed = amount - inner.tokens;
            let mut refills_needed = tokens_needed / self.refill_amount;
            let refills_needed_remainder = tokens_needed % self.refill_amount;

            if refills_needed_remainder > 0 {
                refills_needed += 1;
            }

            let target_time = inner.last_refill + self.refill_interval * refills_needed;

            Err(target_time)
        } else {
            inner.tokens -= amount;
            Ok(())
        }
    }
}

/// Builder for a leaky bucket.
#[derive(Debug)]
pub struct Builder {
    max: Option<u32>,
    tokens: Option<u32>,
    refill_interval: Option<Duration>,
    refill_amount: Option<u32>,
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
    pub const fn max(mut self, max: u32) -> Self {
        self.max = Some(max);
        self
    }

    /// The number of tokens that the bucket should start with.
    ///
    /// If set to larger than `max` at build time, will only saturate to max.
    #[must_use]
    pub const fn tokens(mut self, tokens: u32) -> Self {
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
    pub const fn refill_amount(mut self, refill_amount: u32) -> Self {
        self.refill_amount = Some(refill_amount);
        self
    }

    /// Construct a new leaky bucket.
    #[must_use]
    pub fn build(self) -> LeakyBucket {
        const DEFAULT_MAX: u32 = 120;
        const DEFAULT_TOKENS: u32 = 0;
        const DEFAULT_REFILL_INTERVAL: Duration = Duration::from_secs(1);
        const DEFAULT_REFILL_AMOUNT: u32 = 1;

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
