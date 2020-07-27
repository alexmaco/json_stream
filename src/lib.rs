#![forbid(unsafe_code)]
#![forbid(bare_trait_objects)]
//! # JSON Stream
//!
//! This library provides a lazy pull parser, as well as an emitter for
//! reading and writing JSON to anything implementing [`io::Read`](std::io::Read) and [`io::Write`].
//!
//! The main use is processing JSON values that would not otherwise fit in RAM.
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
