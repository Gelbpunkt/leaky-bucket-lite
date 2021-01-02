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
    tokens: usize,
    max: usize,
    refill_interval: Duration,
    refill_amount: usize,
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
            tokens,
            max,
            refill_interval,
            refill_amount,
            last_refill: Instant::now(),
        }
    }

    async fn handle_message(&mut self, msg: ActorMessage) {
        match msg {
            ActorMessage::Acquire { amount, respond_to } => {
                if self.tokens >= amount {
                    self.tokens -= amount;
                } else {
                    // We do not have enough tokens, calculate when we would have enough
                    let tokens_needed = (amount - self.tokens) as f64;
                    let refills_needed =
                        (tokens_needed / (self.refill_amount as f64)).ceil() as u32;
                    let time_needed = self.refill_interval * refills_needed;

                    let point_in_time = self.last_refill + time_needed;

                    if point_in_time > Instant::now() {
                        // It is in the future, so wait
                        sleep_until(point_in_time).await;
                    }

                    // The point has passed, recalculate everything
                    self.last_refill = point_in_time;
                    self.tokens += self.refill_amount * (refills_needed as usize);
                    if self.tokens > self.max {
                        self.tokens = self.max;
                    }
                    self.tokens -= amount;
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
    use futures::prelude::*;
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

        let task = future::select(one.boxed(), two.boxed());
        let task = future::select(task, delay.boxed());

        task.await;

        let total = one_wakeups + two_wakeups;

        assert!(total > 5 && total < 15);
    }
}
