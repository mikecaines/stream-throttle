use super::ThrottlePool;
use futures::task::{Context, Poll};
use futures::{ready, Future, FutureExt, Stream};
use pin_utils::{unsafe_pinned, unsafe_unpinned};
use std::pin::Pin;

/// Provides a `throttle()` method on all `Stream`'s.
pub trait ThrottledStream {
	/// Returns a new stream, which throttles items from the original stream, according to the
	/// rate defined by `pool`.
	fn throttle(self, pool: ThrottlePool) -> Throttled<Self>
	where
		Self: Stream + Sized,
	{
		Throttled {
			stream_pinned: self,
			pool,
			state_unpinned: State::None,
			slot_pinned: None,
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
	stream_pinned: S,
	pool: ThrottlePool,
	state_unpinned: State,
	slot_pinned: Option<Pin<Box<dyn Future<Output = ()> + Send + 'static>>>,
}

impl<S> Throttled<S>
where
	S: Stream + 'static,
{
	// This was used as a guide:
	// https://docs.rs/futures-util/0.3.1/src/futures_util/stream/stream/take_while.rs.html#101
	unsafe_pinned!(stream_pinned: S);
	unsafe_unpinned!(state_unpinned: State);
	unsafe_pinned!(slot_pinned: Option<Pin<Box<dyn Future<Output = ()> + Send + 'static>>>);
}

impl<S> Stream for Throttled<S>
where
	S: Stream,
{
	type Item = S::Item;

	/// Calls ThrottlePool::queue() to get slot in the throttle queue, waits for it to resolve, and
	/// then polls the underlying stream for an item, and produces it.
	fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		if let State::None = self.state_unpinned {
			// get a slot future from the pool, and store it
			let slot = self.pool.queue().boxed();
			self.as_mut().slot_pinned().set(Some(slot));

			*self.as_mut().state_unpinned() = State::Slot;
		}

		if let State::Slot = self.state_unpinned {
			// poll the slot future
			let _ = ready!(self
				.as_mut()
				.slot_pinned()
				.as_pin_mut()
				.expect("impossible: slot future was None, during State::Slot")
				.poll(cx));

			// clear the slot future, now that it has finished
			self.as_mut().slot_pinned().set(None);

			*self.as_mut().state_unpinned() = State::Stream;
		}

		if let State::Stream = self.state_unpinned {
			// if polling the internal stream produced an item
			if let Some(item) = ready!(self.as_mut().stream_pinned().poll_next(cx)) {
				// reset the state to None
				*self.as_mut().state_unpinned() = State::None;

				// return the item from the internal stream
				return Poll::Ready(Some(item));
			}
			// else the internal stream has ended
			else {
				// set the state to Done, from which it will never change again
				*self.as_mut().state_unpinned() = State::Done;
			}
		}

		Poll::Ready(None)
	}
}

enum State {
	// the stream has not been polled yet, or in the previous poll returned an item
	None,

	// we are polling the internal ThrottlePool::queue() slot Future
	Slot,

	// we are polling the internal Stream
	Stream,

	// the internal stream has ended, nothing more to do
	Done,
}
