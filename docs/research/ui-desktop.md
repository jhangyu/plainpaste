# Desktop & native-toolkit UI SDKs (checked via web search, as of 2026-11-16)

Status labels: [V] = verified in a search result this session; [U] = unverified / sources conflict.

| Name | Maintainer/Origin | Type | Typical stack | Notes (status 2025-26, distinctive trait) |
|---|---|---|---|---|
| Electron | OpenJS Foundation (ex-GitHub) | Shell/runtime (bundles Chromium) | JS/TS + React/Vue/Svelte; pairs with shadcn/ui, MUI, Ant Design, Radix | [V] 45.0.0 on 2026-10-21; new major every ~8 weeks, supports latest 3 majors. Biggest ecosystem, largest bundle. One blog claiming "v33/34" is outdated. |
| Tauri 2 | Tauri Programme (CrabNebula etc.) | Shell/runtime (system webview + Rust) | Rust core + any web UI (React/Svelte/Solid) + shadcn/ui/Tailwind | [V] v2 stable since 2024-10; 2.12 on 2026-09-26 (dropped official Win7). Small bundles; webview differs per OS. |
| Wails | Lea Anthony / community | Shell/runtime (Go + system webview) | Go backend + web UI | [V] v3.0.0 stable 2026-10-26 (~4 yrs after first alpha); GitHub lists v3.1.0 "Latest". v2->v3 is a real port. |
| WinUI 3 / Windows App SDK | Microsoft | Toolkit + design system (Fluent 2) | C#/C++ + XAML | [V] Windows App SDK 2.6.5 stable (2026-11-02); 2.0 line GA 2026-04-29. Official Windows-only native UI; WinUI Gallery 3.0 Oct 2026. |
| WPF UI (lepoco) | lepoco / community | Theme/control lib (Fluent, Win11 look) | C# WPF, MVVM | [V] ~9.8k stars; GitHub topic page shows last update 2026-06-27 (oldest of WPF libs; [U] release cadence). Mica/acrylic, Fluent icons. |
| MaterialDesignInXamlToolkit | Community (MaterialDesignInXAML org) | Theme/control lib (Material Design) | C# WPF | [V] repo updated 2026-11-01; active. Most popular Material theme for WPF. |
| HandyControl | handyorg / community | Control lib | C# WPF | [V] repo updated 2026-10-15. Large control set, common in Chinese-speaking dev community. |
| MahApps.Metro | .NET Foundation project | Theme/control lib (Metro) | C# WPF | [V] repo updated 2026-11-02; supports .NET FW 4.6.2+, .NET 6/8. Long-lived; v3 release lag noted by one older source [U]. |
| ModernWpf (Kinnara) | Community (Yimeng Wu) | Theme lib (WinUI-style) | C# WPF | [U] README says 1.x is the active preview line using official Fluent theme on .NET 10; search metadata suggests older activity, verify commit history. Predecessor of iNKORE fork. |
| iNKORE.UI.WPF.Modern | iNKORE Studios (Yoojun Zhou) | Theme/control lib (Fluent 2, fork of ModernWpf) | C# WPF | [V] Fluent 2 styles, Mica/acrylic; docs site marked unfinished. |
| Avalonia UI | AvaloniaUI OÜ + community | Toolkit (own renderer, Skia) | C# + XAML; Win/macOS/Linux/mobile/WASM | [V] 12.1.4 latest (2026-10-09); 12.0 shipped April 2026. Pixel-identical rendering across OSes; main WPF successor. |
| FluentAvalonia | amwx / community | Theme lib (WinUI/Fluent port) | Avalonia | [V] repo last updated 2026-08-22; no release seen in late-April roundup; AppWindow v3 PR merged then reverted [U] Avalonia 12 readiness. |
| Semi.Avalonia | irihitech | Theme lib (Semi Design, from ByteDance Semi) | Avalonia | [V] 12.1.0.1 packages 2026-07-31; tracks Avalonia 12 and 11.3 lines. Design tokens reused by Ursa. |
| Ursa.Avalonia | irihitech | Control lib (extends Semi) | Avalonia | [V] 2.0.0 preview series on Avalonia 12 RC; repo updated 2026-11-03. Adds UrsaWindow/TitleBar, Ursa.Themes.Semi. |
| SukiUI | kikipoulet | Theme/control lib (distinct modern look) | Avalonia | [V] README announces 6.0; [U] release date and Avalonia 12 compatibility not confirmed. |
| Material.Avalonia | AvaloniaCommunity | Theme lib (Material Design) | Avalonia | [U] not verified this session (no 2026 data found). |
| Uno Platform | nventive -> Uno Platform Inc. | Toolkit (WinUI XAML API, Skia renderer) | C# + WinUI XAML; Win/macOS/Linux/mobile/WASM | [V] 6.6 on 2026-07-29 (Native AOT, opt-in Vulkan); Wikipedia lists 6.6.184 (2026-08-06). Microsoft partnership on SkiaSharp/.NET mobile repos. |
| .NET MAUI | Microsoft | Toolkit (native controls) | C# + MAUI XAML; Win/macOS(Catalyst)/mobile | [U] one source: MAUI 11 preview May 2026, GA planned Nov 2026; no Linux support (per Uno article, biased). |
| Qt (Widgets / Quick Controls) | The Qt Company / KDE | Toolkit | C++/QML, PySide/PyQt | [V] 6.11 released 2026-03-23 (StyleKit tech preview: single Style object for all controls); 6.10 added high-contrast styles, SearchField; Wikipedia claims 6.12 on 2026-09-30 [U]. Built-in styles: Basic, Fusion, Material, Universal, macOS, Windows 11, FluentWinUI3. |
| GTK 4 + libadwaita | GNOME Project | Toolkit + design system (GNOME HIG/Adwaita) | C/Rust/Python/Vala | [V] GNOME 50 (Mar 2026): GTK 4.22, libadwaita 1.9; GNOME 51 released 2026-09-16 (libadwaita 1.10 [U] notes not fetched); GTK 4.24.1 listed 2026-10-01. Adaptive widgets, dark-mode via style.css media queries. |
| KDE Kirigami | KDE Community | Toolkit/convergent UI framework (on Qt Quick) | C++/QML | [V] ships with KDE Frameworks 6 monthly (6.32.0 on 2026-11-13, requires Qt 6.10); Plasma 6.6 LTS. KDE HIG, desktop+mobile. |
| AppKit / SwiftUI (macOS) | Apple | Native toolkit + design system (Liquid Glass) | Swift/Obj-C | [V] WWDC26: macOS 27 "Golden Gate" revamps Liquid Glass; AppKit gets interactive glass, concentricity API; SwiftUI auto-adopts. |
| Compose Multiplatform (Desktop) | JetBrains | Toolkit (Skia, Material 3 default) | Kotlin | [V] 1.12.0 on 2026-08-25 (v2 window/dialog API experimental), 1.12.1 on 09-22; desktop & iOS called production ready. |
| JetBrains Jewel | JetBrains | Design system/theme lib (IntelliJ New UI) | Kotlin Compose Desktop | [V] release notes to v0.40 (2026-08-11); website lists 0.42.0 [U]. IntelliJ-platform-bound; not multiplatform by design. |
| JavaFX | OpenJFX / Gluon (Oracle origin) | Toolkit | Java/Kotlin + CSS | [V] JavaFX 26 (notes rev 2026-07-13) with built-in headless platform; Wikipedia lists 27 on 2026-09-15 [U]. |
| AtlantaFX | mkpaz | Theme lib (Primer/Nord/Cupertino/Dracula themes) | JavaFX | [V] v3.0.0 released 2026-09-24 (CSD decoration styles). Most used JavaFX theme pack. |
| MaterialFX | Alessadro / community | Control lib (Material) | JavaFX | [U] no 2026 release info found; treat as possibly stale. |
| Slint | SixtyFPS GmbH (Slint) | Toolkit (declarative .slint DSL, embedded-friendly) | Rust/C++/JS/Python | [V/U] 1.x; one source says 1.18 Oct 2026, a benchmark pinned 1.16.1 [U]. Dual licensed (GPL/royalty-free/commercial). |
| egui | Emil Ernerfeldt (Rerun.io) | Immediate-mode toolkit | Rust | [V] 0.37 as of Oct 2026; wgpu default since 0.34; still 0.x, breaking changes each release. |
| Iced | Héctor Ramón / System76 backing | Toolkit (Elm-style) | Rust | [V] 0.14 latest; basis of COSMIC desktop (1.2.0 on 2026-07-01). |
| Dioxus | DioxusLabs | Toolkit (React-like, webview or native Blitz) | Rust | [V] 0.7, pre-1.0; experimental native rendering via Blitz, expected improved in 0.8. |
| gpui / gpui-component | Zed Industries / community | Toolkit (GPU-rendered) | Rust | [V] versioning tied to Zed tags; no built-in text input; community "GPUI Component" library listed among top picks (2026-08). |
| Freya | Community (marc2332) | Toolkit (Skia, Dioxus-derived) | Rust | [V] survey says it split from Dioxus into its own GUI model; [U] version not captured. |
| Makepad | Makepad / Project Robius | Toolkit (shader-styled, live-design DSL) | Rust | [V] Widgets 1.0.0 ran on D3D11 in a benchmark. |
| Fyne | Fyne.io / Fyne Labs | Toolkit (Material-inspired look) | Go | [V] v2.8.0 on 2026-07-13 (1,000+ commits, GPU shapes, richer text). |
| Gio | Gio authors (Elias Naur) | Immediate-mode toolkit | Go | [V] v0.10.2 (Aug 2026 newsletter); v0.10.0 May 2026. Pre-1.0. |
| Dear ImGui | Omar Cornut | Immediate-mode toolkit (tooling/debug UIs) | C++ (bindings everywhere) | [V/U] GitHub releases page lists v1.93.0 as latest; an older tracker showed 1.92.6. Dev tools/game editors, not end-user app UI. |
| Flutter (desktop) | Google | Toolkit (Impeller/Skia; Material 3 & Cupertino) | Dart | [V] 3.47.0 on 2026-08-12 per Wikipedia; a blog claims Impeller default on desktop and 3.50 planned Nov 2026 [U]. |
| Delphi FireMonkey (FMX) | Embarcadero | Toolkit (GPU, Skia4Delphi) | Object Pascal | [V] RAD Studio 13.2 Florence on 2026-09-17; FMX Style Designer updated; no separate "FMX 13" product. Still active, niche/enterprise. |

## Notable status changes
- Wails v3 went stable (2026-10-26) after ~4 years of alpha/beta.
- Windows App SDK 2.0 line GA'd 2026-04-29 (now 2.6.5).
- Avalonia 12 shipped (April 2026); Semi/Ursa follow; FluentAvalonia lagging.
- Qt 6.11 adds StyleKit (tech preview).
- macOS 27 reworks Liquid Glass.
- Uno 6.6: Native AOT + Vulkan; Microsoft partnership.

## Uncertain
MAUI 11 GA timing; Qt 6.12 existence; libadwaita 1.10 specifics; SukiUI/Material.Avalonia/MaterialFX/ModernWpf/Freya currency; Slint exact version; Jewel 0.42 vs notes at 0.40; JavaFX 27; Dear ImGui current version; Flutter 3.50.
