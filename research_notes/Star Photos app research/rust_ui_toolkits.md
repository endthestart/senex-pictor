# Rust desktop UI toolkits for Star Photos

State as of 2026-10-27. Crate versions and dates come from the crates.io API (queried 2026-10-27) unless another source is given. Local LightCraft sources were read only, never modified. Local links point at files on the owner's machine.

## How do the candidate toolkits compare on maturity, licence, wgpu integration and desktop features?

### Takeaway
Only three candidates can share a wgpu 30 `Device`/`Queue` with Star Photos' compute code and draw its textures without a CPU copy: egui/eframe (stable API: `WgpuSetupExisting`, `register_native_texture`, paint callbacks), Slint 1.18 (behind the `unstable-wgpu-30` feature) and, in principle, Vello-based stacks that are still pre-release. egui is also the only one that releases often, ships AccessKit and a snapshot-test harness, and is already proven in a 24–45 MP photo editor with EDR output (LightCraft Classic). iced is stuck on wgpu 27 with no release since 2025-12-07 and no accessibility. Floem and Xilem are too immature. Tauri and Dioxus put the UI in a webview, which complicates a zero-copy wgpu canvas. GTK4 and Qt bring LGPL licensing, C/C++ toolchains, and no wgpu interop on macOS.

### Cited Findings

**Cross-cutting: wgpu 30 and colour on macOS**
- wgpu v30.0.0 (2026-07-01) added "Surfaces can now be configured with an explicit color space, enabling HDR and wide-gamut output", a `color_space` field on `SurfaceConfiguration` (default `SurfaceColorSpace::Auto`), and `Surface::display_hdr_info` (luminance/headroom, populated on Metal/macOS, DX12/Vulkan on Windows, web). v30.0.1 was released 2026-08-22. — [wgpu releases](https://github.com/gfx-rs/wgpu/releases)
- wgpu-types 30.0.1 defines `SurfaceColorSpace::{Auto, Srgb, ExtendedSrgbLinear, DisplayP3, ExtendedSrgb, ExtendedDisplayP3, Bt2100Pq, Bt2100Hlg}`. `DisplayP3` is supported on Metal and on Vulkan where the driver exposes it, but "Not reported on DX12". — [local wgpu-types-30.0.1/src/surface.rs](/Users/andermic/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/wgpu-types-30.0.1/src/surface.rs)
- wgpu-hal 30.0.1 Metal sets `CAMetalLayer.colorspace` (extended linear sRGB, extended sRGB, extended Display P3, BT.2100 PQ/HLG) and turns on `wantsExtendedDynamicRangeContent` for HDR colour spaces. For `Srgb` it sets the colour space to `None`, with the code comment "Reset to the layer's default, which treats contents as sRGB". — [local wgpu-hal-30.0.1/src/metal/surface.rs](/Users/andermic/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/wgpu-hal-30.0.1/src/metal/surface.rs)
- Apple's documentation contradicts that wgpu comment: "The default value is `nil`, indicating that the rendered content isn't color-matched." — [Apple CAMetalLayer.colorspace](https://developer.apple.com/documentation/quartzcore/cametallayer/colorspace). LightCraft saw the effect in practice (issue #787): with no layer colour space, previews looked over-saturated on P3 screens. — [lightcraft-classic/docs/display-profiles.md](/Users/andermic/Development/lightcraft-classic/docs/display-profiles.md)

**egui / eframe (emilk/egui; sponsored by Rerun)**
- Release cadence, 2025–2026: 0.33.3 (2025-12-11), 0.34.0 (2026-03-26), 0.35.0 (2026-06-25), 0.36.0 (2026-08-05), 0.36.2 (2026-09-08), 0.37.0 (2026-10-15). Licence MIT OR Apache-2.0. About 28.6 M downloads. — [egui releases](https://github.com/emilk/egui/releases); crates.io API
- Recent work: text rendering moved to skrifa/vello_cpu with hinting (0.34) and harfrust shaping with kerning and ligatures (0.35). 0.37 adds colour emoji and any system glyph. IME composition was overhauled in 0.35. AccessKit 0.24 came in 0.34. 0.37 replaced `WidgetType` with `accesskit::Role` and made kittest fail a test when an input widget has no accessible name. — [egui releases](https://github.com/emilk/egui/releases)
- egui-wgpu 0.36.2 depends on `wgpu = "30.0"`. eframe accepts an app-created device through `WgpuSetup::Existing(WgpuSetupExisting { instance, adapter, device, queue })`. Custom GPU drawing has two routes: `egui_wgpu::CallbackTrait` (prepare / finish_prepare / paint inside egui's render pass) and `Renderer::register_native_texture`. — [local egui-wgpu-0.36.2 setup.rs / renderer.rs](/Users/andermic/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-wgpu-0.36.2/src/setup.rs)
- egui-wgpu picks the window surface format itself (`preferred_framebuffer_format`, 8-bit `Rgba8Unorm`/`Bgra8Unorm`). It has no API for the surface colour space, so stock eframe cannot ask wgpu 30 for P3 or EDR. — [local egui-wgpu-0.36.2/src/lib.rs](/Users/andermic/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-wgpu-0.36.2/src/lib.rs); [lightcraft/apps/lightcraft/src/hdr_display.rs](/Users/andermic/Development/lightcraft/apps/lightcraft/src/hdr_display.rs) ("eframe 0.36 picks the surface format itself (8-bit only …) so today the choice reports SDR")
- Open upstream colour issues: #8283 "Revisit Linear vs sRGB blending" (opened 2026-07-02), #6863 (linear framebuffer warning) and #7484 "wgpu configure surface breaks egui". No issue in the search results mentions P3, EDR or wide gamut. — [egui issues: colour](https://github.com/emilk/egui/issues?q=is%3Aissue+color+space+OR+wide+gamut+OR+HDR); [egui issues: HDR](https://github.com/emilk/egui/issues?q=is%3Aissue%20HDR)
- README: "The main disadvantage of immediate mode is it makes layout more difficult". "Native looking interface" is a non-goal ("If you want a GUI that looks native, egui is not for you"). AccessKit "currently implements the native accessibility APIs on Windows and macOS". — [egui README](https://github.com/emilk/egui)
- Native menus and dialogs come from outside egui: muda 0.21.2 (2026-10-08) and rfd 0.17.2. LightCraft uses both. — crates.io API; [lightcraft-classic/Cargo.toml](/Users/andermic/Development/lightcraft-classic/Cargo.toml)

**iced (iced-rs)**
- The latest release is 0.14.0 (2025-12-07); there has been no stable release in the ~11 months since. Before that came 0.13.1 (2024-09-19). Licence MIT. — crates.io API; [iced releases](https://github.com/iced-rs/iced/releases)
- iced_wgpu 0.14.0 depends on `wgpu ^27.0`, three majors behind Star Photos' wgpu 30, so the two cannot share one `Device` without forking or upgrading iced. — crates.io dependencies API
- 0.14 added reactive rendering, time-travel debugging, headless testing, input methods (IME), concurrent image decoding/uploading and hot reloading. — [Phoronix](https://www.phoronix.com/news/Iced-0.14-Rust-GUI-LIbrary); [iced releases](https://github.com/iced-rs/iced/releases)
- The `Shader` widget renders custom shaders with iced's wgpu backend (since 0.12). 0.14 added a `shader::Pipeline` trait. Multi-window came in 0.12. — [iced shader docs](https://docs.iced.rs/iced/widget/shader/index.html); [iced releases](https://github.com/iced-rs/iced/releases)
- iced_test 0.14.0 runs apps headless, with a `Simulator` (click, tap_key, typewrite), selectors, "snapshot testing" and a `screenshot` function. Whether it compares against golden images is not stated. — [iced_test docs](https://docs.rs/iced_test/latest/iced_test/)
- Accessibility: issue #552 (opened 2020) is still open and pinned, with "No branches or pull requests". The maintainer's last statement was in June 2024: "Accessibility is planned for the release after the next one". The pop-os/iced fork has an AccessKit integration. — [iced #552](https://github.com/iced-rs/iced/issues/552)

**Slint (SixtyFPS GmbH)**
- Cadence: 1.12 (2025-06-16, wgpu support), 1.13 (2025-09-03), 1.14 (2025-10-21), 1.15 (2026-02-04), 1.16 (2026-04-16), 1.17 (2026-06-24), 1.18 (2026-09-16), 1.18.1 (2026-09-21). — [Slint blog](https://slint.dev/blog); crates.io API
- Licence: triple-licensed, "GPL-3.0-only OR LicenseRef-Slint-Royalty-free-2.0 OR LicenseRef-Slint-Software-3.0". The royalty-free licence covers desktop, mobile and web apps on condition that you disclose Slint use (e.g. the `AboutSlint` widget or the badge). Embedded is excluded. — [Slint LICENSE.md](https://github.com/slint-ui/slint/blob/master/LICENSE.md); [Slint pricing](https://slint.dev/pricing)
- wgpu 30: feature `unstable-wgpu-30` and `BackendSelector::require_wgpu_30(...)`. The app either passes its own setup (`WGPUConfiguration::Manual`) or receives Slint's device in a rendering notifier. "It's crucial that you're using the same wgpu::Device and wgpu::Queue". `slint::Image::try_from(texture)` imports a `wgpu::Texture`. The module "may be removed or changed in future minor releases". — [Slint wgpu_30 docs](https://docs.slint.dev/latest/docs/rust/slint/wgpu_30/); [wgpu_29 docs](https://docs.slint.dev/latest/docs/rust/slint/wgpu_29/)
- i-slint-core 1.18.1 depends on accesskit ^0.24, and the winit backend on accesskit_winit ^0.33. 1.18 lets screen readers read text-input content and selection, and adds an experimental Vello renderer. — crates.io dependencies API; [Slint 1.18 blog](https://slint.dev/blog/slint-1.18-released)
- MenuBar arrived in 1.10 (2025-02-28). On Windows and macOS it uses muda, so menus are native. — [Slint 1.10 blog](https://slint.dev/blog/slint-1.10-released)
- From 1.16, Fluent is the default style on every platform. The native-looking styles (Cupertino etc.) "will be phased out". — [Slint blog 2026-03-31](https://slint.dev/blog/default-native-style-change)

**Tauri (web UI + Rust core)**
- tauri 2.12.5 (2026-10-21), with 3.0.0-alpha.5 (2026-10-14) in progress. wry 0.57.1 (2026-10-23). Licence Apache-2.0 OR MIT. — crates.io API
- Tauri v2 has built-in macOS signing and notarization: `APPLE_SIGNING_IDENTITY` or `APPLE_CERTIFICATE`, with notarization via either the App Store Connect API key or Apple ID + app-specific password + team ID; ad-hoc signing uses `"signingIdentity": "-"`. — [Tauri macOS signing](https://v2.tauri.app/distribute/sign/macos/)
- No official way to put a wgpu surface under or inside a Tauri webview turned up. Transparency means making the window, the webview and the HTML background all transparent. A comparable framework (Electrobun) layers a separate GPU surface, which complicates passthrough and transparency (it fails on Linux). — [Electrobun wgpu tag](https://blackboard.sh/electrobun/docs/apis/browser/electrobun-wgpu-tag); [Wry v0.39.4 notes](https://v2.tauri.app/release/wry/v0.39.4/)

**Dioxus**
- dioxus 0.7.10 (2026-07-30) is stable; 0.8.0-alpha.3 came out 2026-10-16. Licence MIT OR Apache-2.0. Desktop runs in a webview by default. Dioxus Native, the WGPU-based HTML/CSS renderer, shipped its "first-ever version" in 0.7, and Blitz is "still considered a 'work in progress'". — crates.io API; [Dioxus 0.7 release](https://dioxuslabs.com/blog/release-070)
- dioxus-native 0.7.10 depends on anyrender_vello 0.6, which pulls in wgpu ^26. The 0.7 post does not say whether apps can draw their own wgpu textures into the native renderer. — crates.io dependencies API; [Dioxus 0.7 release](https://dioxuslabs.com/blog/release-070)

**gtk4-rs / libadwaita (GNOME)**
- gtk4 crate 0.11.0 (2026-02-20) → 0.11.5 (2026-09-20); libadwaita crate 0.9.2 (2026-07-07). The bindings are MIT; GTK itself is a C library (see Gaps on its licence). — crates.io API; [docs.rs gtk4](https://docs.rs/crate/gtk4/latest)
- Colour: `GdkColorState` dates from GTK 4.16. GTK supports the Wayland colour-management protocol and renders HDR internally, but HDR output "requires a compositor that accepts such content". — [GTK blog "The colors of GTK"](https://blog.gtk.org/2024/08/11/the-colors-of-gtk/); [GdkColorState docs](https://docs.gtk.org/gdk4/struct.ColorState.html)
- Accessibility: GTK 4.18 merged an AccessKit backend that makes GTK apps accessible on Windows and macOS "for the first time". It is opt-in (`-Daccesskit=enabled`, `GTK_A11Y=accesskit`). A 2026 LWN report says "no movement in the GTK implementation". — [GTK blog 2025-05-12](https://blog.gtk.org/2025/05/12/an-accessibility-update/); [LWN](https://lwn.net/Articles/1058024)
- The macOS backend shares the OpenGL ("NGL") renderer with Linux. A Metal renderer was only discussed as future work (2021 sources); I found no 2026 status. — [Christian Hergert, GTK 4 NGL renderer](https://blogs.gnome.org/chergert/2021/02/24/gtk-4-ngl-renderer/); [Phoronix](https://www.phoronix.com/news/GTK4-macOS-Improved)

**Qt via cxx-qt / qmetaobject**
- cxx-qt (KDAB) 0.8.1 (2026-02-17), 0.9.0 (2026-06-23), 0.10.0 (2026-08-24); MIT OR Apache-2.0. qmetaobject has not released since 0.2.10 (2023-11-03). — crates.io API; [cxx-qt GitHub](https://github.com/KDAB/cxx-qt)
- Qt colour: `QColorSpace` includes Display P3, and Qt 6.8 added BT.2020 and BT.2100 PQ/HLG. `QRhiSwapChainHdrInfo` reports the current and potential maximum EDR values on macOS. A Qt wiki session called QRhi's HDR swapchains "experimental". — [QColorSpace](https://doc.qt.io/qt-6/qcolorspace.html); [QRhiSwapChainHdrInfo](https://doc.qt.io/qt-6/qrhiswapchainhdrinfo.html); [Qt wiki](https://wiki.qt.io/Color_space_in_Qt_-_issues_and_possibilities)

**Makepad**
- makepad-widgets 1.0.0 (2025-05-13) has no later crates.io release; licence MIT OR Apache-2.0. It has its own GPU renderer (Metal, DX11, OpenGL, WebGL), not wgpu. Its authors call it "an early release", with fuller font support deferred to 1.1. — crates.io API; [r/rust 1.0 thread](https://redlib.hbubli.cc/r/rust/comments/1kllldg/makepad_10_rust_ui_framework); [SourcePulse summary](https://www.sourcepulse.org/projects/183499)

**Xilem / Masonry / Vello (Linebender)**
- xilem and masonry 0.4.0 (2025-10-29) have had no release since; licence Apache-2.0. masonry 0.4 depends on accesskit ^0.21 and vello ^0.6 (wgpu ^26). — crates.io API
- Vello itself releases briskly: 0.8 (2026-03-20), 0.9 (2026-05-15), 0.10 (2026-08-14), 0.11 (2026-10-02, wgpu ^30). — crates.io API
- Linebender's 2026 blog posts were Q1 status updates and SIMD work. None announced a Xilem release. — [Linebender blog](https://linebender.org/blog/)

**floem (Lapce)**
- floem 0.2.0 (2024-11-14) is the latest release, on wgpu ^22; licence MIT. The repository was last pushed 2025-12-06. — crates.io API; [goodfirstissue listing](https://www.goodfirstissue.org/lapce/floem)

**Shared infrastructure**
- winit 0.30.13 is the stable line; 0.31.0-beta.4 came out 2026-10-24. accesskit 0.25.2 came out 2026-10-25. — crates.io API

### Inferences
- Summary matrix (✔ fits, ~ partial, ✘ poor):

| Toolkit | Latest release (date) | Licence risk | Shares wgpu 30 device/textures | Accessibility | Native menus | Maturity fit |
|---|---|---|---|---|---|---|
| egui/eframe | 0.37.0 (2026-10-15) | ✔ MIT/Apache | ✔ stable API (needs patch for P3/EDR surface) | ✔ AccessKit (Win/macOS/Linux) | ~ via muda | ✔ |
| iced | 0.14.0 (2025-12-07) | ✔ MIT | ✘ wgpu 27 | ✘ none upstream | ✘ none found | ~ |
| Slint | 1.18.1 (2026-09-21) | ~ GPLv3 or attribution licence | ~ `unstable-wgpu-30` | ✔ AccessKit | ✔ muda | ✔ (declarative) |
| Tauri | 2.12.5 (2026-10-21) | ✔ | ✘ webview boundary | ✔ via browser | ✔ | ✔ app shell, ✘ canvas |
| Dioxus | 0.7.10 / 0.8-α | ✔ | ✘ webview / ~ native on wgpu 26 | ~ | ~ | ~ |
| gtk4-rs | 0.11.5 (2026-09-20) | ~ GTK under LGPL (unverified) | ✘ GL on macOS | ~ AccessKit opt-in | ~ | ✔ Linux, ✘ macOS |
| Qt (cxx-qt) | 0.10.0 (2026-08-24) | ~ Qt LGPL/GPL (unverified) | ✘ needs C++ bridge | ✔ | ✔ | ✔ toolkit, ~ Rust bridge |
| Makepad | 1.0.0 (2025-05-13) | ✔ | ✘ own renderer | ? | ? | ✘ |
| Xilem/Masonry | 0.4.0 (2025-10-29) | ✔ | ~ Vello/wgpu 26 | ✔ AccessKit | ? | ✘ alpha |
| floem | 0.2.0 (2024-11-14) | ✔ | ✘ wgpu 22 | ? | ? | ✘ stale |

- Every toolkit except egui needs a fork, an upgrade, a CPU copy or a C++ bridge before Star Photos' wgpu 30 compute output can be shown without leaving the GPU. Slint is the only credible second choice on this axis, and only behind an "unstable" feature that can break in minor releases.
- Any toolkit can be colour-managed in the app (render into the display's primaries, as LightCraft does), but true P3/EDR on macOS needs control over the surface's colour space. With egui that means patching egui-wgpu, which LightCraft Classic has already done. With Slint and Qt it depends on how much of the swapchain they expose. With webviews it depends on WebKit's canvas colour-space support, which this research did not confirm.
- Under the wgpu-30 code, `SurfaceColorSpace::Srgb` on Metal still leaves the layer "not color-matched" (per Apple). An app using wgpu 30 directly should pick `DisplayP3` or `ExtendedSrgbLinear` explicitly on macOS instead of relying on `Srgb`.

### Gaps
- GTK's and Qt's licences (GTK LGPL-2.1-or-later; Qt LGPLv3/GPL/commercial) were not re-verified from primary sources in this pass.
- No source confirmed that Slint, GTK4, Qt or WKWebView/Tauri can show Display P3 or EDR window output from Rust in 2026. GTK's macOS colour handling and any 2026 Metal renderer are unknown.
- No source confirmed whether iced's master branch has moved past wgpu 27, or the date of a 0.15 release.
- Slint's testing story (screenshot or snapshot tests) and multi-window API were not checked.
- Makepad's and floem's accessibility, IME and native-menu status was not found.
- How RapidRAW's "direct WGPU renderer" composites with the Tauri webview is not documented in its README.
- "Are we GUI yet" was not consulted; it was not needed for the facts above.

## What are egui's known limitations for professional photo editors, and exactly how does LightCraft work around them?

### Takeaway
egui's limits for a photo editor are an 8-bit, colour-space-blind surface; layout that immediate mode makes harder; a look that is not native; incomplete IME, RTL and bidi text; and accessibility that only covers what each widget names. LightCraft works around every one of these in its own code, at a cost. It colour-manages in the app (render into the display's ICC primaries) and vendors egui-wgpu for an EDR/scRGB surface. It builds its own text field, theme, docking and activity system. It bridges native menus with muda and dialogs with rfd. It tests through AccessKit and a CPU rasterizer. The newer LightCraft goes further and keeps a fork of egui and winit, which it has to migrate on every egui release.

### Cited Findings

**Colour management (display ICC, P3)**
- macOS: "macOS doesn't colour-manage the app's window (wgpu leaves its Metal layer without a colour space, which Core Animation shows unmatched)". LightCraft reads the screen's ICC profile about once a second and renders the loupe into the display's own primaries (`lcc_pipeline::DisplaySpace`). It then applies device curves or LUTs through `lcc_codecs::display::DisplayProfile::correct` (moxcms for LUT profiles), and converts thumbnails and caches in `lcc_engine::display::present`. "What stays sRGB: the histogram …, exports, and the UI itself." — [lightcraft-classic/docs/display-profiles.md](/Users/andermic/Development/lightcraft-classic/docs/display-profiles.md)
- The screen profile comes from AppKit `NSScreen.colorSpace().ICCProfileData()` on the main thread. — [lightcraft-classic/apps/app/src/screen_profile.rs](/Users/andermic/Development/lightcraft-classic/apps/app/src/screen_profile.rs); Linux X11 `_ICC_PROFILE_n`/colord: [screen_profile_linux.rs](/Users/andermic/Development/lightcraft-classic/apps/app/src/screen_profile_linux.rs); Windows registry: [screen_profile_windows.rs](/Users/andermic/Development/lightcraft-classic/apps/app/src/screen_profile_windows.rs); ICC sanity checks: [screen_icc.rs](/Users/andermic/Development/lightcraft-classic/apps/app/src/screen_icc.rs). LightCraft (MIT/Apache) has the same macOS reader plus a colord client: [lightcraft/apps/lightcraft/src/screen_profile.rs](/Users/andermic/Development/lightcraft/apps/lightcraft/src/screen_profile.rs), [colord.rs](/Users/andermic/Development/lightcraft/apps/lightcraft/src/colord.rs)
- Known gaps: Windows does not follow the window across monitors, and pure Wayland cannot tell which monitor the window is on. — [display-profiles.md](/Users/andermic/Development/lightcraft-classic/docs/display-profiles.md)

**HDR / EDR preview**
- LightCraft Classic vendors egui-wgpu 0.36.2 under `[patch.crates-io]`. The patch adds `WgpuConfiguration::hdr_output`, which configures `Rgba16Float` + `SurfaceColorSpace::ExtendedSrgbLinear` (EDR on macOS, scRGB on Windows). It exposes `RenderState::{target_color_space, surface_formats, display_hdr_info}`, scales egui's own drawing to SDR "paper white" through a new `fs_main_extended_linear_framebuffer` shader entry, and adds `open_device` / `on_surface_configured` hooks. The note: "Drop the patch once egui exposes the surface format, colour space and display HDR info itself." — [lightcraft-classic/vendor/README.md](/Users/andermic/Development/lightcraft-classic/vendor/README.md); [Cargo.toml `[patch.crates-io]`](/Users/andermic/Development/lightcraft-classic/Cargo.toml)
- The HDR decisions are pure and unit-tested, falling back to SDR whenever a condition is missing. The loupe's HDR image is an `Rgba16Float` wgpu texture registered with `register_native_texture` / `update_egui_texture_from_wgpu_texture` and filled with `queue.write_texture` from CPU half-floats. — [lightcraft-classic/apps/app/src/hdr_display.rs](/Users/andermic/Development/lightcraft-classic/apps/app/src/hdr_display.rs) (≈ lines 360–400)
- Supporting crates and platform code: `lcc-hdrmeta` sets VK_EXT_hdr_metadata, with Wayland handled in [wayland_hdr.rs](/Users/andermic/Development/lightcraft-classic/apps/app/src/wayland_hdr.rs). X11 has no HDR, which is a platform limit. Gap 14 closed 2026-10-21. — [lightcraft-classic/docs/gaps.md](/Users/andermic/Development/lightcraft-classic/docs/gaps.md)
- The newer LightCraft has the HDR decision code but not the surface patch, so it "reports SDR with the reason". — [lightcraft/apps/lightcraft/src/hdr_display.rs](/Users/andermic/Development/lightcraft/apps/lightcraft/src/hdr_display.rs)

**GPU device and image canvas**
- LightCraft Classic's compute pipeline creates its own wgpu adapter and device (`request_adapter` / `request_device`) and reads results back with `map_async`. It does not share the eframe window's device. SDR results reach egui as CPU `ColorImage`s; HDR results go through native wgpu textures. — [lightcraft-classic/crates/gpu/src/ctx.rs](/Users/andermic/Development/lightcraft-classic/crates/gpu/src/ctx.rs) (≈ lines 416–434, 730); [crates/ui-egui/src/render.rs](/Users/andermic/Development/lightcraft-classic/crates/ui-egui/src/render.rs)
- `render.rs` runs a slot-based background render queue on `lcc_preview::JobPool`. Slots cover the loupe, drag windows, Before, Compare, prefetch and the second window. The loupe goes first, stale requests are replaced, and zoomed views render windowed drafts and tiles. — [render.rs header](/Users/andermic/Development/lightcraft-classic/crates/ui-egui/src/render.rs)
- 100 % inspection on HiDPI: "At 100 % the texture maps texel for texel to screen pixels, so its corner must sit on a whole physical pixel: half a pixel off and the GPU's bilinear filter blurs the view". The view rect is therefore snapped to physical pixels using `pixels_per_point`. — [lightcraft-classic/crates/ui-egui/src/panels/detail.rs](/Users/andermic/Development/lightcraft-classic/crates/ui-egui/src/panels/detail.rs) (≈ lines 255–266)
- Compare has synced zoom and pan ([panels/compare.rs](/Users/andermic/Development/lightcraft-classic/crates/ui-egui/src/panels/compare.rs)). The second window is a native window, with a floating egui window as fallback ([panels/second.rs](/Users/andermic/Development/lightcraft-classic/crates/ui-egui/src/panels/second.rs)). Histogram, waveform and parade scopes are drawn in egui ([panels/histogram.rs](/Users/andermic/Development/lightcraft-classic/crates/ui-egui/src/panels/histogram.rs); gaps.md #30).
- Robustness: wgpu loads every backend driver it enumerates, so LightCraft restricts the backends for both the window and compute. On Windows it probes DX12 → Vulkan → GL → glow with non-panicking error handlers before eframe starts, then hands eframe the device that worked. — [apps/app/src/window_gpu.rs](/Users/andermic/Development/lightcraft-classic/apps/app/src/window_gpu.rs); [docs/gpu-pipeline.md](/Users/andermic/Development/lightcraft-classic/docs/gpu-pipeline.md)

**Native menus, dialogs, appearance**
- The native macOS menu bar uses muda, generated from the same model as the in-window menus. AppKit only offers keys with ⌘/⌃ (or function keys) to the menu bar, so other accelerators are routed to egui, and non-⌘ accelerators are removed while a text field has focus. — [apps/app/src/native_menu.rs](/Users/andermic/Development/lightcraft-classic/apps/app/src/native_menu.rs). This logic is now shared as `craft-menu` ("a macOS backend, and key equivalents fed back to egui"). — [craft-libs crates/menu](https://github.com/storytold/craft-libs)
- rfd dialogs: rfd builds macOS panels on the main queue, so a guard was added to catch Rust panics there (upstream #1996). — [apps/app/src/native_dialog.rs](/Users/andermic/Development/lightcraft-classic/apps/app/src/native_dialog.rs)
- On Linux, dark/light mode is read from the XDG settings portal via zbus because "many Wayland compositors" report no theme to egui. — [apps/app/src/appearance.rs](/Users/andermic/Development/lightcraft-classic/apps/app/src/appearance.rs)

**Look and feel / layout**
- All theming is in-house design tokens: four themes measured from the reference app's dark theme. — [crates/ui-egui/src/theme.rs](/Users/andermic/Development/lightcraft-classic/crates/ui-egui/src/theme.rs) (1,054 lines). Docking and panel persistence: [docking.rs](/Users/andermic/Development/lightcraft-classic/crates/ui-egui/src/docking.rs). Title-bar ownership: `craft-window`.
- Context menus needed a fix because "egui counts the opening click as a click while open" (issue #1877). — [docs/ui-parity.md](/Users/andermic/Development/lightcraft-classic/docs/ui-parity.md)
- The egui 0.37 migration notes that "`Style` defaults are roomier", so menus, tooltips and dialogs can shift and need geometry review. — [lightcraft/docs/egui-037-migration-plan.md](/Users/andermic/Development/lightcraft/docs/egui-037-migration-plan.md)

**Text, IME, fonts**
- A custom `TextField` adds Cut/Copy/Paste menus, Esc-revert and select-on-focus. — [crates/ui-egui/src/text_field.rs](/Users/andermic/Development/lightcraft-classic/crates/ui-egui/src/text_field.rs). Text fields are rated "70%", with "IME and selection edge cases" outstanding. — [docs/ui-parity.md](/Users/andermic/Development/lightcraft-classic/docs/ui-parity.md)
- "no RTL or complex shaping" remains a localisation gap. — [docs/gaps.md](/Users/andermic/Development/lightcraft-classic/docs/gaps.md) (#21). Releases embed craft-fonts for CJK. — [docs/packaging.md](/Users/andermic/Development/lightcraft-classic/docs/packaging.md)
- LightCraft pins a full egui-family and winit fork (storytold/egui `6d518fc`, storytold/winit `a517b38`). The fork carries an E1–E21 patch series plus P2b (Linux PRIMARY selection with winit W15P), P7/P7b (held-key pairing, outgoing IME surrounding text with winit W27), bidi/RTL patches (E5/E12/E13/E16/E17/E20/E21) and E2/E3 (surface fit for very large surfaces, renderer error scopes). The migration plan is 1,552 lines; a fork trial produced 17 library errors and 26 test errors. Visible GUI, native IME, packaging and paired performance "remain open" acceptance gates. — [lightcraft/Cargo.toml](/Users/andermic/Development/lightcraft/Cargo.toml); [lightcraft/docs/egui-037-migration-plan.md](/Users/andermic/Development/lightcraft/docs/egui-037-migration-plan.md)

**Accessibility**
- AccessKit is on through eframe's `accesskit` feature, and VoiceOver, Narrator and Orca read the tree. Hand-painted widgets get names from helpers in [crates/ui-egui/src/access.rs](/Users/andermic/Development/lightcraft-classic/crates/ui-egui/src/access.rs). A headless test fails on any unlabeled interactive node ([tests_access.rs](/Users/andermic/Development/lightcraft-classic/crates/ui-egui/src/tests_access.rs)). Still left: egui's scroll bars are unnamed, and "Mask pins and brush strokes are still pointer-only". — [docs/accessibility.md](/Users/andermic/Development/lightcraft-classic/docs/accessibility.md)
- LightCraft's kittest accessibility test walks the AccessKit tree. — [lightcraft/crates/ui-egui/tests/it/accessibility.rs](/Users/andermic/Development/lightcraft/crates/ui-egui/tests/it/accessibility.rs)

**Background jobs**
- `lcc_engine::activity` is a single registry. A guard per job drives progress, detail and `Cancel::{Flag, Yes, No}`, and the row disappears when the guard drops. One activity stack shows the rows ([panels/activity.rs](/Users/andermic/Development/lightcraft-classic/crates/ui-egui/src/panels/activity.rs)), and Quit asks first while a cancellable task runs. The rule: "don't add a progress window … of your own". — [docs/background-tasks.md](/Users/andermic/Development/lightcraft-classic/docs/background-tasks.md)

**Testing**
- A headless harness (`Headless::snapshot()` → `ColorImage`) runs the real app through its control protocol. A CPU rasterizer of egui's tessellated output ([softpaint.rs](/Users/andermic/Development/lightcraft-classic/crates/ui-egui/src/softpaint.rs)) mirrors egui-wgpu's shader for screenshots without a GPU. Tests assert on pixels and widget rects rather than golden files (e.g. [tests_grid.rs](/Users/andermic/Development/lightcraft-classic/crates/ui-egui/src/tests_grid.rs)). — [crates/ui-egui/src/headless.rs](/Users/andermic/Development/lightcraft-classic/crates/ui-egui/src/headless.rs)
- LightCraft uses `egui_kittest` with the `wgpu` and `snapshot` features, at several pixel densities (e.g. `with_pixels_per_point(scale)` in [panels/bottombar.rs](/Users/andermic/Development/lightcraft/crates/ui-egui/src/panels/bottombar.rs) and [tests/it/docking_workflows.rs](/Users/andermic/Development/lightcraft/crates/ui-egui/tests/it/docking_workflows.rs)). No committed snapshot PNGs were found in either repo. — [lightcraft/crates/ui-egui/Cargo.toml](/Users/andermic/Development/lightcraft/crates/ui-egui/Cargo.toml)

**Performance and packaging**
- At 100k photos: an unchanged frame takes 2–10 ms, a wheel-scroll frame 11–41 ms, the first frame 350–670 ms (measured under heavy load). — [docs/gaps.md](/Users/andermic/Development/lightcraft-classic/docs/gaps.md) (#26). Frame-time HUD: [frame_rate.rs](/Users/andermic/Development/lightcraft-classic/crates/ui-egui/src/frame_rate.rs)
- The macOS packaging script builds a universal `lipo` binary, signs with hardened runtime and entitlements, then runs `xcrun notarytool submit` and `stapler staple`. It ad-hoc signs by default when no secrets are set. Gap: "macOS signing/notarization … depend on CI secrets" and are not tested end to end. — [packaging/macos/package.sh](/Users/andermic/Development/lightcraft-classic/packaging/macos/package.sh); [docs/packaging.md](/Users/andermic/Development/lightcraft-classic/docs/packaging.md)

### Inferences
- The two hardest problems for a pro editor, P3 colour accuracy and EDR, are solved in LightCraft Classic. The cost is a vendored egui-wgpu that has to be re-applied on every egui upgrade, and the UI chrome and histogram stay sRGB.
- LightCraft never shared one wgpu device between compute and the window, so it pays for a GPU→CPU→GPU round trip on SDR previews. Star Photos can avoid that with `WgpuSetupExisting` plus `register_native_texture` or a `CallbackTrait` tile renderer. That path is supported by egui's API but unproven in the sibling apps.
- Much of LightCraft's egui investment is general-purpose and already partly factored into craft-libs: menus, theme, text field, accessibility helpers, activity registry, headless testing. That makes reuse cheaper than any alternative toolkit's learning curve.
- Owning a fork (the newer LightCraft) is a large ongoing cost driven by text, IME and clipboard edge cases. Star Photos has little free text entry and could stay on stock egui plus the small egui-wgpu HDR patch.

### Gaps
- Whether LightCraft's tiled zoom stays smooth at 45 MP on Intel Macs with AMD GPUs was not measured. The docs only say "GPU path proven on Apple only" (gaps.md #27).
- No document records why LightCraft chose egui originally, or whether it evaluated alternatives.
- I did not read the full egui-037 migration plan to extract every fork patch's description.

## Which serious photo or image applications use each toolkit?

### Takeaway
No major, widely used photo editor ships on any Rust GUI toolkit. The serious examples are mixed-stack: RapidRAW pairs Tauri/React with a wgpu backend, and Loupe is GNOME's GTK4/Rust image viewer. egui's flagship is the Rerun data viewer, and LightCraft and LightCraft Classic themselves are the most relevant egui photo editors. In astrophotography, the established tools are Qt (DeepSkyStacker) and GTK (Siril).

### Cited Findings
- **egui:** "egui can be used to create professional looking applications, like [the Rerun Viewer]", and Rerun sponsors egui development. — [egui README](https://github.com/emilk/egui). LightCraft (MIT/Apache) and LightCraft Classic (MIT) are full Lightroom-style editors on egui. — local repos. Oculante, a Rust image viewer with HDR tone-mapping and editing tools, is active (last push reported 2026-05-01). Search results did not confirm that it uses egui. — [oculante on OBS](https://build.opensuse.org/package/show/graphics/oculante); [gittrend](https://gittrend.io/repo/woelper/oculante)
- **Tauri:** RapidRAW is a GPU-accelerated non-destructive RAW editor (AGPL-3.0) built with Tauri, React/TypeScript and Rust/wgpu, using a WGSL processing shader. Its changelog has these entries: 2025-08-15 "Tauri's IPC replaced slow Base64 image transfer"; 2026-04-18 "Implemented direct WGPU renderer and fixed macOS GPU context initialization"; 2026-07-06 Windows offscreen-window fixes. Linux Wayland crashes are "related to WebKit and NVIDIA drivers". — [RapidRAW GitHub](https://github.com/CyberTimon/RapidRAW); [pixls.us introduction](https://discuss.pixls.us/t/introduction-rapidraw-a-gpu-accelerated-open-source-raw-editor/53585)
- **iced:** COSMIC desktop Epoch 1 (System76; stable 2025-12-11; 1.11.0 on 2026-10-20) is "built from scratch using the Iced graphics toolkit". — [LWN](https://lwn.net/Articles/1050268/); [LinuxCompatible](https://www.linuxcompatible.org/story/cosmic-epoch-1110-released-wayland-dialog-protocol-and-hybridgpu-fixes/). No photo editor on iced was found.
- **GTK4 + Rust:** Loupe, GNOME's default image viewer, loads images through glycin (sandboxed, mostly Rust decoders, ICC support). Its Debian package depends on GTK ≥ 4.16 and lcms2. — [Sophie Herold's blog](https://blogs.gnome.org/sophieh/category/gnome/image-viewing/); [GNOME Loupe](https://github.com/gnome/loupe)
- **GTK (C):** Siril, the astrophotography suite, historically targeted GTK+ 3. That source describes version 0.9, so the current toolkit is unverified. — [free-astro.org](https://free-astro.org/index.php/Image_Processing:Main)
- **Qt:** DeepSkyStacker finished its MFC→Qt conversion ("there should be no MFC stuff left") and builds against Qt 6.10. — [DSS release notes](https://newreleases.io/project/github/deepskystacker/DSS/release/6.1.0-Beta4)
- **Makepad:** Robrix (Matrix client) and Moly (LLM client); no image apps. — [r/rust thread](https://redlib.hbubli.cc/r/rust/comments/1kllldg/makepad_10_rust_ui_framework)
- **floem:** Lapce (code editor). — [floem](https://docs.rs/crate/floem/latest)
- **Slint, Dioxus, Xilem:** no photo or image applications turned up in this research.

### Inferences
- RapidRAW is the closest outside analogue to Star Photos: a Rust/wgpu image engine. Its 2026 move to a "direct WGPU renderer" suggests that pushing full-resolution pixels through the webview was a bottleneck, which is evidence against Tauri for a 45 MP canvas.
- DeepSkyStacker (Qt) and Siril (GTK) show that the incumbent astro stackers use big C/C++ toolkits; none is in Rust.

### Gaps
- I did not confirm oculante's GUI toolkit, current Siril's toolkit, or any Slint- or Dioxus-based image app.
- Usage beyond the projects named above was not surveyed systematically.

## Should Star Photos reuse LightCraft's egui stack or adopt an alternative, and with what trade-offs and risks?

### Takeaway
On the evidence, the strongest option is to reuse the egui/eframe stack, staying on stock egui with only Classic's small egui-wgpu HDR patch, which carries the lowest technical and schedule risk. Share the device with Star Photos' wgpu 30 compute and take the reusable pieces from craft-libs and LightCraft Classic. Slint is the only serious alternative, if a declarative UI or native-feeling widgets matter more than reuse; it adds an attribution or GPLv3 licence condition and depends on unstable wgpu interop. Tauri, Dioxus, GTK4, Qt, iced, Makepad, Xilem and floem each fail at least one hard requirement for Star Photos.

### Cited Findings
- Star Photos lists "desktop toolkit" and "outbound distribution license" among "Decisions still open". The first release "is intended for the public". — [star-photos/docs/requirements.md](/Users/andermic/Development/star-photos/docs/requirements.md)
- Star Photos is on wgpu 30 with `unsafe_code = "forbid"` and already vendors lightcraft-classic crates (`lcc-meta`, `lcc-raw`). — [star-photos/Cargo.toml](/Users/andermic/Development/star-photos/Cargo.toml)
- Licences: LightCraft Classic is MIT, except `crates/segment` and `crates/fetch`, which are Apache-2.0 only ([LICENSE](/Users/andermic/Development/lightcraft-classic/LICENSE), [NOTICE](/Users/andermic/Development/lightcraft-classic/NOTICE)). LightCraft is MIT OR Apache-2.0 ([Cargo.toml](/Users/andermic/Development/lightcraft/Cargo.toml)), and craft-libs is MIT OR Apache-2.0.
- craft-libs provides git-pinned crates already used by LightCraft. All are reusable by Star Photos:
  - `craft-cms`: pure-Rust ICC v2/v4 transforms, intents, BPC, soft proofing, gamut checks, built-in sRGB/Display P3.
  - `craft-menu`: muda-based macOS menus with egui key equivalents.
  - `craft-ui`: egui controls and icons.
  - `craft-window`: title-bar ownership.
  - Plus `craft-settings`, `craft-atomic`, `craft-keymap`, `craft-i18n`, `craft-quit`, `craft-platform` and `craft-update`.

  The LightCraft migration plan notes that newer revisions need a separate "foundation publication" before the public can fetch them. — [local craft-libs checkout README](/Users/andermic/.cargo/git/checkouts/craft-libs-d830b991a08c6c3d/1e6f127/README.md); [egui-037 plan](/Users/andermic/Development/lightcraft/docs/egui-037-migration-plan.md)
- Reusable LightCraft Classic pieces, with paths:
  - `vendor/egui-wgpu` (HDR surface).
  - `apps/app/src/{hdr_display.rs, screen_profile*.rs, screen_icc.rs, window_gpu.rs, native_dialog.rs, appearance.rs, wayland_hdr.rs}`.
  - `crates/{gpumem, sysmem, hdrmeta, preview, color}`.
  - From `crates/ui-egui/src`: `{access.rs, text_field.rs, softpaint.rs, headless.rs, frame_rate.rs, panels/activity.rs, panels/histogram.rs, panels/detail.rs (zoom/pan/pixel snap), panels/compare.rs}`.
  - `lcc_engine::activity`.
  - `packaging/macos/package.sh`.

  The descriptions are local paths cited in the egui-limitations section above.
- Slint's royalty-free licence needs visible disclosure (AboutSlint or a badge); otherwise use GPLv3 or buy a licence. Its wgpu 30 interop is "unstable" and "may be removed or changed in future minor releases". — [Slint LICENSE.md](https://github.com/slint-ui/slint/blob/master/LICENSE.md); [Slint wgpu_30](https://docs.slint.dev/latest/docs/rust/slint/wgpu_30/)

### Inferences
- **Option A: reuse egui (stock egui plus the HDR patch).**
  - Pros:
    - Same wgpu 30 as Star Photos; a shared device with zero-copy tiles via `WgpuSetupExisting` plus `register_native_texture` or `CallbackTrait`.
    - Proven P3 colour management and EDR in Classic.
    - AccessKit, kittest, muda and rfd already integrated.
    - The owner's own patterns and crates; MIT/Apache throughout.
    - Fast upstream cadence (four minor releases in 2026: 0.34 to 0.37).
  - Risks:
    - Upkeep of the vendored egui-wgpu patch.
    - Breaking changes on each egui minor (0.37 changed `Ui::id`, ScrollArea and Screenshot APIs).
    - A non-native look.
    - The immediate-mode layout tax on complex panels.
    - UI chrome stays sRGB.
    - Accessibility of custom canvases (mask pins, brushes) is manual.
    - If Star Photos adopts the storytold fork, it inherits the fork's migration burden (the 1,552-line plan). Prefer stock egui unless IME or bidi needs arise.
- **Option B: Slint 1.18.**
  - Pros:
    - Retained, declarative UI with live preview.
    - AccessKit.
    - Native menus via muda.
    - Imports wgpu 30 textures on a shared device.
  - Risks:
    - "Unstable" wgpu feature.
    - Licence condition (attribution or GPLv3) on an outbound licence that is still undecided.
    - Native styles being phased out from 1.16.
    - No P3/EDR surface control evidenced.
    - No reuse of LightCraft's UI code.
    - No sibling-app knowledge.
- **Reject or defer the rest:**
  - Tauri: webview boundary for 45 MP canvases. RapidRAW needed a custom renderer, and WebKitGTK/NVIDIA instability hits Linux.
  - Dioxus: webview, or a native renderer that is still work-in-progress and on wgpu 26.
  - iced: wgpu 27, no accessibility, release gap since 2025-12.
  - GTK4: macOS is a second-class target with a GL renderer, accessibility there is opt-in, and LGPL.
  - Qt: C++ toolchain, licensing, and a QRhi↔wgpu bridge would need unsafe C++ glue.
  - Makepad, Xilem and floem: immature or stale, and Makepad does not use wgpu.
- **Suggested ADR experiment:** a spike measuring 45 MP pan/zoom frame times on Intel+AMD and Apple Silicon Macs with a shared-device `CallbackTrait` tile renderer in eframe. Verify P3 (Classic's ICC path, or wgpu `DisplayP3` through the patched egui-wgpu) and EDR (`ExtendedSrgbLinear`). Run the same spike in Slint only if the owner wants a declarative alternative.

### Gaps
- No measured comparison of canvas performance across toolkits on Intel/AMD Macs was found.
- Whether the owner plans to publish craft-libs and the storytold forks for public, reproducible builds is unknown (see the migration plan's publication gates).
- Whether the eventual outbound licence would make Slint's GPLv3 option acceptable depends on a decision that is still open.
