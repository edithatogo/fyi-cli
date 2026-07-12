import json
from pathlib import Path

from scripts.create_release_manifest import create_manifest


def test_release_manifest_is_sorted_and_content_addressed(tmp_path: Path):
    (tmp_path / "z.bin").write_bytes(b"z")
    (tmp_path / "a.bin").write_bytes(b"a")
    manifest = create_manifest(tmp_path, "0.1.2", "abc123")
    assert [item["name"] for item in manifest["files"]] == ["a.bin", "z.bin"]
    assert manifest["files"][0]["sha256"] == "ca978112ca1bbdcafac231b39a23dc4da786eff8147c4e72b9807785afee48bb"
    assert json.dumps(manifest, sort_keys=True)
