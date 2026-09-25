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
        rust, err, _ = self.kernel()._transpile("macro_rules~ twice\n    ($e:expr) => do:\n        $e * 2\n\ntwice~ 21")
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


if __name__ == "__main__":
    unittest.main()
