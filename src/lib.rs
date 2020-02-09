//! Provides a
//! [`Stream`](../futures/stream/trait.Stream.html) combinator, to limit the rate at which
//! items are produced.
//!
//! ## Key Features
//! - Throttling is implemented via
//! [`poll()`](../futures/future/trait.Future.html#tymethod.poll), and not via any sort of
//! buffering.
//! - The throttling behaviour can be applied to both `Stream`'s and `Future`'s.
//! - Multiple streams/futures can be throttled together as a group.
//! - Feature flags to use various timer implementations.
//!
//! ## Feature Flags
//! - `timer-tokio`: Uses the `tokio::time::delay_for()` timer (default).
//! - `timer-futures-timer`: Uses the `futures_timer::Delay` timer.
//!
//! If you don't use the default timer (`tokio`), make sure to set `default-features = false`
//! in your `Cargo.toml`, when you add `stream_throttle` as a dependency.
//!
//! ## Example throttling of `Stream`
//! ```no_run
//! # use futures::prelude::*;
//! # use std::time::Duration;
//! # use stream_throttle::{ThrottlePool, ThrottleRate, ThrottledStream};
//! #
//! // allow no more than 5 items every 1 second
//! let rate = ThrottleRate::new(5, Duration::new(1, 0));
//! let pool = ThrottlePool::new(rate);
//!
//! let work = futures::stream::repeat(())
//!   .throttle(pool)
//!   .then(|_| futures::future::ready("do something else"))
//!   .for_each(|_| futures::future::ready(()));
//!
//! futures::executor::block_on(work);
//! ```
//!
//! ## Example throttling of `Future`
//! ```no_run
//! # use futures::prelude::*;
//! # use std::time::Duration;
//! # use stream_throttle::{ThrottlePool, ThrottleRate, ThrottledStream};
//! #
//! let rate = ThrottleRate::new(5, Duration::new(1, 0));
//! let pool = ThrottlePool::new(rate);
//!
//! let work = pool.queue()
//!   .then(|_| futures::future::ready("do something else"));
//!
//! futures::executor::block_on(work);
//! ```

mod pool;
mod rate;
mod stream;

pub use pool::*;
pub use rate::*;
pub use stream::*;
