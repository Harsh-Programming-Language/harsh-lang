// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Harsh: a layout transformation over Rust.
//!
//! `lex`, `layout` and `rules` are shared by both directions. `emit` goes
//! Harsh -> Rust; `unbrace` goes Rust -> Harsh. Keeping them in one crate is
//! deliberate: the two directions must agree on the carve-out list and the
//! block-kind table, and separate crates would let those drift silently.

pub mod columns;
pub mod fmt;
pub mod docex;
pub mod driver;
pub mod emit;
pub mod juxt;
pub mod layout;
pub mod lex;
pub mod remap;
pub mod rawzone;
pub mod rules;
pub mod unbrace;
