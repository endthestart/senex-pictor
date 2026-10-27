"""Check the vendored source subset against its recorded content hashes."""
import hashlib
import json
from pathlib import Path

root = Path(__file__).resolve().parent.parent
manifest = json.loads((root / "docs/source-lock.json").read_text())
vendor = root / "vendor/lightcraft-classic"
expected = {entry["path"]: entry["sha256"] for entry in manifest["files"]}
actual = {str(path.relative_to(vendor)): hashlib.sha256(path.read_bytes()).hexdigest()
          for path in vendor.rglob("*") if path.is_file()}
if actual != expected:
    differences = [key for key in sorted(actual.keys() | expected.keys())
                   if actual.get(key) != expected.get(key)]
    raise SystemExit("Vendor integrity mismatch: " + ", ".join(differences))
print(f"Verified {len(expected)} vendored files")
