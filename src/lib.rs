#![forbid(unsafe_code)]
#![forbid(bare_trait_objects)]
//! # JSON Stream
//!
//! This library provides a lazy pull parser, as well as an emitter for
//! reading and writing JSON to anything implementing [`io::Read`](std::io::Read), or [`io::Write`](std::io::Write) respectively.
//!
//! The main use is processing JSON values that cannot be completely buffered in RAM.
//!
//!
//! ## General API Notes
//!
//! * lifetimes in parsers and [`Json`](parse::Json) always point back to the original [`Parser`](parse::Parser)
//! * the [`Json`](parse::Json) enum does not implement PartialEq, since it can hold parsers that have not yielded data, so a comparison cannot be accurate
//! * all parsers have a `fn next(&mut self) -> Option<Json>` method, but that is not part of an impl for [`Iterator`]. This may be possible in the future when generic associated types are stabilized.
//!
//! ## Subparsers
//!
//! Json values are parsed and returned as-is only for null, booleans, and numbers.
//! Since strings, arrays and objects can be arbitrarily large,
//! subparsers are returned for them.
//!
//! ## Error recovery
//!
//! JSON Stream parsers can perform some error recovery. Errors that are nonfatal are marked as such, and after one is returned parsing may continue.
//!
//! ## Sync and Async
//!
//! By default, JSON Stream exposes a sync interface, via the default `sync` feature.
//! To enable async support, enable the `async` feature.
//! The async API is similar to the sync one, with the exception of using
//! AsyncRead and AsyncWrite as the underlying traits for parsing/emitting.
//!
//! ## `serde_json` integration
//!
//! Sometimes, when an object or array is known to be small and have a particular structure, it's useful to be able to deserialize it
//! directly into a `serde_json::Value`, or anything implementing `serde::Deserialize`. The same applies while emitting, for `serde_json::Serialize`.
//!
//! Enable the `serde_json` feature to expose `Serialize`/`Deserializer` implementations that allow
//!
//!
pub mod parse;
