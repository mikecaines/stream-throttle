use super::ThrottlePool;
use futures::{Async, Future, Poll, Stream};

/// Provides a `throttle()` method on all `Stream`'s.
pub trait ThrottledStream {
	/// Returns a new stream, which throttles items from the original stream, according to the
	/// rate defined by `pool`.
	fn throttle(self, pool: ThrottlePool) -> Throttled<Self>
	where
		Self: Stream + Sized,
	{
		Throttled {
			stream: self,
			pool,
			pending: None,
		}
	}
}

impl<T: Stream> ThrottledStream for T {}

/// A stream combinator which throttles its elements via a shared `ThrottlePool`.
///
/// This structure is produced by the `ThrottledStream::throttle()` method.
#[must_use = "streams do nothing unless polled"]
pub struct Throttled<S>
where
	S: Stream + 'static,
{
	stream: S,
	pool: ThrottlePool,

	// the first Option layer represents a pending item for this Throttled stream
	// The second Option layer contains a future produced by ThrottlePool::queue()
	pending: Option<Option<Box<Future<Item = (), Error = S::Error> + Send>>>,
}

impl<S> Stream for Throttled<S>
where
	S: Stream,
{
	type Item = S::Item;
	type Error = S::Error;

	/// Calls ThrottlePool::queue() to get slot in the throttle queue, waits for it to resolve, and
	/// then polls the underlying stream for an item, and produces it.
	fn poll(&mut self) -> Poll<Option<S::Item>, S::Error> {
		// if we don't already have a pending item, get one
		if self.pending.is_none() {
			self.pending = Some(Some(Box::new(
				self.pool
					.queue()
					.map_err(|e| panic!("ThrottlePool::queue() failed: {}", e)),
			)));
		}
		assert!(self.pending.is_some());

		// if we have a queue() future, we are still waiting for the queue slot to expire
		if self.pending.as_mut().unwrap().is_some() {
			// poll the queue slot future, and remove it once it resolves
			try_ready!(self.pending.as_mut().unwrap().as_mut().unwrap().poll());
			self.pending.as_mut().unwrap().take();
		}

		// poll the underlying stream until we get an item, or the stream ends
		match try_ready!(self.stream.poll()) {
			Some(item) => {
				self.pending = None;
				Ok(Async::Ready(Some(item)))
			}

			None => Ok(Async::Ready(None)),
		}
	}
}
