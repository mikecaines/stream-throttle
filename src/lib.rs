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
//! #
//! # use futures::prelude::*;
//! # use futures::stream;
//! # use std::time::{Duration, Instant};
//! # use stream_throttle::{ThrottlePool, ThrottleRate, ThrottledStream};
//! # use tokio_timer::Timer;
//! #
//! let rate = ThrottleRate::new(5, Duration::new(2, 0));
//! let pool = ThrottlePool::new(rate, Timer::default());
//!
//! stream::repeat(())
//! # .map_err(|_: ()| ()) // just need to declare error type in this simple example
//!   .throttle(pool)
//!   .wait();
//! ```
//!
//! ## Example throttling of `Future`
//! ```no_run
//! # extern crate futures;
//! # extern crate stream_throttle;
//! # extern crate tokio_timer;
//! #
//! # use futures::prelude::*;
//! # use futures::future;
//! # use std::time::Duration;
//! # use stream_throttle::{ThrottlePool, ThrottleRate, ThrottledStream};
//! # use tokio_timer::Timer;
//! #
//! let rate = ThrottleRate::new(5, Duration::new(2, 0));
//! let pool = ThrottlePool::new(rate, Timer::default());
//!
//! pool.queue()
//!   .and_then(|_| Ok(()))
//!   .wait();
//! ```

#[macro_use]
extern crate log;
#[macro_use]
extern crate futures;
extern crate tokio_timer;
#[macro_use]
extern crate failure_derive;
extern crate failure;

pub mod error;
mod pool;
mod rate;
mod stream;

pub use stream::*;
pub use pool::*;
pub use rate::*;
