"""Asset-free unit tests for ios/tools/audit_game_data.py."""
import hashlib
import importlib.util
import tempfile
import unittest
from pathlib import Path

PATH = Path(__file__).resolve().parents[1] / "tools" / "audit_game_data.py"
SPEC = importlib.util.spec_from_file_location("audit_game_data", PATH)
audit = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(audit)


class AuditTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name) / "Fallout 3"
        self.data = self.root / "Data"
        self.data.mkdir(parents=True)
        self.fixture("Fallout3.esm", b"TES4\x00\x00fixture")
        self.fixture("Fallout - Meshes.bsa", b"BSA\x00meshes")
        self.fixture("Fallout - Textures.bsa", b"BSA\x00textures")

    def fixture(self, name, content):
        (self.data / name).write_bytes(content)

    def test_detects_original_layout_from_installation_root(self):
        result = audit.inspect(self.root)
        self.assertTrue(result["ready_for_parser_tests"])
        self.assertEqual(result["dlc_detected"], [])
        self.assertEqual(len(result["files"]), 3)
        esm = next(f for f in result["files"] if f["name"] == "Fallout3.esm")
        self.assertEqual(esm["sha256"], hashlib.sha256(b"TES4\x00\x00fixture").hexdigest())
        self.assertNotIn(str(self.root), str(result))

    def test_detects_missing_required_file(self):
        (self.data / "Fallout - Meshes.bsa").unlink()
        result = audit.inspect(self.data)
        self.assertFalse(result["ready_for_parser_tests"])
        self.assertTrue(any("Meshes" in s for s in result["errors"]))

    def test_detects_wrong_archive_header(self):
        self.fixture("Fallout - Textures.bsa", b"FAKE")
        result = audit.inspect(self.root)
        self.assertFalse(result["ready_for_parser_tests"])
        self.assertTrue(any("Textures" in s for s in result["errors"]))

    def test_compares_hashes_for_transfer_integrity(self):
        baseline = audit.inspect(self.data)
        self.fixture("Fallout - Meshes.bsa", b"BSA\x00altered")
        now = audit.inspect(self.data)
        comparison = audit.compare(baseline, now)
        self.assertFalse(comparison["matches"])
        self.assertEqual(comparison["changed"], ["fallout - meshes.bsa"])

    def test_flags_modded_plugin_and_incomplete_dlc(self):
        self.fixture("Anchorage.esm", b"TES4dlc")
        self.fixture("MyMod.esp", b"TES4mod")
        result = audit.inspect(self.data)
        self.assertTrue(result["ready_for_parser_tests"])
        self.assertEqual(result["dlc_detected"], ["Anchorage"])
        self.assertTrue(result["warnings"])


if __name__ == "__main__":
    unittest.main()
