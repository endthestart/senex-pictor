"""Private corpus inventory: independent metadata oracle and source byte identities."""
import argparse
import hashlib
import json
import subprocess
from collections import Counter
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    if args.output.exists():
        parser.error("output already exists")
    tags = ["Make", "Model", "ImageWidth", "ImageHeight", "ISO", "ExposureTime",
            "FNumber", "FocalLength", "Compression", "WhiteBalance", "DateTimeOriginal"]
    command = ["exiftool", "-j", "-n", "-ext", "NEF"]
    command += ["-" + tag for tag in tags] + [str(args.source)]
    rows = json.loads(subprocess.check_output(command))
    if not rows:
        parser.error("no NEF captures found")
    for row in rows:
        source = Path(row.pop("SourceFile"))
        digest = hashlib.sha256()
        with source.open("rb") as stream:
            for block in iter(lambda: stream.read(8 * 1024 * 1024), b""):
                digest.update(block)
        row.update(filename=source.name, bytes=source.stat().st_size,
                   sha256=digest.hexdigest())
    rows.sort(key=lambda row: row["filename"])
    result = {"schema": 1, "private": True, "hashing_complete": True,
              "metadata_oracle": "exiftool", "frame_count": len(rows),
              "models": dict(Counter(row.get("Model", "unknown") for row in rows)),
              "frames": rows}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open("x") as stream:
        json.dump(result, stream, indent=2)
        stream.write("\n")
    print(json.dumps({key: value for key, value in result.items() if key != "frames"}))


if __name__ == "__main__":
    main()
