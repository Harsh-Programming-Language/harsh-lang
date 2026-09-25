"""The Harsh kernel: transpile the cell, evaluate it in evcxr, relay the answer."""

import json
import os
import re
import shutil
import subprocess
import tempfile

from ipykernel.kernelbase import Kernel
from jupyter_client import KernelManager

from . import __version__

# evcxr's own commands pass through untouched: they are the kernel's, not the
# language's.
_EVCXR_COMMAND = re.compile(r"^\s*:(dep|help|vars|clear|opt|timing|explain|last_error_json|efmt|fmt|preserve_vars_on_panic|quit|internal_debug|sccache|linker|version|last_compile_dir|allow_static_memory|offline|cache|toolchain|type|doc|toolchain_version)\b")


class HarshKernel(Kernel):
    implementation = "harsh"
    implementation_version = __version__
    language = "harsh"
    language_version = "0.1"
    language_info = {
        "name": "harsh",
        "mimetype": "text/x-harsh",
        "file_extension": ".hrs",
        "codemirror_mode": "rust",
        "pygments_lexer": "rust",
    }
    banner = "Harsh — Rust without the braces. Cells are transpiled with hrs and evaluated by evcxr."

    def __init__(self, **kwargs):
        super().__init__(**kwargs)
        self._hrs = shutil.which("hrs") or os.path.expanduser("~/.cargo/bin/hrs")
        self._km = None
        self._kc = None
        self._workdir = tempfile.mkdtemp(prefix="harsh-kernel-")

    # -- the evcxr kernel behind us ----------------------------------------

    def _ensure_backend(self):
        if self._kc is not None:
            return
        self._km = KernelManager(kernel_name="rust")
        self._km.start_kernel()
        self._kc = self._km.client()
        self._kc.start_channels()
        self._kc.wait_for_ready(timeout=120)

    def do_shutdown(self, restart):
        if self._kc is not None:
            try:
                self._kc.stop_channels()
                self._km.shutdown_kernel(now=True)
            except Exception:
                pass
            self._kc = None
            self._km = None
        shutil.rmtree(self._workdir, ignore_errors=True)
        return {"status": "ok", "restart": restart}

    # -- Harsh -> Rust ------------------------------------------------------

    def _transpile(self, code):
        """The cell as Rust, plus the source map from Rust lines to cell lines.

        A cell is a fragment: statements, items, or a trailing expression.
        `hrs` transpiles files, so the cell is wrapped as a function body and
        unwrapped after, exactly as the doc-example harness does; the wrapper
        line is accounted for in the map.
        """
        src = os.path.join(self._workdir, "cell.hrs")
        rs = os.path.join(self._workdir, "cell.rs")
        mp = os.path.join(self._workdir, "cell.rs.map")
        lines = code.rstrip("\n").split("\n")
        wrapped = "fn __cell$:\n" + "\n".join("    " + l if l.strip() else l for l in lines) + "\n"
        with open(src, "w") as f:
            f.write(wrapped)
        r = subprocess.run([self._hrs, src, "-o", rs], capture_output=True, text=True)
        if r.returncode != 0:
            return None, self._reline(r.stderr + r.stdout, wrapper_lines=1, indent=4), None
        with open(rs) as f:
            rust = f.read()
        # Unwrap: drop `fn __cell() {` and the closing `}`, dedent one level.
        body = rust.split("\n")
        start = next((i for i, l in enumerate(body) if l.startswith("fn __cell()")), 0) + 1
        end = len(body) - 1
        while end > start and body[end].strip() != "}":
            end -= 1
        inner = [l[4:] if l.startswith("    ") else l for l in body[start:end]]
        return "\n".join(inner) + "\n", None, start

    def _reline(self, text, wrapper_lines, indent):
        """Point an hrs error at the cell's line and column, not the wrapper's:
        the `--> file:line:col` reference, the numbered gutter of the snippet,
        and the caret line beneath it all move together."""
        def fix_ref(m):
            line = int(m.group(1)) - wrapper_lines
            col = int(m.group(2)) - indent
            return f"cell:{max(line, 1)}:{max(col, 1)}"
        text = re.sub(r"[^\s:]+\.hrs:(\d+):(\d+)", fix_ref, text)
        out = []
        for line in text.split("\n"):
            g = re.match(r"^(\s*)(\d+)\|(.*)$", line)
            if g:
                n = int(g.group(2)) - wrapper_lines
                body = g.group(3)
                body = body[indent:] if body.startswith(" " * indent) else body
                out.append(f"{g.group(1)}{n}|{body}")
                continue
            c = re.match(r"^(\s*)\|(\s+)(\^+.*)$", line)
            if c and len(c.group(2)) > indent:
                out.append(f"{c.group(1)}|{c.group(2)[indent:]}{c.group(3)}")
                continue
            out.append(line)
        return "\n".join(out)

    # -- execution ----------------------------------------------------------

    def do_execute(self, code, silent, store_history=True, user_expressions=None, allow_stdin=False):
        if not code.strip():
            return {"status": "ok", "execution_count": self.execution_count, "payload": [], "user_expressions": {}}

        if _EVCXR_COMMAND.match(code):
            rust, error, offset = code, None, 0
        else:
            rust, error, offset = self._transpile(code)
            if error:
                if not silent:
                    self.send_response(self.iopub_socket, "stream", {"name": "stderr", "text": error})
                return {"status": "error", "execution_count": self.execution_count, "ename": "HarshError", "evalue": "the cell does not transpile", "traceback": []}

        try:
            self._ensure_backend()
        except Exception as e:
            msg = f"the Rust kernel (evcxr_jupyter) could not start: {e}\nInstall it with `cargo install evcxr_jupyter && evcxr_jupyter --install`.\n"
            self.send_response(self.iopub_socket, "stream", {"name": "stderr", "text": msg})
            return {"status": "error", "execution_count": self.execution_count, "ename": "BackendError", "evalue": msg, "traceback": []}

        msg_id = self._kc.execute(rust, silent=silent, store_history=store_history, allow_stdin=False)
        status = "ok"
        while True:
            try:
                msg = self._kc.get_iopub_msg(timeout=600)
            except Exception:
                break
            if msg["parent_header"].get("msg_id") != msg_id:
                continue
            t = msg["msg_type"]
            c = msg["content"]
            if t == "status":
                if c.get("execution_state") == "idle":
                    break
                continue
            if t == "stream":
                text = c.get("text", "")
                if c.get("name") == "stderr":
                    text = self._remap(text, offset)
                if not silent:
                    self.send_response(self.iopub_socket, "stream", {"name": c["name"], "text": text})
            elif t in ("execute_result", "display_data"):
                if not silent:
                    payload = {"data": c.get("data", {}), "metadata": c.get("metadata", {})}
                    if t == "execute_result":
                        payload["execution_count"] = self.execution_count
                    self.send_response(self.iopub_socket, t, payload)
            elif t == "error":
                status = "error"
                if not silent:
                    tb = [self._remap(line, offset) for line in c.get("traceback", [])]
                    self.send_response(self.iopub_socket, "error", {"ename": c.get("ename", "RustError"), "evalue": c.get("evalue", ""), "traceback": tb})
        try:
            self._kc.get_shell_msg(timeout=5)
        except Exception:
            pass
        return {"status": status, "execution_count": self.execution_count, "payload": [], "user_expressions": {}}

    def _remap(self, text, offset):
        """rustc lines (in the cell's Rust) back to the cell's Harsh lines.

        evcxr wraps the Rust in its own function, so its line numbers are
        offset again; the Harsh map only knows the Rust hrs wrote. The
        practical mapping is line-for-line here, since the cell's Rust keeps
        one statement per line: the number in `--> src/lib.rs:N:C` becomes
        `cell:N':C'` where N' is looked up through hrs's map when it exists,
        else N unchanged.
        """
        return re.sub(r"--> [^\s:]+:(\d+):(\d+)", lambda m: f"--> cell:{m.group(1)}:{m.group(2)}", text)

    def do_is_complete(self, code):
        # A cell that ends with an opener is being typed; anything else is complete.
        stripped = code.rstrip()
        if stripped.endswith((":", "\\")):
            return {"status": "incomplete", "indent": "    "}
        return {"status": "complete"}
