import hashlib
import contextlib
import io
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import distribution
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


class WorkflowValidation(unittest.TestCase):
    def test_regeneration_restores_release_gates_and_rejects_missing_guards(self):
        source = (distribution.ROOT / ".github/workflows/release.yml").read_text(encoding="utf-8")
        creation = "    if: ${{ github.event_name == 'pull_request' || github.event.created }}\n"
        install = " && needs.custom-install-checks.result == 'success'"
        generated_title = 'ANNOUNCEMENT_TITLE: "${{ fromJson(needs.host.outputs.val).announcement_title }}"'
        tagged_title = 'ANNOUNCEMENT_TITLE: "${{ needs.plan.outputs.tag }}"'
        with tempfile.TemporaryDirectory() as temp, contextlib.redirect_stdout(io.StringIO()):
            root = Path(temp)
            path = root / ".github/workflows/release.yml"
            path.parent.mkdir(parents=True)
            path.write_text(source.replace(creation, "").replace(install, "").replace(tagged_title, generated_title), encoding="utf-8")
            with patch.object(distribution, "ROOT", root):
                distribution.workflow()
                distribution.workflow(check=True)
                expected = path.read_text(encoding="utf-8")
                distribution.workflow()
                self.assertEqual(path.read_text(encoding="utf-8"), expected)
                for job in ("plan", "custom-checks"):
                    with self.subTest(job=job):
                        broken = expected.replace(f"  {job}:\n{creation}", f"  {job}:\n", 1)
                        path.write_text(broken, encoding="utf-8")
                        with self.assertRaisesRegex(AssertionError, "newly created tags"):
                            distribution.workflow(check=True)
                path.write_text(expected.replace(install, ""), encoding="utf-8")
                with self.assertRaisesRegex(AssertionError, "native installation"):
                    distribution.workflow(check=True)
                path.write_text(expected.replace(tagged_title, generated_title), encoding="utf-8")
                with self.assertRaisesRegex(AssertionError, "v-prefixed tag"):
                    distribution.workflow(check=True)


if __name__ == "__main__":
    unittest.main()
