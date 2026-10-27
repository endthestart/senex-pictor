# Workflow UX of general astrophotography stacking suites (Siril, DeepSkyStacker, APP, PixInsight WBPP/Blink/SubframeSelector, Affinity): lessons for a nightscape stacker (as of Oct 2026)

Scope: how deep-sky suites guide users from raw frames to a stack, how they show quality and rejection decisions, calibration matching, display stretch, long-run feedback, and user friction. Dedicated nightscape stackers (Sequator, SLS, StarStaX, Satrelle, NightScape Lab, Photoshop/Lightroom, Luminar) are covered in `nightscape_stackers_ux.md` and not repeated. Cloudy Nights (403), AstroBin's new forum (JS-only), Adam Block forum (DNS failure), web.archive.org and the PixInsight reference docs (404/403) could not be fetched; PixInsight UI facts therefore come from official release mails, a blog walkthrough and forum snippets. Items marked "(search snippet)" were seen only in search-result excerpts, not in a fetched page.

Current versions found: Siril 1.4.0 stable 5 Dec 2025, 1.4.3 7 May 2026, 1.4.4 listed 17 Jun 2026, 1.5 in development ([Siril 1.4.0](https://siril.org/download/2025-12-05-siril-1-4-0/), [Siril 1.4.3](https://siril.org/download/2026-05-06-siril-1-4-3/)). DeepSkyStacker 6.2.2 stable (18 Jul), 6.2.3 Beta 3 (2 Oct), installers now listed for Windows, macOS and Linux ([DSS releases](https://github.com/deepskystacker/DSS/releases)). APP still 2.0.0 beta: beta46 (Jun 2026, "getting close" to 2.0.0 stable and a full manual), beta47 (3 Oct 2026), beta48 (9 Oct 2026) ([beta46](https://www.astropixelprocessor.com/community/release-information/astro-pixel-processor-2-0-0-beta46-release-notes/), [preview-stretch thread](https://www.astropixelprocessor.com/community/rfcs-request-for-changes/preview-stretch-bug/), [quality measures thread banner](https://www.astropixelprocessor.com/community/main-forum/app-quality-measures/)). PixInsight 1.9.3 (28 Feb 2025) ([DIYPhotography](https://www.diyphotography.net/?p=292890)); WBPP 2.9.0 (14 Jan 2026) added Frame Selection; WBPP 3.1.0 (18 Sep 2026) ([PixInsight announcements](https://pixinsight.net/dev/index.php?threads%2Fwbpp-2-9-0-release-announcement.7=), [Weber](https://blog.briangweber.com/wbpp-frame-selection/)). Affinity Photo 2's Astrophotography Stack Persona is now inside the free "Affinity" by Canva; Affinity 3.3 (Sep 2026) added an "astrophotography Studio" ([AlternativeTo](https://alternativeto.net/news/2026/9/canva-prosuite-launches-alongside-affinity-3-3-its-biggest-photo-editing-update-yet/)).

## 1. Workflow structure: scripts/wizards vs numbered tabs vs single dialog; which beginners succeed with

### Takeaway
Every suite offers a "one press" path (DSS "Stack after registering", Siril's OSC_Preprocessing script, APP "click Integrate in 6)", WBPP's Run, Affinity's Stack button), and beginners succeed mainly through it. The structures differ in what they show *before* the run: WBPP's Control Panel (per-group plan with status icons and a calibration diagram) and DSS's pre-stack recap with mismatch warnings are the strongest pre-flight patterns; Siril scripts show nothing until they fail. WBPP's newer pattern of pausing the batch for an interactive decision, then continuing, is the closest analogue to Star Photos' "automatic unless uncertain" flow.

### Cited Findings
#### DeepSkyStacker: single window, left command tabs, one checkbox chain
- Layout: left set of tabs gives access to commands; right area shows the Pictures List and Picture Preview; manual calls the UI "quite simple and intuitive" — [DSS User's Manual](http://deepskystacker.free.fr/english/userguide.htm)
- Quick Start: add picture/dark/flat/offset-bias/dark-flat files, check "Stack after registering" after entering the percentage of pictures to keep, then "go to sleep knowing that after a good night you will view the first results in the processing tab" — [DSS User's Manual](http://deepskystacker.free.fr/english/userguide.htm)
- Before stacking "a screen recapitulates the different steps"; stacking methods per frame type can be changed from links in that recap or "Stacking Parameters" — [DSS User's Manual](http://deepskystacker.free.fr/english/userguide.htm)
- File Groups for multi-night: tabs along the bottom of the picture list; a new empty group tab appears when you add to the last one; Main Group calibration frames can serve all groups, other groups' frames only their own lights — [DSS User's Manual](http://deepskystacker.free.fr/english/userguide.htm), [DSS Technical details](http://deepskystacker.free.fr/english/technical.htm)
- Processing tab is deliberately minimal ("simple processing treatments... to quickly view the result"; heavy work belongs elsewhere) — [DSS User's Manual](http://deepskystacker.free.fr/english/userguide.htm)

#### Siril: free-form tabs plus scripts
- Scripts require a working directory with `lights`, `darks`, `flats`, `biases` subfolders, set via the Home icon; then Scripts > OSC_Preprocessing; intermediate files go to `process`; result is `result_xxx.fit` (xxx = total integration). Tutorial warns the working directory "can change after a failed script" — [Siril scripts tutorial](https://siril.org/tutorials/tuto-scripts/)
- 1.4.0 added a script repository ("Get Scripts" in Preferences, tick scripts to appear in the menu) and Python scripting; users are told to trust script authors — [Siril Scripts docs](https://siril.readthedocs.io/en/stable/Scripts.html), [Siril 1.4.0](https://siril.org/download/2025-12-05-siril-1-4-0/)
- Beginner friction: a user called stacking "totally non-intuitive" and objected that the program "ONLY works with specific named directories"; a reply framed Siril as "either you have scripts... or you do it manually" — [pixls.us 0.99.6 thread](https://discuss.pixls.us/t/manual-for-stacking-in-siril-0-99-6/21506)

#### APP: numbered tabs as the workflow
- Pre-processing menus on the left numbered so users follow calibration steps in order or one at a time (1-6 pre-processing, 9 tools); image centre; console at bottom listing loaded images and completed processes; "a single button press produces a fully integrated stack"; reviewer: interface "may seem rather strange to begin with" — [Sky at Night review, 2018](https://www.skyatnightmagazine.com/reviews/software/astro-pixel-processor-deep-sky-processing-program)
- Developer (2019): "just load all your frames, set settings in menus 2)-6) and click on Integrate in 6)" — [APP users manual thread](https://www.astropixelprocessor.com/community/tutorials-workflows/users-manual/)
- 1.083 allowed tabs 3 (Analyse Stars), 4 (Register), 5 (Normalize) to be skipped in certain cases (search snippet) — [APP 1.083 release](https://www.astropixelprocessor.com/community/release-information/astro-pixel-processor-1-083-stable-ready-for-download/)

#### PixInsight WBPP: batch script with a plan view and interactive pauses
- WBPP 2.0 (Feb 2021) added a "Control Panel... comprehensive and interactive overview of the entire WBPP session", per-group summaries and manual override of settings or masters; Grouping Keywords for multi-night / meridian-flip sorting — [WBPP 2.0 release](https://pixinsight.com/mail/20210216-WBPP-2.0-release/)
- Control Panel status icons: green tick (all calibration present), red circle (process not selected), yellow triangle (needs attention); "Show Calibration Diagram" draws the planned calibration flow for a line item; manual override turns the tick into a blue chain icon; "Save frame groups on exit" restores state; "Add Files" auto-detects frame types from FITS headers, filenames and folder names, "Custom Frames" for missing metadata — [AstroWorldCreations](https://www.astroworldcreations.com/blog/six-most-important-improvements-in-wbpp-20)
- WBPP 2.4.5 (Jun 2022) Local Normalization Selector: "WBPP pauses its normal batch execution", user evaluates and selects frames, a generated reference is "presented for your evaluation", on accept "the pipeline continues" — [WBPP 2.4.5 release](https://pixinsight.com/mail/20220607-WBPP-2.4.5-release/)
- WBPP 2.9.0 (Jan 2026) Frame Selection: optional step on the Lights tab, runs after calibration, debayer and measurement; interactive by default, or automatic with formulas set via a gear icon; OK resumes stacking with remaining frames — [Brian Weber, 16 Jan 2026](https://blog.briangweber.com/wbpp-frame-selection/)
- Documentation scarcity: forum users report not finding built-in manuals and being pointed to third-party videos (search snippet; exact thread uncertain) — [AstroBin WBPP thread](https://app.astrobin.com/forum/topic/34390/wbpp-stacking-script-for-pixinsight)

#### Affinity: a persona/studio with four panels
- File > New Astrophotography Stack; right studio holds Files, Stacking Options, RAW Options, Stacked Images; Stack on the context toolbar; results accumulate in Stacked Images so you can change settings and stack again to compare; Apply switches to the Photo Persona with each stack as a pixel layer and Curves/Levels "using values calculated during stacking" added on top — [Affinity help: Creating an astrophotography stack](https://affinity.help/photo2/en-US.lproj/pages/Astrophotography/astro_creating.html)
- Sky at Night (Affinity Photo 2, Apr 2023): "easy to use", interface "intuitive"; cons: no comet stacking, star alignment struggled with lower-quality images, erratic performance fixed by disabling hardware acceleration — [Sky at Night AP2 review](https://www.skyatnightmagazine.com/reviews/software/serif-affinity-photo-2-software)

### Inferences
- Beginners succeed with whatever gives a single run with sensible defaults (DSS, Affinity, APP tab 6, Siril scripts); they struggle where the structure is implicit (Siril's named folders and working directory; WBPP's grouping keywords).
- The best pattern to copy is WBPP's: a pre-run plan with per-group traffic-light status and a "why" diagram, then an automatic run that pauses only at decision points (frame selection, reference choice) and resumes on OK. This maps directly onto Star Photos' validate → review → run flow.
- Affinity's "Stacked Images" list (re-stack with new settings, compare results side by side, then Apply) is a cheap, high-value pattern for "choose scene treatment".
- APP's numbered tabs teach the pipeline order but expose every stage; Star Photos' stages should be visible as a progress spine but only open when a decision is needed.

### Gaps
- No usability study compares beginner success across these tools; evidence is reviews and forum anecdotes.
- Full WBPP 2.9.0/3.1.0 release notes were truncated; the PixInsight reference docs were unreachable.

## 2. Frame review: blink, per-frame metrics, charts, automatic rejection and overrides

### Takeaway
All suites compute per-frame metrics after star detection/registration (FWHM, roundness/eccentricity, star count, background), but surface them differently: DSS as sortable list columns plus a "keep best X%" percentage; Siril and WBPP Frame Selection as plots linked to a frame list with drawn selections; APP as a relative quality score with a "lights to stack %" slider and no blink; Affinity as only a percentage. Relative composite scores (DSS, APP) are repeatedly misunderstood; users still blink because metrics miss satellites, clouds and gross defects.

### Cited Findings
#### DeepSkyStacker
- Picture list columns: checkbox (only checked frames used), Path, File, Type, Score, dX, dY, Angle, Date/Time, Size, CFA, Depth, Infos, ISO Speed, Exposure, FWHM (average of detected stars), #Stars (+(C) when comet set), Sky Background; sortable by any column; column layout persisted — [DSS User's Manual](http://deepskystacker.free.fr/english/userguide.htm)
- "Check above a threshold..." checks all frames with score ≥ value; reference frame is the best score unless chosen via context menu (score then prefixed "(*)") — [DSS User's Manual](http://deepskystacker.free.fr/english/userguide.htm), [DSS Technical](http://deepskystacker.free.fr/english/technical.htm)
- Score: "the higher the score, the more round and not too big stars were found"; "a relative measure that is only used to sort the pictures"; changing RAW settings requires re-registering — [DSS FAQ](http://deepskystacker.free.fr/english/faq.htm)
- Star editing override: selecting a light frame shows detected stars in green; left-click adds/removes stars; toolbar modes for stars, comet, custom rectangle — [DSS User's Manual](http://deepskystacker.free.fr/english/userguide.htm)
- Registration needs ≥8 stars common to all frames; users should tune the detection threshold to get ≥20; elongated stars are rejected by the detector; registration results cached in `<file>.Info.txt` — [DSS Technical](http://deepskystacker.free.fr/english/technical.htm)
- DSS 6.2.0 has a "Quality Chart" (FWHM plot now replaces outliers with the median); 6.2.2 lowered the star detection limit for short exposures — [DSS releases](https://github.com/deepskystacker/DSS/releases)

#### Siril
- Plot tab (F5): choose metric for Y (and optionally X): FWHM, Roundness, wFWHM, Background, # Stars, X/Y Position, Quality (planetary); legend: purple = plot as configured, green = values sorted by decreasing quality, circle = reference frame, cross = currently loaded frame — [Siril Plot docs](https://siril.readthedocs.io/en/stable/Plot.html)
- Overrides on the plot: right-click a point to exclude it or load it; drag to draw a selection (shows count and boundary values); right-click selection to keep only, exclude, or zoom; export to CSV — [Siril Plot docs](https://siril.readthedocs.io/en/stable/Plot.html)
- Frame selector list: sortable (e.g. by FWHM), its last column mirrors the plot's Y metric; header buttons exclude all / include all / toggle selected; hover shows original filename; selection stored in the `.seq` file — [Siril Plot docs](https://siril.readthedocs.io/en/stable/Plot.html), [Siril Sequences](https://siril.readthedocs.io/en/stable/Sequences.html)
- Stacking-time filtering: keep a "%" or a "k-σ" cutoff on FWHM, weighted FWHM, roundness, background, nb stars or quality; weighting by number of stars, weighted FWHM, noise or integration time — [Siril stacking docs](https://siril.readthedocs.io/en/stable/preprocessing/stacking.html)
- Since 1.2 two-pass registration auto-selects the best reference — [Siril Sequences](https://siril.readthedocs.io/en/stable/Sequences.html)

#### APP
- Quality score (v1.076 description): numberOfStars × median star size × median star roundness, from the median star profile — [APP FAQ](https://www.astropixelprocessor.com/community/faq/quality-score-caculation/); scores "are calculated on the basis of the dataset you provide" (relative) — same source
- Measures discussed by users: star count/density, background, dispersion, SNR, noise, FWHM, roundness, "Quality", "Best to Worst" sort; "the quality score is taking into account the star shape quite a bit"; meaning of "dispersion" went unanswered — [APP quality measures, Mar 2022](https://www.astropixelprocessor.com/community/main-forum/app-quality-measures/)
- Rejection controls: tab 6 "lights to stack xx %" slider with weights = quality; sort by quality to find a break; after normalization, right-click the file list for an analyze-data plot of star shape, SNR, quality, then untick bad subs; double-click loads a frame into the previewer — [APP blink RFC](https://www.astropixelprocessor.com/community/rfcs-request-for-changes/blink-mode-to-remove-bad-frames/), [APP auto-rejection RFC](https://www.astropixelprocessor.com/community/rfcs-request-for-changes/automatic-subframe-rejection/)
- Blink requested since Jun 2019, still "on the wish list" in 2021; blinking slow in 1.083b2; frames with star trails scored higher than clean ones; suggestion to preload binned previews for instant switching — [APP blink RFC](https://www.astropixelprocessor.com/community/rfcs-request-for-changes/blink-mode-to-remove-bad-frames/)
- Users asked for automatic rejection of frames whose "stars weren't quite round"; answer was set rejection to non-automatic, weight by Quality, integrate ~90% — [APP auto-rejection RFC, Dec 2020](https://www.astropixelprocessor.com/community/rfcs-request-for-changes/automatic-subframe-rejection/)

#### PixInsight
- Blink: Process > Blink, select frames, slideshow or manual click-through, frames auto screen-stretched without changing data, then move/copy kept or rejected frames to another folder — [Telescope Live: What is the Blink](https://telescope.live/blog/what-blink)
- WBPP Frame Selection metrics: FWHM, Eccentricity, PSF Signal Weight, Median (clouds/moon), Stars; each graphed, frames listed in "buckets", double-click to view a frame, criteria/formulas top-right, manual "Disable frame"; the blogger admits not fully understanding PSF Signal Weight — [Weber](https://blog.briangweber.com/wbpp-frame-selection/)
- Users report the Frame Selector cannot multi-select frames for disabling (search snippet) — [AstroBin thread](https://app.astrobin.com/forum/topic/216385/pleiades-astrophoto-pixinsight/wbpp-frame-selector-new-how-do-you-decide-which-frames-to-reject); thread title itself ("how do you decide which frames to reject?") shows threshold choice is the open problem
- Common SubframeSelector practice (search snippet): check PSF Signal Weight (higher better), FWHM, eccentricity <~0.5, and set arcsec/pixel scale before measuring — [AstroBin FWHM vs eccentricity](https://app.astrobin.com/forum/topic/157348/fwhm-vs-eccentricity-how-to-filter-the-good-frame-in-subframe-selector-in-pi)

#### Affinity
- Files panel: "Select best light frames" takes a percentage to keep; quality judged by artefacts "such as star trails and periodic errors"; otherwise only remove files from the list — [Affinity Files panel](https://affinity.help/photo2/en-US.lproj/pages/Astrophotography/astro_panelFiles.html)

### Inferences
- Linked plot + list + preview (Siril, WBPP Frame Selection) is the converging pattern; drawing a selection on a metric plot is the fastest bulk override.
- A single relative score (DSS, APP) is opaque and misranks frames (APP: trailed frames scored higher). Star Photos should flag frames with named reasons (trailed, cloud, aircraft/satellite, low star count, brightness jump) and show the metric behind each flag, consistent with the Satrelle pattern in the companion note.
- Blink remains wanted even with metrics; preloaded low-res previews (APP user suggestion) make it instant.
- "Keep best X%" is easy but blind; pairing it with a plot showing where the cut falls would address APP users who "sort by quality to find a break".

### Gaps
- No fetched source documents Affinity's per-frame metrics display (if any) or DSS 6's Quality Chart UI beyond the changelog line.
- PixInsight's official Blink/SubframeSelector docs were unreachable; Blink's per-frame vs common STF behaviour unverified.

## 3. Calibration frames: request, matching, validation, mismatch reporting

### Takeaway
DSS and WBPP match calibration frames automatically from metadata and report mismatches in a pre-run view; Siril makes the user pick masters manually and mostly fails at runtime; APP auto-matches and warns via popups or console lines that users find cryptic. Overrides exist in DSS (session-only ISO/exposure forcing) and WBPP (per-group manual master with a chain icon).

### Cited Findings
- DSS: lights auto-associated with darks by ISO and exposure; flats and bias by ISO only; "best matching frames will be proposed and a warning will be issued if ISO speed and/or exposure time are not matching"; ISO/exposure can be "forced" temporarily via context menu or Properties, stored only for the session — [DSS User's Manual](http://deepskystacker.free.fr/english/userguide.htm)
- DSS master reuse: masters reused while file list and settings unchanged, tracked in a `.Description.txt`; otherwise rebuilt automatically, "transparent for the user" — [DSS Technical](http://deepskystacker.free.fr/english/technical.htm)
- Siril: calibration uses only masters selected by the user (button or preferences library); synthetic bias via "=2048" or "=64*$OFFSET"; dark optimisation "Auto-evaluation" or "Use exposure"; no documented check of exposure/ISO/temperature/CFA against lights; size mismatch stops calibration (commonly JPGs mixed with RAWs); cosmetic-correction "Estimate" turns red above 1% of pixels — [Siril calibration docs](https://siril.readthedocs.io/en/stable/preprocessing/calibration.html)
- Siril 1.4.0 added automatic "Dark scaling" to exposure time — [Siril 1.4.0](https://siril.org/download/2025-12-05-siril-1-4-0/)
- WBPP: Control Panel status icons and calibration diagram per group; manual override per line item — [AstroWorldCreations](https://www.astroworldcreations.com/blog/six-most-important-improvements-in-wbpp-20)
- WBPP pitfalls (search snippets): a 10 s dark exposure tolerance produced a false "Flat frame exposure differs from master dark exposure" error; widening tolerance merged groups unexpectedly; grouping keyword must not appear elsewhere in the path; dates as keyword values split groups at midnight — [Adam Block forum: exposure error](https://forum.adamblockstudios.com/forums/discussion/comment/3162), [Adam Block forum: keyword grouping](https://forum.adamblockstudios.com/forums/discussion/comment/4676)
- APP: popup "CRITICAL WARNING: flat-field calibration can not be performed correctly!" when flats lack matching dark-flats or bias; user: "Not sure I get it"; moderator later: "Perhaps you hit a rare condition that triggered the popup" — [APP forum, May 2021](https://www.astropixelprocessor.com/community/main-forum/critical-warning-flat-field-calibration-can-not-be-performed-correctly-2/)
- APP console line "we have detected a possible sensor offset issue between the MasterBias and the Masterdark !" fires when master dark median < master bias median; moderator: "a warning and not necessarily a problem" — [APP forum, Aug 2019](https://www.astropixelprocessor.com/community/tutorials-workflows/calibrate-warning-we-have-detected-a-possible-sensor-offset-issue-between-the-masterbias-and-the-masterdark/)
- Affinity: calibration types (darks, bias, flats, dark flats) are added by setting Type before Add Files; file groups per night; no documented matching or validation — [Affinity: About astrophotography stacking](https://affinity.help/photo2/en-US.lproj/pages/Astrophotography/astro_about.html), [Affinity Files panel](https://affinity.help/photo2/en-US.lproj/pages/Astrophotography/astro_panelFiles.html)

### Inferences
- Match from EXIF (ISO, exposure, temperature where present, dimensions) and show the plan before running, as DSS/WBPP do; report a mismatch as "these 12 darks are 30 s, lights are 20 s: use anyway / scale / skip" rather than a modal "CRITICAL WARNING".
- Validate cheaply at import (dimension/format mixes like Siril's JPG-in-RAW-folder failure) instead of failing mid-run.
- Distinguish "warning, probably fine" from "blocking" visually; APP's offset warning shows the cost of not doing so.

### Gaps
- DSS 6.x's current wording of the mismatch warning and recap dialog was not seen (manual predates 6.x).

## 4. Stretching and display: linear vs stretched preview, histogram, colour; separating display from data

### Takeaway
Every suite separates a display stretch from the data, and each says so explicitly: Siril ("Only Linear mode... reflects the true pixel values"), PixInsight STF (screen rendition only), DSS's preview gamma ("only a previewing trick"), Affinity (stretch delivered as adjustment layers). Confusion arises where preview filters leak into export (APP) or users must manually transfer a display stretch to data (PixInsight).

### Cited Findings
- Siril display modes: Linear (default), Logarithm, Square root, Squared, Asinh, AutoStretch (with "High definition"), Histogram; chain toggle links/unlinks RGB in AutoStretch; negative and false-colour views; "Only Linear mode (with both visualisation sliders at their minimum and maximum positions) reflects the true pixel values" — [Siril toolbar docs](https://siril.readthedocs.io/en/stable/GUI/toolbar.html)
- Siril script output "first appears nearly black in Linear mode, so switch the rendering mode to 'Autostretch'" — [Siril scripts tutorial](https://siril.org/tutorials/tuto-scripts/)
- Siril 1.4.0: Curves tool, ROI preview of processes on a small region, full ICC colour management — [Siril 1.4.0](https://siril.org/download/2025-12-05-siril-1-4-0/)
- PixInsight STF "applied to the screen rendition of an image, without modifying its pixel data"; Auto Stretch button; to make it permanent users copy to HistogramTransformation (search snippet) — [PixInsight STF docs](https://pixinsight.com/doc/tools/ScreenTransferFunction/ScreenTransferFunction.html)
- DSS: preview gamma slider "no modification is made to the real image as it is only a previewing trick"; Processing tab has RGB histogram with per-channel Gaussian overlays, RGB levels (linear, sqrt, cbrt, log, log(log), log(sqrt), asinh), 6-parameter luminance curve, saturation; save with or without applying adjustments (TIFF stores them for later) — [DSS User's Manual](http://deepskystacker.free.fr/english/userguide.htm); 6.2.0 added asinh stretch and colour balance tab, 6.2.1 MTF stretch with auto-stretch, 6.2.2 Saturation/vibrance tab — [DSS releases](https://github.com/deepskystacker/DSS/releases)
- Affinity: stacking at 32-bit linear unbounded float; on Apply, Curves and Levels with stacking-derived values are added as layers — [Affinity about](https://affinity.help/photo2/en-US.lproj/pages/Astrophotography/astro_about.html), [Affinity creating](https://affinity.help/photo2/en-US.lproj/pages/Astrophotography/astro_creating.html). Opening a FITS auto-adds a Tone Stretching adjustment layer (Basic, Arcsinh, Logarithmic, Structural, Color Preserving; Stretch Factor, Gamma, Compression) — [Vaonis, Nov 2025](https://vaonis.com/blogs/travel-journal/image-processing-tools-affinity-becomes-free-and-gains-a-new-astrophotography-feature-tutorial)
- APP: stretch checkbox; DDP is the automatic stretch; sharpen sliders are part of the preview filter so saving with stretch off drops them; unticking "auto" locks stretch across viewer modes (linear, l-calibrated, normalized, registered) (search snippets) — [APP lock stretch RFC](https://www.astropixelprocessor.com/community/rfcs-request-for-changes/ability-to-lock-the-stretch-value/), [APP odd sharpen](https://www.astropixelprocessor.com/community/main-forum/odd-sharpen-question/), [APP registered viewer](https://www.astropixelprocessor.com/community/main-forum/viewing-the-same-sub-frame-of-images-from-different-fits-captures/)
- APP beta47 (3 Oct 2026) new preview filter "Adaptive Deep Sky Stretch" (colour preserving, background level and signal width sliders) and "General Levels Stretch"; old filter "has not been updated for a couple of years"; 2023 complaint that the preview stretch went "funky and way over exposed" with black borders, no resolution — [APP preview stretch thread](https://www.astropixelprocessor.com/community/rfcs-request-for-changes/preview-stretch-bug/)
- APP light pollution tool: draw ≥5 sample boxes, press calculate — [Sky at Night APP review](https://www.skyatnightmagazine.com/reviews/software/astro-pixel-processor-deep-sky-processing-program)

### Inferences
- Default the review views to an auto display stretch (frames look black otherwise) but label it as display-only; keep export stretch as an explicit, separate choice. Siril's one-line truth statement is a good model for the UI copy.
- Stretch statistics should ignore empty/black borders (APP complaint) and be lockable across frames so blink comparisons are fair (APP "auto" lock, Siril linked chain).
- Affinity's "stretch as editable adjustment" is the cleanest handoff for a photographer audience.

### Gaps
- Colour calibration UIs (Siril PCC/SPCC, PI SPCC, APP star colour calibration) were not researched in depth; Affinity 3.2/3.3 astro stretch and colour features seen only in summaries.

## 5. Progress, logs, errors on long runs; resuming; memory and disk warnings

### Takeaway
Long-run feedback is thin everywhere: DSS has a progress dialog with ETA and a crash-safe AutoSave.tif; APP and Siril print to a console; WBPP has an Execution Monitor that users report going blank. Resume/caching is the more developed pattern (DSS .Info.txt and master reuse, Siril .seq state, WBPP saved groups and 3.1.0 interrupted-run recovery). Siril's mandatory free-disk-space pre-check misreports on macOS external drives and cannot be bypassed.

### Cited Findings
- DSS: stacked result saved automatically to AutoSave.tif (then AutoSave001...) in the reference frame's folder, "even if your PC crashes after the stacking process"; registration cached in .Info.txt; masters reused — [DSS User's Manual](http://deepskystacker.free.fr/english/userguide.htm), [DSS Technical](http://deepskystacker.free.fr/english/technical.htm)
- DSS warns about memory/disk for drizzle (4x/9x size) and intermediate files; warns when Drizzle and Bayer Drizzle are combined — [DSS User's Manual](http://deepskystacker.free.fr/english/userguide.htm), [DSS Technical](http://deepskystacker.free.fr/english/technical.htm); 6.2.0 fixed the progress dialog's remaining-time estimate and turned a broken-FITS exception into an error message; 6.2.3b3 removes inaccessible file lists from the MRU instead of crashing — [DSS releases](https://github.com/deepskystacker/DSS/releases)
- Siril: scripts need "several times the size of the original images" free; delete `process` after — [Siril scripts tutorial](https://siril.org/tutorials/tuto-scripts/); memory budget default 0.9 × available (ratio or fixed mode) (search snippet) — [Siril preferences](https://siril.readthedocs.io/en/stable/preferences/preferences_command.html); 1.4.0 "Better memory management" for sequences — [Siril 1.4.0](https://siril.org/download/2025-12-05-siril-1-4-0/)
- Siril "Not enough free disk space": macOS users saw Siril report a fraction of real free space (1.2.3/1.2.4), N/A on external drives; asked whether the check can be bypassed, answer "No" (search snippets) — [pixls: bypass](https://discuss.pixls.us/t/bypass-siril-free-disk-space-calculations/46839), [pixls: external drive](https://discuss.pixls.us/t/error-not-enough-space-but-using-external-drive/46560), [pixls: wrong disk space](https://discuss.pixls.us/t/wrong-disk-space-on-siril/29672)
- Siril state: normalisation results stored in the .seq so re-stacking with new rejection settings is faster; "Recompute" forces it — [Siril stacking docs](https://siril.readthedocs.io/en/stable/preprocessing/stacking.html)
- APP: console at bottom of window; CFG tab sets RAM allowance; SSD and >8 GB recommended — [Sky at Night APP review](https://www.skyatnightmagazine.com/reviews/software/astro-pixel-processor-deep-sky-processing-program); beta46 lowered memory use — [APP beta46](https://www.astropixelprocessor.com/community/release-information/astro-pixel-processor-2-0-0-beta46-release-notes/)
- WBPP: Execution Monitor reported going blank during an RGB run (search snippet) — [AstroBin](https://app.astrobin.com/forum/topic/252916/help-needed/execution-monitor-window-blank-during-wbpp-rgb-batch-processing); "Save frame groups on exit"; PixInsight locks while the script is open — [AstroWorldCreations](https://www.astroworldcreations.com/blog/six-most-important-improvements-in-wbpp-20); WBPP 3.1.0 (Sep 2026) "Reliable recovery of an interrupted execution" and "Reuse last reference frames" — [PixInsight announcements](https://pixinsight.net/dev/index.php?threads%2Fwbpp-2-9-0-release-announcement.7=)
- WBPP integration failure "Master Light file was not generated" traced to one bad light plus an over-subtracting dark (search snippet) — [AstroBin](https://app.astrobin.com/forum/topic/141079/pixinsight-error-with-image-integration-wbppfbpp)

### Inferences
- Cache per-frame analysis and intermediate results keyed by inputs + settings (DSS .Info.txt / .Description.txt pattern) so changing a review decision re-runs only integration.
- Write a provisional result early and atomically (DSS AutoSave) so a crash never loses a finished stack.
- Disk-space checks must use the target volume's real free space (APFS, external drives) and offer a "continue anyway" path; Siril shows the cost of a wrong hard block.
- Errors should name the frame and step (WBPP's "Master Light not generated" hid a single bad frame).

### Gaps
- Siril's progress bar/console and APP console behaviour on long runs were not documented in fetched pages; WBPP's Execution Monitor and report layout not seen first-hand.

## 6. Recurring user complaints and praise

### Takeaway
Praise centres on one-button results (APP, DSS, Affinity) and free/open tooling (Siril). Complaints repeat: missing or out-of-date manuals (APP since 2019, PixInsight), opaque relative quality scores, no blink in APP, Siril's folder/working-directory conventions, WBPP grouping keywords and tolerances, and cryptic calibration warnings.

### Cited Findings
- APP manual: promised "with high priority" Mar 2019, "near completion" Jan 2020; third-party quick-start guide praised for explaining "what all the sliders and buttons actually do" — [APP users manual thread](https://www.astropixelprocessor.com/community/tutorials-workflows/users-manual/); still "getting close" to a full manual Jun 2026 — [APP beta46](https://www.astropixelprocessor.com/community/release-information/astro-pixel-processor-2-0-0-beta46-release-notes/)
- APP praised for mosaics, ease of use, developer support; criticised for no comprehensive instructions and an interface that takes time — [Sky at Night APP review](https://www.skyatnightmagazine.com/reviews/software/astro-pixel-processor-deep-sky-processing-program)
- APP quality score: Q score near 3000 vs 500-700 in another session went unexplained — [APP FAQ](https://www.astropixelprocessor.com/community/faq/quality-score-caculation/); trailed frames scored higher than clean ones — [APP blink RFC](https://www.astropixelprocessor.com/community/rfcs-request-for-changes/blink-mode-to-remove-bad-frames/)
- Siril: "totally non-intuitive", named-directory requirement — [pixls.us](https://discuss.pixls.us/t/manual-for-stacking-in-siril-0-99-6/21506); GUI quirk where "Go register" was disabled until an area was selected; preprocessing options greyed out after unticking dark/flat — [pixls.us nightscape tutorial](https://discuss.pixls.us/t/processing-a-nightscape-in-siril-a-tutorial/10996)
- WBPP: Post-Calibration drizzle toggle reportedly cleared all tabs in v2.5.9 (search snippet) — [Adam Block forum](https://forum.adamblockstudios.com/forums/discussion/comment/2874); grouping/tolerance confusion (see section 3)
- Affinity praised as intuitive; criticised for no comet mode, alignment failures on poor data, compressed TIFF default — [Sky at Night AP2 review](https://www.skyatnightmagazine.com/reviews/software/serif-affinity-photo-2-software)
- DSS: only stacks with ≥8 common stars, so users must tune the detection threshold — [DSS Technical](http://deepskystacker.free.fr/english/technical.htm)

### Inferences
- Built-in, contextual explanation (what each control does and why a decision was made) is a differentiator; every suite outsources it to videos and forums.
- Every warning should say what will happen if ignored.

### Gaps
- Cloudy Nights and Reddit sentiment was not retrievable; complaint frequency cannot be quantified.

## 7. Landscape/nightscape-specific features

### Takeaway
None of the five suites handles a fixed foreground. Siril users stack twice (star-aligned sky, unaligned ground) and blend in GIMP; APP has no foreground support (see companion note); DSS and PixInsight have nothing comparable. Wide-angle distortion was a known Siril weakness versus Sequator; 1.4.0's distortion correction may change that but needs plate solving.

### Cited Findings
- Siril nightscape tutorial (Jan 2019): stack aligned on stars ("the foreground will be blurred because earth moved"), re-run from calibrated frames without registration via a provided `processing_from_raw_foreground.ssf`, blend in GIMP with a mask; global star registration needed for untracked rotation — [pixls.us](https://discuss.pixls.us/t/processing-a-nightscape-in-siril-a-tutorial/10996)
- Siril (Feb 2023): 42 mm untracked stack had worse corner distortion than Sequator; reply: "Currently Siril has no such a reduce distorsion feature" — [pixls.us](https://discuss.pixls.us/t/stacking-non-tracked-images-got-severe-distortion/35487)
- Siril 1.4.0: "Distortion correction during alignment" using WCS SIP data and "Master distortion files" — [Siril 1.4.0](https://siril.org/download/2025-12-05-siril-1-4-0/); framing indicator colours (since 1.3.1): blue = shift, red = linear, green = with distortion — [Siril Sequences](https://siril.readthedocs.io/en/stable/Sequences.html)
- Siril-landscape forum advice (search snippets): Siril "geared for deep space"; users crop out the foreground before stacking or switch to Sequator — [pixls.us: averaging images](https://discuss.pixls.us/t/averaging-images-aligning-sky-on-stars/38224), [AstroBin: Siril in astro landscape](https://app.astrobin.com/forum/topic/130156/use-of-siril-in-astro-landscape)
- APP: no foreground stacking as of 2026; developer said in Feb 2025 he'd try an "exclude landscape area" option — [APP forum](https://www.astropixelprocessor.com/community/main-forum/stacking-with-foreground/)
- DSS's Custom Rectangle defines output area (and reduces memory) but is not a foreground tool — [DSS Technical](http://deepskystacker.free.fr/english/technical.htm)

### Inferences
- Integration-inspection tools from these suites transfer directly to Star Photos' "inspect alignment/integration" step: Siril's "Create rejection maps" (per-pixel rejection fraction) and framing indicator, DSS's cosmetic-correction map (hot = white, cold = dark grey) and green detected-star overlay — [Siril stacking](https://siril.readthedocs.io/en/stable/preprocessing/stacking.html), [DSS User's Manual](http://deepskystacker.free.fr/english/userguide.htm).
- Star-detection thresholds tuned for deep-sky (DSS ≥8 common stars, elongated-star rejection) can fail on short, wide nightscape frames; DSS lowered its limit for short exposures in 6.2.2.

### Gaps
- Whether Siril 1.4's SIP distortion correction works on untracked ultra-wide nightscape frames is untested in any source found.
- Affinity's separate (non-astro) File > New Stack median stacking was not researched.
