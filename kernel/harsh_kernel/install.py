"""`python3 -m harsh_kernel.install`: register the kernel spec with Jupyter."""

import json
import os
import sys
import tempfile

from jupyter_client.kernelspec import KernelSpecManager

SPEC = {
    "argv": [sys.executable, "-m", "harsh_kernel", "-f", "{connection_file}"],
    "display_name": "Harsh",
    "language": "harsh",
}


def main():
    with tempfile.TemporaryDirectory() as d:
        os.mkdir(os.path.join(d, "harsh"))
        with open(os.path.join(d, "harsh", "kernel.json"), "w") as f:
            json.dump(SPEC, f, indent=2)
        KernelSpecManager().install_kernel_spec(os.path.join(d, "harsh"), "harsh", user=True)
    print("Harsh kernel installed. It needs `hrs` on your PATH and the evcxr Rust kernel:")
    print("  cargo install harsh-lang evcxr_jupyter && evcxr_jupyter --install")


if __name__ == "__main__":
    main()
