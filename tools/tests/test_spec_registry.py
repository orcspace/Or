import importlib.util
import json
from pathlib import Path
import shutil
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("registry", ROOT / "tools/spec_registry.py")
registry = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(registry)


class RegistryTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        shutil.copytree(ROOT / "docs", self.root / "docs")
        shutil.copy2(ROOT / "AGENTS.md", self.root / "AGENTS.md")

    def test_source_edit_cannot_pass_with_old_traceability(self):
        path = self.root / "AGENTS.md"
        path.write_text(path.read_text(encoding="utf-8").replace("build 22000+", "build 99999+", 1), encoding="utf-8")
        self.assertTrue(any("Source drift" in e for e in registry.validate(self.root)))

    def test_new_unregistered_requirement_fails(self):
        path = self.root / "docs/MCP.md"
        with path.open("a", encoding="utf-8") as handle:
            handle.write("\n- An untracked safety requirement.\n")
        self.assertTrue(any("exactly one" in e for e in registry.validate(self.root)))

    def test_fake_done_without_evidence_fails(self):
        path = self.root / "docs/requirements.json"
        records = json.loads(path.read_text(encoding="utf-8"))
        records[0]["status"] = "done"
        records[0]["evidence"] = []
        path.write_text(json.dumps(records), encoding="utf-8")
        self.assertTrue(any("done without evidence" in e for e in registry.validate(self.root)))

    def test_missing_verification_route_fails(self):
        path = self.root / "docs/requirements.json"
        records = json.loads(path.read_text(encoding="utf-8"))
        records[0]["tests"] = []
        path.write_text(json.dumps(records), encoding="utf-8")
        self.assertTrue(any("verification method" in e for e in registry.validate(self.root)))


if __name__ == "__main__":
    unittest.main()
