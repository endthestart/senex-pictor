# Comparing renderings and guided-vs-advanced controls: UX patterns for Star Photos

Scope: (a) how photographers judge subtle differences between renderings of the same scene (stacked vs single frame, two looks, before/after a setting change), including why the three-way JPEG toggle on the aurora review page was hard to use; (b) guided defaults with progressive disclosure of advanced controls. Product behaviour is from 2022–2026 sources where available. Research is cited regardless of date.

## 1. Comparison modes, when each works, and what perception research says about subtle differences

### Takeaway
For spotting *whether and where* two renderings differ, an instant in-place toggle (flicker or blink with no blank frame, at the same zoom and position) is far better than any other mode. Without a gap between frames, observers found changes in about 1.4 alternations (0.9 s). With an 80 ms blank between frames, the same changes took 7–17 alternations, sometimes more than 80. Side-by-side viewing makes the eye jump back and forth and relies on working memory, so it is the weakest way to find subtle differences, though it suits judging a whole "look" and comparing three or more versions. Split or wipe is best for differences along a boundary, such as a horizon or tree line. Zooming and amplifying differences roughly double sensitivity, according to JPEG AIC research. Mature tools combine these modes: a survey grid to narrow down, a two-up compare with linked zoom, and a toggle key for before/after.

### Cited Findings

**Perception research: flicker, change blindness, side-by-side search**
- Rensink, O'Regan and Clark (1997) alternated an original and a modified image. Each image showed for 240 ms with an 80 ms grey blank between, and each image appeared twice before switching. With blanks, changes in "marginal interest" regions took an average of 17.1 alternations (10.9 s) to identify, and some images took "over 80 alternations (50 s) to identify a change that was obvious once noticed." Changes in "central interest" regions took 7.3 alternations (4.7 s) — [Rensink et al. 1997, Psychological Science (PDF)](https://www2.psych.ubc.ca/~rensink/publications/download/PsychSci97-RR.pdf)
- In the same study, removing the blanks produced "a completely different pattern": identification took only 1.4 alternations (0.9 s), with no difference between central-interest and marginal-interest changes. The authors conclude "the key factor is attention, without which observers are blind to change." — [Rensink et al. 1997 (PDF)](https://www2.psych.ubc.ca/~rensink/publications/download/PsychSci97-RR.pdf)
- Rensink's follow-up abstract says strong change blindness occurs "when brief blank fields are placed between alternating displays." It concludes that our representation of the world is sparse and coordinated by attention — [Rensink 1998 abstract, UBC](https://www2.psych.ubc.ca/~rensink/publications/abs.98.4.html)
- A 2018 review summarises the finding as: even large changes are hard to notice if they happen while motion transients appear elsewhere in the image. The 1999 "mudsplashes" study (O'Regan, Rensink and Clark, *Nature* 398:34) showed that small distracting splashes are enough to cause change blindness, without any full blank — [Perspectives on Psychological Science 2018](https://journals.sagepub.com/doi/10.1177/1745691617707269) (via search abstract); [summary of the mudsplashes work](https://atticusli.com/replication-crisis/change-blindness/) (secondary source)
- Comparative visual search, meaning finding the mismatch between two side-by-side, nearly identical images, has been studied with eye tracking (Pomplun, 1998 onward). People make many saccades between the two images instead of holding features in memory. When saccades become costlier, for example because the images are farther apart, people rely more on working memory — [Pomplun thesis 1998](https://www.cs.umb.edu/~marc/pubs/pomplun_thesis1998.pdf); [Atkins, Moise & Rohling 2007 (SFU TR)](https://www2.cs.sfu.ca/~stella/papers/2007/tr.pdf); [Pomplun et al., CogSci 2001](https://www.cs.umb.edu/~marc/pubs/pomplun_et_al_cogsci2001.pdf) (from search summaries)
- Atkins et al. describe side-by-side difference search as two phases: a scan for a likely candidate, then a confirmation phase of repeated back-and-forth saccades. Subjects "prefer to make multiple saccades rather than perform cognitive effort to remember features" from one image to the other — [Atkins et al. 2007](https://www2.cs.sfu.ca/~stella/papers/2007/tr.pdf) (from search summary)

**Image-quality standards: flicker and "boosting" for subtle differences**
- ISO/IEC 29170-2 (JPEG AIC-2) defines a *flicker test* that classifies a compressed image as visually lossless or lossy. The threshold is one just-noticeable difference (JND) under flicker. The test says only whether a difference is visible and gives no graded quality. One study used an 8 Hz flicker and added a "no decision" option — [Subjective assessment of visually lossless compression, PMC9918960](https://www.ncbi.nlm.nih.gov/pmc/articles/PMC9918960/); [JPEG AIC fine-grained assessment, arXiv 2410.09501](https://arxiv.org/html/2410.09501)
- JPEG AIC-3 research uses three "boosting" techniques to help observers see subtle differences:
  - **Zoom:** crop to half size in each dimension, then upscale with Lanczos.
  - **Artifact amplification:** scale the pixel differences from the reference by ×2 in each channel.
  - **Flicker:** alternate test and reference at 10 Hz, 100 ms each.
  — [arXiv 2410.09501](https://arxiv.org/html/2410.09501)
- In the *boosted triplet comparison*, two candidates sit side by side and each flickers against the reference. Observers choose the side with the stronger flicker (Left / Right / Not sure). The unboosted control has observers hold a toggle button to switch between decoded and source images, capped at 2 Hz, "which reduces eye movement compared with side-by-side viewing." Boosted scales came out "larger than the unboosted ones by a factor of about 2," and the 1-JND threshold moved from distortion level 4 to about level 2. The flicker-only AIC-2 threshold is expected to fall between the two. The paper reports no direct statistical test of boosted versus plain — [arXiv 2410.09501](https://arxiv.org/html/2410.09501)

**Astronomy: blink comparators**
- The blink comparator flips between two plates of the same sky taken at different times. Anything that moved blinks, while fixed stars do not — [Lowell Observatory collection: Tombaugh with the blink comparator](https://collectionslowellobservatory.omeka.net/items/show/1377); [Planetary Society, Pluto discovery pt. 1](https://planetary.org/articles/pluto-discovery-1)
- Tombaugh mounted plate pairs taken several days apart and worked across them through a microscope in small strips. By January 1930 he had examined about 1.5 million stars. He blinked for about half an hour, then stepped away for at least 15 minutes, because "I knew I dare not overdo it or my attention would lapse and I could miss something." — [Planetary Society, Pluto discovery pt. 4](https://www.planetary.org/articles/pluto-discovery-4)
- In modern astrophotography, PixInsight's Blink process is used to cull subframes: users blink through subs and uncheck the bad ones. Users disagree about whether the default STF mode applies one stretch to all frames or stretches each frame separately — [AstroBin forum: How to use Blink efficiently](https://app.astrobin.com/forum/topic/211600/how-to-use-blink-efficiently); [AstroBackyard: Using Blink in PixInsight](https://astrobackyard.com/blink-pixinsight)

**How current photo tools implement comparison modes**
- **Lightroom Classic, Survey and Compare:**
  - Survey view (N) tiles two or more photos.
  - Compare view (C) shows two side by side as *Select* and *Candidate*. *Make Select* promotes the candidate and *Swap* reverses the pair.
  - With *Link Focus* (lock icon) on, zoom and pan apply to both photos. *Sync* realigns them after they have been unlinked.
  - Zoom works in Compare but not in Survey, and both views live in the Library module.
  — [Adobe: Browse and compare photos](https://helpx.adobe.com/ca/lightroom-classic/desktop/viewing-photos/browse-compare-photos.html) (page returned 403; content from search summaries); [Julieanne Kost: Compare and Survey view](https://jkost.com/blog/?p=22615)
- **Lightroom Classic, Develop before/after:**
  - **Y** toggles side-by-side before/after.
  - **Alt/Option+Y** gives top/bottom.
  - **Shift+Y** gives split screen; sources disagree on its exact behaviour.
  - **\\** flips between before and after in a single loupe view.
  — [Julieanne Kost 2024: History panel and Before & After view](https://jkost.com/blog/2024/08/the-history-panel-and-before-after-view-in-lightroom-classic.html); [Lightroom Killer Tips](https://lightroomkillertips.com/lightroom-for-beginners-3-before-after-tips/); [Shotkit](https://shotkit.com/lightroom-before-and-after-tool/)
- **Capture One, Compare Variant:** pins one variant in the Viewer with an orange frame, and other selected variants show beside it. The Viewer must be in Multi View. Capture One Studio can show up to 11 compare variants. Users have asked for a split view *between two variants*, which suggests it is missing — [Capture One: Compare Variant overview](https://support.captureone.com/hc/en-us/articles/360002481017); [community: Set as Compare Variant](https://support.captureone.com/hc/fr/community/posts/360012377578-Set-as-Compare-Variant); [community: before/after between two variants](https://support.captureone.com/hc/fr/community/posts/360010717178-Before-After-function-compare-two-variants-instead-of-a-variant-with-the-unedited-raw-file)
- **Capture One, Before/After:** **Y** toggles it and **Shift+Y** switches between Full View, which shows the "before", and the Split View Slider. In split mode, adjustments update live on the "after" side. In Full View, touching a slider turns the feature off — [Capture One: Comparing images with Before/After](https://support.captureone.com/hc/en-us/articles/360008828758-Comparing-images-with-the-Before-After-feature) (page returned 403; content from search summary)
- **darktable, snapshots:**
  - The default display is an overlay split with a draggable line, and a rotate icon switches between vertical and horizontal splits.
  - A separate toggle gives side-by-side. An "S" arrow marks which side is the snapshot.
  - Ctrl-click a snapshot name to label it, and each snapshot can be *restored* into the current history.
  - Snapshots last for the session and can be reused on other images.
  - Combine snapshots with the history stack to compare against any earlier state.
  — [darktable manual: snapshots](https://docs.darktable.org/usermanual/development/en/module-reference/utility-modules/darkroom/snapshots/)
- **darktable, duplicate manager:** "Hold down the left mouse button on a thumbnail to temporarily show that version in the center view", and releasing returns to the current version. This is a hold-to-peek flicker between versions. Versions can carry text labels — [darktable 5.6: duplicate manager](https://docs.darktable.org/usermanual/5.6/en/module-reference/utility-modules/darkroom/duplicate-manager/)
- **Photoshop, Quick Compare** (beta, announced 3 Sep 2026): compares the current state with "how the file looked when you opened it this session."
  - *Single View* flips between the two states and *Split View* reveals them with a slider. There is no side-by-side yet.
  - Users asked for other reference states, and snapshots "do not persist beyond the editing session."
  - Split View misaligned on cropped images, which Adobe acknowledged as a bug.
  — [Adobe community announcement](https://community.adobe.com/announcements-698/quick-compare-is-now-in-photoshop-beta-1640291)
- **Affinity Photo, Develop persona:** normal, split and mirror (side-by-side) views. The views can be synced so that "before" reflects the current adjustments instead of the original raw — [Affinity help: Developing a raw image](https://affinity.help/photo/en-US.lproj/pages/Raw/raw.html) (from search summary)
- **FastRawViewer 2.0:** added 2- and 4-window modes with synchronized zoom and pan. Before that, users tiled separate instances that could not sync. A preference keeps the zoom ratio and position when moving to the next file — [diglloyd: FastRawViewer 2.0](https://diglloyd.com/blog/2021/20210812_1044-LibRaw-FastRawViewer.html); [FastRawViewer forum](https://www.fastrawviewer.com/node/534)
- **NVIDIA ICAT:** compares up to 4 images or videos. Split Screen gives synchronized side-by-side, and Single Screen overlays two with a reveal slider. It also aligns, zooms and pans. The tool was built for internal image-quality testing (e.g., DLSS) before its public release — [NVIDIA ICAT](https://nvidia.com/en-us/geforce/technologies/icat/); [TechSpot](https://www.techspot.com/community/topics/nvidia-releases-new-free-image-comparison-tool.272238/)
- **Butterfly Viewer** (open source): "sliding overlays", "synchronized pan and zoom", and auto-arranging side-by-side windows. It was built to inspect high-res and raking-light photos of paintings — [GitHub olive-groves/butterfly_viewer](https://github.com/olive-groves/butterfly_viewer)
- **DxO PhotoLab:** previews DeepPRIME noise reduction only inside a small loupe at 100–1600%, in two sizes, because full-image processing is too slow to preview. Forum users repeatedly ask for a larger or full-screen preview — [heise: PhotoLab 8](https://www.heise.de/en/news/DxO-PhotoLab-8-with-improved-noise-reduction-9859095.html?view=print); [ProVideo Coalition](https://www.provideocoalition.com/dxo-photolab-8-deepprime-xd2s-hue-masks-and-a-loupe/); [DxO forum: Noise reduction preview](https://forum.dxo.com/t/noise-reduction-preview/29882)
- **Topaz Photo AI:** a "comparison view" places several denoise models, or the same model with different settings, side by side. Reviewers call it the best way to choose. The sources don't confirm whether the panes share a crop and zoom — [Photofocus](https://photofocus.com/photography/topaz-photo-ai-v2-vs-lightroom-noise-reduction-showdown/); [Topaz community](https://community.topazlabs.com/t/topaz-photo-ai-v2-0-3/53127?page=3)

### Inferences

**Which mode for which job in Star Photos**
- **Find where versions differ:** use an instant in-place A/B toggle on a key (Space, or 1/2/3), plus darktable-style hold-to-peek. It must have *no* blank, fade, decode delay or layout change between frames. Rensink's data show that an 80 ms gap moves detection from about 1 alternation to 7–17+. Both renderings must be GPU-resident textures before the toggle is offered. Nothing else on screen should change on the swap (caption width, histogram redraw, UI reflow), because transients elsewhere can mask the change (the mudsplash result).
- **Judge a boundary:** use split or wipe for the horizon or tree line between the stacked sky and the foreground, where halos live. The slider should rotate between vertical and horizontal (darktable) so it can run across or along the tree line.
- **Judge the overall look, or 3+ versions:** use side-by-side or a survey grid to narrow down, then switch to a 2-up compare plus toggle (Lightroom Survey → Compare → Make Select). Always link zoom and pan by default (Link Focus / FastRawViewer / ICAT).
- **Sub-pixel qualities** (noise grain, star bloat, halo width): only visible at 100–200%. Use auto-chosen region crops (§2) and an optional amplified-difference view (JPEG AIC ×2 amplification).
- **Hybrid layout (JPEG AIC triplet):** show two candidate looks side by side, each toggling against a common reference such as the single frame or the current settings. The user asks "which one changed the sky more?" without memorising across saccades.

**Why the aurora review page was hard, and what would have fixed it**
- **Downscaled JPEGs:** these likely averaged away exactly the differences that distinguish the versions: per-pixel noise, star sharpness and thin halos. JPEG compression adds its own artifacts on top.
- **Three-way toggle:** this turns the task into three pairwise memory comparisons. The research favours pairwise toggling (2-AFC/triplet designs). The page should offer *pick two, then toggle*.
- **Captions describing each version:** these say nothing about *where to look*. Attention drives detection (Rensink), and changes in "marginal interest" regions such as the sky background or tree edges are found about 2× slower. Fixes:
  - Name the region with each caption ("look at the dark sky above the trees: B has less grain").
  - Show a fixed grid of 100% crops (rows = regions, columns = versions).
  - Make number keys swap instantly with preloaded full-resolution images and linked zoom.
- **"Each version has pros and cons":** this is a per-attribute trade-off. A per-region scorecard (noise / star sharpness / sky darkness / foreground brightness / halo) helps more than one global choice. It also suggests the product might let the user *combine* winners, e.g. the stacked sky with a single-frame foreground. That last point is a product decision for the owner.

**Fatigue:** Tombaugh's half-hour sessions suggest long blink sessions degrade attention. Short, targeted comparisons with crops and named regions beat open-ended scanning.

### Gaps
- No study was found that directly compares flicker, slider and side-by-side for *aesthetic preference* between photo edits, as opposed to change detection or compression-artifact visibility. The evidence for toggles is strong for *detecting* differences, but not for deciding which version is better.
- Adobe helpx pages returned 403, so Lightroom Compare/Survey and shortcut details come from search summaries and Adobe-affiliated blogs. Sources disagree on Shift+Y.
- Photo Mechanic's compare features were not researched.
- imgsli.com, a popular web comparison site, redirected to an unrelated domain and was not used.
- Topaz's comparison-view sync behaviour and current Capture One per-tool before/after were not verified.

## 2. Keeping comparisons fair and pointing users at regions of interest

### Takeaway
Fair comparison means the same display transform, zoom, crop and position for every version, with stable labels. Astronomy tools add explicit "equalize stretch" commands because per-image auto-stretch fakes or hides differences. Pointing users at the right region matters as much as the mode. Attention-driven change blindness means unprompted viewers miss differences in "uninteresting" areas like the sky background or tree edges. Tools already zoom into fixed regions (DxO loupe, JPEG AIC crop-and-zoom), but none found auto-select diagnostic crops. That would be a Star Photos differentiator.

### Cited Findings
- MaxIm DL's *Equalize Screen Stretch* "adjusts the screen stretch of all images so that their brightness and contrast are equal." It applies the selected image's min/max, adjusted for background and dynamic range, and is "very useful when the images are to be blinked." Results are less good when histogram shapes differ a lot — [MaxIm DL help](https://cdn.diffractionlimited.com/help/maximdl/HID_VIEW_STRETCH_ALL.htm)
- PixInsight Blink has a mode that computes an STF stretch on the current image and applies the same stretch to all. A user found that frames taken under different conditions needed re-stretching. This shows the tension between "same transform" (fair) and "per-frame auto" (readable) — [AstroBin forum](https://app.astrobin.com/forum/topic/211600/how-to-use-blink-efficiently)
- Lightroom Compare's *Link Focus* applies zoom to both photos, and *Sync* realigns them after unlinking — [Adobe help via search](https://helpx.adobe.com/ca/lightroom-classic/desktop/viewing-photos/browse-compare-photos.html). FastRawViewer can keep zoom and position when moving to the next file — [FastRawViewer forum](https://www.fastrawviewer.com/node/534)
- Fairness bugs exist in shipping tools: Photoshop Quick Compare's split view misaligns on cropped images — [Adobe community](https://community.adobe.com/announcements-698/quick-compare-is-now-in-photoshop-beta-1640291)
- Labelling which side is which:
  - darktable shows an "S" arrow on the snapshot side and lets users label snapshots.
  - Capture One marks the pinned compare variant with an orange frame.
  — [darktable snapshots](https://docs.darktable.org/usermanual/development/en/module-reference/utility-modules/darkroom/snapshots/); [Capture One Compare Variant](https://support.captureone.com/hc/en-us/articles/360002481017)
- Affinity lets the "before" track the current adjustments, so a comparison isolates the latest change instead of everything since the raw — [Affinity help](https://affinity.help/photo/en-US.lproj/pages/Raw/raw.html) (search summary). darktable gets the same effect by snapshotting any history state — [darktable snapshots](https://docs.darktable.org/usermanual/development/en/module-reference/utility-modules/darkroom/snapshots/)
- JPEG AIC "zoom" boosting crops to half size and upscales with Lanczos so observers inspect detail at magnification. Its flicker uses the same crop for test and reference — [arXiv 2410.09501](https://arxiv.org/html/2410.09501)
- DxO's 100–1600% loupe for noise reduction is a fixed-region magnified preview. Users find it too small to judge from — [ProVideo Coalition](https://www.provideocoalition.com/dxo-photolab-8-deepprime-xd2s-hue-masks-and-a-loupe/); [DxO forum](https://forum.dxo.com/t/noise-reduction-preview/29882)
- Changes in regions observers did not mention when describing the scene ("marginal interest") took 17.1 alternations to find under flicker, versus 7.3 for central interest — [Rensink et al. 1997](https://www2.psych.ubc.ca/~rensink/publications/download/PsychSci97-RR.pdf)

### Inferences
- **Fairness rules for Star Photos:**
  1. Every compared version goes through the identical display transform. Never auto-stretch per version for display.
  2. Linked zoom and pan are on by default.
  3. Labels have fixed size and position, with a persistent marker for which side is A.
  4. "Before" can be the previous settings, not only the single frame (Affinity/darktable).
  5. Crops are pixel-aligned across versions, which a stacker can guarantee because all renderings share the reference frame geometry.
- **Optional "match brightness" toggle:** use only when the user wants to judge noise or sharpness independently of overall brightness. Label it clearly, because sky darkness and foreground brightness are themselves attributes being judged. This follows the MaxIm DL caveat.
- **Auto-chosen regions of interest:** a stacker already knows the sky/foreground mask and star catalogue. Candidates:
  - Darkest smooth sky patch: noise and gradient.
  - Brightest unsaturated star and a median star: bloat, trailing, colour.
  - Strongest-gradient segment of the sky/ground boundary: halos and mask seams at tree edges.
  - Darkest foreground area: foreground noise and brightness.
  - The aurora or Milky Way core: structure and colour.
  
  Show them as a crop strip with one row per region and one column per version, at 100% or 200%. Clicking a crop centres the main view there. This takes the JPEG AIC zoom-boosting idea and the DxO loupe and fixes the loupe's "too small, wrong place" complaint by picking the places for the user.
- **Per-crop numeric readouts:** background median, noise σ, star FWHM. These turn "pros and cons" into a visible trade-off table. The metrics are standard in astro tools. Their exact UI treatment is a design choice, not found in sources.

### Gaps
- No product was found that automatically picks diagnostic crops (tree edges, dark sky, brightest star) for comparison. This appears novel, so there is no usability evidence for it.
- No research was found on the best crop count or magnification for this kind of comparison grid.

## 3. Showing uncertainty or confidence in automatic decisions without overwhelming users

### Takeaway
Validated human-AI guidelines (Amershi et al., CHI 2019, Microsoft) call for three things: say how well automation works, explain why it did what it did, and make correction cheap. Photo tools put these into practice with on-demand mask overlays in several display modes, toggled by one key, and with blink-and-checkbox frame culling. Show confidence on demand and locally, at the boundary or at the rejected frame, not as an always-on heat map.

### Cited Findings
- Amershi et al. (CHI 2019) distilled more than 160 recommendations into 18 guidelines. These were validated with 49 design practitioners against 20 AI products — [Microsoft Research blog](https://www.microsoft.com/en-us/research/blog/guidelines-for-human-ai-interaction/); [paper PDF](https://www.microsoft.com/en-us/research/wp-content/uploads/2019/01/Guidelines-for-Human-AI-Interaction-camera-ready.pdf). Guidelines relevant here, verbatim from the paper:
  - G1 "Make clear what the system can do."
  - G2 "Make clear how well the system can do what it can do. Help the user understand how often the AI system may make mistakes." The paper's example is hedging language ("we think you'll like").
  - G9 "Support efficient correction. Make it easy to edit, refine, or recover when the AI system is wrong."
  - G10 "Scope services when in doubt. Engage in disambiguation or gracefully degrade the AI system's services when uncertain about a user's goals." The example offers 3–4 suggestions instead of auto-completing.
  - G11 "Make clear why the system did what it did."
  - G14 "Update and adapt cautiously. Limit disruptive changes when updating and adapting the AI system's behaviors."
  - G17 "Provide global controls." Notably, the paper's G17 example is a photo organizer.
- Lightroom Classic shows AI masks (Select Subject/Sky) as an overlay, red by default — [Adobe: Oct 2021 feature summary](https://helpx.adobe.com/lightroom-classic/desktop/help/whats-new/2022.html). Overlay modes:
  - Color Overlay, Color Overlay on B&W, Image on B&W, Image on Black, Image on White, White on Black.
  - The overlay can show the affected *or* the unaffected area.
  - **O** shows or hides it and **Shift+O** cycles colours.
  — [Lightroom Killer Tips, Mar 2022](https://lightroomkillertips.com/exploring-masking-overlay-options/)
- In Lightroom, "Automatically Toggle Overlay" shows the overlay on hover and can be switched off. Users report confusion when it is invisible (opacity 0, or set to the unaffected area), so overlay state needs to be visible in the UI — [Adobe community](https://community.adobe.com/t5/lightroom-classic-discussions/having-a-show-overlay-issue/m-p/14099915)
- PixInsight Blink culling works by blinking subs and unchecking the bad ones. Users ask for "select all checked/unchecked", which shows that the include/exclude state of each frame is the central object — [AstroBin forum](https://app.astrobin.com/forum/topic/211600/how-to-use-blink-efficiently)

### Inferences
- **Sky/ground mask:** default to no overlay. One key toggles a Lightroom-style overlay with a mode menu ("Image on Black" for the sky, coloured tint for the foreground). Add a *boundary confidence* view that tints only uncertain pixels near the horizon or tree line. This keeps it local and follows G2 without a full-image heat map. Correction should be one brush away (G9).
- **Low-confidence segmentation:** instead of silently picking, offer 2–3 candidate masks as thumbnails (G10), e.g. "include distant hills in sky?"
- **Rejected frames:** show a filmstrip with include/exclude toggles and a one-line reason per rejection (G11), e.g. "trailed stars: FWHM 2× median" or "plane trail". A blink of a rejected frame against the reference shows the problem directly (blink comparator). Re-including a frame should be one click and should trigger an A/B of the result (G9).
- **Hedged language and summary counts** ("Used 18 of 20 frames; 2 looked trailed"): use these instead of raw scores in the guided view, with per-frame metrics behind disclosure (§4). Keep automatic decisions stable across re-runs unless inputs changed (G14).

### Gaps
- No photo tool was found that shows explicit per-pixel *confidence* for AI masks. Lightroom shows the mask but not its uncertainty, so there is no direct precedent or usability data.
- Academic uncertainty-visualization literature (e.g., on whether non-experts read confidence displays correctly) was not gathered within this budget.
- Sequator, Starry Landscape Stacker and others may have relevant mask UIs. Those belong to the sibling note on nightscape stackers, so they are not covered here.

## 4. Progressive disclosure patterns in photo apps

### Takeaway
Photo apps converge on three things. First, a one-click or one-slider guided layer: auto/ML Enhance, a master "Light" slider, or presets with live preview and an Amount slider. Second, one disclosure step to the full controls. Third, curated module sets for beginners versus full module lists. Live hover preview plus an intensity slider is the most consistent way to make "looks" choosable without technical knowledge.

### Cited Findings
- **Apple Photos (Mac):** each adjustment, e.g. Light, is a master slider with a disclosure arrow. Clicking the arrow next to *Options* reveals the detailed sliders: exposure, highlights, shadows, brightness, contrast, and so on. Option-drag reportedly extends a slider's range from ±1 to ±2 — [Apple Support: Adjust a photo's light](https://support.apple.com/guide/photos/pht806aea6a6); [Smalldog guide](https://special.smalldog.com/kibbles-and-bytes/2020/08/06/getting-the-picture-right-photo-editing-in-photos-app); [Kirkville](https://kirkville.com/tag/photo/page/30/)
- **Lightroom Classic, profiles and presets:**
  - The Profile Browser previews a profile in the image on hover. Holding Alt/Option temporarily disables the preview, and it can be turned off in preferences for speed.
  - Creative profiles get an Amount slider from 0 to 200, defaulting to 100.
  - Since June 2022 (v11.4), presets can carry an Amount slider too ("Support Amount Slider").
  — [Adobe: June 2022 feature summary](https://helpx.adobe.com/ca/lightroom-classic/desktop/help/whats-new/2022-4.html); [Adobe community](https://community.adobe.com/t5/lightroom-classic-discussions/lightroom-classic-profile-browser-amount-slider-is-not-working/td-p/13319076); [TourBox: Lightroom profiles](https://www.tourboxtech.com/en/news/lightroom-profiles.html)
- **darktable module groups:**
  - Preset layouts: *modules: all*, *workflow: scene-referred*, *workflow: display-referred*, *workflow: beginner*, *search only* and *modules: deprecated*.
  - *workflow: beginner* is "a minimal starting set" (base, grading, effects) that new users copy and extend as they gain experience.
  - The *quick access panel*, which collects key controls from any module, appears in every preset except *deprecated* and *search only*.
  — [darktable 5.6: module groups](https://docs.darktable.org/usermanual/5.6/en/darkroom/organization/module-groups/). The quick access panel replaced the basic adjustments module in 3.6 — [darktable 3.6 release](https://www.darktable.org/2021/07/darktable-3-6/)
- **Pixelmator Pro:** colour-adjustment preset thumbnails sit above an ML Enhance button that "balances colors, evens out exposure", with sliders and wheels below. A draggable divider collapses or expands the preset area — [Apple Support: Pixelmator Pro presets](https://support.apple.com/guide/pixelmator-pro/pix20726fa4e/mac); [MacTrast](https://www.mactrast.com/?p=144884)
- **Luminar Neo:** splits tools into Essentials (exposure, contrast and other basics), Creative, Portrait and Professional (colour correction, sharpening, noise reduction). Presets live in a separate library — [Skylum: Essential tools](https://support.skylum.com/editing-tools/essential-tools); [Photofocus](https://photofocus.com/photography/a-brief-look-at-the-workflow-in-luminar-neo/); [Photo Review](https://www.photoreview.com.au/reviews/digital-darkroom-reviews/software-digital-darkroom/skylum-luminar-neo/)
- **History and snapshots:**
  - darktable snapshots last for the session, carry labels and can be restored into history.
  - Photoshop snapshots "do not persist beyond the editing session."
  - Lightroom pairs the History panel with Before/After.
  — [darktable snapshots](https://docs.darktable.org/usermanual/development/en/module-reference/utility-modules/darkroom/snapshots/); [Adobe community](https://community.adobe.com/announcements-698/quick-compare-is-now-in-photoshop-beta-1640291); [Julieanne Kost 2024](https://jkost.com/blog/2024/08/the-history-panel-and-before-after-view-in-lightroom-classic.html)
- **Per-tool before/after:** Capture One tutorials describe seeing before/after for an individual adjustment, not only the whole edit — [Thomas Fitzgerald, 2020](https://blog.thomasfitzgeraldphotography.com/blog/2020/7/capture-one-quick-tip-how-to-see-before-and-after-on-individual-adjustments) (title only; details not verified)

### Inferences
- **Guided layer for Star Photos:** a "Looks" strip of live-rendered thumbnails of *this* stack, not generic samples, with hover-to-preview in the main view (Lightroom profile browser), plus one Amount/strength slider (0–200%, default 100). Hover preview must use the same instant-swap rendering path as the A/B toggle so preview equals result.
- **One disclosure step:** each guided control (e.g., "Sky darkness", "Foreground brightness", "Noise cleanup") expands to its underlying parameters, following the Apple Photos master-slider-plus-Options pattern. This stays within two disclosure levels (§5).
- **Advanced mode:** a darktable-style curated "beginner" set versus a full set, or a Luminar-style Essentials versus Professional split. A quick-access panel lets advanced users pin parameters into the guided view.
- **Per-control compare:** each guided control should support press-and-hold to view the image with that control at default. This applies darktable's hold-to-peek to a single setting and answers "what did this setting do?" with an instant toggle. Snapshots should persist with the project, not just the session, since Photoshop users explicitly ask for that.

### Gaps
- Pixelmator Pro hover-preview behaviour and Luminar Neo live preset previews were not confirmed.
- No usability data was found comparing the master-slider and preset-thumbnail approaches for novices.
- Per-control reset conventions (e.g., double-click to reset) were not sourced.

## 5. Guidance from UX literature (Nielsen Norman Group, Apple HIG)

### Takeaway
Nielsen Norman Group (NN/g) and Apple agree:
- Put frequent controls up front and hide advanced ones behind one clearly labelled disclosure.
- Never go beyond two levels.
- Use only one disclosure button per view.

Wizards (staged disclosure) suit infrequent setup tasks whose steps barely interact. Stacking and look decisions interact strongly, since changing alignment changes noise and halos, and users must compare across steps. So Star Photos should use a non-modal step overview that permits going back, not a locked wizard.

### Cited Findings
- **NN/g on progressive disclosure:**
  - It "defers advanced or rarely used features to a secondary screen", and showing something initially signals importance.
  - It improves learnability, efficiency and error rate.
  - "It must be obvious how users progress from the primary to the secondary disclosure levels."
  - Designs with more than two levels "usually perform poorly" because users get lost.
  - Use task analysis and frequency-of-use data, plus usability testing, to decide the split.
  — [NN/g: Progressive Disclosure](https://www.nngroup.com/articles/progressive-disclosure/)
- **NN/g on staged disclosure:** wizards work best when a task divides into "distinct steps that have little interaction"; dependent steps that force back-and-forth are a problem — [NN/g: Progressive Disclosure](https://www.nngroup.com/articles/progressive-disclosure/)
- **NN/g on wizards:**
  - Use them "for novice users or infrequent processes".
  - If users need steps in any order, the functionality "is not suited to be provided as a wizard."
  - Wizards "limit the users' control and creativity", carry higher interaction cost and make cross-step comparison hard.
  - Offer frequent users a faster alternative, let users save and resume, and use prior choices as defaults.
  — [NN/g: Wizards](https://www.nngroup.com/articles/wizards/)
- **Apple HIG, disclosure controls:**
  - "Place controls that people are most likely to use at the top of the disclosure hierarchy so they're always visible, with more advanced functionality hidden by default."
  - Label what is hidden, e.g. "Advanced Options".
  - "Use no more than one disclosure button in a single view."
  - Examples are Keynote's export options and the macOS Save sheet.
  — [Apple HIG: Disclosure controls](https://developer.apple.com/design/human-interface-guidelines/disclosure-controls) (content via search summary of the HIG page and a mirror)

### Inferences
- **Step structure:** treat the Star Photos pipeline (import → frame selection → alignment/registration → sky/ground → look → export) as a visible, clickable step overview with sensible automatic defaults at each step, not a modal wizard. Users must be able to jump back and compare results across steps. The comparison view from §1 should be the hub every step returns to.
- **Two disclosure levels:** guided controls, then the parameters behind each. Anything deeper should be grouped in a single, clearly labelled "Advanced" area per panel (Apple: one disclosure per view). Use frequency of use, e.g. telemetry-free usability sessions with nightscape photographers, to decide what is promoted.
- **Remember preferences:** remember the user's last look and comparison mode as defaults (NN/g wizards: reuse prior choices).

### Gaps
- The live Apple HIG page was not fetched directly. Wording comes from search summaries and a mirror, so check the exact phrasing before quoting.
- No NN/g or Apple guidance specific to image-comparison UIs was found. The comparison guidance in §1–2 rests on perception research and vendor practice.
