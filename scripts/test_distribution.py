import hashlib
from pathlib import Path
import tempfile
import unittest

from distribution import archive, safe_member


class ArchiveValidation(unittest.TestCase):
    def test_rejects_traversal_and_absolute_paths_on_all_hosts(self):
        for name in ("../rstop", "/usr/bin/rstop", "x/../../rstop", r"..\rstop", "C:/rstop.exe"):
            with self.subTest(name=name), self.assertRaises(RuntimeError):
                safe_member(name)
        safe_member("rstop-x86_64-unknown-linux-gnu/completions/rstop.bash")

    def test_checksum_failure_prevents_unpacking_or_execution(self):
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / "rstop-x86_64-unknown-linux-gnu.tar.xz"
            original = b"untrusted archive bytes"
            path.write_bytes(original + b"modified")
            path.with_name(path.name + ".sha256").write_text(hashlib.sha256(original).hexdigest())
            with self.assertRaisesRegex(AssertionError, "checksum"):
                archive(temp, "x86_64-unknown-linux-gnu")


if __name__ == "__main__":
    unittest.main()
