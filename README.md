# stream_throttle
Provides a 
[Rust](https://www.rust-lang.org) 
[`Stream`](https://docs.rs/futures/0.1.21/futures/stream/trait.Stream.html)
combinator, to limit the rate at which items are produced.

[![Crates.io](https://img.shields.io/crates/v/stream_throttle.svg)](https://crates.io/crates/stream_throttle)
[![API Documentation](https://docs.rs/stream_throttle/badge.svg)](https://docs.rs/stream_throttle/)

## Key Features
- Throttling is implemented via
[`poll()`](https://docs.rs/futures/0.1.21/futures/future/trait.Future.html#tymethod.poll), 
and not via any sort of buffering.
- The throttling behaviour can be applied to both `Stream`'s and `Future`'s.
- Multiple streams/futures can be throttled together as a group.

## Example throttling of `Stream`
```rust
let rate = ThrottleRate::new(5, Duration::new(2, 0));
let pool = ThrottlePool::new(rate, Timer::default());

stream::repeat(())
  .throttle(pool)
  .wait();
```

## Example throttling of `Future`
```rust
let rate = ThrottleRate::new(5, Duration::new(2, 0));
let pool = ThrottlePool::new(rate, Timer::default());

pool.queue()
  .and_then(|_| Ok(()))
  .wait();
```
