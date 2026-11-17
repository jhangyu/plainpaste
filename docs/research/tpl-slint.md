# Slint UI SDK / design-system / template resources (checked 2026-11-16)

Project pin: slint 1.18.1 (Cargo.toml), style "fluent" via build.rs. Latest upstream: **v1.19.0 (2026-11-16)**; 1.18.1 on 2026-09-21; 1.18.0 on 2026-09-16 (gh api releases).
Tags: [V] = verified this session via gh api / WebFetch / WebSearch; [U] = not fully verified.

| Name | Category | Pairs with | Maintainer | Status 2025-26 (version, last release/push, ~stars) | URL | Tag |
|---|---|---|---|---|---|---|
| fluent (`fluent-light` / `fluent-dark`) | built-in style | Rust/C++/JS/Python; all platforms | SixtyFPS GmbH (Slint) | Default on ALL platforms since 1.16 (blog 2026-03-31); in 1.18/1.19 | https://docs.slint.dev/latest/docs/slint/reference/std-widgets/style/ | [V] |
| material (`material-light` / `material-dark`) | built-in style | Android/touch; any platform | Slint | Shipped in 1.18; was the old "native" choice on Android (1.5 docs) | same URL | [V] |
| cupertino (`cupertino-light` / `-dark`) | built-in style | macOS/iOS look | Slint | Exists; blog says non-Fluent styles get fewer updates, "gradually deprecated"; Cupertino called out as "uncanny valley" | https://slint.dev/blog/default-native-style-change | [V] |
| cosmic (`cosmic-light` / `-dark`) | built-in style | COSMIC desktop (Linux) | Slint | Exists in docs/source (`internal/compiler/widgets/cosmic`); not useful for Android/macOS/Windows | same style URL | [V] |
| qt | built-in style | Linux with Qt installed | Slint | Exists; needs Qt at build time; Slint says Qt can't be default because of the Qt dependency; deprecation direction | https://github.com/slint-ui/slint/discussions/11206 | [V] |
| native (alias) | built-in style | per-platform alias | Slint | 1.5: cupertino on macOS, fluent on Windows, material on Android, qt on Linux. Still selectable ("set to native keeps platform-adaptive behaviour" per blog); alias mapping in 1.19 not re-checked | https://releases.slint.dev/1.5.1/docs/slint/src/advanced/style | [U] |
| Material Components 1.1 (`ui-libraries/material`) | component lib | Slint ≥1.18; Android, touch, desktop | Slint (official) | Lives inside slint-ui/slint monorepo; README: "official Material Design 3 component set", v1.1 needs Slint 1.18+; APK + WASM demos | https://github.com/slint-ui/slint/tree/master/ui-libraries/material | [V] |
| slint-ui/material-components (old repo) | component lib (legacy) | — | Slint | **ARCHIVED**, last push 2025-09-22, ~50★; moved into slint monorepo. Do not use | https://github.com/slint-ui/material-components | [V] |
| material-rust-template | starter template | Rust + Material components | Slint (official) | ~24★, pushed 2026-11-16; also -cpp/-python/-nodejs variants (8/2/2★) | https://github.com/slint-ui/material-rust-template | [V] |
| slint-rust-template | starter template | Rust desktop | Slint (official) | MIT, ~341★, pushed 2026-11-16; no GitHub releases (clone/zip). README has no Android or cargo-generate mention (grep empty) | https://github.com/slint-ui/slint-rust-template | [V] |
| slint-cpp-template / slint-python-template / slint-nodejs-template | starter template | C++ (CMake) / Python / Node | Slint (official) | ~80 / ~10 / ~35★, all pushed 2026-11-16 | https://github.com/slint-ui/slint-cpp-template | [V] |
| slint-mcu-rust-template | starter template | MCU/embedded (not relevant) | Slint | ~40★, pushed 2026-11-16 | https://github.com/slint-ui/slint-mcu-rust-template | [V] |
| Android build guide (cargo-apk2 / cargo-apk / cargo-ndk+Gradle / xbuild; C++ CMake+Gradle) | starter template (docs) | Rust or C++ on Android | Slint | No official Android Rust template repo found (slint-android-tests repo: 404). Docs: min API 26, NativeActivity only, NDK r30+ for prebuilt Skia, entry point `slint_main`; page is "unreleased/next" docs | https://docs.slint.dev/latest/docs/slint/guide/platforms/mobile/android/ | [V] |
| Figma to Slint plugin (`tools/figma-inspector`, plugin id slint.figma.plugin) | design tool | Figma | Slint (official) | Since 1.10; exports Figma variables (color/number/string/bool) to .slint structs; in-repo version 1.20.0 (dev); open PR #13287 proposes replacing inspector with offline preview + project export | https://docs.slint.dev/latest/docs/slint/guide/tooling/figma-inspector/ | [V] |
| Slint LSP + VS Code ext + live-preview; editors for Zed/Kate/Sublime/tree-sitter | design tool | any editor | Slint | `tools/lsp`, `editors/{vscode,zed,kate,sublime,tree-sitter-slint}` present in repo; vscode ext version 1.20.0 (dev) | https://github.com/slint-ui/slint/tree/master/editors | [V] |
| SlintPad (`tools/slintpad`) + slint-viewer | design tool | browser / CLI | Slint | Directory exists in repo; web page https://slintpad.com not fetched; docs page 404 on my URL guess | https://github.com/slint-ui/slint/tree/master/tools/slintpad | [U] |
| Slint Royalty-Free License 2.0 | licence | proprietary desktop/mobile/web | SixtyFPS GmbH | Current licence option #2 | https://slint.dev/terms-and-conditions | [V] |
| Slint Software License 3.0 (paid: Enterprise / Small Enterprise / Startup & Individual) | licence | proprietary incl. embedded | SixtyFPS GmbH | Small Ent.: ≤50 employees & ≤€10M; Startup/Indiv.: ≤10 employees, ≤€2M, <5 yrs; prices not on pricing page | https://slint.dev/pricing | [V] |
| GPL-3.0 | licence | open-source apps | FSF / Slint | Free, copyleft; embedded OK under GPLv3 | https://slint.dev/pricing | [V] |
| SurrealismUI | community kit | Slint Rust | single maintainer (Surrealism-All) | ~388★, last push 2024-12-29 → **stale** | https://github.com/Surrealism-All/SurrealismUI | [V] |
| oxide-ui / slint-themekit / kitka / rd-slint-ui-kit / xui | community kit | Slint | single maintainers | 0–1★ each, pushed 2026-03…11 → **hobby, single-maintainer, not recommended** | https://github.com/william-franco/oxide-ui | [V] |
| slint-dotnet (microhobby) | community binding | C# | single maintainer | ~50★, last push 2024-08-11 → **stale** | https://github.com/microhobby/slint-dotnet | [V] |
| Platform support: Android | platform | Rust/C++ | Slint | Backend crate `i-slint-backend-android-activity` releases track Slint (1.18.1 on 2026-09-21); crate docs still say "work in progress" (probably stale text); enable `backend-android-activity-06` on `slint`; internal crate, pin exact version | https://lib.rs/crates/i-slint-backend-android-activity | [V] |
| Platform support: macOS / Windows | platform | winit backend + femtovg/skia/software renderers | Slint | Desktop is primary target; Servo-with-Slint Windows update posted 2026-05-07; no 2026 macOS/Windows-specific regressions found in post titles (release notes not read) | https://slint.dev/blog | [U] |

## Style facts (a)
- Styles in docs "latest": fluent, material, cupertino, cosmic, qt (+ `-light`/`-dark` variants for fluent/material/cupertino/cosmic; qt has none). `native` is no longer documented as a style in latest docs but the 2026-03 blog says setting it keeps platform-adaptive behaviour [U on exact 1.19 behaviour].
- Default: **fluent on all platforms** (latest docs and blog; older docs 1.13.1 said native). Styles follow system dark/light unless a `-light`/`-dark` variant is chosen. Select via `SLINT_STYLE` env or `CompilerConfiguration::with_style` (as build.rs does); `Platform.style-name` readable in .slint.
- Direction: non-Fluent styles get fewer updates and will be gradually deprecated; Slint plans easier custom styles. Material 3 is now the official in-repo component library (ui-libraries/material), a separate thing from the built-in `material` style.

## Licence summary
Official Royalty-Free License 2.0 wording (slint.dev/terms-and-conditions): grants "a world-wide, royalty-free, non-exclusive license to use, reproduce, make available, modify" the Software in "Desktop, Mobile, or Web Applications"; condition: "Display the AboutSlint widget in an 'About' screen or dialog" (reachable from top-level menu, else on splash screen) or "Display the Slint attribution badge on a public webpage"; restrictions: "does not permit the use of the Software within Embedded Systems", "does not permit to distribute or make the Software publicly available alone", and "does not permit the distribution of Application that exposes the APIs, in part or in total, of the Software"; no registration mentioned. Pricing page: "Build proprietary applications excluding Embedded Systems." Embedded needs a paid plan (Slint Software License 3.0, per-device royalty "starting at $1.00") or GPLv3. So proprietary Android + macOS + Windows apps are free under Royalty-Free 2.0 with AboutSlint/badge attribution; payment only for embedded devices, or if you want no attribution/need the paid-plan terms (the pricing table's attribution row was ambiguous, verify before relying). Caveat: wording above came via WebFetch summarisation of the page, not raw HTML; read the licence text itself before shipping. Note the repo's own LICENSE.md says the framework is "triple-licensed"; docs/examples are MIT.

## Top picks for Android+macOS+Windows
1. **Built-in `fluent` style (already used)** — default everywhere, actively developed, dark/light auto; consistent across 3 platforms. [V]
2. **Official Material Components 1.1 (slint monorepo `ui-libraries/material`)** — M3 set aimed at Android/touch; requires Slint ≥1.18 (project is 1.18.1); APK demo available. Use for the mobile UI if Fluent feels desktop-ish. [V]
3. **Android guide: cargo-ndk + Gradle (or cargo-apk2)** — only documented path; plan NDK r30+, API ≥26, NativeActivity only. [V]
4. **Figma to Slint plugin (variable export → .slint tokens)** + **Slint VS Code extension / LSP live-preview** — official design-to-code route. [V]
5. **Royalty-Free License 2.0** — free proprietary use on these 3 platforms; just add AboutSlint. [V]

Avoid: archived slint-ui/material-components repo, SurrealismUI (stale), single-maintainer community kits.

## Uncertain
- Exact 1.19 behaviour of `native` alias; whether cupertino/qt deprecation has a removal date.
- SlintPad page/URL not fetched; macOS/Windows platform notes from titles only (no release-note read).
- Licence quotes via WebFetch summary; attribution-optional row on pricing page ambiguous; pricing amounts not found.
- No official Android starter-template repo found; Android docs page is flagged "next/unreleased".
