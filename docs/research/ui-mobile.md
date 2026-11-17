# Mobile & cross-platform mobile UI SDKs (checked via WebSearch, 2026-11-16)

Tags: [V] = verified online this session; [U] = not verified this session (background knowledge only).

| Name | Maintainer/Origin | Type | Typical stack | Notes (status 2025-26, distinctive trait) |
|---|---|---|---|---|
| Flutter Material widgets → `material_ui` package | Google / Flutter team | Component lib (Material 3) | Flutter/Dart | [V] Material & Cupertino being decoupled from the SDK into standalone pub.dev packages `material_ui` / `cupertino_ui` (tracking flutter/flutter#101479). Old `package:flutter/material.dart` imports still compile; one third-party source says formal deprecation is scheduled. |
| Flutter Cupertino → `cupertino_ui` package | Google / Flutter team | Component lib (iOS look) | Flutter/Dart | [V] Official Cupertino has NO iOS 26 Liquid Glass yet; team says all new iOS 26 work lands only in `cupertino_ui`; redesign expected late 2026 (issue #170310 + secondary source). |
| cupertino_native_better / liquid_glass_widgets / adaptive_platform_ui | Community (pub.dev) | Liquid Glass gap-fillers | Flutter/Dart | [V] Native iOS 26 glass via hybrid composition; shader-based glass; adaptive iOS26/Cupertino/Material switching. Third-party, maturity unknown. |
| forui | duobase.io | Design system (shadcn-style, minimal) | Flutter | [V] pub.dev shows 0.27.7 (pre-1.0; other snapshots show older numbers). ≥0.26 requires Flutter ≥3.47. |
| shadcn_ui (Flutter) | mariuti.com (community port) | Component lib (shadcn/ui port) | Flutter | [V] pub.dev 0.59.0, published ~6 days before snapshot; very active, pre-1.0. |
| macos_ui | macosui community | macOS-native look widgets | Flutter desktop | [V] Active; v2.2.0+1+ needs Flutter ≥3.35. Desktop, not mobile. Tahoe glass support unknown [U]. |
| fluent_ui (Flutter pkg) | bdlukaa (community) | Fluent-style widgets | Flutter (Windows/desktop) | [V partial] Described as actively developed by a comparison article; no version found. Not Microsoft-official [U]. |
| moon_design / GetWidget | Moon (Coingaming) / GetWidget | Flutter UI kits | Flutter | [U] No 2026 status found; check pub.dev before relying. |
| Jetpack Compose Material3 (+ M3 Expressive) | Google / AndroidX | Official Android design system | Kotlin, Compose | [V] Latest stable 1.4.0; 1.5.0-rc01 on 2026-11-04 → Expressive APIs promoted with 1.5.0 stable. Third-party sources conflict on which Expressive components are already stable. MDC-Android Views 1.14.0 = final Views release. Compose BOM 2026.08.00 = Compose 1.12.0. |
| Compose Multiplatform | JetBrains | Cross-platform UI (Android/iOS/desktop/web) | Kotlin / KMP | [V] iOS stable since 1.8.0 (May 2025); 1.12.1 on 2026-09-22, 1.13.0-alpha in progress; web (Wasm) still Beta as of Sep 2026. Material3 + MaterialExpressiveTheme available. |
| Compose Unstyled | Composable Horizons | Headless/unstyled primitives | Compose Multiplatform (6 targets) | [V] Renderless accessible components, no imposed visual design; DeepWiki lists 1.49.6 (may be stale). |
| SwiftUI + Liquid Glass (Apple HIG) | Apple | Platform-native design language + UI framework | Swift, SwiftUI/UIKit/AppKit | [V] Liquid Glass announced WWDC25 (iOS/macOS 26): `glassEffect(_:in:)`, `GlassEffectContainer`, `.buttonStyle(.glass)`. iOS 27 (WWDC26) refines: less default transparency, user clear/tinted slider. Sources conflict on exact opt-out removal timing (Xcode 27 SDK). |
| React Native Paper | Callstack | Material 3 component lib | React Native | [V] Stable 5.15.4; 6.0.0-alpha.0 (2026-06-15, TextInput rewrite). M3 Expressive not yet supported (issue #5150). |
| Tamagui | Community (Nate Weaver) | Universal UI kit + optimizing compiler | React Native + Web | [V partial] v2 reached after RC in early 2026; 2.7.x as of Aug 2026. Distinct: compile-time style extraction, shared RN/web. |
| gluestack-ui | GeekyAnts (successor to NativeBase) | Copy-paste components (shadcn-style) | React Native/Expo + NativeWind | [V] v4 stayed alpha; v5 alpha 2026-03-16, v5 stable ~Jun 2026 on NativeWind v5/Tailwind v4, Expo Router; Next.js support dropped. Also supports Uniwind (v4.1 alpha). |
| NativeWind / Uniwind | NativeWind (community) / Uniwind (community) | Tailwind styling layer for RN | React Native | [V partial] NativeWind v5 pre-release in early-2026 sources; no explicit stable announcement found. Uniwind claims full Tailwind v4 and ~2× speed (vendor claim). |
| React Native Reusables | Community | shadcn/ui port | React Native + NativeWind | [V partial] Young, thin primitives; NativeWind v5 migration status unknown. |
| React Native Elements (@rneui) | Community | Cross-platform UI kit | React Native | [V] 5.0 released Jan 2026 but sources say slow development; rarely chosen for new apps. |
| UI Kitten | Akveo | Eva Design System components | React Native | [V] Sources conflict: slowed activity vs still maintained. Low momentum. |
| NativeBase | GeekyAnts | Legacy component lib | React Native | [U] Superseded by gluestack-ui. |
| Expo UI (@expo/ui) | Expo | Native SwiftUI + Jetpack Compose from JS | Expo / React Native | [V] SDK 55: Compose beta, SwiftUI aligned; stable in SDK 56 with universal component layer plus platform-specific `swift-ui` / `jetpack-compose` packages. SDK 55 needed dev builds. |
| Ionic Framework + Capacitor | Ionic | Web-tech hybrid UI | Angular/React/Vue + Capacitor | [V] Both maintained & open source (Ionic 8; upgrade guides for Ionic 9). Commercial products ended Feb 2025: Appflow blocks new apps from 2026-10-01, access to 2027-12-31; Capawesome named migration partner. |
| Lynx | ByteDance | Cross-platform UI framework | ReactLynx + Rspeedy (Rspack) | [V] Open-sourced Mar 2025; 3.6 (Jan 2026), 3.7 (Apr), 3.8 (2026-06-30); lynx-ui announced 2026-05-22; roadmap targets 4.0 and monthly releases; Web + OpenHarmony support. Embedded-framework model. |
| ArkUI (HarmonyOS) | Huawei | Declarative native UI | ArkTS (TypeScript-based) | [V partial] Official native UI for HarmonyOS/OpenHarmony; OpenHarmony 6.1 released 2026-03-09. Huawei runs a KMP/CMP community beta for HarmonyOS; Tencent ovCompose brings Compose to ArkUI. No HarmonyOS 6 specifics found. |
| .NET MAUI built-in controls | Microsoft | Official cross-platform UI | C#/XAML | [V] MAUI 10 (with .NET 10 LTS) supported to 2027-05-11; MAUI 9 out of support since 2026-05-12. |
| CommunityToolkit.Maui | .NET Foundation community | Behaviors/controls toolkit | .NET MAUI | [V] .NET 10 apps should stay on the 15.x line; newest releases target .NET 11 only; Tizen dropped. |
| Syncfusion Toolkit for .NET MAUI | Syncfusion | Open-source controls (MIT, Microsoft-endorsed) | .NET MAUI | [V] Included in the .NET 9 and .NET 10 MAUI templates as optional sample content. |
| Telerik UI for .NET MAUI | Progress Telerik | Commercial controls | .NET MAUI | [V partial] Only confirmed as a listed vendor on Microsoft's MAUI page; no 2026 release info. |

## Notable status changes (2025-26)
- Flutter splitting Material/Cupertino out of the SDK; Cupertino Liquid Glass still pending.
- Compose Material3 1.5.0 (Expressive) in RC.
- Expo UI stable in SDK 56.
- gluestack-ui v5 stable (NativeWind v5); v4 never stable.
- Ionic Appflow wind-down; Ionic/Capacitor continue.
- Lynx moving to monthly releases, 4.0 expected.

## Uncertain
- NativeWind v5 stable status; React Native Reusables migration.
- moon_design, GetWidget, fluent_ui, macos_ui, NativeBase, Telerik: no 2026 status.
- Which Material Expressive Compose APIs are already stable (sources conflict).
- Exact timing of Liquid Glass opt-out removal (sources conflict).
- Maintainer attributions of "Community"/bare names are from memory, not checked.
