#[cfg(feature = "parking-lot")]
use parking_lot::RwLock;
use std::sync::Arc;
#[cfg(not(feature = "parking-lot"))]
use std::sync::RwLock;
use tokio::{
    sync::Semaphore,
    time::{sleep_until, Duration, Instant},
};

#[derive(Debug)]
struct LeakyBucketInner {
    /// How many tokens this bucket can hold.
    max: f64,
    /// Interval at which the bucket gains tokens.
    refill_interval: Duration,
    /// Amount of tokens gained per interval.
    refill_amount: f64,

    /// Current tokens in the bucket.
    tokens: RwLock<f64>,
    /// Last refill of the tokens.
    last_refill: RwLock<Instant>,

    /// To prevent more than one task from acquiring at the same time,
    /// a Semaphore is needed to guard access.
    semaphore: Semaphore,
}

impl LeakyBucketInner {
    fn new(max: f64, tokens: f64, refill_interval: Duration, refill_amount: f64) -> Self {
        Self {
            tokens: RwLock::new(tokens),
            max,
            refill_interval,
            refill_amount,
            last_refill: RwLock::new(Instant::now()),
            semaphore: Semaphore::new(1),
        }
    }

    /// Updates the tokens in the leaky bucket and returns the current amount
    /// of tokens in the bucket.
    #[inline]
    fn update_tokens(&self) -> f64 {
        #[cfg(feature = "parking-lot")]
        let mut last_refill = self.last_refill.write();
        #[cfg(not(feature = "parking-lot"))]
        let mut last_refill = self.last_refill.write().expect("RwLock poisoned");
        #[cfg(feature = "parking-lot")]
        let mut tokens = self.tokens.write();
        #[cfg(not(feature = "parking-lot"))]
        let mut tokens = self.tokens.write().expect("RwLock poisoned");

        let time_passed = Instant::now() - *last_refill;
        let refills_since =
            (time_passed.as_secs_f64() / self.refill_interval.as_secs_f64()).floor();
        *tokens += self.refill_amount * refills_since;
        *last_refill += self.refill_interval.mul_f64(refills_since);

        if *tokens > self.max {
            *tokens = self.max;
        }

        *tokens
    }

    #[inline]
    fn tokens(&self) -> f64 {
        self.update_tokens()
    }

    async fn acquire(&self, amount: f64) {
        // Make sure this is the only task accessing the tokens in a real
        // "write" rather than "update" way.
        let _permit = self.semaphore.acquire().await;

        let current_tokens = self.update_tokens();

        if current_tokens < amount {
            let tokens_needed = amount - current_tokens;
            let refills_needed = (tokens_needed / self.refill_amount).ceil();

            let target_time = {
                #[cfg(feature = "parking-lot")]
                let last_refill = self.last_refill.read();
                #[cfg(not(feature = "parking-lot"))]
                let last_refill = self.last_refill.read().expect("RwLock poisoned");

                *last_refill + self.refill_interval.mul_f64(refills_needed)
            };

            sleep_until(target_time).await;

            self.update_tokens();
        }

        #[cfg(feature = "parking-lot")]
        {
            *self.tokens.write() -= amount;
        }
        #[cfg(not(feature = "parking-lot"))]
        {
            *self.tokens.write().expect("RwLock poisoned") -= amount;
        }
    }
}

/// The leaky bucket.
#[derive(Clone, Debug)]
pub struct LeakyBucket {
    inner: Arc<LeakyBucketInner>,
}

impl LeakyBucket {
    fn new(max: f64, tokens: f64, refill_interval: Duration, refill_amount: f64) -> Self {
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
    pub fn builder() -> Builder {
        Builder::new()
    }

    /// Get the max number of tokens this rate limiter is configured for.
    #[must_use]
    pub fn max(&self) -> f64 {
        self.inner.max
    }

    /// Get the current number of tokens available.
    #[must_use]
    pub fn tokens(&self) -> f64 {
        self.inner.tokens()
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
    ///         .max(5.0)
    ///         .tokens(0.0)
    ///         .refill_interval(Duration::from_secs(5))
    ///         .refill_amount(1.0)
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
        self.acquire(1.0).await;
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
    ///         .max(5.0)
    ///         .tokens(0.0)
    ///         .refill_interval(Duration::from_secs(5))
    ///         .refill_amount(1.0)
    ///         .build();
    ///
    ///     println!("Waiting for permit...");
    ///     // should take about 25 seconds to acquire.
    ///     rate_limiter.acquire(5.0).await;
    ///     println!("I made it!");
    /// }
    /// ```
    pub async fn acquire(&self, amount: f64) {
        self.inner.acquire(amount).await;
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
    pub fn new() -> Self {
        Self {
            max: None,
            tokens: None,
            refill_interval: None,
            refill_amount: None,
        }
    }

    /// Set the max value for the builder.
    #[must_use]
    pub fn max(mut self, max: f64) -> Self {
        self.max = Some(max);
        self
    }

    /// The number of tokens that the bucket should start with.
    ///
    /// If set to larger than `max` at build time, will only saturate to max.
    #[must_use]
    pub fn tokens(mut self, tokens: f64) -> Self {
        self.tokens = Some(tokens);
        self
    }

    /// Set the max value for the builder.
    #[must_use]
    pub fn refill_interval(mut self, refill_interval: Duration) -> Self {
        self.refill_interval = Some(refill_interval);
        self
    }

    /// Set the refill amount to use.
    #[must_use]
    pub fn refill_amount(mut self, refill_amount: f64) -> Self {
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
