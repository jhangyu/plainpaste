# Templates & design-system resources: .NET (Avalonia/Uno) and React Native desktop

Data pulled 2026-11-16 via GitHub API (stars rounded, "contrib" = contributor count incl. anon) and web search.
Tags: [V] = verified this session (repo/API/docs), [U] = unverified or inferred (no official statement found).

## .NET

| Name | Category | Pairs with | Maintainer | Status 2025-26 | URL |
|---|---|---|---|---|---|
| Avalonia core (12.x) | UI SDK | Win/macOS/Linux/iOS/Android/WASM [V]; Android requires .NET 10 | AvaloniaUI org (company), ~750 contrib | 12.1.6 (2026-11-09), pushed 11-16, ~31.8k★ | https://github.com/AvaloniaUI/Avalonia [V] |
| Avalonia.Templates (`dotnet new install Avalonia.Templates`: avalonia.app / mvvm / xplat) | Official template | Avalonia 12; `avalonia.xplat` = Desktop+Mobile+Web; `avalonia.mvvm` is desktop-only tag | AvaloniaUI org, ~52 contrib | 12.1.2 (2026-09-02), pushed 11-09, ~674★ | https://github.com/AvaloniaUI/Avalonia.Templates [V] |
| Avalonia.Samples | Official samples/starters (MVVM, CommunityToolkit.Mvvm) | Avalonia | AvaloniaUI org | pushed 2026-11-16, ~1.2k★ | https://github.com/AvaloniaUI/Avalonia.Samples [V] |
| FluentAvalonia (FluentAvaloniaUI) | Design system (WinUI/Fluent controls) | Avalonia; a 12.1.6 app on v3.1.0 verified on Win/macOS/Android in a third-party PR | **Single person** (amwx, user account), ~51 contrib | No GitHub release; last push 2026-08-22, ~1.6k★ — slowing | https://github.com/amwx/FluentAvalonia [V]; PR evidence https://github.com/timschneeb/GalaxyBudsClient/pull/773 [U] |
| Semi.Avalonia | Design system (Semi Design) | Avalonia 12 | irihitech org, ~24 contrib | v12.1.0.1 (2026-07-31), pushed 11-16, ~2.0k★ | https://github.com/irihitech/Semi.Avalonia [V] |
| Ursa.Avalonia | Control library (extends Semi) | Avalonia 12 + Semi.Avalonia | irihitech org, ~33 contrib | v2.3.1 (2026-11-06), ~1.6k★ | https://github.com/irihitech/Ursa.Avalonia [V] |
| SukiUI | Design system (styled desktop UI) | Avalonia; desktop focus | Mostly one lead (kikipoulet), ~51 contrib | 7.0.1 (2026-05-19), pushed 11-14, ~2.7k★ | https://github.com/kikipoulet/SukiUI [V] |
| Material.Avalonia | Design system (Material 3) | Avalonia | AvaloniaCommunity org, ~43 contrib | v3.21.0 (2026-09-25), pushed 10-30, ~1.2k★ | https://github.com/AvaloniaCommunity/Material.Avalonia [V] |
| Dock (wieslawsoltes) | Docking layout lib | Avalonia (desktop-oriented) | **Single person**, repo owner | pushed 2026-11-06, ~1.5k★ | https://github.com/wieslawsoltes/Dock [V] |
| ReactiveUI | MVVM framework | Avalonia via ReactiveUI.Avalonia (default in `avalonia.mvvm`-style starters is CommunityToolkit.Mvvm) | ReactiveUI org, multi-maintainer | 27.0.1 (2026-11-16), ~8.6k★ | https://github.com/reactiveui/ReactiveUI [V] |
| CommunityToolkit.Mvvm (CommunityToolkit/dotnet) | MVVM source generators | Avalonia + Uno (Uno's `-presentation mvvm` uses it) | Microsoft + community | v8.4.2 (2026-03-25) | https://github.com/CommunityToolkit/dotnet [V] |
| Uno Platform core (Uno.Sdk 6+/7) | UI SDK | Android, iOS, WASM, Skia Desktop (Windows, **macOS**, Linux), WinAppSDK [V]; Skia default renderer | Uno Platform (company), ~367 contrib | 7.0.65 (2026-11-13), ~10.1k★ | https://github.com/unoplatform/uno [V] |
| Uno.Templates (`dotnet new install Uno.Templates`; `dotnet new unoapp -preset recommended -presentation mvvm -theme material -toolkit`; VS/Rider wizard) | Official template | Uno; presets for Material/Fluent/Cupertino, MVVM/MVUX, XAML/C# Markup | Uno Platform company | 7.0.22 (2026-11-13), pushed 11-16, ~29★ (stars low, CLI-delivered) | https://github.com/unoplatform/uno.templates [V]; docs https://platform.uno/docs/articles/getting-started/wizard/using-wizard.html [V] |
| Uno.Themes (Uno.Material / Uno.Cupertino) | Design system | Uno | Uno Platform company, ~70 contrib | 9.0.1 (2026-11-11), ~186★ | https://github.com/unoplatform/Uno.Themes [V] |
| Uno.Toolkit.UI | Control/helper library | Uno (+ Material/Cupertino) | Uno Platform company, ~49 contrib | 11.0.1 (2026-11-11), ~109★ | https://github.com/unoplatform/uno.toolkit.ui [V] |
| Uno Material Toolkit for Figma + "Uno Platform for Figma" plugin | Figma design kit + design-to-XAML/C# Markup export | Uno + Material 3 (must design with kit components) | Uno Platform company | Active; plugin RTM'd (blog) — release date not checked | https://platform.uno/uno-material-toolkit-for-figma/ [V]; https://www.figma.com/community/file/1110792522046146058/uno-platform-material-toolkit [V] |
| Uno.Extensions | App scaffolding libs (DI, navigation, config, MVUX) | Uno | Uno Platform company, ~101★ | 8.0.1 (2026-11-11) | https://github.com/unoplatform/uno.extensions [V] |
| Uno.Samples | Official samples/starters | Uno | Uno Platform company | pushed 2026-11-16, ~265★ | https://github.com/unoplatform/Uno.Samples [V] |

## React Native desktop

| Name | Category | Pairs with | Maintainer | Status 2025-26 | URL |
|---|---|---|---|---|---|
| react-native-windows | Platform target | RN core; Windows | Microsoft, ~244 contrib | 0.85.0 (2026-10-20), ~17.4k★ | https://github.com/microsoft/react-native-windows [V] |
| react-native-macos | Platform target | RN core; macOS | Microsoft (fork of facebook/react-native; contributor count inflated by upstream history) | 0.83.0 (2026-09-30), ~4.4k★ — version trails RNW | https://github.com/microsoft/react-native-macos [V] |
| react-native-test-app | App scaffold for all targets (Android/iOS/macOS/Windows) | RN + RNW + RN-macOS | Microsoft | 5.4.17 (2026-11-04), ~676★ | https://github.com/microsoft/react-native-test-app [V] |
| FluentUI React Native | Design system | Windows, macOS, iOS, Android (README platform table) | Microsoft | pushed 2026-11-16, ~1.4k★ | https://github.com/microsoft/fluentui-react-native [V] |
| Expo (SDK + templates: blank / default / tabs / bare-minimum) | Starter templates | Android/iOS/web; **no official Windows/macOS target** (no desktop template in expo/templates) | Expo (company) | expo-template-tabs 58.0.34 (2026-11-12), ~53.6k★ | https://github.com/expo/expo [V]; desktop support [U] (community RNW/RN-macOS in Expo not verified) |
| Ignite (Infinite Red) | Starter boilerplate | RN/Expo; Android/iOS; no desktop target documented | Infinite Red (company), ~245 contrib | v11.5.0 (2026-03-18), pushed 11-11, ~20k★ | https://github.com/infinitered/ignite [V]; desktop [U] |
| Tamagui | UI SDK + starters | Web/iOS/Android; no official Windows/macOS statement found in docs/issues searched | Mostly one lead (Nate Wienert) + ~256 contrib | v3.3.3 (2026-11-13), ~14.3k★ | https://github.com/tamagui/tamagui [V]; desktop [U] |
| gluestack-ui | UI SDK (copy-paste components, NativeWind-based) | Web/iOS/Android; no desktop statement found | GeekyAnts (company), ~92 contrib | v5.0.0 (2026-06-25), pushed 09-02, ~5.4k★ | https://github.com/gluestack/gluestack-ui [V]; desktop [U] |
| React Native Paper | UI SDK (Material) | Android/iOS/web; no desktop statement found | Callstack (company), ~387 contrib | v5.15.4 (2026-11-02), ~14.5k★ | https://github.com/callstack/react-native-paper [V]; desktop [U] |
| NativeWind | Styling (Tailwind for RN) | v4 had `windows:`/`macos:` variants; v5 dropped them (issue #1758, closed 2026-09-14, outcome not read) | ~75 contrib, small core team | 4.2.7 (2026-09-14), ~8.1k★ | https://github.com/nativewind/nativewind/issues/1758 [V]; user reports NativeWind failing on RNW+macOS: https://github.com/nativewind/nativewind/issues/1386 [V] |

## Top picks for Android + macOS + Windows

1. **Avalonia 12 + Avalonia.Templates (`avalonia.xplat`) + Semi.Avalonia/Ursa** — one codebase on all three; org-backed, active (12.1.6, 2026-11). Gap: Android needs .NET 10; Semi/Ursa mobile polish not verified [U]; official `avalonia.mvvm` template is desktop only.
2. **Uno Platform (`dotnet new unoapp`, Material + Toolkit + MVVM) + Uno Material Toolkit Figma + Figma plugin** — only stack with first-party design-to-code; Android + Skia Desktop (Win/macOS) + WinAppSDK. Gap: Skia desktop on macOS is not native AppKit controls; Figma plugin requires designing inside the Material kit (Material 3 look only).
3. **FluentAvalonia on Avalonia** — best WinUI look; verified once on Win/macOS/Android with Avalonia 12.1.6. Gap: single maintainer, no release in GitHub, last push 2026-08 → treat as risk.
4. **react-native-windows + react-native-macos + FluentUI React Native** (via react-native-test-app) — the only RN combo with official Windows+macOS+Android UI kit. Gaps: macOS version (0.83) lags Windows (0.85); two separate Microsoft forks to keep aligned; no Expo desktop path.
5. RN with Tamagui / gluestack / Paper / NativeWind: **not recommended for desktop** — no official Windows/macOS support found; NativeWind v5 dropped desktop variants.

Uncertain: Uno Figma plugin release date; whether NativeWind #1758 was resolved as "support" or "won't"; exact FluentAvalonia 12 compatibility statement (only third-party PR); Tamagui/Paper/gluestack desktop status inferred from absence of evidence; Avalonia mobile support of Semi/Ursa/SukiUI.
