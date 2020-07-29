#![forbid(unsafe_code)]
#![forbid(bare_trait_objects)]
//! # JSON Stream
//!
//! This library provides a lazy pull parser, as well as an emitter for
//! reading and writing JSON to anything implementing [`io::Read`](std::io::Read) and [`io::Write`](std::io::Write).
//!
//! The main use is processing JSON values that would not otherwise fit in RAM.
//!
//!
//! ## General API Notes
//!
//! * the API is still under development, and suggestions and welcome
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
//! JSON Stream parsers can perform some error recovery.
//!
//! ## Sync and Async
//!
//! By default, JSON Stream exposes a sync interface, via the default `sync` feature.
//! To enable async support, enable the `async` feature.
//! The async API is similar to the sync one, with the notable exception of using
//! AsyncRead and AsyncWrite as the underlying traits for parsing/emitting.
//!
//!
pub mod parse;
