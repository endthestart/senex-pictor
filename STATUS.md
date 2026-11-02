# Pictor status

Updated 2026-11-01. The closing note for the owner's cleanup week; detail is in
[docs/status.md](docs/status.md) and the plan for the suite is in the
[SENEX photo suite roadmap](https://claude.ai/artifact/XdZEY2gDarpfCR2HzhvNSi).

Everything is on `main`, pushed: no other branches, tags, stashes or unpushed commits, so nothing
waits to merge. On 2026-11-01 all 61 workspace tests passed and formatting was clean; the UI tests
that need real run folders last ran on 2026-10-30. There is no CI and no public release.

Done: the stacker from RAW folder to finished image, the six-step desktop app, the design shared
with SENEX Corvus ([tokens v1](docs/DESIGN-TOKENS.md)), and the speed-ups (a BWCA run went from
154 s to 65 s, outputs byte-identical). The HDR bracket merger is a parked prototype
(`tools/proto/hdr.py`), not part of the app.

## Blocked on whom

| What | Waiting on |
| --- | --- |
| Running the app's six steps by hand on a real folder | Owner |
| Freeing the Work drive (below) | Owner's OK, with the NAS mounted to check the RAW copies |
| First release and the GitHub publish | Owner's decision, after the run-through |
| Before publishing: personal paths in tracked docs and history; renaming the Gitea repo (`andermic-star-stacker`) | Owner's decision on how; then this repo |
| A notarized build (today's is ad hoc signed) | Owner: an Apple Developer ID |
| The site's "Pictor source opens with its first release" | Website session, told when it becomes true |
| HDR: a tripod bracket at fixed ISO 64 | Parked until new work resumes |

## Star data

`/Volumes/Work/star-photos-data` is 636 GiB by `du`, on a drive that is 94% full (58 GiB free).
Nothing has been deleted. Some folders are APFS clones, so deleting frees less than these sizes.

| Group | GiB | Verdict |
| --- | --- | --- |
| RAW and edited copies, manifests, sequence checks, colour matrices (with the `hdr-walk` RAWs) | 64 | Stay until checked against the NAS (not mounted on 2026-11-01); the test deck needs the RAWs |
| `validation/`: `datasets` and the four `*-validation` runs that docs/status.md cites, small reports | 10 | Stay |
| `app-test/`: `yellowstone-ui`, `yellowstone-correct`, `yellowstone-stopped` | 22 | Stay for the UI tests |
| `*/work*`: Python prototype caches, kept only as comparison evidence | 181 | Can go; the app does not use them |
| `*/process`: runs of the pipeline from 2026-10-26/27 | 173 | Can go; `pictor process` remakes them |
| `validation/`: `iter*` and the `ingest-*`, `bwca-36`, `yellowstone-lit` baselines | 103 | Can go |
| `*/rust`: Rust port parity stores | 63 | Can go |
| `app-test/`: `bwca`, `yellowstone`; `hdr-walk/proto`; `yellowstone-2013-08-27/hero` | 21 | Can go |

That is about 96 GiB to keep and 540 GiB that can go.

## Move list

| Thing | Today | After the move off the Mac Pro |
| --- | --- | --- |
| Source | Gitea | The same; any clone builds |
| macOS package, Metal and UI tests | Mac Pro | Needs a Mac: the Mac Pro woken on demand, or the owner's laptop. Needs rustup stable with both macOS targets, Xcode's command-line tools and `cargo-about` |
| Compile checks, engine tests | Mac Pro | A non-Mac machine should do. The Linux and Windows checks are clean from the Mac; the tests have not run on Linux |
| CI | None | Not set up. Gitea Actions could run fmt, clippy and engine tests; UI tests need a GPU and packaging a macOS runner |
| Star data | Work drive, an external PCIe SSD | Stays on that drive; full-sequence runs happen wherever it is plugged in |
| Originals | NAS | The same |
| Lens database for `flat` | darktable.app's lensfun | darktable on the new Mac, or `PICTOR_LENSFUN` |
| `.sources` (upstream clones and oracles, 500 MB) | Repo folder, ignored by git | Copy it to whichever machine refreshes the vendored code |
| Local caches (`target` 41 GiB, `.cache`, `.venv`, `.venv-sam`, `.models`, `outputs`) | Repo folder, ignored by git | Do not move; rebuild `target`, and `.venv` only for `tools/icon.py` and the prototypes |
