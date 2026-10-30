"""Validate native header reports against the independent private corpus inventory."""
import argparse
import json
import resource
import subprocess
import time
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("inventory", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=False)
    binary = Path("target/release/pictor").resolve()
    corpus = json.loads(args.inventory.read_text())
    results = {"schema": 1, "header_checks": [], "development_checks": [],
               "complete": False, "platform": subprocess.check_output(
                   ["uname", "-a"], text=True).strip()}
    started = time.monotonic()
    try:
        for frame in corpus["frames"]:
            path = args.source / frame["filename"]
            result = json.loads(subprocess.check_output([binary, "inspect", path]))
            expected = {"sha256": frame["sha256"], "bytes": frame["bytes"]}
            if any(result["source"][key] != value for key, value in expected.items()):
                raise ValueError(f"source identity mismatch: {path.name}")
            checks = {"model": frame["Model"], "iso": frame["ISO"],
                      "exposure_seconds": frame["ExposureTime"], "aperture": frame["FNumber"],
                      "focal_length_mm": frame["FocalLength"],
                      "sensor": [frame["ImageWidth"], frame["ImageHeight"]]}
            if any(result[key] != value for key, value in checks.items()):
                raise ValueError(f"metadata mismatch: {path.name}")
            results["header_checks"].append({"filename": path.name, "passed": True})
        print(f"Header/identity checks passed: {len(results['header_checks'])}", flush=True)
        frames = corpus["frames"]
        for index in [0, len(frames) // 2, len(frames) - 1]:
            frame = frames[index]
            destination = args.output / Path(frame["filename"]).stem
            start = time.monotonic()
            command = [binary, "develop", args.source / frame["filename"], destination]
            native = json.loads(subprocess.check_output(command))
            tags = json.loads(subprocess.check_output([
                "exiftool", "-j", "-n", "-ImageWidth", "-ImageHeight", "-BitsPerSample",
                "-SampleFormat", destination / "camera-linear.tif"]))[0]
            if [tags["ImageWidth"], tags["ImageHeight"]] != [native["width"], native["height"]]:
                raise ValueError("export dimensions mismatch")
            if str(tags["BitsPerSample"]) != "32 32 32" or str(tags["SampleFormat"]) != "3 3 3":
                raise ValueError(f"export is not float32 RGB: {tags}")
            report = json.loads((destination / "report.json").read_text())
            if report["source"]["sha256"] != frame["sha256"]:
                raise ValueError("developed source identity mismatch")
            if (destination / "INCOMPLETE").exists():
                raise ValueError("incomplete output")
            check = {"filename": frame["filename"], "passed": True,
                     "wall_seconds": time.monotonic() - start,
                     "native": native, "tiff_tags": tags,
                     "peak_child_rss_platform_units": resource.getrusage(resource.RUSAGE_CHILDREN).ru_maxrss}
            results["development_checks"].append(check)
            print(json.dumps({k: check[k] for k in ["filename", "passed", "wall_seconds"]}), flush=True)
        results["complete"] = True
    finally:
        results["total_seconds"] = time.monotonic() - started
        (args.output / "validation.json").write_text(json.dumps(results, indent=2) + "\n")


if __name__ == "__main__":
    main()
