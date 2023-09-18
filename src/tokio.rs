use crate::TryAcquireError;
use std::sync::Arc;
use tokio::{
    sync::{Mutex, MutexGuard},
    time::{sleep_until, Duration, Instant},
};

#[derive(Debug)]
struct LeakyBucketInner {
    /// How many tokens this bucket can hold.
    max: u32,
    /// Interval at which the bucket gains tokens.
    refill_interval: Duration,
    /// Amount of tokens gained per interval.
    refill_amount: u32,

    locked: Mutex<LeakyBucketInnerLocked>,
}

#[derive(Debug)]
struct LeakyBucketInnerLocked {
    /// Current tokens in the bucket.
    tokens: u32,
    /// Last refill of the tokens.
    last_refill: Instant,
}

impl LeakyBucketInner {
    fn new(max: u32, tokens: u32, refill_interval: Duration, refill_amount: u32) -> Self {
        Self {
            max,
            refill_interval,
            refill_amount,
            locked: Mutex::new(LeakyBucketInnerLocked {
                tokens,
                last_refill: Instant::now(),
            }),
        }
    }

    /// Updates the tokens in the leaky bucket and returns the current amount
    /// of tokens in the bucket.
    #[inline]
    fn update_tokens(&self, locked: &mut MutexGuard<'_, LeakyBucketInnerLocked>) -> u32 {
        let time_passed = Instant::now() - locked.last_refill;

        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let refills_since =
            (time_passed.as_secs_f64() / self.refill_interval.as_secs_f64()).floor() as u32;

        locked.tokens += self.refill_amount * refills_since;
        locked.last_refill += self.refill_interval * refills_since;

        locked.tokens = locked.tokens.min(self.max);

        locked.tokens
    }

    #[inline]
    async fn tokens(&self) -> u32 {
        self.update_tokens(&mut self.locked.lock().await)
    }

    async fn next_refill(&self) -> Instant {
        let mut locked = self.locked.lock().await;
        self.update_tokens(&mut locked);

        locked.last_refill + self.refill_interval
    }

    async fn acquire(&self, amount: u32) {
        let mut locked = self.locked.lock().await;
        if let Err(target_time) = self.try_acquire_locked(amount, &mut locked) {
            sleep_until(target_time).await;

            self.update_tokens(&mut locked);
            locked.tokens -= amount;
        }
    }

    fn try_acquire(&self, amount: u32) -> Result<(), TryAcquireError> {
        self.try_acquire_locked(
            amount,
            &mut self
                .locked
                .try_lock()
                .map_err(|_e| TryAcquireError::new_locked())?,
        )
        .map_err(|i| TryAcquireError::new_insufficient_tokens(i.into()))
    }

    fn try_acquire_locked(
        &self,
        amount: u32,
        locked: &mut MutexGuard<'_, LeakyBucketInnerLocked>,
    ) -> Result<(), Instant> {
        assert!(
            amount <= self.max,
            "Acquiring more tokens than the configured maximum is not possible"
        );

        let current_tokens = self.update_tokens(locked);

        if current_tokens < amount {
            let tokens_needed = amount - current_tokens;
            let mut refills_needed = tokens_needed / self.refill_amount;
            let refills_needed_remainder = tokens_needed % self.refill_amount;

            if refills_needed_remainder > 0 {
                refills_needed += 1;
            }

            Err(locked.last_refill + self.refill_interval * refills_needed)
        } else {
            locked.tokens -= amount;
            Ok(())
        }
    }
}

/// The leaky bucket.
#[derive(Clone, Debug)]
pub struct LeakyBucket {
    inner: Arc<LeakyBucketInner>,
}

impl LeakyBucket {
    fn new(max: u32, tokens: u32, refill_interval: Duration, refill_amount: u32) -> Self {
        let inner = Arc::new(LeakyBucketInner::new(
            max,
            tokens,
            refill_interval,
            refill_amount,
        ));

        Self { inner }
    }

    /// Construct a new leaky bucket through a builder.
    #[must_use]
    pub const fn builder() -> Builder {
        Builder::new()
    }

    /// Get the max number of tokens this rate limiter is configured for.
    #[must_use]
    pub fn max(&self) -> u32 {
        self.inner.max
    }

    /// Get the refill_interval this rate limiter is configured for.
    #[must_use]
    pub fn refill_interval(&self) -> Duration {
        self.inner.refill_interval
    }

    /// Get the current number of tokens available.
    pub async fn tokens(&self) -> u32 {
        self.inner.tokens().await
    }

    /// Get the next time at which the tokens will be refilled.
    pub async fn next_refill(&self) -> Instant {
        self.inner.next_refill().await
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
    /// use leaky_bucket_lite::LeakyBucket;
    /// use std::time::Duration;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let rate_limiter = LeakyBucket::builder()
    ///         .max(5)
    ///         .tokens(0)
    ///         .refill_interval(Duration::from_secs(5))
    ///         .refill_amount(1)
    ///         .build();
    ///
    ///     println!("Waiting for permit...");
    ///     // should take about 5 seconds to acquire.
    ///     rate_limiter.acquire_one().await;
    ///     println!("I made it!");
    /// }
    /// ```
    #[inline]
    pub async fn acquire_one(&self) {
        self.acquire(1).await;
    }

    /// Acquire the given `amount` of tokens.
    ///
    /// # Example
    ///
    /// ```rust
    /// use leaky_bucket_lite::LeakyBucket;
    /// use std::time::Duration;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let rate_limiter = LeakyBucket::builder()
    ///         .max(5)
    ///         .tokens(0)
    ///         .refill_interval(Duration::from_secs(5))
    ///         .refill_amount(1)
    ///         .build();
    ///
    ///     println!("Waiting for permit...");
    ///     // should take about 25 seconds to acquire.
    ///     rate_limiter.acquire(5).await;
    ///     println!("I made it!");
    /// }
    /// ```
    ///
    /// # Panics
    ///
    /// This method will panic when acquiring more tokens than the configured maximum.
    #[inline]
    pub async fn acquire(&self, amount: u32) {
        self.inner.acquire(amount).await;
    }

    /// Try to acquire one token, without waiting for the internal lock.
    ///
    /// # Errors
    ///
    /// A [`TryAcquireError`] is returned if the operation can't be completed immediately.
    ///
    /// # Example
    ///
    /// ```rust
    /// use leaky_bucket_lite::LeakyBucket;
    /// use std::time::Duration;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let rate_limiter = LeakyBucket::builder()
    ///         .max(5)
    ///         .tokens(1)
    ///         .refill_interval(Duration::from_secs(1))
    ///         .refill_amount(1)
    ///         .build();
    ///
    ///     assert!(matches!(rate_limiter.try_acquire_one(), Ok(())));
    ///     assert!(matches!(rate_limiter.try_acquire_one(), Err(e) if e.is_insufficient_tokens()));
    ///     tokio::spawn({
    ///         let rate_limiter = rate_limiter.clone();
    ///         async move {
    ///             rate_limiter.acquire(2).await;
    ///         }
    ///     });
    ///     tokio::time::sleep(Duration::from_millis(100)).await;
    ///     assert!(matches!(rate_limiter.try_acquire_one(), Err(e) if e.is_locked()));
    /// }
    /// ```
    ///
    /// # Panics
    ///
    /// This method will panic when acquiring more tokens than the configured maximum.
    #[inline]
    pub fn try_acquire_one(&self) -> Result<(), TryAcquireError> {
        self.try_acquire(1)
    }

    /// Try to acquire the given `amount` of tokens, without waiting for the internal lock.
    ///
    /// # Errors
    ///
    /// A [`TryAcquireError`] is returned if the operation can't be completed immediately.
    ///
    /// # Example
    ///
    /// ```rust
    /// use leaky_bucket_lite::LeakyBucket;
    /// use std::time::Duration;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let rate_limiter = LeakyBucket::builder()
    ///         .max(5)
    ///         .tokens(1)
    ///         .refill_interval(Duration::from_secs(1))
    ///         .refill_amount(1)
    ///         .build();
    ///
    ///     assert!(matches!(rate_limiter.try_acquire(1), Ok(())));
    ///     assert!(matches!(rate_limiter.try_acquire(1), Err(e) if e.is_insufficient_tokens()));
    ///     tokio::spawn({
    ///         let rate_limiter = rate_limiter.clone();
    ///         async move {
    ///             rate_limiter.acquire(2).await;
    ///         }
    ///     });
    ///     tokio::time::sleep(Duration::from_millis(100)).await;
    ///     assert!(matches!(rate_limiter.try_acquire(1), Err(e) if e.is_locked()));
    /// }
    /// ```
    ///
    /// # Panics
    ///
    /// This method will panic when acquiring more tokens than the configured maximum.
    #[inline]
    pub fn try_acquire(&self, amount: u32) -> Result<(), TryAcquireError> {
        self.inner.try_acquire(amount)
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
