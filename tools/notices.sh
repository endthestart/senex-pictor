#!/bin/sh
# Writes app/assets/notices.txt, the app's Third-Party Notices: every crate it ships on any
# platform with its licence texts (cargo-about), then notices the crates' licence fields miss:
# vendored LightCraft Classic's attribution, egui's Hack font, mimalloc's C sources, Inter and
# the camera colour matrices.
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
Adobe's published ColorMatrix values for the Nikon Z 6_2 and D700, as listed in LibRaw 0.22.1
(LGPL-2.1 or CDDL-1.0) and read through rawpy.
EOF
} >> "$out"
echo "wrote $out ($(wc -c < "$out") bytes)"
