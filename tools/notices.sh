#!/bin/sh
# Writes app/assets/notices.txt, the app's Third-Party Notices: every crate it ships on any
# platform with its licence texts (cargo-about), then notices the crates' licence fields miss:
# vendored LightCraft Classic's attribution, egui's Hack font, mimalloc's C sources, Inter, the
# camera colour matrices and lensfun's lens database.
set -eu
cd "$(dirname "$0")/.."
out=app/assets/notices.txt
cargo about generate --fail -c tools/notices/about.toml --manifest-path app/Cargo.toml \
  tools/notices/notices.hbs -o "$out"
crate_dir() {
  cargo metadata --format-version 1 --manifest-path app/Cargo.toml |
    python3 -I -c 'import json, os, sys; print(next(os.path.dirname(p["manifest_path"]) for p in json.load(sys.stdin)["packages"] if p["name"] == sys.argv[1]))' "$1"
}
section() {
  printf '\n==============================================================================\n%s\n\n' "$1"
  cat "$2"
}
{
  section "LightCraft Classic (vendor/lightcraft-classic): notice" vendor/lightcraft-classic/NOTICE
  section "LightCraft Classic (vendor/lightcraft-classic): licence" vendor/lightcraft-classic/LICENSE
  section "Hack font, bundled by egui (epaint_default_fonts)" "$(crate_dir epaint_default_fonts)/fonts/Hack-Regular.txt"
  section "mimalloc C sources, built by libmimalloc-sys" "$(crate_dir libmimalloc-sys)/c_src/mimalloc/v2/LICENSE"
  section "Inter 4 (Regular, SemiBold), SIL Open Font License 1.1" app/assets/fonts/OFL-Inter.txt
  section "Camera colour matrices (data/colour-matrices.json)" /dev/stdin <<'EOF'
From the adobe_coeff table of LibRaw 0.22.1 (src/tables/colordata.cpp; most values are Adobe
DNG Converter's ColorMatrix), copyright 2019-2025 LibRaw LLC, with code from dcraw.c by Dave
Coffin. LibRaw is available under LGPL-2.1 or CDDL-1.0: https://www.libraw.org/
EOF
  section "Lens database (data/lensfun)" /dev/stdin <<'EOF'
lensfun's lens database (version_1, as published 2026-11-04 at https://lensfun.github.io/), by
the lensfun contributors, licensed under Creative Commons Attribution-ShareAlike 3.0 Unported:
https://creativecommons.org/licenses/by-sa/3.0/
Pictor ships the XML files unchanged and reads their vignetting entries.
EOF
} >> "$out"
echo "wrote $out ($(wc -c < "$out") bytes)"
