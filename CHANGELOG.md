# Changelog

## [0.5.0] - 2023-09-04
### Changed
- Added `ThrottlePool::queue_with_hold()`
  - This method returns a value that will keep the slot "in use", until dropped.
  - An example use of this would be in networking. Whereas using `queue()` can be used to prevent the *start* of 
    subsequent requests, `queue_with_hold()` can be used to also ensure no new requests are started until all 
    in-flight requests are *completed*.
- Update `tokio` v1.32.
- Update `pin-utils` to v0.1.

## [0.4.0] - 2021-04-07
### Changed
- Update to `tokio` v1.0.

## [0.3.0] - 2020-02-09
### Added
- Optional support for `futures-timer` crate via feature flag.
### Changed
- Update to `std` `Future` and `tokio` v0.2.