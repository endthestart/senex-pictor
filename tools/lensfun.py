"""Copy lensfun's database into data/lensfun/ and write src/lens/bundled.rs, which builds it in.

Usage: python3 -I tools/lensfun.py VERSION_1_DIR PUBLISHED

VERSION_1_DIR is lensfun.github.io/db/version_1.tar.bz2 unpacked; PUBLISHED is its date
(YYYY-MM-DD). The XML files are copied unchanged (CC BY-SA 3.0).
"""
import re
import shutil
import sys
from pathlib import Path


def main(source, published):
    if not re.fullmatch(r"\d{4}-\d{2}-\d{2}", published):
        raise SystemExit(f"PUBLISHED is not a date: {published}")
    root = Path(__file__).resolve().parent.parent
    files = sorted(Path(source).glob("*.xml"))
    if not files:
        raise SystemExit(f"no XML files in {source}")
    data = root / "data/lensfun"
    shutil.rmtree(data, ignore_errors=True)
    data.mkdir(parents=True)
    for f in files:
        shutil.copyfile(f, data / f.name)
    entries = "\n".join(f'    ("{f.name}", include_str!("../../data/lensfun/{f.name}")),'
                        for f in files)
    (root / "src/lens/bundled.rs").write_text(
        "//! lensfun's database (CC BY-SA 3.0), built in; written by `tools/lensfun.py`.\n\n"
        f"/// When lensfun published this copy.\n"
        f'pub const PUBLISHED: &str = "{published}";\n\n'
        "/// Every XML file of the database, by name.\n"
        f"pub const FILES: &[(&str, &str)] = &[\n{entries}\n];\n")
    print(f"copied {len(files)} files to {data}")


if __name__ == "__main__":
    if len(sys.argv) != 3:
        raise SystemExit(__doc__)
    main(*sys.argv[1:])
