# Make review the product, reuse LightCraft's egui

Star Photos should be built on LightCraft's egui/eframe stack and should compete on the thing every existing stacker hides: the evidence behind its automatic decisions. Sequator and Starry Landscape Stacker (SLS) are praised for speed and automatic star detection. Neither scores frames, shows alignment quality or compares results in the app, so users cull frames by hand, decode cryptic algorithm names, spend hours cleaning masks and export several stacks to compare in Photoshop. Deep-sky suites supply the patterns worth copying. PixInsight's WBPP shows a pre-run plan with traffic-light status and pauses for a decision before resuming, and Siril links a metric plot to its frame list. The 2025–26 entrants Satrelle and NightScape Lab are converging on Star Photos' own guided flow, so automation alone will not set it apart. egui is the only Rust toolkit that can share Star Photos' wgpu 30 device through a stable API, and LightCraft Classic has already solved Display P3 and EDR output on it; Slint is the only credible fallback. The aurora review was hard because it broke the perceptual conditions for spotting subtle differences: three versions instead of two, compressed images, and captions that said what differed but not where to look. The fix is an instant toggle with no gap between two pixel-aligned full-resolution renderings, plus automatically chosen diagnostic crops. Only two openly licensed RAW nightscape sequences exist: a CC0 Canon fisheye set of 2×5 frames and a CC BY-SA set of 5+1 frames from pixls.us. A real test and example deck therefore has to be solicited under CC0 from pixls.us members who already hold series of 15 to 250 frames.

## Competitors automate the stack but hide the evidence

Every tool in this space now automates the mechanics: sorting frames, picking a reference and aligning stars. What separates them is how much of the decision-making they show. Nightscape tools are weakest exactly where Star Photos' guided flow puts its review steps.

### Nightscape stackers trade mask effort for speed and stay opaque

Sequator remains the benchmark for speed and simplicity. It is free but closed-source and Windows-only, its last release (1.6.2 r2) dates from **August 2024**, and its author has no plans for a Mac version ([Sequator download](https://sites.google.com/view/sequator/download), [Sequator Q&A](https://sites.google.com/view/sequator/qa)). Users drop files into role buckets (star, base, noise and vignetting images). The base frame defaults to the middle file, and the output path is chosen before processing starts ([Sequator manual](https://sites.google.com/view/sequator/manual), [Improve Photography](https://improvephotography.com/?p=50102)). Its foreground model asks little of the user. A rough boundary line, gradient or brushed sky region is enough, and *Freeze ground* then separates moving pixels from stable ones ([Sequator manual](https://sites.google.com/view/sequator/manual)). Reviewers praise exactly that: sky masking took seconds, the app made sense within minutes, and **10 frames stacked in about a minute versus three in Photoshop** ([Improve Photography](https://improvephotography.com/?p=50102)).

The price is opacity. The author deliberately ships no frame-quality score, because a simple index cannot tell thin cloud from aircraft, rising stars or fireworks, so culling stays manual. Alignment failures are diagnosed through a FAQ that maps symptoms to settings: corner trails point to distortion correction, and halos point to bad flats. A single light-painted frame is discarded as noise ([Sequator Q&A](https://sites.google.com/view/sequator/qa)). The built-in look tools are weak: one reviewer judged its light-pollution reduction terrible and its star enhancement useless ([Improve Photography](https://improvephotography.com/?p=50102)).

Starry Landscape Stacker (**$39.99**, macOS) is Sequator's Mac counterpart and makes the opposite trade: more automatic detection, more cleanup ([App Store](https://apps.apple.com/app/id550326617)). It sorts lights, darks and flats automatically, with a classification panel for corrections. It puts a red dot on each detected star and grows a blue sky mask from those dots with *Find Sky*, then aligns and composites in one run ([SLS processing guide](https://www.sites.google.com/site/starrylandscapestacker/processing-images-with-starry-landscape-stacker-version-1-9)). Users rate its star detection as excellent ([Woodworth](https://adamwoodworth.substack.com/p/star-stacking-for-pinpoint-stars)). App Store reviewers also report masking that **takes hours with no way to save midway**, meteors deleted along with jet trails, confusing exposure errors and no white-balance preview ([App Store](https://apps.apple.com/app/id550326617)). The horizon trade-off is hidden in algorithm names. *Min Horizon Noise*, the default, risks duplicated stars near the horizon, and *Max Horizon Accuracy* avoids them at the cost of noise. Comparing the two means saving several outputs and blending them in an editor ([SLS processing guide](https://www.sites.google.com/site/starrylandscapestacker/processing-images-with-starry-landscape-stacker-version-1-9)).

Trees in wind defeat every tool. In a 15-frame test, SLS produced streaked canopy edges, ghosted and orphaned branches, and stars missing from gaps, all of which needed extensive clone stamping. Kandao Raw+, which aligns small segments without any mask, kept the branches intact but ringed bright stars with salt-and-pepper artifacts ([Stephen Bay](https://stephenbayphotography.com/blog/star-stacking-with-kandao-raw/)).

The newest entrants are converging on Star Photos' design, which narrows the window for differentiation. Satrelle runs on Apple Silicon and costs **$59** one-time; after its trial, a free mode caps stacks at 200 frames. Its Frame Review grid automatically flags headlights, planes, clouds and stray darks for one-click exclusion, and its half-resolution Quick Preview renders in seconds. It blends a separately lit foreground exposure and warns when foreground edits make the land brighter than the sky ([Satrelle](https://satrelle.com/)). NightScape Lab has been announced at **$129** for macOS, Windows and Linux but is not yet released. It aligns sky and ground separately and keeps the sky mask visible and editable, because automatic tools sometimes misplace the horizon ([NightScape Lab](https://nightscapelab.com/)).

Demand outside these products is explicit. Astro Pixel Processor (APP) still lacks foreground stacking in its October 2026 betas, while its users say that feature would make them buy it ([APP forum](https://www.astropixelprocessor.com/community/main-forum/stacking-with-foreground/)). pixls.us users have asked for an open-source Sequator equivalent since 2020 ([pixls.us](https://discuss.pixls.us/t/open-source-tool-for-sky-stacking-and-foreground-extraction/17204)). No tool found shows per-frame alignment quality or offers an in-app A/B comparison of stacking algorithms. None makes the scene treatment (hero frame, light painting, keeping meteors) an explicit step of its own.

### Deep-sky suites show the plan first and pause only for decisions

The astro suites have no fixed-foreground support at all. Siril users stack twice, once aligned on the stars and once unaligned, and blend the two in GIMP ([pixls.us Siril tutorial](https://discuss.pixls.us/t/processing-a-nightscape-in-siril-a-tutorial/10996)). But these suites present an automatic pipeline far better.

PixInsight's WBPP opens with a Control Panel before anything runs. Each frame group gets a green, red or yellow status icon and a diagram of its planned calibration flow, and a manual override shows its own icon ([AstroWorldCreations](https://www.astroworldcreations.com/blog/six-most-important-improvements-in-wbpp-20)). Since version 2.4.5, WBPP pauses the batch so the user can evaluate a generated reference, then continues once the user accepts it ([WBPP 2.4.5](https://pixinsight.com/mail/20220607-WBPP-2.4.5-release/)). Version 2.9.0 (January 2026) added an optional Frame Selection pause, interactive by default and formula-driven on request ([Weber](https://blog.briangweber.com/wbpp-frame-selection/)). Version 3.1.0 (September 2026) added recovery of interrupted runs ([PixInsight](https://pixinsight.net/dev/index.php?threads%2Fwbpp-2-9-0-release-announcement.7=)).

DeepSkyStacker (DSS) recaps the steps before stacking. It matches darks to lights by ISO and exposure automatically and warns on a mismatch ([DSS manual](http://deepskystacker.free.fr/english/userguide.htm)). It also caches registration per file and writes an AutoSave.tif so a crash never loses a finished stack ([DSS technical](http://deepskystacker.free.fr/english/technical.htm)). Siril links a metric plot (FWHM, roundness, background, star count) to a sortable frame list, and dragging a selection on the plot excludes frames in bulk ([Siril plot docs](https://siril.readthedocs.io/en/stable/Plot.html)).

The suites' failures are just as instructive.

**Relative composite scores mislead.** DSS documents its score as useful only for sorting ([DSS FAQ](http://deepskystacker.free.fr/english/faq.htm)). APP's score, star count × size × roundness relative to the dataset, ranked trailed frames above clean ones. A blink mode first requested in 2019 was still on APP's wish list in 2021 ([APP FAQ](https://www.astropixelprocessor.com/community/faq/quality-score-caculation/), [APP blink RFC](https://www.astropixelprocessor.com/community/rfcs-request-for-changes/blink-mode-to-remove-bad-frames/)).

**Warnings without consequences confuse.** APP's "CRITICAL WARNING" popup about flat calibration left a user unsure what it meant, and a moderator later put it down to a rare condition ([APP forum](https://www.astropixelprocessor.com/community/main-forum/critical-warning-flat-field-calibration-can-not-be-performed-correctly-2/)).

**Implicit structure loses beginners.** Siril's required named folders drew complaints that stacking was unintuitive ([pixls.us](https://discuss.pixls.us/t/manual-for-stacking-in-siril-0-99-6/21506)). Its mandatory free-space check misreads macOS external drives and cannot be bypassed ([pixls.us](https://discuss.pixls.us/t/bypass-siril-free-disk-space-calculations/46839)).

**Display stretch is sound in principle but leaks in practice.** Siril states plainly that "Only Linear mode ... reflects the true pixel values" ([Siril toolbar docs](https://siril.readthedocs.io/en/stable/GUI/toolbar.html)). APP's preview sharpening, by contrast, silently drops out of exports saved without the stretch ([APP forum](https://www.astropixelprocessor.com/community/main-forum/odd-sharpen-question/)).

**Documentation lags everywhere.** APP's full manual, promised in 2019, was still unfinished in June 2026 ([APP beta46](https://www.astropixelprocessor.com/community/release-information/astro-pixel-processor-2-0-0-beta46-release-notes/)).

Affinity offers the cheapest high-value pattern: each stack accumulates in a Stacked Images list, so users can change settings, re-stack and compare before applying a result ([Affinity help](https://affinity.help/photo2/en-US.lproj/pages/Astrophotography/astro_creating.html)).

The combined lesson is that beginners succeed through a one-press path with sensible defaults. Trust, however, comes from seeing the plan before the run and the reasons after it. Star Photos already computes the evidence its competitors hide: rejection maps for aircraft and satellite trails, coverage maps, and held-out registration residuals of about **0.2 px** median on the owner's sequences ([Star Photos plan](/Users/andermic/Development/star-photos/docs/plan.md)). The product opportunity is to show that evidence, not to compute more.

## egui is the only toolkit that shares wgpu 30 without a fork

The toolkit question comes down to one hard requirement. Star Photos' wgpu 30 compute output must reach the screen without leaving the GPU, at 45 MP, colour-managed on wide-gamut Mac displays. egui/eframe meets it through stable API. egui-wgpu 0.36.2 depends on wgpu 30, and eframe accepts an app-created instance, adapter, device and queue through `WgpuSetup::Existing`. Custom drawing goes through `register_native_texture` or a `CallbackTrait` paint callback ([local egui-wgpu setup.rs](/Users/andermic/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-wgpu-0.36.2/src/setup.rs)). egui is MIT/Apache-licensed and shipped **four minor releases in 2026**, through 0.37.0 on 15 October. It integrates AccessKit and the kittest test harness ([egui releases](https://github.com/emilk/egui/releases)), and LightCraft Classic already proves it on 24–45 MP photo editing.

| Toolkit | Latest release | Shares a wgpu 30 device and textures | Main disqualifier or risk |
|---|---|---|---|
| egui/eframe | 0.37.0 (2026-10-15) | Yes, stable API | 8-bit surface with no colour-space control; non-native look |
| Slint | 1.18.1 (2026-09-21) | Behind the `unstable-wgpu-30` feature | Interop can change in minor releases; GPLv3 or attribution licence |
| iced | 0.14.0 (2025-12-07) | No (wgpu 27) | No upstream accessibility; no release in about 11 months |
| Tauri / Dioxus | 2.12.5 / 0.7.10 | No (webview; Dioxus Native is on wgpu 26) | Full-resolution pixels must cross a webview |
| GTK4 / Qt | gtk4-rs 0.11.5 / cxx-qt 0.10.0 | No | OpenGL renderer on macOS (GTK); C++ bridge and LGPL (Qt) |
| Makepad, Xilem, floem | 1.0.0 / 0.4.0 / 0.2.0 | No | Own renderer, alpha, or stale |

egui's limits for a photo app are real. egui-wgpu picks an 8-bit surface itself and exposes no colour-space setting ([local egui-wgpu lib.rs](/Users/andermic/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-wgpu-0.36.2/src/lib.rs)). Its own README concedes that immediate mode makes layout harder and that a native look is not a goal ([egui README](https://github.com/emilk/egui)).

There is also a colour trap beneath every wgpu toolkit on macOS. wgpu 30 added explicit surface colour spaces ([wgpu releases](https://github.com/gfx-rs/wgpu/releases)). But its `Srgb` option on Metal leaves the layer's colour space unset, which Apple documents as content that is not colour-matched ([Apple CAMetalLayer.colorspace](https://developer.apple.com/documentation/quartzcore/cametallayer/colorspace)). LightCraft hit exactly this as over-saturated previews on P3 screens ([LightCraft Classic display-profiles.md](/Users/andermic/Development/lightcraft-classic/docs/display-profiles.md)). Star Photos must therefore either select `DisplayP3` or `ExtendedSrgbLinear` explicitly, or colour-manage inside the app. Windows needs its own path, because DX12 does not report `DisplayP3` ([local wgpu-types surface.rs](/Users/andermic/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/wgpu-types-30.0.1/src/surface.rs)).

LightCraft has already paid for most of the workarounds Star Photos needs:

- **Display colour.** Classic reads the screen's ICC profile about once a second and renders the loupe into the display's own primaries. It has profile readers for macOS, Linux and Windows ([display-profiles.md](/Users/andermic/Development/lightcraft-classic/docs/display-profiles.md)).
- **EDR output.** It vendors egui-wgpu to add an `Rgba16Float` + `ExtendedSrgbLinear` EDR surface, with egui's own drawing scaled to SDR paper white ([vendor README](/Users/andermic/Development/lightcraft-classic/vendor/README.md)).
- **Sharp 100% views.** It snaps 100% views to physical pixels, because a half-pixel offset blurs the image through bilinear filtering ([detail.rs](/Users/andermic/Development/lightcraft-classic/crates/ui-egui/src/panels/detail.rs)).
- **Viewing.** A slot-based render queue puts the loupe first, and a synced compare view is available ([render.rs](/Users/andermic/Development/lightcraft-classic/crates/ui-egui/src/render.rs), [compare.rs](/Users/andermic/Development/lightcraft-classic/crates/ui-egui/src/panels/compare.rs)).
- **Background jobs.** A single cancellable activity registry tracks them ([background-tasks.md](/Users/andermic/Development/lightcraft-classic/docs/background-tasks.md)).
- **Testing.** A CPU rasterizer of egui output makes headless snapshot tests possible ([headless.rs](/Users/andermic/Development/lightcraft-classic/crates/ui-egui/src/headless.rs)).
- **Shared crates.** Much of this is already factored into craft-libs (ICC transforms in craft-cms, native menus in craft-menu, controls, window, settings, keymap, i18n and update) under MIT OR Apache-2.0 ([craft-libs README](/Users/andermic/.cargo/git/checkouts/craft-libs-d830b991a08c6c3d/1e6f127/README.md)).
- **Licence.** Classic is MIT except two Apache-2.0-only crates, `segment` and `fetch` ([NOTICE](/Users/andermic/Development/lightcraft-classic/NOTICE)).

Three cautions temper the reuse case:

1. **Zero-copy display is unproven.** Classic never shared one device between compute and window. Its compute pipeline creates its own device and reads results back, so SDR previews round-trip through the CPU ([ctx.rs](/Users/andermic/Development/lightcraft-classic/crates/gpu/src/ctx.rs)). egui's API supports zero-copy display, but no sibling app has proven it.
2. **Avoid the fork.** The newer LightCraft pins a full egui and winit fork. Its 0.37 migration plan runs to **1,552 lines**, and a trial build produced 17 library and 26 test errors ([egui-037 migration plan](/Users/andermic/Development/lightcraft/docs/egui-037-migration-plan.md)). That fork exists mainly for text input, IME and bidirectional text, which Star Photos barely needs. Stock egui plus Classic's small HDR patch is the lower-risk base.
3. **craft-libs must be published first.** Newer craft-libs revisions need a separate public publication step before outside builders can fetch them ([migration plan](/Users/andermic/Development/lightcraft/docs/egui-037-migration-plan.md)). Star Photos' first release is meant for the public ([requirements](/Users/andermic/Development/star-photos/docs/requirements.md)), so this step has to happen.

Slint 1.18 is the only credible alternative. It offers declarative UI, AccessKit and native menus, and it can import a wgpu 30 texture on a shared device. But that interop sits behind a feature its docs say may change in minor releases, and the licence requires GPLv3 or visible attribution ([Slint wgpu_30](https://docs.slint.dev/latest/docs/rust/slint/wgpu_30/), [Slint licence](https://github.com/slint-ui/slint/blob/master/LICENSE.md)). Its native-looking styles are being phased out ([Slint blog](https://slint.dev/blog/default-native-style-change)), and none of LightCraft's code would carry over.

RapidRAW is the closest outside analogue: a Tauri/React RAW editor with a wgpu engine. It added a direct WGPU renderer in April 2026, after earlier replacing slow Base64 image transfer over IPC ([RapidRAW](https://github.com/CyberTimon/RapidRAW)). That history is direct evidence that full-resolution pixels through a webview become a bottleneck.

## Compare two renderings with no gap between them

### Flicker without a blank finds differences in about one swap

Perception research gives an unusually sharp answer to how to compare renderings. Rensink, O'Regan and Clark alternated an original and a modified image. With an **80 ms grey blank** between frames, observers needed 7.3 alternations to find a change in a region they found interesting and **17.1 in a marginal one**, and some changes took more than 80. With the blank removed, they found changes in **1.4 alternations (0.9 s)**, and the interesting-versus-marginal difference vanished ([Rensink et al. 1997](https://www2.psych.ubc.ca/~rensink/publications/download/PsychSci97-RR.pdf)). Small distracting transients elsewhere in the view cause the same blindness as a blank ([Perspectives on Psychological Science 2018](https://journals.sagepub.com/doi/10.1177/1745691617707269)). Side-by-side search, by contrast, proceeds through repeated eye jumps between the images, because people avoid holding features in memory ([Atkins et al. 2007](https://www2.cs.sfu.ca/~stella/papers/2007/tr.pdf)).

Image-quality standards build on the same finding. JPEG AIC-3 research "boosts" subtle differences in three ways: it crops to half size and upscales, amplifies pixel differences ×2, and flickers at 10 Hz. Boosting enlarged the measured quality scales about **twofold** and moved the just-noticeable-difference threshold from distortion level 4 to about level 2 ([arXiv 2410.09501](https://arxiv.org/html/2410.09501)).

This evidence is strongest for *detecting* differences, not for choosing the better look. For judging a whole rendering or narrowing down three or more versions, a side-by-side or survey grid remains appropriate. Mature tools combine the modes:

- **Lightroom** goes from Survey to a two-up Compare with linked focus, and a backslash toggles before/after in place ([Julieanne Kost](https://jkost.com/blog/?p=22615), [Kost 2024](https://jkost.com/blog/2024/08/the-history-panel-and-before-after-view-in-lightroom-classic.html)).
- **darktable** offers a split view whose dividing line rotates, plus hold-to-peek on version thumbnails ([darktable snapshots](https://docs.darktable.org/usermanual/development/en/module-reference/utility-modules/darkroom/snapshots/), [duplicate manager](https://docs.darktable.org/usermanual/5.6/en/module-reference/utility-modules/darkroom/duplicate-manager/)).

Shipping tools still get fairness wrong. Photoshop's Quick Compare beta (September 2026) misaligns its split view on cropped images ([Adobe community](https://community.adobe.com/announcements-698/quick-compare-is-now-in-photoshop-beta-1640291)), and DxO users find its noise-reduction preview loupe too small to judge from ([DxO forum](https://forum.dxo.com/t/noise-reduction-preview/29882)).

### Why three aurora renderings resisted judgment

The aurora review page broke nearly every condition the research identifies.

1. **Three versions instead of two.** Three renderings turn one comparison into three pairwise comparisons held in memory, whereas the research designs present pairs.
2. **Compressed, likely downscaled images.** JPEGs, probably downscaled, averaged away the attributes that distinguished the versions: grain, star sharpness and thin halos. They also added compression artifacts of their own.
3. **Captions said what, not where.** The captions described each version but did not say where to look. Changes in regions viewers consider marginal, such as the sky background or tree edges, take more than twice as many alternations to find ([Rensink et al. 1997](https://www2.psych.ubc.ca/~rensink/publications/download/PsychSci97-RR.pdf)).
4. **A browser swap is not instant.** Swapping images in a browser risks a decode delay or a layout shift, and either acts as the blank or distracting transient that causes change blindness. This is an inference: the page's actual swap timing was not measured.
5. **The verdict was per-attribute.** The conclusion that each version had pros and cons is a per-attribute trade-off that one global choice cannot express. The owner's eventual decision was itself stated per attribute: a 9-frame moment with deep blacks ([Star Photos plan](/Users/andermic/Development/star-photos/docs/plan.md)).

The remedy follows directly:

- **Pick two, then toggle.** Choose two versions and toggle them instantly at full resolution, with linked zoom.
- **Crop grid by region.** Add a crop grid with one row per named region and one column per version.
- **Per-region scorecard.** Score noise, star sharpness, sky darkness, foreground brightness and halos per region.
- **Combine winners (owner decision).** Possibly let the user combine the winning regions from different versions.

### Fair comparison needs one display transform and chosen regions

Fairness means every version passes through the identical display transform, zoom and position. Astronomy tools learned this the hard way. MaxIm DL has an explicit command that equalizes the screen stretch across images before blinking ([MaxIm DL help](https://cdn.diffractionlimited.com/help/maximdl/HID_VIEW_STRETCH_ALL.htm)). PixInsight Blink users run into the tension between one shared stretch, which is fair, and a per-frame auto-stretch, which is readable but can fake or hide differences ([AstroBin](https://app.astrobin.com/forum/topic/211600/how-to-use-blink-efficiently)).

A stacker can go further than any photo editor, because all of its renderings share the reference geometry, and it already knows the sky mask and the star list. It can therefore choose diagnostic crops automatically:

- the darkest smooth sky patch, for noise and gradient;
- the brightest unsaturated star and a median star, for bloat, trailing and colour;
- the segment of the horizon with the strongest gradient, for halos and mask seams;
- the darkest foreground, for foreground noise and brightness;
- the aurora or Milky Way core, for structure and colour.

Clicking a crop recentres the main view there. This takes JPEG AIC's zoom boosting and fixes DxO's "too small, wrong place" loupe by choosing the places for the user. No shipping product does this, so it is a genuine differentiator, but there is also no usability evidence for it yet. An optional "match brightness" toggle helps when judging noise alone. It must be clearly labelled, because sky darkness and foreground brightness are themselves among the attributes being judged.

### Guided controls need one disclosure level and a non-modal step list

Nielsen Norman Group finds that designs with more than two disclosure levels usually perform poorly. It also finds that wizards suit only steps that barely interact ([NN/g progressive disclosure](https://www.nngroup.com/articles/progressive-disclosure/), [NN/g wizards](https://www.nngroup.com/articles/wizards/)). Apple's HIG limits a view to one disclosure button ([Apple HIG](https://developer.apple.com/design/human-interface-guidelines/disclosure-controls)). Stacking steps interact strongly: alignment changes noise, and the mask changes halos. Star Photos' six steps should therefore be a visible, clickable step list with automatic defaults, not a locked wizard, and the comparison view should be the hub every step returns to.

Within each step, photo apps converge on one guided layer above the full controls:

- **Apple Photos** puts a master slider over a single Options disclosure ([Apple Support](https://support.apple.com/guide/photos/pht806aea6a6)).
- **Lightroom** previews a profile on hover and adds an Amount slider from 0 to 200 ([Adobe](https://helpx.adobe.com/ca/lightroom-classic/desktop/help/whats-new/2022-4.html)).
- **darktable** offers a curated beginner module set alongside the full list ([darktable module groups](https://docs.darktable.org/usermanual/5.6/en/darkroom/organization/module-groups/)).

For automatic decisions, Microsoft's 18 human-AI guidelines, validated with 49 practitioners across 20 products, ask systems to say how well they work, explain why they acted, make correction cheap, offer options when in doubt, and change behaviour cautiously ([Microsoft Research](https://www.microsoft.com/en-us/research/blog/guidelines-for-human-ai-interaction/)). Translated to Star Photos:

- the mask overlay is off by default and toggled with one key, as Lightroom does with O ([Lightroom Killer Tips](https://lightroomkillertips.com/exploring-masking-overlay-options/));
- uncertainty is tinted only along the horizon band, not shown as a full-image heat map;
- a short summary such as "used 18 of 20 frames; 2 looked trailed" replaces raw scores;
- two or three candidate masks are offered when segmentation is unsure;
- decisions stay stable across re-runs unless the inputs change.

No photo tool shows per-pixel mask confidence today, so this design also lacks precedent.

## Only two open sequences exist, so the deck must be solicited

The open supply of RAW nightscape sequences is almost empty:

- **Stacker vendors** publish no openly licensed nightscape sample sets. Their practice data is deep-sky, unlicensed or broken ([AstroBackyard](https://astrobackyard.com/dss-practice-files/)).
- **Wikimedia Commons and Flickr** cannot host RAW files at all ([Commons file types](https://commons.wikimedia.org/wiki/Commons:File_types), [Flickr](https://www.flickrhelp.com/hc/en-us/articles/4404079649300-Flickr-upload-requirements)).
- **raw.pixls.us** is CC0, but it explicitly asks for daylight, low-ISO single frames and not series ([raw.pixls.us](https://raw.pixls.us/)).
- **pixls.us PlayRaw** threads must be freely licensed, but each post sets its own licence ([PlayRaw rules](https://discuss.pixls.us/t/about-the-play-raw-category/11956)). The category holds more than 25 night-sky RAWs, almost all single frames.

| Candidate | Licence | Frames | Best use |
|---|---|---|---|
| Zenodo 7411889 (Canon EOS 6D, 8 mm fisheye, 30 s, ISO 6400) ([Zenodo](https://zenodo.org/records/7411889)) | CC0, verified | 2 bursts × 5 CR2, ~231 MB | Wide-field distortion edge case; scene and foreground unverified |
| pixls #17204 ([pixls](https://discuss.pixls.us/t/open-source-tool-for-sky-stacking-and-foreground-extraction/17204)) | CC BY-SA 4.0, verified | 5 sky + 1 foreground | Smoke test of sky/foreground separation; mega.nz link unverified |
| Siril nightscape tutorial (Fujifilm X-T2, 12 mm f/2) ([pixls](https://discuss.pixls.us/t/processing-a-nightscape-in-siril/12167)) | None stated for RAWs | 10 lights + 50 calibration frames | Strongest set with calibration frames and X-Trans; needs written permission |
| Hasselblad 3FR ([GitHub release](https://github.com/Benitoite/raw-test/releases/tag/3FR)) | CC0 | 1 | Medium-format decoder coverage |
| PlayRaw single frames (e.g. [#10103](https://discuss.pixls.us/t/play-raw-nightscape-from-orange-free-state/10103), aurora [#54119](https://discuss.pixls.us/t/aurora-borealis-in-northern-illinois-usa/54119)) | CC BY / CC BY-SA | 1 each | Format diversity (CR2, CR3, NEF, ARW, RAF, ORF, RW2, PEF), "stack of 1" baselines |

Licence type matters as much as availability. CC BY-SA attaches to derivatives, so crops, previews and reference stacks made from #17204 must also ship as BY-SA with attribution ([CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/)). The deck therefore needs a per-file licence and attribution manifest, kept separate from the code licence. NonCommercial and NoDerivatives files belong out of an open-source project whose downstream use may be commercial. That excludes the one public example of the exact tracked-sky-plus-foreground case, Jackson Lake, which is licensed CC BY-NC ([pixls #38152](https://discuss.pixls.us/t/jackson-lake-state-park-tracked-milky-way/38152)).

Better data exists in private hands. Several pixls.us members have said publicly that they hold the series a deck needs:

- **jchnkl:** 16 frames plus a 160-second foreground of a light-polluted Franconian chapel, with an offer to share the RAWs ([pixls](https://discuss.pixls.us/t/the-milky-way-over-rural-franconia-feel-free-to-critique/12969)).
- **erratasama:** 60 frames of Mount Timpanogos ([pixls](https://discuss.pixls.us/t/mount-timpanogos-at-night-with-friendly-cloud/29584)).
- **Waveluke:** more than 100 Sony A7 II frames with darks and a flat field ([pixls](https://discuss.pixls.us/t/noise-floor-and-flatfield-correction/20974)).
- **nathanm:** 250 × 5-second Canon R5 frames that show corner star elongation ([pixls](https://discuss.pixls.us/t/siril-needs-distortion-correction-in-stacking/20991)).
- **fujiokae:** 90 untracked Nikon D7200 frames with the same corner distortion problem ([pixls](https://discuss.pixls.us/t/stacking-non-tracked-images-got-severe-distortion/35487)).
- **plaven:** a 172-frame Canon 5D III timelapse ([pixls](https://discuss.pixls.us/t/foss-time-lapse-of-the-milky-way/2992)).

Members already host large CC0 RAWs as GitHub release assets, with limits of under 2 GB per asset and 1,000 assets per release ([pixls #54544](https://discuss.pixls.us/t/where-to-host-large-file-size-raws-for-a-playraw/54544)). A Zenodo record adds a DOI and an explicit licence field ([Zenodo](https://zenodo.org/records/7411889)).

The practical route is a public pixls call with a short brief: CC0 preferred, full untouched RAW series, darks if shot, and a one-line scene note. The project hosts the result. Direct messages go to the named holders, and the Siril tutorial author is asked to state a licence. Because the owner's archive is excluded from the open deck, any remaining gaps need sets shot or commissioned from other contributors.

The deck should cover each scene type below with at least 15 consecutive tripod frames. Pixls stacking guidance puts the useful range at 10–20 lights ([pixls](https://discuss.pixls.us/t/first-outing-of-the-new-year-the-creations/10658)).

- **Showcase:** a dark-sky Milky Way over a simple horizon.
- **Complex edges:** tree silhouettes with gaps in them.
- **Motion:** wind-blown foliage and water reflections.
- **Sky conditions:** strong light pollution, moonlight, aurora, passing cloud.
- **Transients:** satellites, aircraft and meteors.
- **Light changes:** light painting and car headlights.
- **Camera faults:** a tripod bump.
- **Optics:** a fisheye or 14 mm-or-wider lens at f/1.4–2.
- **Long series:** 100 or more frames.
- **Calibration:** one set with darks, flats and bias, and one with lights only.
- **Separate exposures:** separately exposed sky and foreground.
- **Sensors:** Bayer, X-Trans and Micro Four Thirds.

## Design directions: adopt, avoid, decide, and validate

The patterns above map onto Star Photos' six steps as follows.

| Step | Adopt | Avoid |
|---|---|---|
| Import and validate | Roles sorted automatically from EXIF, with an override panel (SLS). A pre-run plan with traffic-light status (WBPP, DSS). Mismatches stated with their consequence and a choice ("12 darks are 30 s, lights are 20 s: use, scale or skip"). Cheap format and size checks at import | Choosing the output path up front (Sequator). Modal critical-warning popups (APP). Hard blocks with no "continue anyway" (Siril disk check) |
| Review frames | A grid or filmstrip with flags that name their reason and show the metric behind each (Satrelle, Siril). Blink a flagged frame against the reference. A short summary of what was used and why | A single relative score (DSS, APP). A blind "keep best X%" |
| Review sky/ground | Automatic mask with the overlay off by default and one toggle key. Uncertainty tinted only along the boundary. A rough-correction brush. 2–3 candidate masks when unsure. A savable mask with autosave | Hours of dot-painting with no midway save (SLS). Full-image confidence heat maps |
| Choose scene treatment | Named treatments (hero frame, long-exposure foreground, keep transients), each with a live thumbnail of this stack. A Stacked Images-style list for re-running and comparing (Affinity) | Algorithm names as choices (*Min Horizon Noise*). Silently discarding a single lit frame (Sequator) |
| Inspect alignment and integration | Per-frame residuals, rejection and coverage maps (already computed). A horizon-band A/B between noise-favouring and accuracy-favouring integration | Black-box alignment with diagnosis only in a FAQ |
| Compare (hub) | Two renderings, GPU-resident, swapped in place on a key with no intermediate frame. Linked zoom and pan, identical display transform. Automatically chosen diagnostic crops. A split view with a rotatable line for horizons | Three-way toggles. Downscaled JPEGs. Auto-stretch per version |
| Controls and export | Guided controls with one disclosure level. Hold-to-peek per control. Display stretch labelled as display-only. Linear 16-bit and 32-bit float export. A realism guard against land brighter than sky | Locked wizards. Heavy look filters (Sequator). Preview filters that leak into export (APP) |
| Long runs | A half-resolution quick preview (Satrelle). LightCraft's single cancellable activity registry. Analysis cached against inputs and settings. A provisional result written atomically (DSS AutoSave) | Separate progress windows. Errors that do not name the frame and the step |

The realism guard has direct support from the owner. The Yellowstone export read as over-processed, with the foreground pushed too far to match the sky ([Star Photos plan](/Users/andermic/Development/star-photos/docs/plan.md)), which is exactly the failure that Satrelle's warning about land brighter than sky targets.

Several decisions belong to the owner, and most are already listed as open in the requirements ([requirements](/Users/andermic/Development/star-photos/docs/requirements.md)):

1. **Outbound licence.** It decides whether Slint's GPLv3 option is even viable. It also decides whether CC BY-SA fixtures and the reference outputs derived from them can live in the main repository or need a separately licensed data repository.
2. **Toolkit ADR.** Choose between stock egui with Classic's HDR patch (recommended), adopting the storytold fork, or Slint. The same decision covers when craft-libs gets its public publication, so outside builders can reproduce releases.
3. **In-app look depth.** Choose between neutral linear output with a few guided controls, the stance of Sequator and SLS and closer to the owner's note about keeping the foreground near the frame as shot, and Satrelle-style sky-only grading.
4. **Combining winners.** Decide whether users may combine winning regions across renderings beyond the already-decided hero-frame workflow, for example the sky from one integration and the horizon band from another.
5. **Dataset policy.** Decide whether the deck accepts only CC0 or also CC BY-SA, who runs the pixls call, and whether hosting uses GitHub releases or Zenodo.
6. **Usability testing.** Decide whether automatically chosen crops and boundary-confidence tints, both without precedent, get a small test with nightscape photographers before they become the default.

The toolkit validation spike should test the riskiest unproven claim, zero-copy display on a shared device, before any UI work begins. It is a minimal eframe app with these steps:

1. Create Star Photos' wgpu 30 device and pass it to eframe through `WgpuSetup::Existing`.
2. Render a 45 MP `Rgba16Float` stack through a `CallbackTrait` tile renderer with no CPU readback.
3. Measure pan and zoom frame times at fit, 100% and 200% on an Apple Silicon Mac and on an Intel Mac with an AMD GPU. LightCraft's GPU path is proven on Apple hardware only ([LightCraft gaps](/Users/andermic/Development/lightcraft-classic/docs/gaps.md)).
4. Verify P3 accuracy both through Classic's in-app ICC path and through wgpu `DisplayP3` with the patched egui-wgpu. Verify EDR through `ExtendedSrgbLinear`, and never rely on `Srgb` on Metal.
5. Swap two GPU-resident renderings on a key and confirm with frame capture that the next presented frame shows the other rendering, with no blank or layout change in between.
6. Confirm pixel snapping at 100%, AccessKit names on canvas controls, a kittest snapshot at two pixel densities, and a muda menu with an rfd dialog.

The owner should set numeric pass thresholds in the ADR. A reasonable proposal, not drawn from the sources, is steady 60 fps panning at 100% on the slowest target Mac. Build the same spike in Slint only if egui fails a threshold or the owner wants a declarative UI badly enough to accept its licence terms.

## Conclusion

The competitive landscape has shifted while Star Photos was being designed. Automatic frame flagging, editable sky masks and separately aligned foregrounds were differentiators in 2024 and are table stakes in Satrelle and NightScape Lab by 2026. What no competitor offers is proof: per-frame alignment residuals, rejection maps, and a comparison surface built on how human vision actually detects differences. Star Photos already computes that evidence. It has two further advantages no commercial entrant can match: it is open source and cross-platform, the combination pixls and APP users have requested for years. The aurora episode is best read as a tooling failure rather than a matter of taste, and it doubles as the first requirements test for the product's core screen. The in-app compare hub should be built and validated before the guided controls that feed it.

The data scarcity is also an opportunity. No nightscape stacking benchmark exists, and the open supply is two short sequences, so a CC0 deck solicited from pixls contributors and hosted with a DOI would be the first public test set in this niche. It would give Star Photos a reproducible quality bar and something to give back to the community whose open-source requests it answers. The main uncertainties are the zero-copy shared-device path in egui, which the spike resolves, and the untested usability of automatically chosen crops and boundary-confidence displays, which only sessions with real nightscape photographers can settle.
