# leaky-bucket-lite

[![Documentation](https://docs.rs/leaky-bucket-lite/badge.svg)](https://docs.rs/leaky-bucket-lite)
[![Crates](https://img.shields.io/crates/v/leaky-bucket-lite.svg)](https://crates.io/crates/leaky-bucket-lite)
[![Actions Status](https://github.com/Gelbpunkt/leaky-bucket-lite/workflows/Rust/badge.svg)](https://github.com/Gelbpunkt/leaky-bucket-lite/actions)

A token-based rate limiter based on the [leaky bucket] algorithm, mainly a lazy reimplementation of [udoprog's leaky-bucket] with less dependencies and overhead.

If the tokens are already available, the acquisition will be instant through
a fast path, and the acquired number of tokens will be added to the bucket.

If they aren't, the task that tried to acquire the tokens will be suspended
until the required number of tokens has been added.

### Usage

Add the following to your `Cargo.toml`:

```toml
leaky-bucket-lite = "0.3"
```

### Features

leaky-bucket-lite provides 3 implementations:

- `leaky_bucket_lite::LeakyBucket` (thread-safe, available via the `tokio` feature, which is on by default)
- `leaky_bucket_lite::sync_threadsafe::LeakyBucket` (thread-safe, available via the `sync-threadsafe` feature
- `leaky_bucket_lite::sync::LeakyBucket` (not thread-safe, available via the `sync` feature).

For potential performance increase with `sync-threadsafe` using `parking_lot`'s locking objects, enable the `parking-lot` feature.

### Example

```rust
use leaky_bucket_lite::LeakyBucket;
use std::time::Duration;

#[tokio::main]
async fn main() {
    let rate_limiter = LeakyBucket::builder()
        .max(5)
        .tokens(0)
        .refill_interval(Duration::from_secs(1))
        .refill_amount(1)
        .build();

    println!("Waiting for permit...");
    // should take about 5 seconds to acquire.
    rate_limiter.acquire(5).await;
    println!("I made it!");
}
```

[leaky bucket]: https://en.wikipedia.org/wiki/Leaky_bucket
[udoprog's leaky-bucket]: https://github.com/udoprog/leaky-bucket

License: MIT/Apache-2.0
