# Pictor status

Updated 2026-11-05. The closing note for the owner's cleanup week; detail is in
[docs/status.md](docs/status.md) and the plan for the suite is in the
[SENEX photo suite roadmap](https://claude.ai/artifact/XdZEY2gDarpfCR2HzhvNSi).

Everything is on `main`, pushed to Gitea and to [GitHub](https://github.com/endthestart/senex-pictor):
no other branches, stashes or unpushed commits, so nothing waits to merge. On 2026-11-05 all 64 workspace tests passed with all five UI test folders set, and
formatting and lints were clean. There is no CI. The first public release is the 0.1.0 preview,
and its limits are at the top of the [README](README.md).

Done: the stacker from RAW folder to finished image, the six-step desktop app, the design shared
with SENEX Corvus ([tokens v1](docs/DESIGN-TOKENS.md)), the speed-ups (a BWCA run went from
154 s to 65 s, outputs byte-identical), and, since 0.1.0 and not yet released, built-in colour
matrices for about 800 cameras and a built-in lens database. The HDR bracket merger is a parked prototype
(`tools/proto/hdr.py`), not part of the app.

## Blocked on whom

| What | Waiting on |
| --- | --- |
| Running the app's six steps by hand on a real folder | Owner |
| Renaming the Gitea repo (`andermic-star-stacker`) | Owner |
| A notarized build (today's is ad hoc signed) | Owner: an Apple Developer ID |
| The site's "Pictor source opens with its first release", now out of date | Website session, told 2026-11-02 |
| HDR: a tripod bracket at fixed ISO 64 | Parked until new work resumes |

## Star data

The private data (`$STAR_DATA`, on the Work drive) is 104 GiB by `du`, on a drive that is 38% full (about 583 GiB free). On
2026-11-02 the 3,180 RAW and edited copies in the six manifests matched their NAS originals by size
and SHA-256, then about 540 GiB of caches and old runs were deleted. The `hdr-walk` RAWs have no
manifest and were not checked.

| What stays | GiB | Why |
| --- | --- | --- |
| RAW and edited copies, manifests, sequence checks, colour matrices (with the `hdr-walk` RAWs) | 64 | The test deck needs the RAWs |
| `validation/`: `datasets` and the four `*-validation` runs, small reports | 10 | docs/status.md cites them |
| `app-test/`: `yellowstone-ui`, `yellowstone-correct`, `yellowstone-stopped`, `bwca-stack` | 31 | The UI tests use them |

## Move list

| Thing | Today | After the move off the Mac Pro |
| --- | --- | --- |
| Source | Gitea (`origin`) and public GitHub (`github`) | The same; any clone builds |
| macOS package, Metal and UI tests | Mac Pro | Needs a Mac: the Mac Pro woken on demand, or the owner's laptop. Needs rustup stable with both macOS targets, Xcode's command-line tools and `cargo-about` |
| Compile checks, engine tests | Mac Pro | A non-Mac machine should do. The Linux and Windows checks are clean from the Mac; the tests have not run on Linux |
| CI | None | Not set up. Gitea Actions could run fmt, clippy and engine tests; UI tests need a GPU and packaging a macOS runner |
| Star data | Work drive, an external PCIe SSD | Stays on that drive; full-sequence runs happen wherever it is plugged in |
| Originals | NAS | The same |
| `.sources` (upstream clones and oracles, 500 MB) | Repo folder, ignored by git | Copy it to whichever machine refreshes the vendored code |
| Local caches (`target` 41 GiB, `.cache`, `.venv`, `.venv-sam`, `.models`, `outputs`) | Repo folder, ignored by git | Do not move; rebuild `target`, and `.venv` only for `tools/icon.py` and the prototypes |
