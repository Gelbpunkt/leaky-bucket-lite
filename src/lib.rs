#![deny(missing_docs, clippy::all)]
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
//!         .max(5)
//!         .tokens(0)
//!         .refill_interval(Duration::from_secs(1))
//!         .refill_amount(1)
//!         .build();
//!
//!     println!("Waiting for permit...");
//!     // should take about 5 seconds to acquire.
//!     rate_limiter.acquire(5).await?;
//!     println!("I made it!");
//!     Ok(())
//! }
//! ```
//!
//! [leaky bucket]: https://en.wikipedia.org/wiki/Leaky_bucket
use tokio::{
    sync::{mpsc, oneshot},
    time::{sleep_until, Duration, Instant},
};

/// Error type used in this crate.
pub type Error = oneshot::error::RecvError;

struct BucketActor {
    receiver: mpsc::UnboundedReceiver<ActorMessage>,
    tokens: f64,
    max: f64,
    refill_interval: Duration,
    refill_amount: f64,
    last_refill: Instant,
}
enum ActorMessage {
    Acquire {
        amount: usize,
        respond_to: oneshot::Sender<()>,
    },
}

impl BucketActor {
    fn new(
        max: usize,
        tokens: usize,
        refill_interval: Duration,
        refill_amount: usize,
        receiver: mpsc::UnboundedReceiver<ActorMessage>,
    ) -> Self {
        Self {
            receiver,
            tokens: tokens as f64,
            max: max as f64,
            refill_interval,
            refill_amount: refill_amount as f64,
            last_refill: Instant::now(),
        }
    }

    async fn handle_message(&mut self, msg: ActorMessage) {
        match msg {
            ActorMessage::Acquire { amount, respond_to } => {
                let amount = amount as f64;
                let time_passed = Instant::now() - self.last_refill;
                let refills_since = time_passed.as_secs_f64() / self.refill_interval.as_secs_f64();
                self.last_refill = Instant::now();
                self.tokens += refills_since * self.refill_amount;
                if self.tokens > self.max {
                    self.tokens = self.max;
                }

                if self.tokens >= amount {
                    self.tokens -= amount;
                } else {
                    let tokens_needed = amount - self.tokens;
                    let refills_needed = tokens_needed / self.refill_amount;
                    let target_time = Instant::now() + self.refill_interval.mul_f64(refills_needed);

                    sleep_until(target_time).await;

                    self.last_refill = target_time;
                    self.tokens = 0.0;
                }
                let _ = respond_to.send(());
            }
        }
    }
}

async fn run_bucket_actor(mut actor: BucketActor) {
    while let Some(msg) = actor.receiver.recv().await {
        actor.handle_message(msg).await;
    }
}

/// The leaky bucket.
#[derive(Clone, Debug)]
pub struct LeakyBucket {
    sender: mpsc::UnboundedSender<ActorMessage>,
    max: usize,
}

impl LeakyBucket {
    fn new(max: usize, tokens: usize, refill_interval: Duration, refill_amount: usize) -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        let actor = BucketActor::new(max, tokens, refill_interval, refill_amount, receiver);
        tokio::spawn(run_bucket_actor(actor));

        Self { sender, max }
    }

    /// Construct a new leaky bucket through a builder.
    pub fn builder() -> Builder {
        Builder::new()
    }

    /// Get the max number of tokens this rate limiter is configured for.
    pub fn max(&self) -> usize {
        self.max
    }

    /// Acquire a single token.
    ///
    /// This is identical to [`acquire`] with an argument of `1`.
    ///
    /// [`acquire`]: LeakyBucket::acquire
    ///
    /// # Example
    ///
    /// ```rust
    /// use leaky_bucket_lite::LeakyBucket;
    /// use std::{error::Error, time::Duration};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn Error>> {
    ///     let rate_limiter = LeakyBucket::builder()
    ///         .max(5)
    ///         .tokens(0)
    ///         .refill_interval(Duration::from_secs(5))
    ///         .refill_amount(1)
    ///         .build();
    ///
    ///     println!("Waiting for permit...");
    ///     // should take about 5 seconds to acquire.
    ///     rate_limiter.acquire_one().await?;
    ///     println!("I made it!");
    ///
    ///     Ok(())
    /// }
    /// ```
    #[inline]
    pub async fn acquire_one(&self) -> Result<(), Error> {
        self.acquire(1).await
    }

    /// Acquire the given `amount` of tokens.
    ///
    /// # Example
    ///
    /// ```rust
    /// use leaky_bucket_lite::LeakyBucket;
    /// use std::{error::Error, time::Duration};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn Error>> {
    ///     let rate_limiter = LeakyBucket::builder()
    ///         .max(5)
    ///         .tokens(0)
    ///         .refill_interval(Duration::from_secs(5))
    ///         .refill_amount(1)
    ///         .build();
    ///
    ///     println!("Waiting for permit...");
    ///     // should take about 25 seconds to acquire.
    ///     rate_limiter.acquire(5).await?;
    ///     println!("I made it!");
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn acquire(&self, amount: usize) -> Result<(), Error> {
        let (send, recv) = oneshot::channel();
        let msg = ActorMessage::Acquire {
            amount,
            respond_to: send,
        };

        let _ = self.sender.send(msg);
        recv.await
    }
}

/// Builder for a leaky bucket.
#[derive(Debug)]
pub struct Builder {
    max: Option<usize>,
    tokens: Option<usize>,
    refill_interval: Option<Duration>,
    refill_amount: Option<usize>,
}

impl Builder {
    /// Create a new builder with all defaults.
    pub fn new() -> Self {
        Self {
            max: None,
            tokens: None,
            refill_interval: None,
            refill_amount: None,
        }
    }

    /// Set the max value for the builder.
    #[inline(always)]
    pub fn max(mut self, max: usize) -> Self {
        self.max = Some(max);
        self
    }

    /// The number of tokens that the bucket should start with.
    ///
    /// If set to larger than `max` at build time, will only saturate to max.
    #[inline(always)]
    pub fn tokens(mut self, tokens: usize) -> Self {
        self.tokens = Some(tokens);
        self
    }

    /// Set the max value for the builder.
    #[inline(always)]
    pub fn refill_interval(mut self, refill_interval: Duration) -> Self {
        self.refill_interval = Some(refill_interval);
        self
    }

    /// Set the refill amount to use.
    #[inline(always)]
    pub fn refill_amount(mut self, refill_amount: usize) -> Self {
        self.refill_amount = Some(refill_amount);
        self
    }

    /// Construct a new leaky bucket.
    pub fn build(self) -> LeakyBucket {
        const DEFAULT_MAX: usize = 120;
        const DEFAULT_TOKENS: usize = 0;
        const DEFAULT_REFILL_INTERVAL: Duration = Duration::from_secs(1);
        const DEFAULT_REFILL_AMOUNT: usize = 1;

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

#[cfg(test)]
mod tests {
    use super::Builder;
    use std::time::{Duration, Instant};
    use tokio::time;

    #[tokio::test]
    async fn test_leaky_bucket() {
        let interval = Duration::from_millis(20);

        let leaky = Builder::new()
            .tokens(0)
            .max(10)
            .refill_amount(10)
            .refill_interval(interval)
            .build();

        let mut wakeups = 0u32;
        let mut duration = None;

        let test = async {
            let start = Instant::now();
            leaky.acquire(10).await.unwrap();
            wakeups += 1;
            leaky.acquire(10).await.unwrap();
            wakeups += 1;
            leaky.acquire(10).await.unwrap();
            wakeups += 1;
            duration = Some(Instant::now().duration_since(start));
        };

        test.await;

        assert_eq!(3, wakeups);
        assert!(duration.expect("expected measured duration") > interval * 2);
    }

    #[tokio::test]
    async fn test_concurrent_rate_limited() {
        let interval = Duration::from_millis(20);

        let leaky = Builder::new()
            .tokens(0)
            .max(10)
            .refill_amount(1)
            .refill_interval(interval)
            .build();

        let mut one_wakeups = 0;

        let one = async {
            loop {
                leaky.acquire(1).await.unwrap();
                one_wakeups += 1;
            }
        };

        let mut two_wakeups = 0u32;

        let two = async {
            loop {
                leaky.acquire(1).await.unwrap();
                two_wakeups += 1;
            }
        };

        let delay = time::sleep(Duration::from_millis(200));

        let task = async {
            tokio::select! {
                _ = one => {},
                _ = two => {},
            }
        };

        tokio::select! {
            _ = task => {},
            _ = delay => {},
        }

        let total = one_wakeups + two_wakeups;

        assert!(total > 5 && total < 15);
    }
}
