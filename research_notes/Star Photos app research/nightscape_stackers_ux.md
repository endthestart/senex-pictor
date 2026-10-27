# Nightscape stacking tools: user-facing workflow, choices, praise and complaints (as of Oct 2026)

Scope: tripod nightscape (sky + fixed foreground) stackers. Tools: Sequator (Windows), Starry Landscape Stacker "SLS" (macOS), StarStaX, Photoshop/Lightroom manual workflow, Luminar Neo, plus newer/adjacent tools found: Satrelle (macOS, 2025-26), NightScape Lab (announced, cross-platform), Kandao Raw+, Astro Pixel Processor (no foreground support). Reddit and DPReview could not be fetched (blocked/403), so forum sentiment comes from App Store reviews, pixls.us, IceInSpace, Lonely Speck comments, blogs and tutorials.

## 1. Import and validation of a sequence (frame count, bad frames, darks/flats)

### Takeaway
Dedicated stackers sort frames into roles (lights / darks / flats / mask) and pick a reference frame automatically, but none of the established tools score or auto-reject bad frames; culling is manual (SLS "Exclude Images", Sequator file list). The newest entrant, Satrelle, is the first found with a grid "Frame Review" that auto-flags headlights, planes, clouds and stray darks.

### Cited Findings
#### Tool roster and current versions
- Sequator: free, closed-source, Windows 10/11 64-bit only (Linux via Wine), latest 1.6.2 r2 released Aug 18 2024; earlier notable releases: 1.5.1 (Oct 2018) drag-and-drop + time-lapse batch, 1.5.5 (Jun 2019) "better foreground quality for freeze ground", 1.6.0 (Mar 2021) improved star detection + lens EXIF — [Sequator download](https://sites.google.com/view/sequator/download)
- Sequator's author: "No plan yet" for Mac/iOS/Android; not open source — [Sequator Q&A](https://sites.google.com/view/sequator/qa)
- SLS: Mac App Store, $39.99, current 1.10.1, macOS 11.5+, developer Ralph Hill, free watermarked trial on dev site — [App Store](https://apps.apple.com/app/id550326617); trial watermark — [Woodworth, Dec 2025](https://adamwoodworth.substack.com/p/star-stacking-for-pinpoint-stars). Michael Frye says SLS "hasn't changed much" in years — [Frye](https://education.michaelfrye.com/sls/)
- StarStaX: free; macOS 0.91, Windows 0.71; no RAW input (JPG/TIFF ≤16-bit/PNG only, develop RAW first); Sept 2026 site news seeks macOS testers for an upcoming version — [StarStaX](https://www.markus-enzweiler.de/software/starstax/)
- Satrelle: macOS Apple Silicon, macOS 12+, Pro $59 one-time, 7-day trial then Free mode capped at 200 frames; by Marc Rassel Photography (Hand Forged LLC); no version/date stated — [Satrelle](https://satrelle.com/)
- NightScape Lab (nightscapestacker.com redirects to nightscapelab.com): "Coming soon", $129 one-time, macOS/Windows/Linux, licence re-check every 14 days, "presented by Milky Way Planner" — [NightScape Lab](https://nightscapelab.com/)
- Astro Pixel Processor: still no foreground/landscape stacking as of its Oct 2026 betas (beta47/48); developer said Feb 2025 he'd try adding a "exclude landscape area" option; users asking for it Oct 2025 ("would make me buy APP") — [APP forum](https://www.astropixelprocessor.com/community/main-forum/stacking-with-foreground/)
- Luminar Neo (v1.28.1) has HDR Merge, Focus Stacking (depth of field), Noiseless AI, Sky AI, masking; no star-aligned/noise-averaging stack — [App Store, Luminar Neo](https://apps.apple.com/app/id1584373150)

#### Sequator import
- Left-side list of items: "Star images", "Base image", "Noise images" (darks), "Vignetting images" (flats), "Output"; dragging files in pops a category box asking which role; loaded files appear in tabs top-left; items are opened by double-click — [Sky at Night, Dec 2023](https://www.skyatnightmagazine.com/astrophotography/star-trails-software-sequator)
- Base image defaults to the middle image in file-name order if not set; "Unify exposure" checkbox (bottom-left panel when Star images is highlighted) for mixed exposures; noise frames should be shot before and after lights (temperature); "Homogenize vignetting" averages/blurs flats across quadrants; vignetting images rejected if resolution differs — [Sequator manual](https://sites.google.com/view/sequator/manual), [Q&A](https://sites.google.com/view/sequator/qa)
- Output path/filename is set up front (like "Save As"), before processing — [improvephotography](https://improvephotography.com/?p=50102)
- "Select best pixels" and "Freeze ground" need ≥4 images with unchanged exposure — [manual](https://sites.google.com/view/sequator/manual)
- No quality score: author judged a simple index "too rough" (can't tell thin cloud, aircraft, rising stars, fireworks), so review is manual; sigma-clipping may help — [Q&A](https://sites.google.com/view/sequator/qa)
- Input caveats: RAW recommended; TIFF must be uncompressed 16-bit and unprocessed beyond vignetting; ON1-exported TIFFs and ProPhoto RGB are known incompatible; Lightroom edits on RAW/DNG don't carry over (export TIFF first) — [Q&A](https://sites.google.com/view/sequator/qa), [improvephotography](https://improvephotography.com/?p=50102)
- Complaint: Sequator sometimes strips EXIF from TIFF output (user wrote a script to restore it) — [pixls.us 2020](https://discuss.pixls.us/t/open-source-tool-for-sky-stacking-and-foreground-extraction/17204)

#### SLS import
- Open via launch dialog, File > Open (⌘O) or Finder "Open With"; select all files at once and the app sorts frame types; "Frame Classification Panel" shows each file's detected type and lets you fix it when unsure or counts look wrong — [SLS Processing](https://www.sites.google.com/site/starrylandscapestacker/processing-images-with-starry-landscape-stacker-version-1-9)
- Roles: lights (min 2, ≥10 recommended, author uses 15+), darks (10, or 5 for low-noise sensors; master darks OK), at most one master flat, optional mask image (external or previous "-sky.tif") which skips straight to alignment — [SLS Processing](https://www.sites.google.com/site/starrylandscapestacker/processing-images-with-starry-landscape-stacker-version-1-9), [Frame types](https://www.sites.google.com/site/starrylandscapestacker/frame-types)
- Capture guidance: ≥10 identical exposures, total capture under 10 minutes — [Using SLS](https://www.sites.google.com/site/starrylandscapestacker/using-starry-landscape-stacker-version-1-9); ≥5 min of lights, 10 min for promising scenes; turn off in-camera long-exposure NR; before 1.9 most users were told to skip darks — [Shooting](https://www.sites.google.com/site/starrylandscapestacker/shooting-starry-landscapes-for-version-1-9)
- Validation messages: 1.7 relaxed the exposure-consistency check (fewer warnings) and added a stronger warning for unknown colour profiles (rare crash risk) — [New in 1.7](https://www.sites.google.com/site/starrylandscapestacker/new-in-version-1-7). App Store reviewer: exposure-related error messages confuse new users — [App Store](https://apps.apple.com/app/id550326617)
- Manual exclusion: "Exclude Images" disclosure includes/excludes the current frame; Option/Alt turns buttons into "Exclude All"/"Include All" — [SLS Processing](https://www.sites.google.com/site/starrylandscapestacker/processing-images-with-starry-landscape-stacker-version-1-9)

#### Others
- Satrelle: point at a folder (RAW/TIFF/JPEG); "Frame Review shows the sequence as a grid and auto-flags headlights, planes, clouds, and stray darks for one-click exclusion"; optional darks; flats not mentioned; "Cloud rejection" is a Pro feature — [Satrelle](https://satrelle.com/)
- NightScape Lab: "Frames" library tab imports a night; lights and darks sorted automatically; ≥2 lights; RAW and TIFF/JPEG/PNG accepted but stacked separately; flats not mentioned — [NightScape Lab](https://nightscapelab.com/)
- StarStaX: averaged dark-frame subtraction; no RAW — [StarStaX](https://www.markus-enzweiler.de/software/starstax/)
- Photoshop: File > Scripts > Load Files into Stack; don't open RAWs in ACR first or you can't load as layers; no dark/flat/bias calibration — [Night Sky Pix, 2021](https://nightskypix.com/how-to-stack-images-in-photoshop-for-astrophotography/)

### Inferences
- Role auto-sorting with an override panel (SLS) is the established pattern; per-frame auto-flagging with reasons (Satrelle) is the differentiator in 2025-26 and matches Star Photos' "review frames" step.
- Sequator's rationale against a single quality score (can't distinguish cloud/aircraft/rising stars) argues for flagging with a named reason per frame rather than a numeric score.
- Up-front output path (Sequator) is a legacy pattern; modern tools defer export.

### Gaps
- No source describes what Sequator or SLS show for a malformed sequence (mixed ISO, mixed sizes) beyond the warnings above. Satrelle's flagging criteria/thresholds are not documented.

## 2. Sky/foreground boundary: auto mask, painted mask, freeze ground; how shown and corrected

### Takeaway
Three models exist: (a) Sequator: user paints a rough sky region (or line/gradient) and the "Freeze ground" algorithm detects moving vs stable pixels; (b) SLS: auto-detected red star dots seed a "Find Sky" region-grow shown as a blue overlay, then brush cleanup; (c) newer tools (NightScape Lab, Satrelle) show an editable sky/ground mask preview and align sky and ground separately. Trees with sky gaps are the universal pain point; neither classic tool handles a scene-level "hero frame/light painting" foreground well.

### Cited Findings
#### Sequator
- "Sky region" offers "Boundary line" (star detection and light-pollution analysis only above the line), "Gradient" (parallel blending zone) and "Irregular mask" (brush-painted); mouse wheel sets brush size, right button erases; the mask need only roughly cover the stars and must not touch the foreground — [manual](https://sites.google.com/view/sequator/manual)
- Behaviour outside the sky region depends on mode: Accumulation accumulates, Select best pixels does simple average, Freeze ground runs foreground detection, Trails keeps the base image — [manual](https://sites.google.com/view/sequator/manual)
- "Freeze ground" detects moving vs stable areas and stacks them separately; camera must not move; best for wide-angle shots of ≥10 min — [manual](https://sites.google.com/view/sequator/manual). Q&A: "A precise mask isn't needed"; roughly mark around trees; the algorithm selects stable pixels and interpolates, including near-horizon fog and sky glow; tracked images not supported — [Q&A](https://sites.google.com/view/sequator/qa)
- Most common cause of poor quality per the author: no sky mask set for freeze ground, or ISO/WB not fixed across frames — [Q&A](https://sites.google.com/view/sequator/qa)
- Praise: sky masking "took seconds" and handled complex branches well — [improvephotography](https://improvephotography.com/?p=50102); mask "really approximate" yet foreground isolated well; simplicity "really blew me away" — [pixls.us 2020](https://discuss.pixls.us/t/open-source-tool-for-sky-stacking-and-foreground-extraction/17204)
- Sequator requires manual sky selection "with no automated aids" — [Stephen Bay, Jun 2021](https://stephenbayphotography.com/blog/star-stacking-with-kandao-raw/)
- IceInSpace users (search snippet, not fully read): freeze ground "works quite well" but ~30 s exposures needed or blur persists; with freeze ground selected, "Reduce Distortion effects" appears to default to "auto" — [IceInSpace](https://www.iceinspace.com.au/forum/showthread.php?p=1477930)
- Light painting: a single light-painted frame "is treated as noise"; options: light every frame identically, tone-curve after, or composite afterwards; single light painting works in star-trails mode — [Q&A](https://sites.google.com/view/sequator/qa)

#### SLS
- "Adjust Stars"/"Adjust Dots": app computes a mean image and places a red dot on each detected star; user paints dots into empty sky and erases dots on ground; toggle "automatic dots"/"no dots" for bright foregrounds; Shift-click draws a line of dots; overlay opacity and image brightness/contrast sliders help see the boundary — [SLS Processing](https://www.sites.google.com/site/starrylandscapestacker/processing-images-with-starry-landscape-stacker-version-1-9)
- "Find Sky" grows a blue sky-mask overlay from the dots; tools: paint, erase, zoom, mask opacity, show/hide; toggle "mask with islands of sky" (default) vs "without islands"; leave uncertain dark areas unmasked; keep a few pixels gap from limbs/horizon — [SLS Processing](https://www.sites.google.com/site/starrylandscapestacker/processing-images-with-starry-landscape-stacker-version-1-9)
- Keys: s = paint sky/add dots, g = paint ground/erase dots, [ ] brush size, o = toggle overlay — [SLS shortcuts](https://www.sites.google.com/site/starrylandscapestacker/keyboard-shortcuts)
- Trees: erase dots on trees, dot sky around trees and in gaps, zoom to dot small gaps, Find Sky, then for remaining gaps "paint only the star tracks, not the whole gap"; areas too dark to show stars can go either way — [Dealing with Trees](https://www.sites.google.com/site/starrylandscapestacker/trees-and-other-foreground-objects-with-gaps)
- External masks: non-zero alpha (or grey) = sky, 0 = ground (search snippet) — [Externally Generated Masks](https://www.sites.google.com/site/starrylandscapestacker/externally-generated-masks); saved "-sky.tif" mask is reusable — [Processing](https://www.sites.google.com/site/starrylandscapestacker/processing-images-with-starry-landscape-stacker-version-1-9)
- Reflections: separate guide, choose "No dots", paint dots in the reflection, Find Sky (search snippet) — [Aligning Reflections](https://www.sites.google.com/site/starrylandscapestacker/aligning-reflections-of-stars)
- Praise: "incredibly good at automatically finding the stars to make a mask for the sky" — [Woodworth, Dec 2025](https://adamwoodworth.substack.com/p/star-stacking-for-pinpoint-stars). Complaints: masking can take hours; no way to save mid-mask — [App Store reviews](https://apps.apple.com/app/id550326617)
- Windy trees (15-frame test): SLS gave sky streaking at canopy edge, ghosted/duplicated branches, removed stars in small gaps, deleted thin branches, black smudges, "orphan" branches; needed extensive clone stamping — [Stephen Bay](https://stephenbayphotography.com/blog/star-stacking-with-kandao-raw/)
- Foreground: SLS "has always stacked the foreground" (developer reply) — [App Store](https://apps.apple.com/app/id550326617); "Cars and people with flashlights are sometimes removed" — [Shooting](https://www.sites.google.com/site/starrylandscapestacker/shooting-starry-landscapes-for-version-1-9)

#### Newer and other tools
- NightScape Lab: "Masking" step with "Preview Sky Mask" showing sky/ground split, paintable, reusable; sky and ground aligned separately; developer says the mask is visible/editable because auto tools sometimes misplace the horizon; "Foreground Sky Merge" combines best ground and best sky — [NightScape Lab](https://nightscapelab.com/). A search-engine summary called its mask "AI"; the page itself does not say AI (conflict, unverified).
- Satrelle: "Foreground masking" keeps land sharp; "Clean foreground (Pro)" multi-frame foreground blend; "Foreground exposure blend (Pro)" blends a separately lit exposure; water reflection editor (set waterline, paint water; reflects sky only); mask-editing tools not detailed — [Satrelle](https://satrelle.com/)
- Kandao Raw+: no sky mask at all; aligns small image segments so swaying branches and stars in gaps came out right; artifacts: "salt & pepper ring" around large stars — [Stephen Bay](https://stephenbayphotography.com/blog/star-stacking-with-kandao-raw/); outputs 16-bit DNG — [DIYPhotography](https://www.diyphotography.net/kandao-raw-offers-automated-image-stacking-for-raw-files-without-ghosting/)
- Photoshop manual: duplicate layers into a "Sky" group; mean-stack the foreground; paint a soft-brush black mask over foreground on one sky layer, Alt-click to inspect, Alt-drag copy to each layer; refine mask at the end — [Night Sky Pix](https://nightskypix.com/how-to-stack-images-in-photoshop-for-astrophotography/). Blending by Select > Sky then hide-all mask (search snippet) — [Photofocus 2025](https://photofocus.com/photography/milky-way-huntin-season-is-on-for-2025-processing-foreground-and-blending/)
- Experienced users often shoot separate long, low-ISO foreground exposures (ISO 1600, several to 15 min) and blend in Photoshop — [Woodworth](https://adamwoodworth.substack.com/p/star-stacking-for-pinpoint-stars); light-painted foreground frames composited in GIMP after Sequator — [pixls.us 2021](https://discuss.pixls.us/t/astrophotography-with-vintage-lenses-specifically-the-wonderful-sigma-super-wide-ii-2-8-24-a-stacking-and-postprocessing-tutorial/26347)

### Inferences
- The two classic philosophies trade user effort for algorithm effort: Sequator (rough mask + motion detection) gets praised for speed; SLS (precise mask) gets praised for auto-detection but complained about for cleanup time. A Star Photos auto mask with a rough-correction brush and a "leave a gap / unsure zone" concept would combine both.
- No classic tool exposes the foreground treatment as a named choice; "hero frame with light painting" is handled entirely outside (Photoshop/GIMP), and Sequator explicitly discards single lit frames. Satrelle's "Foreground exposure blend" and NightScape Lab's "Foreground Sky Merge" are the only in-app equivalents found, so Star Photos' explicit scene-treatment step is a real gap in the market.
- A persistent, savable mask (SLS "-sky.tif") is valued; mid-mask autosave is a requested missing feature.

### Gaps
- Exact on-screen mask rendering in Sequator (colour, opacity) is not documented in text. Satrelle's and NightScape Lab's mask algorithms and edit tools are undocumented; NightScape Lab is unreleased.

## 3. Alignment results, rejected frames, errors, progress on long runs

### Takeaway
Alignment is largely a black box in all tools: no per-frame residuals, no star-match overlays. Errors are surfaced as messages or troubleshooting lists; progress UI is barely documented. Tools compensate with speed claims and, in Satrelle's case, a fast half-resolution "Quick Preview" before the full run.

### Cited Findings
- Sequator "failed to align" causes listed only in the Q&A: out-of-focus stars, under/over-exposed sky, heavily trailed stars, compressed TIFF — [Q&A](https://sites.google.com/view/sequator/qa)
- Sequator troubleshooting maps symptoms to settings: false corner trails → enable distortion correction; halos/middle circle → bad/missing vignetting frames; stars too bright → star-light enhancement; no noise reduction → same frame stacked repeatedly; image downsized → "Merge pixels" on — [Q&A](https://sites.google.com/view/sequator/qa)
- Sequator "Reduce distortion effects": "Lens" vs "Complex" (DeepTracking for ultra-wides and refraction near horizon; needs many stars; acceptable at ≥5 min total with 20 mm-equivalent) — [manual](https://sites.google.com/view/sequator/manual)
- Sequator "Yellow flag": marks options Sequator recommends under the current conditions — [manual](https://sites.google.com/view/sequator/manual)
- Sequator stacks in RAM if possible, else writes temp files to the output folder (disk becomes bottleneck) — [manual](https://sites.google.com/view/sequator/manual)
- Sequator speed: 10 images in ~1 min vs ~3 min in Photoshop — [improvephotography](https://improvephotography.com/?p=50102); star-trail stack "less than 30 seconds" after clicking "Start" — [Sky at Night](https://www.skyatnightmagazine.com/astrophotography/star-trails-software-sequator)
- Sequator alignment "imperfect" per a 2023 user; white balance odd — [pixls.us 2023](https://discuss.pixls.us/t/averaging-images-aligning-sky-on-stars/38224)
- SLS: single "Align and Composite" button runs both automatically; reference defaults to the frame nearest the midpoint in time, overridable via "Align With" → "Align to current image" — [SLS Processing](https://www.sites.google.com/site/starrylandscapestacker/processing-images-with-starry-landscape-stacker-version-1-9)
- SLS 1.8: >2× faster alignment/composition; better with few stars; frames where the camera moved enough to show horizon movement "can no longer be aligned" — [New in 1.8](https://www.sites.google.com/site/starrylandscapestacker/new-in-version-1-8). 1.9.5 replaced the alignment algorithm (faster, better with few sharp stars); reviewers say alignment struggles with long total exposure time — [App Store](https://apps.apple.com/app/id550326617)
- Lonely Speck found SLS successful with "8-12 exposure max", mixed beyond (older version, 2015-18) — [Lonely Speck](https://www.lonelyspeck.com/milky-way-exposure-stacking-with-manual-alignment-in-adobe-photoshop/)
- Photoshop Auto-Align "will rarely work" for this; error "layers don't overlap enough"; manual check is Difference blend (black = aligned) with Transform/Warp — [Night Sky Pix](https://nightskypix.com/how-to-stack-images-in-photoshop-for-astrophotography/). Auto-align failures attributed to clouds, distortion, light pollution — [Lonely Speck](https://www.lonelyspeck.com/milky-way-exposure-stacking-with-manual-alignment-in-adobe-photoshop/). Readers report "one long canvas" instead of alignment and slight rotation mismatch with the foreground smart object — [Lonely Speck comments](https://www.lonelyspeck.com/stacking-noise-reduction/comment-page-6/)
- Satrelle: "Quick Preview renders a half-resolution draft in seconds", then run the full master "and watch it build"; scheduled batch queue can keep the Mac awake overnight — [Satrelle](https://satrelle.com/)
- StarStaX "Cumulative mode" saves the stack after each frame (build-up video) — [StarStaX](https://www.markus-enzweiler.de/software/starstax/)

### Inferences
- An inspect-alignment step with visible evidence (per-frame match quality, residual heat map near edges/horizon) would be novel; competitors only expose symptoms after the fact via troubleshooting docs.
- Sequator's symptom → setting table is effectively an error UI living in a FAQ; Star Photos could surface the same mappings in-app when it detects the symptom.

### Gaps
- No text source describes Sequator's or SLS's progress bars, cancel behaviour or per-frame status. Screenshots pages carry no captions.

## 4. Tone/look controls after stacking vs deferring to an editor

### Takeaway
Classic stackers deliberately output a neutral 16-bit (or 32-bit) TIFF and defer look to Lightroom/Photoshop; Sequator's built-in look filters are widely judged weak. The newest Mac tool (Satrelle) moves grading in-app with sky-only and foreground-only panels. Luminar Neo is purely the downstream editor.

### Cited Findings
- Sequator options: "Reduce light pollution" (strength; "deep sky" linear vs "uneven" non-linear; "I.A.A." aggressive near dark foreground), "Enhance star light" (strength; needs gradient/irregular mask if foreground), "Auto brightness", "HDR" (fills bit depth), "Reduce dynamic noises" (hot pixels without darks), "Merge pixels" (2×2 binning), colour space sRGB (default) / Adobe RGB / linear sRGB; output 16-bit TIFF or 8-bit JPEG; "Align only" writes aligned linear TIFFs; time-lapse stacks every N frames — [manual](https://sites.google.com/view/sequator/manual)
- Author design stance: deliberately few options; features easily done elsewhere are left out — [Q&A](https://sites.google.com/view/sequator/qa)
- User verdicts: light pollution "ghastly results", Enhance Starlight not useful, distortion no benefit, keep Merge Pixels off; slightly more colour noise than Photoshop — [improvephotography](https://improvephotography.com/?p=50102); for star light "less is more" — [Sky at Night](https://www.skyatnightmagazine.com/astrophotography/star-trails-software-sequator)
- Over-exposure fix: disable Auto brightness, use HDR only; then linear colour space — [Q&A](https://sites.google.com/view/sequator/qa)
- SLS: output TIFF finished in Lightroom/Photoshop/Affinity; save outputs from several algorithms and blend in an editor (e.g., Min Horizon Noise + Max to keep a meteor) — [SLS Processing](https://www.sites.google.com/site/starrylandscapestacker/processing-images-with-starry-landscape-stacker-version-1-9); 1.10 adds optional 32-bit float output (off by default), lens distortion correction (off by default), in-app noise reduction; reviewers lack a preview for temperature/tint input — [App Store](https://apps.apple.com/app/id550326617)
- Pre-stack prep matters most: Frye says Lightroom/ACR prep is "the most important part" — [Frye](https://education.michaelfrye.com/sls/); one guide: reduce contrast, +1 stop, no NR or sharpening, identical on all frames, export TIFF — [microfournerds](https://microfournerds.com/blog/starry-landscape-stacker); Sequator: export 16-bit TIFF without breaking linearity — [Q&A](https://sites.google.com/view/sequator/qa)
- Satrelle: "Foreground editor (Pro)" Lightroom-style exposure/contrast/highlights/shadows/colour/clarity under the mask with live preview and a warning when land is brighter than sky; "Finishing and grade (Pro)" sky-only faint-trail reveal, star colour recovery, warm/cool, contrast, zenith-darkening gradient; star glow/bloom and colour boost; live adjustments update in about a second; 16-bit TIFF Adobe RGB export — [Satrelle](https://satrelle.com/)
- NightScape Lab: "Process" step applies white balance/tone/presence to the base image; base images are 16-bit linear ProPhoto TIFF; export JPEG/TIFF/PNG in sRGB/P3/Adobe RGB/ProPhoto; Lightroom Classic plugin and Photoshop panel — [NightScape Lab](https://nightscapelab.com/)
- StarStaX: blend modes only (Lighten, Darken, Average, Addition, Subtraction, Multiplication; gap filling; comet mode) — [StarStaX](https://www.markus-enzweiler.de/software/starstax/)
- Luminar Neo as finisher: Galactic Hunter's Milky Way results "almost always much better than anything else" via AI presets with strength %, HDR Merge, panorama; no background extraction; not built for start-to-finish astro (Aug 2023) — [Galactic Hunter](https://www.galactic-hunter.com/post/skylum-luminar-neo). Sky Enhancer AI is a single slider (search snippet) — [Skylum blog](https://blog.skylum.com/ai-sky-enhancer-makes-your-photos-beautiful). 2026 test: sky replacement is the highlight, struggles with wires crossing the sky (search snippet) — [Galaxus](https://www.galaxus.ch/en/page/luminar-neo-the-sky-really-is-the-limit-41616)
- Lightroom/Photoshop single-frame AI denoise as stacking alternative: user reports "squiggly" artifacts in the Milky Way with Lightroom Denoise (search snippet) — [NPN](https://community.naturephotographers.network/t/autumn-milky-way-repost/43584); stacking judged best quality but most time-consuming (2016) — [Imaging Resource](https://www.imaging-resource.com/?p=23023)
- Star removal for separate star/sky treatment: StarNet2 produces starless + star-mask files (search snippet) — [Woodworth, StarNet2](https://adamwoodworth.substack.com/p/easier-milky-way-editing-with-starnet2)

### Inferences
- Experienced users want linear, high-bit, colour-managed output and do the look elsewhere; beginners want a decent result in-app. A minimal set (exposure, WB, foreground brightness, optional light-pollution gradient) with linear 16/32-bit TIFF export fits both; heavy "look" filters attract criticism.
- Satrelle's "land brighter than sky" warning is a good example of a realism guard for foreground edits.

### Gaps
- No documentation of Sequator's light-pollution algorithm defaults or strength scale. Whether Luminar Neo added any astro-specific tool in 2025-26 could not be confirmed (none in App Store listing).

## 5. Comparison tools (before/after, single frame vs stack, zoom)

### Takeaway
Comparison support is thin: zoom and frame stepping exist, but no tool documents an in-app single-frame vs stack or A/B algorithm comparison. Users compare by saving multiple outputs and toggling in an editor.

### Cited Findings
- SLS: ←/→ previous/next image; z toggles Fit/Actual Pixels; ⌘+/⌘−/⌘0/⌘9; o toggles overlay — [shortcuts](https://www.sites.google.com/site/starrylandscapestacker/keyboard-shortcuts); zoom Fit to 8× — [1.7](https://www.sites.google.com/site/starrylandscapestacker/new-in-version-1-7); pinch zoom, clickable zoom %, grey image border — [1.8](https://www.sites.google.com/site/starrylandscapestacker/new-in-version-1-8)
- SLS composition choice is compared by saving several algorithm outputs and blending/comparing in an editor — [SLS Processing](https://www.sites.google.com/site/starrylandscapestacker/processing-images-with-starry-landscape-stacker-version-1-9)
- Sequator has a built-in viewer (not colour-managed for Adobe RGB) — [manual](https://sites.google.com/view/sequator/manual); no before/after documented
- StarStaX: "Smooth zoom", "100% view", selectable interpolation, GPU navigation — [StarStaX](https://www.markus-enzweiler.de/software/starstax/)
- Satrelle: half-res Quick Preview; its website uses a before/after slider on 100% crops (marketing; not stated as an app feature) — [Satrelle](https://satrelle.com/). NightScape Lab's site compares matched centre crops vs SLS (10 lights + 10 darks) — [NightScape Lab](https://nightscapelab.com/)
- Photoshop: Difference blend for alignment; Alt-click to view mask; layer toggling — [Night Sky Pix](https://nightskypix.com/how-to-stack-images-in-photoshop-for-astrophotography/)

### Inferences
- A built-in 100% loupe that A/Bs single frame vs stack, and stack A vs stack B (e.g., noise-favouring vs horizon-accuracy), synced on horizon and corners, would answer the questions users currently answer by exporting and toggling.

### Gaps
- No source documents a before/after control inside Sequator, SLS or Satrelle.

## 6. What users praise and complain about; common failure modes

### Takeaway
Praise centres on speed and simplicity (Sequator), automatic star/sky detection (SLS), and replacing tedious Photoshop alignment. Complaints: platform lock-in, mask cleanup time, tree/branch artifacts, horizon star duplication vs noise trade-off, weak look filters, opaque errors, and loss of meteors/light painting.

### Cited Findings
- Photoshop manual stacking: "a tedious pain", often fails to align edge stars — [Woodworth](https://adamwoodworth.substack.com/p/star-stacking-for-pinpoint-stars); ~25 min to align six images vs "literally 2 minutes" in a dedicated tool — [microfournerds](https://microfournerds.com/blog/starry-landscape-stacker); sky blurred "to a grey mess" — [Lonely Speck comments](https://www.lonelyspeck.com/stacking-noise-reduction/comment-page-6/)
- Sequator praise: understood "after just a few minutes" — [improvephotography](https://improvephotography.com/?p=50102); complaints: Windows-only, closed source, EXIF stripping — [pixls.us 2020](https://discuss.pixls.us/t/open-source-tool-for-sky-stacking-and-foreground-extraction/17204)
- SLS praise: "incredibly easy to use", NR better than reviewer's Photoshop workflow, auto reorientation, jet-trail removal, responsive developer; complaints: hours of masking, no mid-mask save, meteors removed with jets, no WB preview, long-exposure-time alignment, confusing exposure errors, wish for min/max weighting slider — [App Store](https://apps.apple.com/app/id550326617)
- Horizon trade-off is built into SLS's algorithm names: "Min Horizon Noise" (default) risks duplicated/suppressed stars near the horizon; "Max Horizon Accuracy" avoids duplication at the cost of noise — [SLS Processing](https://www.sites.google.com/site/starrylandscapestacker/processing-images-with-starry-landscape-stacker-version-1-9), [1.7](https://www.sites.google.com/site/starrylandscapestacker/new-in-version-1-7)
- Long sequences let the horizon cover much of the stacked star field (fewer samples near horizon) — [pixls.us 2023](https://discuss.pixls.us/t/averaging-images-aligning-sky-on-stars/38224)
- Trees: SLS ghosting/streaking/orphan branches with wind — [Stephen Bay](https://stephenbayphotography.com/blog/star-stacking-with-kandao-raw/); halos/middle circle in Sequator from vignetting, not masks — [Q&A](https://sites.google.com/view/sequator/qa)
- Foreground blur with freeze ground on wide lenses/short subs (search snippet) — [IceInSpace](https://www.iceinspace.com.au/forum/showthread.php?p=1477930)
- Edge noise: magenta noise at corners after vignette correction at high ISO; coma at wide apertures — [Woodworth](https://adamwoodworth.substack.com/p/star-stacking-for-pinpoint-stars)
- Clouds blur; smoke/dust ruin frames; planes/meteors/satellites removed — [SLS Shooting](https://www.sites.google.com/site/starrylandscapestacker/shooting-starry-landscapes-for-version-1-9)
- APP users want a Sequator alternative with foreground support — [APP forum](https://www.astropixelprocessor.com/community/main-forum/stacking-with-foreground/); pixls.us users want an open-source equivalent — [pixls.us 2020](https://discuss.pixls.us/t/open-source-tool-for-sky-stacking-and-foreground-extraction/17204)

### Inferences
- The horizon trade-off should be presented as a visible choice with a preview of the horizon band, not a cryptic algorithm name.
- Meteor/light-painting retention (SLS users blend a Max output; Sequator discards single lit frames) supports an explicit "keep transient" option alongside the scene treatment.

### Gaps
- Reddit (r/astrophotography, r/AskAstrophotography), DPReview and Cloudy Nights threads were not retrievable (blocked/403); YouTube comments not accessible. Sentiment above is from other venues.

## 7. Defaults chosen and what experienced users override

### Takeaway
Defaults favour low noise and automation (middle/time-midpoint reference, noise-minimising composite, sRGB, auto brightness); experienced users override toward accuracy and linearity (horizon accuracy, linear colour space, exclusion of bad frames, separate foreground exposures, look filters off).

### Cited Findings
- Sequator defaults: base = middle file; "Align stars" on — [improvephotography](https://improvephotography.com/?p=50102); colour space sRGB — [manual](https://sites.google.com/view/sequator/manual); recommended options marked with a yellow flag — [manual](https://sites.google.com/view/sequator/manual)
- Sequator overrides: Irregular mask for uneven foregrounds; turn off LP/star enhancement/merge pixels — [improvephotography](https://improvephotography.com/?p=50102); freeze ground + dynamic distortion removal + linear colour space with darktable round-trip — [pixls.us 2023](https://discuss.pixls.us/t/averaging-images-aligning-sky-on-stars/38224); HDR-only instead of auto brightness for highlights — [Q&A](https://sites.google.com/view/sequator/qa); median deliberately unsupported (sigma-clipping preferred) — [Q&A](https://sites.google.com/view/sequator/qa)
- SLS defaults: "Min Horizon Noise", "mask with islands of sky", reference nearest time midpoint — [SLS Processing](https://www.sites.google.com/site/starrylandscapestacker/processing-images-with-starry-landscape-stacker-version-1-9); 32-bit float and lens distortion correction off — [App Store](https://apps.apple.com/app/id550326617)
- SLS overrides: "Max Horizon Accuracy" when stars duplicate; "Max" output for meteors; "without islands" when foreground gets mistaken for sky; Align With/Exclude — [SLS Processing](https://www.sites.google.com/site/starrylandscapestacker/processing-images-with-starry-landscape-stacker-version-1-9), [1.7](https://www.sites.google.com/site/starrylandscapestacker/new-in-version-1-7)
- Photoshop: Auto-Align "Auto" then manual; Median for sky (tolerant of misalignment), Mean for foreground — [Night Sky Pix](https://nightskypix.com/how-to-stack-images-in-photoshop-for-astrophotography/)
- Capture overrides by experts: ~20 sky frames, NPF exposure pushed ~25% longer, ISO 3200-6400, separate long foreground exposures — [Woodworth](https://adamwoodworth.substack.com/p/star-stacking-for-pinpoint-stars); R5 at ISO 1600 and 4 s — [SLS Shooting](https://www.sites.google.com/site/starrylandscapestacker/shooting-starry-landscapes-for-version-1-9)

### Inferences
- Star Photos could default to noise-favouring integration but show the horizon band comparison at the inspect step so the override is discoverable; linear 16-bit plus optional 32-bit float export should be first-class for experts.

### Gaps
- No source states Sequator's default light-pollution, star-enhancement or auto-brightness states on first launch.
