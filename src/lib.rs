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
//!
//! ## Example throttling of `Stream`
//! ```no_run
//! # use futures::prelude::*;
//! # use std::time::Duration;
//! # use stream_throttle::{ThrottlePool, ThrottleRate, ThrottledStream};
//! #
//! let rate = ThrottleRate::new(5, Duration::new(1, 0));
//! let pool = ThrottlePool::new(rate);
//!
//! let work = futures::stream::repeat(())
//!   .throttle(pool)
//! 	.then(|_| futures::future::ready("do something else"))
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
