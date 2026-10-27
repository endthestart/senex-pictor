"""Compare decoded sensor levels with oracle camera databases (ADR 0007, oracle-only)."""
import argparse
import hashlib
import json
import subprocess
import xml.etree.ElementTree as ET
from collections import defaultdict
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
RAW_SUFFIXES = {".nef", ".cr2", ".cr3", ".arw", ".dng", ".raf", ".orf", ".rw2", ".pef"}


def oracle_files():
    lock = json.loads((ROOT / "docs/oracle-lock.json").read_text())
    files = {}
    for oracle in lock["oracles"]:
        for entry in oracle["files"]:
            path = ROOT / entry["local"]
            if hashlib.sha256(path.read_bytes()).hexdigest() != entry["sha256"]:
                raise SystemExit(f"oracle file does not match docs/oracle-lock.json: {path}")
            files[entry["path"]] = path
    return files


def strip_comments(text):
    out, i, in_string = [], 0, False
    while i < len(text):
        c = text[i]
        if in_string:
            out.append(c)
            if c == "\\":
                out.append(text[i + 1])
                i += 1
            elif c == '"':
                in_string = False
        elif c == '"':
            in_string = True
            out.append(c)
        elif text.startswith("//", i):
            i = text.find("\n", i) - 1 if "\n" in text[i:] else len(text)
        elif text.startswith("/*", i):
            i = text.index("*/", i) + 1
        else:
            out.append(c)
        i += 1
    return "".join(out)


def camconst(path):
    data = json.loads(strip_comments(path.read_text(encoding="utf-8")))
    entries = {}
    for entry in data["camera_constants"]:
        names = entry["make_model"]
        for name in names if isinstance(names, list) else [names]:
            entries[name.lower()] = entry
    return entries


def camconst_white(entry, iso):
    white = entry.get("ranges", {}).get("white")
    if not isinstance(white, list):
        return white
    for rule in white:
        isos = rule["iso"] if isinstance(rule["iso"], list) else [rule["iso"]]
        if iso in isos:
            return rule["levels"]
    return None


def rawspeed(path):
    entries = defaultdict(list)
    for camera in ET.parse(path).getroot().iter("Camera"):
        crop = camera.find("Crop")
        areas = camera.find("BlackAreas")
        entries[(camera.get("make"), camera.get("model"))].append({
            "mode": camera.get("mode"),
            "supported": camera.get("supported", "yes"),
            "crop": None if crop is None else {k: int(v) for k, v in crop.attrib.items()},
            "sensors": [dict(s.attrib) for s in camera.findall("Sensor")],
            "black_areas": [] if areas is None else [{"kind": a.tag} | dict(a.attrib) for a in areas],
        })
    return entries


def rawspeed_sensor(sensors, iso):
    for sensor in sensors:
        if iso is not None and str(iso) in sensor.get("iso_list", "").split():
            return sensor
        if "iso_min" in sensor and iso is not None and int(sensor["iso_min"]) <= iso <= int(sensor.get("iso_max", 10**9)):
            return sensor
    defaults = [s for s in sensors if not ({"iso_list", "iso_min", "iso_max"} & s.keys())]
    return defaults[0] if defaults else None


def decode(binary, path):
    run = subprocess.run([binary, "levels", path], capture_output=True, text=True)
    if run.returncode != 0:
        return {"file": str(path), "error": run.stderr.strip()}
    return json.loads(run.stdout) | {"file": str(path)}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    parser.add_argument("sources", type=Path, nargs="+")
    args = parser.parse_args()
    if args.output.exists():
        parser.error("output already exists")
    files = oracle_files()
    rt, rs = camconst(files["rtengine/camconst.json"]), rawspeed(files["data/cameras.xml"])
    paths = sorted(p for s in args.sources for p in ([s] if s.is_file() else s.rglob("*"))
                   if p.suffix.lower() in RAW_SUFFIXES)
    binary = ROOT / "target/release/star-photos"
    with ThreadPoolExecutor(4) as pool:
        decoded = list(pool.map(lambda p: decode(binary, p), paths))
    groups = defaultdict(list)
    for d in decoded:
        if "error" not in d:
            groups[(d["make"], d["model"], d["iso"], d["bits"])].append(d)
    report = {"oracles": json.loads((ROOT / "docs/oracle-lock.json").read_text())["oracles"],
              "decode_errors": [d for d in decoded if "error" in d], "groups": []}
    for (make, model, iso, bits), frames in sorted(groups.items(), key=lambda g: tuple(map(str, g[0]))):
        blacks = [sum(f["black"]["values"]) / len(f["black"]["values"]) for f in frames]
        whites = sorted({w for f in frames for w in f["white"]})
        maxima = [f["active_maximum"] for f in frames]
        rt_entry = rt.get((model or "").lower()) or rt.get(f"{make} {model}".lower())
        # Star Photos decodes CFA data only, so sRAW modes and other bit depths do not apply.
        rs_modes = [m | {"sensor": rawspeed_sensor(m["sensors"], iso)} for m in rs.get((make, model), [])
                    if m["mode"] is None or m["mode"].startswith(f"{bits}bit")]
        flags = []
        if all(m < min(whites) for m in maxima):
            flags.append("decoded white is above every stored sample (inferred or fallback)")
        rt_white = rt_entry and camconst_white(rt_entry, iso)
        if isinstance(rt_white, (int, float)) and rt_white not in whites:
            flags.append(f"camconst white {rt_white} differs from decoded {whites}")
        for m in rs_modes:
            sensor = m["sensor"]
            measured = bool(m["black_areas"])
            if sensor and not measured and abs(float(sensor["black"]) - sum(blacks) / len(blacks)) > 2:
                flags.append(f"rawspeed {m['mode'] or 'default'} black {sensor['black']} differs from decoded")
            if sensor and float(sensor["white"]) not in whites:
                flags.append(f"rawspeed {m['mode'] or 'default'} white {sensor['white']} differs from decoded {whites}")
        group = {"make": make, "model": model, "iso": iso, "bits": bits, "frames": len(frames),
                 "decoded_black_mean": [min(blacks), max(blacks)], "decoded_white": whites,
                 "active_maximum": [min(maxima), max(maxima)],
                 "active_area": sorted({json.dumps(f["active_area"], sort_keys=True) for f in frames}),
                 "rawspeed": rs_modes,
                 "camconst": None if rt_entry is None else {
                     "white_for_iso": camconst_white(rt_entry, iso),
                     "raw_crop": rt_entry.get("raw_crop"), "masked_areas": rt_entry.get("masked_areas"),
                     "has_levels": "ranges" in rt_entry},
                 "flags": flags}
        report["groups"].append(group)
        print(f"{model} ISO {iso} {bits}-bit: {len(frames)} frames; black {min(blacks):.2f}..{max(blacks):.2f}; "
              f"white {whites}; max sample {min(maxima):.0f}..{max(maxima):.0f}; "
              f"rawspeed {[(m['mode'], m['sensor'], len(m['black_areas'])) for m in rs_modes]}; "
              f"camconst white {group['camconst'] and group['camconst']['white_for_iso']}")
        for flag in flags:
            print(f"  ! {flag}")
    print(f"decode errors: {len(report['decode_errors'])}")
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open("x") as stream:
        json.dump(report, stream, indent=2)
        stream.write("\n")


if __name__ == "__main__":
    main()
