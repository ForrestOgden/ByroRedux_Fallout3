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
        self.root = Path(self.temp.name) / "Fallout: New Vegas"
        self.data = self.root / "Data"
        self.data.mkdir(parents=True)
        self.fixture("FalloutNV.esm", b"TES4\x00\x00fixture")
        self.fixture("Fallout - Meshes.bsa", b"BSA\x00meshes")
        self.fixture("Fallout - Textures.bsa", b"BSA\x00textures")

    def fixture(self, name, content):
        (self.data / name).write_bytes(content)

    def test_detects_original_layout_from_installation_root(self):
        result = audit.inspect(self.root)
        self.assertTrue(result["ready_for_parser_tests"])
        self.assertEqual(result["dlc_detected"], [])
        self.assertEqual(len(result["files"]), 3)
        esm = next(f for f in result["files"] if f["name"] == "FalloutNV.esm")
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

    def test_fallout_3_master_is_not_accepted_as_new_vegas(self):
        (self.data / "FalloutNV.esm").unlink()
        self.fixture("Fallout3.esm", b"TES4old")
        result = audit.inspect(self.data)
        self.assertFalse(result["ready_for_parser_tests"])
        self.assertTrue(any("FalloutNV.esm" in error for error in result["errors"]))

    def test_official_bonus_content_without_main_bsa_is_not_flagged(self):
        self.fixture("CaravanPack.esm", b"TES4bonus")
        self.fixture("GunRunnersArsenal.esm", b"TES4guns")
        result = audit.inspect(self.data)
        self.assertTrue(result["ready_for_parser_tests"])
        self.assertEqual(result["dlc_detected"], ["GunRunnersArsenal", "CaravanPack"])
        self.assertFalse(any("Main BSA" in warning for warning in result["warnings"]))

    def test_flags_modded_plugin_and_incomplete_dlc(self):
        self.fixture("DeadMoney.esm", b"TES4dlc")
        self.fixture("MyMod.esp", b"TES4mod")
        result = audit.inspect(self.data)
        self.assertTrue(result["ready_for_parser_tests"])
        self.assertEqual(result["dlc_detected"], ["DeadMoney"])
        self.assertTrue(result["warnings"])


if __name__ == "__main__":
    unittest.main()
