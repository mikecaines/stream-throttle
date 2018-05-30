//! Provides a
//! [`Stream`](../futures/stream/trait.Stream.html) combinator, to limit the rate at which
//! items are produced. Multiple streams can be throttled according to a shared rate.

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
