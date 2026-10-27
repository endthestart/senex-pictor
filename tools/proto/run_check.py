"""Sequence check (sequence.py) for a folder of RAW frames; prints runs and the recommendation.

Usage: .venv/bin/python tools/proto/run_check.py RAW_DIR [OUT_JSON]
"""
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from proto import sequence  # noqa: E402

if __name__ == "__main__":
    raw = Path(sys.argv[1])
    paths = sorted(p for p in raw.iterdir() if p.suffix.lower() in (".nef", ".dng") and not p.name.startswith("."))
    report = sequence.check(paths)
    if len(sys.argv) > 2:
        Path(sys.argv[2]).write_text(json.dumps(report, indent=1))
    for r in report["runs"]:
        print(f"{r['first']}–{r['last']}: {r['frames']} frames after {r['starts_after']} (step scatter {r['step_scatter_px']} px); usable {r['usable_frames']}"
              f" ({r['usable_first']}–{r['usable_last']}); overexposed {len(r['overexposed'])}; lit {len(r['lit'])} {r['lit'][:6]}; drift {r['drift_px']} px")
    print("recommended:", report["recommended"]["usable_first"], "to", report["recommended"]["usable_last"])
