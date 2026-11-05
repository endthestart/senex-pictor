"""Write data/colour-matrices.json from LibRaw's adobe_coeff table (src/tables/colordata.cpp).

Usage: python3 -I tools/colour_matrices.py COLORDATA_CPP LIBRAW_VERSION

LibRaw matches a model to the first entry whose name is a prefix of it; Pictor matches whole
names only, so a camera newer than the table fails instead of borrowing an older model's matrix.
Each entry keeps LibRaw's name plus the cameras that LibRaw's own comments say share it.
"""
import json
import re
import sys
from pathlib import Path

ALSO = {
    ("Canon", "EOS 5DS"): ["EOS 5DS R"],
    ("Epson", "R-D1"): ["R-D1s", "R-D1x"],
    ("Fujifilm", "F5"): ["F500EXR", "F505EXR", "F550EXR"],
    ("Fujifilm", "F6"): ["F600EXR", "F605EXR", "F660EXR"],
    ("Fujifilm", "F77"): ["F770EXR", "F775EXR"],
    ("Fujifilm", "F7"): ["F700", "F710EXR"],
    ("Fujifilm", "F8"): ["F800EXR"],
    ("Fujifilm", "GFX 100"): ["GFX 100S", "GFX100S", "GFX 100 IR"],
    ("Fujifilm", "GFX 50"): ["GFX 50S", "GFX 50R", "GFX 50S II"],
    ("Fujifilm", "HS2"): ["HS20EXR", "HS22EXR"],
    ("Fujifilm", "HS3"): ["HS30EXR", "HS33EXR", "HS35EXR"],
    ("Fujifilm", "S5200"): ["S5600"],
    ("Fujifilm", "S6"): ["S6000fd", "S6500fd"],
    ("Fujifilm", "S9000"): ["S9500"],
    ("Fujifilm", "S9100"): ["S9600"],
    ("Fujifilm", "S20"): ["S200EXR", "S205EXR"],
    ("Fujifilm", "XQ"): ["XQ1", "XQ2"],
    ("Fujifilm", "X-T1"): ["X-T1IR", "X-T1 IR", "X-T10"],
    ("Fujifilm", "X-T2"): ["X-T20"],
    ("Fujifilm", "X-T3"): ["X-T30", "X-T30 II"],
    ("Fujifilm", "X-T5"): ["X-T50"],
    ("Leica", "M10"): ["M10-D", "M10-P"],
    ("Leica", "M (Typ 2"): ["M (Typ 240)", "M (Typ 262)", "M-D (Typ 262)"],
    ("Leica", "S"): ["S-E (Typ 006)", "S (Typ 006)"],
    ("Leica", "T"): ["TL", "T (Typ 701)"],
    ("Leica", "X"): ["X (Typ 113)", "X-U (Typ 113)", "XV", "X Vario (Typ 107)"],
    ("Nikon", "1 "): ["1 J1", "1 J2", "1 S1", "1 V1"],
    ("Nikon", "COOLPIX 2100"): ["E2100"],
    ("Nikon", "COOLPIX 3200"): ["E3200"],
    ("Nikon", "COOLPIX 4300"): ["E4300"],
    ("Nikon", "COOLPIX 5400"): ["E5400"],
    ("Nikon", "COOLPIX 8400"): ["E8400"],
    ("Nikon", "COOLPIX 8700"): ["E8700"],
    ("Nikon", "COOLPIX 8800"): ["E8800"],
    ("Nikon", "D300"): ["D300s"],
    ("Nikon", "D800"): ["D800E"],
    ("Nikon", "D70"): ["D70s"],
    ("Nikon", "D2H"): ["D2Hs"],
    ("Nikon", "D2X"): ["D2Xs"],
    ("Nikon", "D4"): ["D4S"],
    ("Nikon", "Z 6"): ["Z 6_2"],
    ("Nikon", "Z 7"): ["Z 7_2"],
    ("Olympus", "E-20"): ["E-20,E-20N,E-20P"],
    ("Olympus", "E-M10"): ["E-M10 Mark II", "E-M10 Mark III"],
    ("Olympus", "SP-3"): ["SP310", "SP320"],
    ("Olympus", "TG-"): ["TG-5", "TG-6", "TG-7"],
    ("Pentax", "K1"): ["K100D", "K100D Super", "K110D"],
    ("Pentax", "K-1"): ["K-1 Mark II"],
    ("Pentax", "K-3"): ["K-3 II"],
    ("Pentax", "K-5 II"): ["K-5 IIs"],
    ("Panasonic", "DMC-CM1"): ["DMC-CM10"],
    ("Samsung", "Galaxy S6"): ["Galaxy S6 Edge"],
    ("Samsung", "Galaxy S7"): ["Galaxy S7 Edge"],
    ("Samsung", "Galaxy S8"): ["Galaxy S8+"],
    ("Samsung", "Galaxy S9"): ["Galaxy S9+"],
    ("Samsung", "NX30"): ["NX300", "NX300M"],
    ("Samsung", "NX2"): ["NX20", "NX200", "NX210"],
    ("Samsung", "NX10"): ["NX100"],
    ("Sony", "DSC-HX9"): ["DSC-HX95", "DSC-HX99"],
    ("Sony", "ZV-1"): ["ZV-1B", "ZV-1M2"],
    ("Sony", "DSC-RX100M"): ["DSC-RX100M2", "DSC-RX100M3", "DSC-RX100M4", "DSC-RX100M5"],
    ("Sony", "DSC-RX10"): ["DSC-RX10M2", "DSC-RX10M3"],
    ("Sony", "DSC-RX0"): ["DSC-RX0M2"],
    ("Sony", "DSLR-A2"): ["DSLR-A200", "DSLR-A230"],
    ("Sony", "DSLR-A3"): ["DSLR-A350", "DSLR-A380", "DSLR-A390"],
    ("Sony", "DSLR-A5"): ["DSLR-A550", "DSLR-A560"],
    ("Sony", "ILCE-7RM4"): ["ILCE-7RM4A"],
    ("Sony", "ILCE-7RM3"): ["ILCE-7RM3A"],
    ("Sony", "ILCE-7S"): ["ILCE-7SM2"],
    ("Sony", "ILCE-7"): ["ILCE-7M2"],
    ("Sony", "ILCE-9"): ["ILCE-9M2"],
    ("Sony", "ILCE-"): ["ILCE-3000", "ILCE-5000", "ILCE-5100", "ILCE-6000", "ILCE-QX1"],
    ("Sony", "NEX-"): ["NEX-C3", "NEX-F3", "NEX-VG20"],
    ("Sony", "SLT-A5"): ["SLT-A57", "SLT-A58"],
}

# Other names the files write for those cameras (regional names and spellings), checked against
# rawspeed's camera list where it lists them (an oracle; nothing of it ships).
REGIONAL = {
    ("Canon", "EOS 300D"): ["EOS DIGITAL REBEL", "EOS Kiss Digital"],
    ("Canon", "EOS 350D"): ["EOS DIGITAL REBEL XT", "EOS Kiss Digital N"],
    ("Canon", "EOS 400D"): ["EOS DIGITAL REBEL XTi", "EOS Kiss Digital X"],
    ("Canon", "EOS 450D"): ["EOS DIGITAL REBEL XSi", "EOS Kiss X2"],
    ("Canon", "EOS 1000D"): ["EOS DIGITAL REBEL XS", "EOS Kiss F"],
    ("Canon", "EOS 500D"): ["EOS REBEL T1i", "EOS Kiss X3"],
    ("Canon", "EOS 550D"): ["EOS REBEL T2i", "EOS Kiss X4"],
    ("Canon", "EOS 600D"): ["EOS REBEL T3i", "EOS Kiss X5"],
    ("Canon", "EOS 1100D"): ["EOS REBEL T3", "EOS Kiss X50"],
    ("Canon", "EOS 650D"): ["EOS REBEL T4i", "EOS Kiss X6i"],
    ("Canon", "EOS 700D"): ["EOS REBEL T5i", "EOS Kiss X7i"],
    ("Canon", "EOS 100D"): ["EOS REBEL SL1", "EOS Kiss X7"],
    ("Canon", "EOS 1200D"): ["EOS REBEL T5", "EOS Kiss X70"],
    ("Canon", "EOS 750D"): ["EOS Rebel T6i", "EOS Kiss X8i"],
    ("Canon", "EOS 760D"): ["EOS Rebel T6s", "EOS 8000D"],
    ("Canon", "EOS 1300D"): ["EOS Rebel T6", "EOS Kiss X80"],
    ("Canon", "EOS 800D"): ["EOS Rebel T7i", "EOS Kiss X9i"],
    ("Canon", "EOS 77D"): ["EOS 9000D"],
    ("Canon", "EOS 200D"): ["EOS Rebel SL2", "EOS Kiss X9"],
    ("Canon", "EOS 1500D"): ["EOS 2000D", "EOS Rebel T7", "EOS Kiss X90"],
    ("Canon", "EOS 3000D"): ["EOS 4000D", "EOS Rebel T100"],
    ("Canon", "EOS 250D"): ["EOS Rebel SL3", "EOS Kiss X10", "EOS 200D II"],
    ("Canon", "EOS 850D"): ["EOS Rebel T8i", "EOS Kiss X10i"],
    ("Canon", "EOS M50"): ["EOS Kiss M"],
    ("Canon", "EOS M50 Mark II"): ["EOS Kiss M2"],
    ("Fujifilm", "GFX 100 II"): ["GFX100 II"],
    ("Fujifilm", "GFX 100S II"): ["GFX100S II"],
    ("Fujifilm", "GFX 50"): ["GFX50S II"],
    ("Panasonic", "DC-G99"): ["DC-G90", "DC-G91", "DC-G95"],
    ("Panasonic", "DC-GF9"): ["DC-GX800", "DC-GX850"],
    ("Panasonic", "DMC-GX85"): ["DMC-GX80", "DMC-GX7MK2"],
    ("Panasonic", "DC-GX9"): ["DC-GX7MK3"],
    ("Panasonic", "DC-GF10"): ["DC-GF90", "DC-GX880"],
    ("Panasonic", "DC-FZ80"): ["DC-FZ82", "DC-FZ85"],
    ("Panasonic", "DMC-FZ2500"): ["DMC-FZ2000"],
    ("Panasonic", "DC-FZ1000M2"): ["DC-FZ10002"],
    ("Panasonic", "DMC-FZ300"): ["DMC-FZ330"],
    ("Panasonic", "DMC-LX9"): ["DMC-LX10", "DMC-LX15"],
    ("Panasonic", "DC-ZS200"): ["DC-TZ200", "DC-TZ202"],
    ("Panasonic", "DC-ZS70"): ["DC-TZ90", "DC-TZ91", "DC-TZ92", "DC-TZ93"],
    ("Panasonic", "DC-ZS80"): ["DC-TZ95", "DC-TZ96"],
    ("Panasonic", "DMC-ZS100"): ["DMC-TZ100", "DMC-TZ101", "DMC-TZ110", "DMC-TX1"],
    ("Panasonic", "DMC-ZS60"): ["DMC-TZ80", "DMC-TZ81", "DMC-TZ85"],
    ("Panasonic", "DMC-ZS50"): ["DMC-TZ70", "DMC-TZ71"],
    ("Panasonic", "DMC-ZS40"): ["DMC-TZ60", "DMC-TZ61"],
    ("Pentax", "K-5 II"): ["K-5 II s"],
    ("Sony", "ILCE-6100"): ["ILCE-6100A"],
    ("Sony", "ILCE-6400"): ["ILCE-6400A"],
}

ENTRY = re.compile(r'\{\s*LIBRAW_CAMERAMAKER_(\w+)\s*,\s*"([^"]*)"\s*,\s*[-\w]+\s*,\s*[-\w]+\s*,'
                   r'\s*\{([^}]*)\}\s*\}')


def table(source):
    body = source[source.index("} table[] = {"):source.index("// clang-format on")]
    body = re.sub(r"#ifdef USE_6BY9RPI.*?#endif", "", body, flags=re.S)
    body = re.sub(r"/\*.*?\*/|//[^\n]*", "", body, flags=re.S)
    for m in ENTRY.finditer(body):
        yield m.group(1), m.group(2), [int(x) for x in m.group(3).split(",") if x.strip()]


def main(path, version):
    cameras, seen = [], set()
    for maker, name, trans in table(Path(path).read_text(encoding="utf-8")):
        if not trans or not trans[0] or any(trans[9:]):
            continue  # no matrix, or four colours
        also = ALSO.pop((maker, name), []) + REGIONAL.pop((maker, name), [])
        own = [] if not name or name.endswith((" ", "-")) else [name]
        models = [n for n in [*own, *also] if (maker, n.lower()) not in seen]
        if not models:
            continue
        seen.update((maker, n.lower()) for n in models)
        rows = [[v / 10000 for v in trans[r * 3:r * 3 + 3]] for r in range(3)]
        cameras.append({"maker": maker, "models": models, "xyz_to_camera": rows})
    if ALSO or REGIONAL:
        raise SystemExit(f"entries not in the table: {sorted([*ALSO, *REGIONAL])}")
    source = (f"Adobe ColorMatrix (XYZ to camera, D65) from LibRaw {version}'s adobe_coeff table, "
              "LGPL-2.1 or CDDL-1.0")
    lines = ",\n".join("    " + json.dumps(c, separators=(", ", ": ")) for c in cameras)
    out = Path(__file__).resolve().parent.parent / "data/colour-matrices.json"
    out.write_text(f'{{\n  "source": {json.dumps(source)},\n  "cameras": [\n{lines}\n  ]\n}}\n')
    print(f"wrote {out} ({len(cameras)} matrices)")


if __name__ == "__main__":
    if len(sys.argv) != 3:
        raise SystemExit(__doc__)
    main(*sys.argv[1:])
