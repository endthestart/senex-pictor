# Openly licensed RAW nightscape sequences for a Star Photos test and example deck

Research date: 2026-10-27. Method: discuss.pixls.us Discourse search/JSON API (topic first posts and licence links read directly), Zenodo REST API, Dryad and figshare APIs, GitHub and Hugging Face search, plus web search and page fetches. Camera/exposure data for the Zenodo/Dryad CR2 set was read from the first 128 KB of each file (EXIF header only, no full download). "Unverified" marks anything not confirmed on the source page.

## discuss.pixls.us PlayRaw threads with night-sky RAWs (licence per post, camera, frame count)

### Takeaway
PlayRaw has 25+ freely licensed night-sky RAWs, but almost all are single frames. Only one thread shares a real stacking set under a confirmed open licence (#17204: 5 sky + 1 foreground frame, CC BY-SA 4.0). The best multi-frame set, the Siril nightscape tutorial (10 lights plus 50 calibration frames, X-Trans), has no licence stated for its RAWs. Several posters say they hold 10–250-frame sequences of the same scene, which makes them good people to ask.

### Cited Findings
**Category rule**
- Category rule: "raw files must be freely licensed. Most use the Creative Commons license." There is no default licence, so each post has to be checked. — [About the Play Raw category](https://discuss.pixls.us/t/about-the-play-raw-category/11956)

**Multi-frame sets (the main candidates)**
- **#17204 "Open Source Tool for Sky Stacking and Foreground Extraction" (2020).** An archive of 6 RAWs: IMG_0433–IMG_0437 "used for stacking to get a nice sky" and IMG_0431 for the foreground, plus 2 JPGs. The RAWs were shot by @flick87 and "posted with permission with license CC-BY-SA 4.0". They are hosted on mega.nz. Camera, size and whether the link still works were not verified; the IMG_ prefix suggests Canon, which is an inference. — [discuss.pixls.us/t/17204](https://discuss.pixls.us/t/open-source-tool-for-sky-stacking-and-foreground-extraction/17204)
- **Siril nightscape tutorial dataset (Sebastien Guyader, 2019).**
  - Contents: Bias (20 files), Darks (15), Flats (15) and Lights (10), plus Siril .ssf scripts. — [pixls tutorial thread](https://discuss.pixls.us/t/processing-a-nightscape-in-siril-a-tutorial/10996)
  - Size and download: the pixls.us zip (~1 GB) is reported broken. The author re-shared a Google Drive folder in 2021 and again in 2024. — [pixls article thread #12167](https://discuss.pixls.us/t/processing-a-nightscape-in-siril/12167); [Google Drive folder](https://drive.google.com/drive/folders/1sBvwkm948kJNjinGirDVNc0QLAiA4fTP)
  - The folder currently lists the Bias/Darks/Flats/Lights subfolders and both .ssf scripts (seen in the folder page HTML).
  - Gear and scene: Fujifilm X-T2 (X-Trans RAF) with a Samyang 12mm f/2.0 at f/2.0, ISO 800, 10″ exposures. The scene is the Milky Way over a large concrete cross at Pointe des Châteaux, Guadeloupe. The full original stack used 200 bias, 25 darks and 25 flats. — ["The Creations" thread](https://discuss.pixls.us/t/first-outing-of-the-new-year-the-creations/10658)
  - Licence: none stated for the RAW files. The pixls.us footer says content is CC BY-SA 4.0 "unless otherwise noted", but the photo credit is marked CC BY-NC 4.0. **Licence UNVERIFIED; written permission from the author is needed.** — [pixls.us article](https://pixls.us/articles/processing-a-nightscape-in-siril/)
- **#21840 "With a little luck…".** Five RW2 brackets (Panasonic LX100 II, handheld against a railing) of a gas station in fog, CC BY-NC-SA. This is neither night sky nor a free licence, so exclude it. — [discuss.pixls.us/t/21840](https://discuss.pixls.us/t/with-a-little-luck/21840)

**Single-frame night-sky PlayRaws under CC0, CC BY or CC BY-SA.** Useful for decoder and single-frame coverage and as "stack of 1" baselines.
- #10103 "Nightscape from Orange Free State": Nikon D610 at ISO 6400, second-class dark sky. The NEF is on Google Drive, "CC BY-SA 4.0". — [link](https://discuss.pixls.us/t/play-raw-nightscape-from-orange-free-state/10103)
- #11433 "Milky Way over Geroldsee" (near the Alps): Fujifilm RAF, 24.2 MB, CC BY-SA 4.0. The poster stacked 5 frames in Sequator, but only 1 RAF was posted. — [link](https://discuss.pixls.us/t/playraw-milky-way-over-geroldsee/11433)
- #12873 "My first try at night sky": Nikon D3300 at 18mm, f/5.6, 25 s, ISO 3200. The NEF is on OneDrive, CC BY 4.0. — [link](https://discuss.pixls.us/t/my-first-try-at-night-sky/12873)
- #13468 "The screw and the night sky": Pentax PEF, 19.8 MB, rock formation at El Torcal (Spain), CC BY-SA 4.0. — [link](https://discuss.pixls.us/t/the-screw-and-the-night-sky/13468)
- #14164 "PlayRaw CR3: Deneb w/ Milky Way": Canon CR3, CC BY 4.0. It was hosted "temporarily at filebin", so the link is probably dead (unverified). — [link](https://discuss.pixls.us/t/playraw-cr3-deneb-w-milky-way/14164)
- #16715 "Urban Nightscape | Cape Town": NEF, 36.6 MB, shot from an 18th floor, CC BY-SA. This is a city-lights scene; whether stars are visible is unverified. — [link](https://discuss.pixls.us/t/urban-nightscape-cape-town/16715)
- #19122 "Neowise (C/2020 F3)": Fujifilm RAF, 52.8 MB, CC BY-SA. — [link](https://discuss.pixls.us/t/neowise-c-2020-f3/19122)
- #19226 "Neowise over Holy Cross mountain": Canon CR2, 26.6 MB, with clouds and light pollution, CC BY-SA. — [link](https://discuss.pixls.us/t/neowise-over-holly-cross-mountain/19226)
- #19262 "Neowise and light pollution": Olympus ORF, 16.3 MB, shot from a parking lot, CC BY-SA. — [link](https://discuss.pixls.us/t/play-raw-neowise-and-light-pollution/19262)
- #19398 "Milky Way / … denoise challenge" (Humboldtsee): Sony ARW, 23.6 MB, CC BY-SA. — [link](https://discuss.pixls.us/t/milky-way-astronomical-processing-incl-denoise-challenge/19398)
- #19945 "Milky Way, again…": Olympus ORF, 13.5 MB, CC BY-SA. The poster says "I shot some 10 pictures of that scene" (not shared). — [link](https://discuss.pixls.us/t/milky-way-again/19945)
- #29584 "Mount Timpanogos at night": Fujifilm RAF, 31.9 MB, CC BY-SA. The poster took a sequence of 60 frames ("I'd upload all 60 images, but I've got 32MB per raw") and posted only 1. — [link](https://discuss.pixls.us/t/mount-timpanogos-at-night-with-friendly-cloud/29584)
- #31905 "Milky Way Post-Processing": NEF, 35.5 MB, CC BY-SA. — [link](https://discuss.pixls.us/t/milky-way-post-processing/31905)
- #36940 "Good night……": Canon CR2, 22.3 MB, CC BY-SA. The scene content is unverified. — [link](https://discuss.pixls.us/t/good-night/36940)
- #45561 "Night Sky with stars": Fujifilm RAF, 19.8 MB, with red and green tower lights, CC BY-SA. — [link](https://discuss.pixls.us/t/night-sky-with-stars/45561)
- #47386 "How about some antennas and the aurora?": Canon CR3, 6.9 MB, with aurora and spotlights, CC BY-SA. — [link](https://discuss.pixls.us/t/how-about-some-antennas-and-the-aurora/47386)
- #52180 "Another Milky Way": Fujifilm RAF, 22.8 MB, with an artificial-light "step" in the sky, CC BY-SA. — [link](https://discuss.pixls.us/t/another-milky-way/52180)
- #53617 Comet C/2025 A6 (Lemmon): NEF, 26.1 MB, 6 s untracked, CC BY-SA. — [link](https://discuss.pixls.us/t/astrophotography-support-for-processing-raw/53617)
- #54119 "Aurora Borealis in northern Illinois": Panasonic RW2, 33.1 MB, 4 s exposure with the camera braced on a car door, CC BY-SA. — [link](https://discuss.pixls.us/t/aurora-borealis-in-northern-illinois-usa/54119)
- #54732 "Back again with more antennas and aurora": Canon CR3, 22.9 MB, with aurora and a floodlit foreground, CC BY-SA. — [link](https://discuss.pixls.us/t/back-again-with-more-antennas-and-aurora/54732)
- #60855 "-First Post- Nightsky": Sony a6300 with Samyang 12mm f/2, ARW, 23.7 MB, CC BY-SA. — [link](https://discuss.pixls.us/t/first-post-nightsky/60855)
- #44989 "Milky Way Processing Feedback": Hasselblad .3FR, released "under the creative commons CC0 1.0 Universal license". It was too large for the forum, so it is now hosted as a GitHub release (B0001131.3FR, 211 MB, "CC0 v1.0"). — [thread](https://discuss.pixls.us/t/milky-way-processing-feedback/44989); [GitHub Benitoite/raw-test release 3FR](https://github.com/Benitoite/raw-test/releases/tag/3FR)
- #58659 "another Milky Way shot": Hasselblad 3FR (zip, 135.9 MB) with an "XCD … 21mm F/4" lens, "CC0 public domain". The same poster as #44989 consistently uses CC0. — [link](https://discuss.pixls.us/t/another-milky-way-shot/58659)
- #4983 "[PlayRaw] Milky Way" (Black Forest, 2017): Olympus ORF, 14.1 MB, ISO 1600. **Ambiguous licence:** the text says "Attribution-NonCommercial-ShareAlike 4.0" but the badge links to CC BY-SA 4.0. — [link](https://discuss.pixls.us/t/playraw-milky-way/4983)

**Night-sky PlayRaws to exclude (NonCommercial or no licence)**
- #31505 Milky Way over mountains, NEF, CC BY-NC 4.0. — [link](https://discuss.pixls.us/t/milky-way-over-my-mountains/31505)
- #38152 Jackson Lake tracked Milky Way: a 4-minute tracked sky frame plus a separate foreground frame (2 × RW2) and a mask TIFF, from an H-alpha-modified camera, CC BY-NC 4.0. This is exactly the "tracked sky + foreground blend" case, but its licence is NC. — [link](https://discuss.pixls.us/t/jackson-lake-state-park-tracked-milky-way/38152)
- #39057 Coolum, CC BY-NC-SA. — [link](https://discuss.pixls.us/t/milky-way-over-coolum-qld-australia/39057)
- #43340 Sodus Bay, CC BY-NC-SA. — [link](https://discuss.pixls.us/t/resources-for-editing-astrophotography/43340)
- #51947, 2 ARW zips, CC BY-NC. — [link](https://discuss.pixls.us/t/a-hot-night-in-the-valley-better-among-the-peaks/51947)
- #18888, Siril-stacked TIFFs only, CC BY-NC-SA. — [link](https://discuss.pixls.us/t/milky-way-gimp/18888)
- #33769 Salzburger Land RAF has no licence. — [link](https://discuss.pixls.us/t/milky-way-under-attack-from-salzburger-land/33769)
- #59936: 3 CR3 frames, no licence. — [link](https://discuss.pixls.us/t/raw-editing-for-milky-way/59936)

**pixls users who say they hold multi-frame sequences but have not released them (people to ask)**
- jchnkl: 16 median-stacked frames (ISO 640, 5″, 55mm FF) plus a 160″ ISO 100 foreground of the Walberla chapel, Franconia, with noticeable light pollution. He offered "I can also share the RAWs if there's interest (… ~90MB each)" and also shot a sunset-to-Milky-Way timelapse. — [discuss.pixls.us/t/12969](https://discuss.pixls.us/t/the-milky-way-over-rural-franconia-feel-free-to-critique/12969)
- sguyader: 15-frame portrait panorama with a Samyang 12mm f/2 at ISO 3200 and 25 s; a separate set over the sea built from stacks of 10 lights and 20 darks. — [#11802](https://discuss.pixls.us/t/my-first-shot-of-our-galaxys-center/11802); [#13888](https://discuss.pixls.us/t/yet-again-the-milky-way/13888)
- Waveluke: a 100+ image Milky Way stack/panorama plus 17 darks and a flatfield, Sony A7 II, 50mm f/2. — [#20974](https://discuss.pixls.us/t/noise-floor-and-flatfield-correction/20974)
- nathanm: 250 × 5 s exposures, Canon R5 with Sigma 40mm f/1.4 at f/1.6. The thread documents star elongation in the corners when stacking in Siril compared with APP. — [#20991](https://discuss.pixls.us/t/siril-needs-distortion-correction-in-stacking/20991)
- fujiokae: 90 × 8 s, Nikon D7200 at 42mm on a tripod, f/4.5, ISO 3200 (Cygnus). This is an untracked case where Siril showed corner distortion compared with Sequator. — [#35487](https://discuss.pixls.us/t/stacking-non-tracked-images-got-severe-distortion/35487)
- Arnaud_Mariat: 120 all-sky (fulldome) frames. — [#40679](https://discuss.pixls.us/t/allsky-360-fulldome-images-stacking/40679)
- plaven: 172-frame Milky Way timelapse, Canon 5D Mk III, 25 s, f/2.8, 16mm, dark sky in Australia. — [#2992](https://discuss.pixls.us/t/foss-time-lapse-of-the-milky-way/2992)
- arctic: 30 stacked 6 s frames of Orion, Canon 1000D, under high light pollution. — [#858](https://discuss.pixls.us/t/astrophotograpy-first-attempt/858)
- Daniel_Catalina (Geroldsee, 5 frames), wiegemalt (~10 frames), erratasama (60 frames): see above.

### Inferences
- The only open sequence confirmed on pixls (#17204, 5+1 frames, CC BY-SA) is enough for a smoke test. It is too short to show differences between stacking methods clearly. The Siril dataset (10 lights, X-Trans, with calibration frames) is the strongest pixls set, but it can be used only if Guyader explicitly grants CC BY-SA or CC0 for the RAWs.
- Single-frame PlayRaws give good decoder and sensor diversity: Canon CR2 and CR3, Nikon NEF, Sony ARW, Fujifilm RAF (X-Trans), Olympus ORF, Panasonic RW2, Pentax PEF and Hasselblad 3FR. They also cover harder scenes (aurora with floodlights, light pollution, comets, tower lights). They do not exercise alignment.
- The pixls community norm is per-post CC licensing, and posters are used to being asked, so asking named contributors for their full sequences under CC0 or CC BY-SA is a realistic route.

### Gaps
- Not verified: whether the mega.nz (#17204), Google Drive (#10103, Siril dataset), OneDrive (#12873) and filebin (#14164) links still work, and the archive sizes for #17204 and #10103.
- The camera model for #17204 was not confirmed (only the IMG_ file naming was seen).
- Only the first page (≤20 posts) of long threads was scanned. Later replies may contain extra frames or licence changes.
- Scene content of #36940 and #16715 was not visually checked.

## raw.pixls.us: night or long exposures?

### Takeaway
raw.pixls.us is a CC0 decoder-coverage archive. It deliberately asks for daylight, low-ISO, single-setting samples and does not want series, so it is not a source of night sequences. Use it only for format and decoder coverage.

### Cited Findings
- Uploads are released under CC0 (the upload form links to the CC0 dedication). There is a filter to "view only the non-CC0 samples", so a few samples carry other licences. — [raw.pixls.us](https://raw.pixls.us/)
- Wanted samples are "well-lit, detailed scenery at low ISO" (for example daylight landscape). Unwanted are series varying ISO, aperture, shutter, white balance, lighting or lens, plus Adobe-DNG-Converter DNGs. — [raw.pixls.us](https://raw.pixls.us/)
- The table columns are Make, Model, Mode, Pixls, Remark, License, Date, Raw and Exif. There is no exposure or scene filter. — [raw.pixls.us](https://raw.pixls.us/)

### Inferences
- raw.pixls.us is the right source for a "can we decode every compression mode" fixture set (CR3/cRAW, Sony lossless, Nikon HE, Fuji compressed RAF), which is separate from the night deck.
- raw.pixls.us does not want night sequences, so Star Photos-specific sequences need their own home (for example a GitHub release or Zenodo record).

### Gaps
- No machine-readable index with exposure data was found, so I could not check whether any raw.pixls.us sample happens to be a night or long exposure.

## Stacker sample data, Wikimedia Commons, and Zenodo/figshare/Kaggle/academic datasets

### Takeaway
None of the stacker vendors (Sequator, Starry Landscape Stacker, DeepSkyStacker, Astro Pixel Processor, Siril) publishes an openly licensed nightscape sample set. Their practice data is deep-sky, unlicensed or broken. Wikimedia Commons and Flickr cannot host RAW at all. The one academic find is a **CC0 Dryad/Zenodo set: two 5-frame bursts of Canon EOS 6D CR2 night-sky frames (8mm fisheye, 30 s, ISO 6400)**. Everything else on Zenodo is FITS from telescopes or all-sky cameras, JPEG, or NC/ND-licensed.

### Cited Findings
**Stacker vendors**
- Starry Landscape Stacker: the developer documentation says only light frames are required and recommends about 10 darks. No downloadable sample pack was found. — [SLS site](https://www.sites.google.com/site/starrylandscapestacker); [Using SLS](https://www.sites.google.com/site/starrylandscapestacker/using-starry-landscape-stacker-version-1-9)
- Sequator: no official sample data was found. It appears on pixls only as the stacker people used, for example the Geroldsee 5-frame stack. — [#11433](https://discuss.pixls.us/t/playraw-milky-way-over-geroldsee/11433)
- DeepSkyStacker: no official sample set was found. AstroBackyard's "DSS practice files" are a deep-sky North America Nebula set (Canon EOS Ra, Rokinon 135mm, Google Drive) with no format, frame count or licence stated and no redistribution terms. — [AstroBackyard DSS practice files](https://astrobackyard.com/dss-practice-files/)
- Astro Pixel Processor: no official sample-data page was found. The forum points to community deep-sky sets (for example M20 data by Diego Colonnello) and offers made on request. — [APP forum: sample data files](https://www.astropixelprocessor.com/community/tutorials-workflows/sample-data-files-to-work-through-some-tutorials/)
- Siril official tutorials: the "full image processing (scripts)" tutorial's practice zip (lights/darks/biases/flats hosted on astrosurf.com/colmic) was reported unavailable in 2024. It is deep-sky data. The only Siril nightscape dataset is the third-party pixls tutorial described above. — [pixls #42164](https://discuss.pixls.us/t/zip-archive-of-practice-data-not-available/42164); [siril.org tuto-scripts](https://siril.org/tutorials/tuto-scripts/)
- Academia Sinica (W.-H. Wang) releases deep-sky RAWs more than a year old for practice, "with limitations on using processed images". That is not an open licence, and the data is deep-sky. — [ASIAA astrophoto page](https://group.asiaa.sinica.edu.tw/whwang/Astrophoto/index.html)

**Wikimedia Commons and Flickr (cannot host RAW)**
- Wikimedia Commons accepts SVG/PNG/JPEG/GIF/TIFF/WebP/XCF etc. DNG appears only under "Requested at least once, but not currently supported". CR2/NEF/ARW are not accepted, and the guidance is to convert RAW to JPEG before uploading. **Commons therefore holds no RAW night series.** — [Commons:File types](https://commons.wikimedia.org/wiki/Commons:File_types)
- Flickr: "RAW photos are not supported at this time"; files must be converted before upload. Flickr CC night series are therefore JPEG at best. — [Flickr upload requirements](https://www.flickrhelp.com/hc/en-us/articles/4404079649300-Flickr-upload-requirements)

**Zenodo / Dryad (the open-licence finds)**
- **Zenodo 7411889 / Dryad doi:10.5061/dryad.v6wwpzh0m, "X, Y, and Z: A bird's eye view on light pollution"** (Kolláth, Degen, Degen).
  - Licence: **CC0**.
  - Files: 10 CR2 files, each 22.6–24.0 MB (about 231 MB total), plus night-sky spectra. The README calls them "Two groups of digital camera raw images in Canon CR2 raw format". — [Zenodo record](https://zenodo.org/records/7411889)
  - EXIF read from the file headers: Canon EOS 6D, "8mm" lens (fisheye), f/3.5, 30 s, ISO 6400, 5472×3648.
  - Group 1: IMG_2975–2979, 2021-09-08 23:56:40 to 2021-09-09 00:00:03, frames about 45–56 s apart.
  - Group 2: IMG_3001–3005, 2021-09-09 22:32:37 to 22:35:56, with a 22-minute gap before IMG_3005 (22:57:45).
  - Not verified: scene content (whether there is a horizon or foreground, which direction the camera pointed, the site) and whether the camera was tripod-mounted.
- Zenodo 3737999, Copenhagen all-sky camera: CC BY 4.0. Each file is a dark-subtracted, averaged 16-bit FITS split into R/G/B from a ZWO camera (16 s subs), 4 nights, about 200 MB. It is processed FITS, not camera RAW. — [Zenodo 3737999](https://zenodo.org/records/3737999)
- Zenodo 14222498, Svalbard (KHO/UNIS) Sony A7S all-sky camera, 2016-01-07 05:45–07:30 UT: CC BY 4.0, **JPEG only**, 98 MB. This is a secondary aurora sequence. — [Zenodo 14222498](https://zenodo.org/records/14222498)
- Zenodo 13340185, "Meteor Camera Images – Sony IMX29": a stationary meteor camera, CC BY 4.0, 1.33 GB zip ("please make sure I get credited"). The file format was not verified; it is probably not DSLR RAW. — [Zenodo 13340185](https://zenodo.org/records/13340185)
- Zenodo 18657514, CASCAR all-sky: **CC BY-ND 4.0**, 800×600 JPEG, captured from sunrise to sunset (daytime only). Exclude. — [Zenodo 18657514](https://zenodo.org/records/18657514)
- Zenodo 11125041, GONet v2 all-sky camera calibration: CC BY 4.0, 4.48 GB zip. Format and night content were not verified. — [Zenodo 11125041](https://zenodo.org/records/11125041)
- MILAN Sky Survey: raw deep-sky frames from a Stellina telescope (3096×2080). The Zenodo licence is "other-nc" (Attribution-NonCommercial-NoDerivatives per search). It has no foreground and an NC-ND licence, so exclude it. — [Zenodo 6874896](https://zenodo.org/records/6874896); [Data in Brief article](https://www.sciencedirect.com/science/article/pii/S2352340923002524)
- Moana Project: CC BY 4.0 ("attribution: 'Moana Project'"). It is raw FITS from a 254 mm Newtonian with an ASI1600MM covering deep-sky, comet and eclipse targets. It has a narrow field and no landscape, so it does not fit a nightscape deck. — [erellaz.com/moana/open-datasets](https://erellaz.com/moana/open-datasets/)
- IAU OAE Astrophotography Contest entries on Zenodo (for example 10359762 "Star Trails"): CC BY 4.0, but single finished JPEGs. Secondary or example imagery only. — [Zenodo 10359762](https://zenodo.org/records/10359762)

**Other repositories and raw-burst datasets**
- figshare API searches ("milky way raw", "night sky raw images", "nightscape", "star trails images", "all-sky camera night images", "astrophotography DSLR raw") returned no relevant image datasets. — [figshare search API](https://api.figshare.com/v2/articles/search)
- GitHub repo search found only nightscape-stacker tools (no data). Hugging Face dataset search for "nightscape"/"milky way"/"astrophotography"/"night sky" found no image datasets. — [GitHub](https://github.com/search?q=nightscape+stacking&type=repositories); [Hugging Face datasets](https://huggingface.co/datasets)
- HDR+ Burst Photography Dataset: "released under a Creative Commons license (CC-BY-SA)", with "main intention … scientific purposes".
  - Content: DNG bursts from Nexus 5/6/5X/6P and Pixel/Pixel XL phones; 765 GiB in total, or a 37 GiB curated subset of 153 bursts.
  - Night-sky or tripod content is not described.
  - It is useful for DNG and burst-alignment code, not for nightscapes. — [hdrplusdata.org/dataset.html](https://hdrplusdata.org/dataset.html)
- Low-light RAW research sets (RAW-NOD: Sony RX100 VII and Nikon D750, low-light object detection; SID "Learning to See in the Dark") contain night street scenes, not sky. Their licences were not verified. — [RAW-NOD](https://github.com/igor-morawski/RAW-NOD); [SID](https://cchen156.github.io/SID.html)

### Inferences
- The Dryad/Zenodo CC0 set is the only find that is redistributable without conditions: genuine RAW, a fixed camera, and consecutive frames at night. It has only 5 frames per burst, and an 8mm fisheye at ISO 6400 is an unusual (hard) geometry. It makes a good decoder plus wide-field-distortion edge case, but not the clean showcase sequence.
- For a fisheye or very wide set, the pixls threads on Siril corner-star elongation (#20991, #35487) suggest this is exactly where registration methods differ.

### Gaps
- Kaggle was not searched (its API needs authentication). Based on the Zenodo and figshare results, Kaggle night-sky sets are likely to be JPEG, but this is unverified.
- archive.org was not searched directly through its API. Web search found no CC RAW Milky Way collections there.
- I could not confirm the Dryad set's scene (horizon or foreground) without viewing the images.

## Creative Commons timelapse RAW sequences (Flickr, archive.org, Vimeo)

### Takeaway
No CC-licensed RAW timelapse sequence of a night sky was found on Flickr, archive.org or Vimeo. Flickr cannot store RAW, and CC-licensed Vimeo or other timelapses are distributed as rendered video only.

### Cited Findings
- Flickr does not support RAW uploads. — [Flickr upload requirements](https://www.flickrhelp.com/hc/en-us/articles/4404079649300-Flickr-upload-requirements)
- A Vimeo 4-hour Milky Way timelapse is CC BY 4.0, but only the video is offered; no RAW frames are mentioned. — [vimeo.com/860635340](https://vimeo.com/860635340)
- CG Channel's free HD sky timelapses (Marc Lorenz) are under "a creative commons licence for personal or commercial use", but they are rendered video, not Milky Way RAWs. — [CG Channel](https://www.cgchannel.com/2012/04/download-three-free-hd-timelapse-skies)
- A Fort Lewis College Observatory Milky Way timelapse frame is CC BY-NC-SA 4.0, and it is a rendered frame. — [Fort Lewis](https://fortlewis.edu/observatory/image_detail.asp?ID=204)
- The pixls FOSS Milky Way timelapse (172 frames, Canon 5D Mk III) was published only as a YouTube video. — [#2992](https://discuss.pixls.us/t/foss-time-lapse-of-the-milky-way/2992)
- "Free practice RAW" offers from photographers and courses state no licence or come bundled with paid courses (for example Elia Saikaly's 3 Everest RAWs and Skylum/Teachable packs). They cannot be assumed redistributable. — [eliasaikaly.com](https://eliasaikaly.com/?p=18431); [Skylum marketplace](https://marketplace.skylum.com/skies/milky-way-galaxy-skies)

### Inferences
- A CC licence on a rendered timelapse does not cover the unpublished RAW frames. The owner would have to release the RAWs separately.

### Gaps
- Vimeo pages that might offer original-file downloads were not checked one by one. No search hit mentioned RAW downloads.

## Per-candidate licence and redistribution summary

### Takeaway
Only two sequence candidates are usable today without asking anyone: Dryad CC0 (2×5 CR2) and pixls #17204 (CC BY-SA, 5+1 frames). The Siril dataset needs permission. The CC BY-SA single frames and the CC0 Hasselblad frames can join the deck as decoder and single-frame cases.

### Cited Findings
| Candidate | Licence (verified?) | Redistribute in public repo? | Camera / lens | Exposure | Frames | Size | Scene |
|---|---|---|---|---|---|---|---|
| Dryad/Zenodo 7411889 ([link](https://zenodo.org/records/7411889)) | CC0 (verified) | Yes, no conditions | Canon EOS 6D, 8mm fisheye | 30 s, f/3.5, ISO 6400 | 2 bursts × 5 | ~231 MB | Night sky, light-pollution study; foreground unverified |
| pixls #17204 ([link](https://discuss.pixls.us/t/open-source-tool-for-sky-stacking-and-foreground-extraction/17204)) | CC BY-SA 4.0 (verified in post) | Yes, with attribution to flick87 and BY-SA for derivatives | Canon? (unverified) | unverified | 5 sky + 1 foreground | unverified | Milky Way nightscape with foreground |
| Siril nightscape tutorial ([thread](https://discuss.pixls.us/t/processing-a-nightscape-in-siril/12167)) | **Not stated** (site CC BY-SA "unless otherwise noted"; photo CC BY-NC) | **Ask first** | Fujifilm X-T2, Samyang 12mm f/2 | 10 s, f/2, ISO 800 | 10 lights + 20 bias + 15 darks + 15 flats | ~1 GB | Milky Way over a cross, Guadeloupe coast |
| raw-test 3FR / #58659 ([release](https://github.com/Benitoite/raw-test/releases/tag/3FR)) | CC0 (verified) | Yes | Hasselblad 3FR, 21mm f/4 (#58659) | unverified | 1 each | 211 MB / 136 MB zip | Milky Way, mountain |
| PlayRaw single frames listed above | CC BY / CC BY-SA (verified per post) | Yes, with attribution (+SA) | Mixed (NEF, CR2, CR3, ARW, RAF, ORF, RW2, PEF) | varied | 1 each | 7–53 MB each | MW, aurora, comet, light pollution |
| Copenhagen all-sky ([link](https://zenodo.org/records/3737999)) | CC BY 4.0 | Yes | ZWO | 16 s | averaged FITS | ~200 MB | All-sky, urban; not RAW |
| Svalbard A7S all-sky ([link](https://zenodo.org/records/14222498)) | CC BY 4.0 | Yes | Sony A7S | unverified | JPEG sequence | 98 MB | Aurora (secondary, JPEG) |
| MILAN, CASCAR, pixls NC posts | NC and/or ND | No (exclude) | — | — | — | — | — |

- Licence texts used for the ShareAlike and NC/ND judgements: [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/), [CC BY-NC 4.0](https://creativecommons.org/licenses/by-nc/4.0/), [CC0](https://creativecommons.org/publicdomain/zero/1.0/)

### Inferences
- CC BY-SA RAWs can go in a public test repo. Derived fixtures made from them (crops, downscaled previews, reference stacks published as example outputs) would also need to be BY-SA with attribution. Keep a per-file `LICENSE`/`ATTRIBUTION` manifest separate from the code licence.
- NC files are a poor fit for an open-source project whose downstream redistribution may be commercial. ND files block crops and reference outputs. Exclude both.
- CC0 is the cleanest choice for anything the project itself solicits.

### Gaps
- Exposure, camera and size for #17204, and the scene of the Dryad set, still need checking (open the archives or view the images).

## Best practical options if few open sequences exist, and what a good test deck looks like

### Takeaway
The open supply is effectively 2 short sequences. The realistic route to "a true night sky deck" is to solicit CC0 (or CC BY-SA) sequences from the pixls community, starting with named members who already hold 10–250-frame series. Host them as GitHub release assets or a Zenodo record, and fill gaps against an explicit scene checklist.

### Cited Findings
- pixls members share large RAWs via GitHub release assets. One member noted "<2GB size and 1000 assets per release" limits and free bandwidth, and the CC0 3FR is already hosted this way. — [pixls #54544](https://discuss.pixls.us/t/where-to-host-large-file-size-raws-for-a-playraw/54544); [raw-test release](https://github.com/Benitoite/raw-test/releases/tag/3FR)
- A 132 MB RAW was too large to attach directly on pixls. — [pixls #54544](https://discuss.pixls.us/t/where-to-host-large-file-size-raws-for-a-playraw/54544)
- Zenodo records carry a DOI and an explicit licence field (for example CC0 for 7411889), which suits citable test data. — [Zenodo 7411889](https://zenodo.org/records/7411889)
- Typical stacking guidance on pixls is 10–20 lights with 20 darks/flats. Guyader measured noise against frame count and found that 10–20 lights is a good number. — ["The Creations" thread](https://discuss.pixls.us/t/first-outing-of-the-new-year-the-creations/10658)
- Real failure modes reported on pixls that a deck should reproduce:
  - corner star elongation from lens distortion during registration (Siril compared with APP and Sequator) — [#20991](https://discuss.pixls.us/t/siril-needs-distortion-correction-in-stacking/20991); [#35487](https://discuss.pixls.us/t/stacking-non-tracked-images-got-severe-distortion/35487)
  - aligning stars while the landscape stays fixed — [#38224](https://discuss.pixls.us/t/averaging-images-aligning-sky-on-stars/38224)
  - separate sky and foreground exposures — [#17204](https://discuss.pixls.us/t/open-source-tool-for-sky-stacking-and-foreground-extraction/17204)
  - flatfield or vignetting with the noise floor — [#20974](https://discuss.pixls.us/t/noise-floor-and-flatfield-correction/20974)
  - Bortle 9 light pollution — [#26671](https://discuss.pixls.us/t/removing-bortle-9-light-pollution-in-siril-beginner/26671)
  - the poor result of stacking 120 all-sky frames — [#40679](https://discuss.pixls.us/t/allsky-360-fulldome-images-stacking/40679)

### Inferences
**Practical plan, in priority order**
1. Use now: the Dryad CC0 bursts, pixls #17204 (CC BY-SA) and the CC0 Hasselblad frames. Add CC BY-SA single frames for format coverage.
2. Ask (via a pixls DM or a new "Call for CC0 nightscape sequences" thread):
   - sguyader to state a licence for the Siril dataset;
   - erratasama (60 RAF, Mt Timpanogos);
   - jchnkl (16 frames plus a long foreground, light-polluted Franconia, plus a sunset-to-Milky-Way timelapse);
   - wiegemalt (~10 ORF);
   - Daniel_Catalina (5-frame Geroldsee);
   - Waveluke (100+ A7 II frames with darks and flats);
   - nathanm (250 × 5 s R5);
   - plaven (172-frame 5D III timelapse);
   - Arnaud_Mariat (120 all-sky frames);
   - TheJEB and BorisRabit (aurora);
   - psalm19pix, who already publishes CC0 (Hasselblad).
3. Run a community call with a clear brief: CC0 preferred, full untouched RAW series, darks if shot, a short scene note, and hosting on Zenodo or GitHub releases paid for by the project.
4. Commission or shoot fresh CC0 sets by non-owner contributors for the gaps. The owner's own archive is excluded.

**Scene-variety checklist for the deck.** Each item should have ≥15 consecutive tripod frames unless noted.
- **Baseline showcase:** dark sky (Bortle 1–3), Milky Way core, crisp simple horizon, 14–24mm, 10–25 s, ISO 1600–6400. This makes method differences easy to judge.
- **Complex foreground edge:** tree silhouettes with sky gaps, a building or rock arch. This tests sky/foreground mask quality.
- **Moving foliage:** wind in trees or grass. Tests ghosting and rejection.
- **Water:** a lake or sea with sky reflections that rotate with the sky, and waves.
- **Light pollution:** a strong skyglow gradient (Bortle 6–9), sodium/LED colour casts, lit buildings.
- **Moonlight:** a moonlit foreground and a brighter sky background.
- **Aurora:** fast-changing sky structure that stacks badly.
- **Clouds:** passing or partial cloud.
- **Transients:** satellites, aircraft, meteors, for outlier rejection.
- **Light painting and car headlights:** foreground illumination that changes between frames.
- **Camera drift or bump:** tripod settling, a nudged frame, wind shake.
- **Lens extremes:** fisheye or ≤14mm wide-angle with strong distortion and coma at f/1.4–2. Tests registration in the corners.
- **Long series:** 100+ frames for star trails or timelapse and performance testing.
- **Calibration:** at least one set with darks, flats and bias. At least one set with lights only.
- **Separate exposures:** tracked sky plus untracked foreground, or a long low-ISO foreground frame (blend workflows).
- **Sensor and format diversity:** Bayer (Canon CR2/CR3, Nikon NEF, Sony ARW), X-Trans (Fuji RAF), Micro Four Thirds (ORF/RW2), Pentax PEF, medium format (3FR), phone DNG; landscape and portrait orientation.
- **Thermal noise:** a warm night with hot pixels or amp glow.

### Gaps
- Whether the named pixls contributors still have their full sequences, and whether they would license them as CC0 or CC BY-SA, is unknown until they are asked.
- I found no published "nightscape stacking benchmark" with scene criteria to cite. The checklist above is synthesised from the failure modes reported on pixls.
