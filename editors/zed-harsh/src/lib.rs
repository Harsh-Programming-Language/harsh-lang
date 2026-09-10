// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! The Zed extension shim: Zed requires a WASM crate to launch any language
//! server. This one does nothing but hand Zed the `hrs-lsp` command from
//! PATH (installed by `cargo install --path .` in the hrust checkout).
//! Build with `cargo build --target wasm32-wasip1`, which Zed does itself
//! when installing a dev extension.

use zed_extension_api::{self as zed, LanguageServerId, Result};

struct HarshExtension;

impl zed::Extension for HarshExtension {
    fn new() -> Self {
        HarshExtension
    }

    fn language_server_command(
        &mut self,
        _id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        let path = worktree
            .which("hrs-lsp")
            .ok_or_else(|| "hrs-lsp not found on PATH; run `cargo install --path .` in the hrust checkout".to_string())?;
        Ok(zed::Command { command: path, args: Vec::new(), env: Vec::new() })
    }
}

zed::register_extension!(HarshExtension);
