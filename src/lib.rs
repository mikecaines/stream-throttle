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
//! # extern crate futures;
//! # extern crate stream_throttle;
//! # extern crate tokio_timer;
//! # extern crate tokio;
//! #
//! # use futures::prelude::*;
//! # use futures::stream;
//! # use std::time::Duration;
//! # use stream_throttle::{ThrottlePool, ThrottleRate, ThrottledStream};
//! #
//! let rate = ThrottleRate::new(5, Duration::new(1, 0));
//! let pool = ThrottlePool::new(rate);
//!
//! let work = stream::repeat(())
//!   .throttle(pool)
//!   .for_each(|_| Ok(()));
//!
//! tokio::run(work);
//! ```
//!
//! ## Example throttling of `Future`
//! ```no_run
//! # extern crate futures;
//! # extern crate stream_throttle;
//! # extern crate tokio_timer;
//! # extern crate tokio;
//! #
//! # use futures::prelude::*;
//! # use futures::future;
//! # use std::time::Duration;
//! # use stream_throttle::{ThrottlePool, ThrottleRate, ThrottledStream};
//! #
//! let rate = ThrottleRate::new(5, Duration::new(1, 0));
//! let pool = ThrottlePool::new(rate);
//!
//! let work = pool.queue()
//!   .then(|_| Ok(()));
//!
//! tokio::run(work);
//! ```

#[macro_use]
extern crate log;
#[macro_use]
extern crate futures;
extern crate futures_timer;
extern crate futures_util;
#[macro_use]
extern crate failure_derive;
extern crate failure;
#[cfg(test)]
extern crate async_std;

pub mod error;
mod pool;
mod rate;
mod stream;

pub use pool::*;
pub use rate::*;
pub use stream::*;
