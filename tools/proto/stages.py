"""Cached pipeline stages shared by the iteration runners."""
import json
import pickle
import time


def log(report, key, value):
    report[key] = value
    print(f"{key}: {json.dumps(value, default=float)[:400]}", flush=True)


def run(functions, ctx, stage_dir, selected):
    """Run `selected` stages in order; load the cached result of every other earlier stage."""
    stage_dir.mkdir(parents=True, exist_ok=True)
    report_path = stage_dir / "report.json"
    ctx["report"] = json.loads(report_path.read_text()) if report_path.exists() else {}
    ctx["stage_dir"] = stage_dir
    for name, function in functions.items():
        path = stage_dir / f"{name}.pkl"
        if name in selected:
            start = time.time()
            ctx[name] = function(ctx)
            ctx["report"].setdefault("seconds", {})[name] = round(time.time() - start, 1)
            with path.open("wb") as stream:
                pickle.dump(ctx[name], stream, protocol=pickle.HIGHEST_PROTOCOL)
            report_path.write_text(json.dumps(ctx["report"], indent=2, default=float))
        elif path.exists():
            with path.open("rb") as stream:
                ctx[name] = pickle.load(stream)
