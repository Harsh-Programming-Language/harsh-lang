"""The kernel's own half, testable without Jupyter or evcxr: a cell of Harsh
becomes the Rust evcxr will see, and an error points at the cell's line."""

import os, sys, tempfile, unittest
sys.path.insert(0, os.path.dirname(__file__))
os.environ.setdefault("PATH", "")
from harsh_kernel.kernel import HarshKernel


class Cell(unittest.TestCase):
    def kernel(self):
        # No Jupyter session: build the object without the base class's socket setup.
        k = HarshKernel.__new__(HarshKernel)
        k._hrs = os.environ.get("HRS", os.path.expanduser("~/.cargo/bin/hrs"))
        k._workdir = tempfile.mkdtemp(prefix="harsh-kernel-test-")
        return k

    def test_statements_and_a_trailing_expression(self):
        rust, err, _ = self.kernel()._transpile("let x = 2\nx * 21")
        self.assertIsNone(err)
        self.assertEqual(rust, "let x = 2;\nx * 21\n")

    def test_an_item(self):
        rust, err, _ = self.kernel()._transpile("fn twice n: i32 -> i32:\n    n * 2")
        self.assertIsNone(err)
        self.assertIn("fn twice(n: i32) -> i32 {", rust)

    def test_a_harsh_macro_unfolds_in_the_cell(self):
        rust, err, _ = self.kernel()._transpile("macro_rules~ twice\n    (($e:expr)) => do:\n        $e * 2\n\ntwice~ 21")
        self.assertIsNone(err)
        self.assertNotIn("macro_rules", rust)
        self.assertIn("21 * 2", rust)

    def test_an_error_names_the_cells_line(self):
        # a `match` without its `\\`: refused, and the line is the cell's
        rust, err, _ = self.kernel()._transpile("let x = 1\nmatch x:\n    _ => 0")
        self.assertIsNone(rust)
        self.assertIn("cell:2:", err)

    def test_an_evcxr_command_passes_through(self):
        from harsh_kernel.kernel import _EVCXR_COMMAND
        self.assertTrue(_EVCXR_COMMAND.match(":dep rand = \"0.8\""))
        self.assertFalse(_EVCXR_COMMAND.match("let x = 1"))


class EvcxrReport(unittest.TestCase):
    """evcxr's real report (evcxr 0.17, captured 2026-09-25, colours and all):
    a one-line struct literal in the cell is four lines of Rust, so evcxr's
    gutter says 6 where the cell's line is 3."""
    REPORT = '\x1b[31m[E0308] Error:\x1b[0m mismatched types\n   \x1b[38;5;246m╭\x1b[0m\x1b[38;5;246m─\x1b[0m\x1b[38;5;246m[\x1b[0mcommand:1:1\x1b[38;5;246m]\x1b[0m\n   \x1b[38;5;246m│\x1b[0m\n \x1b[38;5;246m6 │\x1b[0m \x1b[38;5;249m \x1b[0m\x1b[38;5;249m \x1b[0m\x1b[38;5;249m \x1b[0m\x1b[38;5;249m \x1b[0m\x1b[38;5;249ml\x1b[0m\x1b[38;5;249me\x1b[0m\x1b[38;5;249mt\x1b[0m\x1b[38;5;249m \x1b[0m\x1b[38;5;249mx\x1b[0m\x1b[38;5;249m:\x1b[0m\x1b[38;5;249m \x1b[0m\x1b[38;5;155mi\x1b[0m\x1b[38;5;155m3\x1b[0m\x1b[38;5;155m2\x1b[0m\x1b[38;5;249m \x1b[0m\x1b[38;5;249m=\x1b[0m\x1b[38;5;249m \x1b[0m\x1b[38;5;201m"\x1b[0m\x1b[38;5;201mt\x1b[0m\x1b[38;5;201me\x1b[0m\x1b[38;5;201mx\x1b[0m\x1b[38;5;201mt\x1b[0m\x1b[38;5;201m"\x1b[0m\x1b[38;5;249m;\x1b[0m\n \x1b[38;5;240m  │\x1b[0m            \x1b[38;5;155m─\x1b[0m\x1b[38;5;155m┬\x1b[0m\x1b[38;5;155m─\x1b[0m   \x1b[38;5;201m─\x1b[0m\x1b[38;5;201m─\x1b[0m\x1b[38;5;201m─\x1b[0m\x1b[38;5;201m┬\x1b[0m\x1b[38;5;201m─\x1b[0m\x1b[38;5;201m─\x1b[0m  \n \x1b[38;5;240m  │\x1b[0m             \x1b[38;5;155m╰\x1b[0m\x1b[38;5;155m─\x1b[0m\x1b[38;5;155m─\x1b[0m\x1b[38;5;155m─\x1b[0m\x1b[38;5;155m─\x1b[0m\x1b[38;5;155m─\x1b[0m\x1b[38;5;155m─\x1b[0m\x1b[38;5;155m─\x1b[0m\x1b[38;5;155m─\x1b[0m\x1b[38;5;155m─\x1b[0m\x1b[38;5;155m─\x1b[0m\x1b[38;5;155m─\x1b[0m\x1b[38;5;155m─\x1b[0m expected due to this\n \x1b[38;5;240m  │\x1b[0m                     \x1b[38;5;201m│\x1b[0m    \n \x1b[38;5;240m  │\x1b[0m                     \x1b[38;5;201m╰\x1b[0m\x1b[38;5;201m─\x1b[0m\x1b[38;5;201m─\x1b[0m\x1b[38;5;201m─\x1b[0m\x1b[38;5;201m─\x1b[0m expected `i32`, found `&str`\n\x1b[38;5;246m───╯\x1b[0m'

    def test_the_gutter_names_the_cells_line(self):
        k = Cell.kernel(self)
        rust, err, lines = k._transpile('if true:\n    let p = (Pt\\ x = 1, y = 2)\n    let x: i32 = "text"')
        self.assertIsNone(err)
        self.assertEqual(lines[6], 3, lines)
        out = k._remap(self.REPORT, lines)
        self.assertIn("3 \u2502", out)
        self.assertNotIn("6 \u2502", out)
        self.assertIn("cell:1:1", out)
        self.assertNotIn("command:", out)
        # nothing else of the report changes: the same lines, the same labels
        self.assertEqual(out.count("\n"), self.REPORT.count("\n"))
        self.assertIn("expected `i32`, found `&str`", out)



class StandardDistribution(unittest.TestCase):
    """A cell that uses Harsh's `hrs_std` gets it in evcxr's session, once:
    `hrs dist` writes it, and evcxr takes it by `:dep` (2026-09-26)."""

    def kernel(self):
        k = HarshKernel.__new__(HarshKernel)
        k._hrs = os.environ.get("HRS", os.path.expanduser("~/.cargo/bin/hrs"))
        k._workdir = tempfile.mkdtemp(prefix="harsh-kernel-test-")
        return k

    def test_a_cell_without_hrs_std_needs_nothing(self):
        k = self.kernel()
        rust, err, _ = k._transpile("let x = 2\nx * 21")
        self.assertIsNone(k._std_dep(rust))
        self.assertFalse(os.path.exists(os.path.join(k._workdir, "dist")))

    def test_a_matrix_cell_gets_hrs_std_by_path(self):
        k = self.kernel()
        rust, err, _ = k._transpile("let v = v~ [1, 3, 4]\nv")
        self.assertIsNone(err)
        dep = k._std_dep(rust)
        self.assertTrue(dep.startswith(":dep hrs_std = { path = "), dep)
        path = dep.split('"')[1]
        self.assertTrue(os.path.isfile(os.path.join(path, "Cargo.toml")))
        self.assertTrue(os.path.isfile(os.path.join(path, "src", "lib.rs")))

    def test_once_added_it_is_not_added_again(self):
        k = self.kernel()
        rust, _, _ = k._transpile("let v = v~ [1, 3, 4]\nv")
        k._std_ready = True
        self.assertIsNone(k._std_dep(rust))

    def test_an_hrs_without_dist_says_what_to_do(self):
        k = self.kernel()
        rust, _, _ = k._transpile("let v = v~ [1, 3, 4]\nv")
        k._hrs = "/bin/false"
        with self.assertRaises(RuntimeError) as e:
            k._std_dep(rust)
        self.assertIn("hrs 0.1.34 or later", str(e.exception))

if __name__ == "__main__":
    unittest.main()
