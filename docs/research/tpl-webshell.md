# Web-tech shells (Tauri 2 / Electron + React UI SDKs) — templates & design resources

Verified 2026-11-16 via `gh api` (stars/pushed/contributors) and WebSearch. [V] = status confirmed by GitHub API this session; [U] = from WebSearch/vendor pages only, not API-confirmed.

| Name | Category | Pairs with | Maintainer | Status 2025-26 | URL |
|---|---|---|---|---|---|
| create-tauri-app | starter template (official; React/Preact/Svelte/Vue/Solid etc. flavors, no UI-kit/mobile-specific flavor) | Tauri 2 + React | tauri-apps org | js-v4.7.5 2026-10-09; pushed 2026-11-15; ~1.7k★ [V] | https://github.com/tauri-apps/create-tauri-app |
| Tauri core (v2) | framework (desktop + Android/iOS) | any web UI | tauri-apps org | v2.12.6 2026-11-16; ~113k★ [V] | https://github.com/tauri-apps/tauri |
| tauri-action | CI build/release (Win/macOS/Linux, mobile build docs) | Tauri 2 | tauri-apps org | pushed 2026-11-14; ~1.6k★ [V] | https://github.com/tauri-apps/tauri-action |
| agmmnn/tauri-ui (create-tauri-ui) | starter template, desktop only | Tauri 2 + shadcn/ui + Next/Vite + Tailwind | single person (4 contributors) | create-tauri-ui@1.2.1 2026-10-31; ~2.3k★ [V] — desktop only, no Android | https://github.com/agmmnn/tauri-ui |
| dannysmith/tauri-template | starter template, desktop | Tauri 2 + React + shadcn/Tailwind | single person (2 contributors) | pushed 2026-10-22; ~317★ [V] — single-maintainer | https://github.com/dannysmith/tauri-template |
| MrLightful/create-tauri-react | starter template, desktop | Tauri 2 + React + shadcn/Tailwind | single person (2 contributors) | pushed 2026-11-02; ~242★ [V] — single-maintainer | https://github.com/MrLightful/create-tauri-react |
| kitlib/tauri-app-template | starter template, desktop | Tauri 2 + React 19 + shadcn/ui | single person (1 contributor) | pushed 2026-03-20; ~101★ [V] — single-maintainer, small | https://github.com/kitlib/tauri-app-template |
| mcgeq/tauri-app-template | starter template, desktop + Android | Tauri 2 + React 19 + shadcn/ui | single person | pushed 2026-08-07; ~0★ [V] — Android-capable but unproven; flag as unvetted | https://github.com/mcgeq/tauri-app-template |
| Roboticela Vite-ExpressJS-Tauri-Template-DevKit | starter template, Win/macOS/Linux/Android CI (apk/aab) | Tauri 2 + React 19 + Vite | small org | pushed 2026-04-26; ~2★ [V] — unproven; useful only as CI reference | https://github.com/Roboticela/Vite-ExpressJS-Tauri-Template-DevKit |
| electron-vite | starter tooling (Electron + Vite; templates for React/Vue/Svelte) | Electron + React | single owner alex8088 (15 contributors) | pushed 2026-10-14; ~5.6k★ [V] | https://github.com/alex8088/electron-vite |
| electron-vite-react | starter template | Electron + Vite + React | org electron-vite (25 contributors) | pushed 2026-09-01; ~2.4k★ [V] | https://github.com/electron-vite/electron-vite-react |
| Electron Forge | packaging/starter toolchain (Vite+TS templates) | Electron (macOS+Windows only; no Android) | electron org | v8.0.1 2026-09-29; ~7.2k★ [V] | https://github.com/electron/forge |
| shadcn/ui (CLI, blocks, official templates in registry) | component distribution + blocks | React + Tailwind + Radix or Base UI | org shadcn-ui; very many contributors | pushed 2026-11-13; ~128k★ [V] | https://github.com/shadcn-ui/ui |
| shadcn/ui blocks (ui.shadcn.com/blocks) | blocks (dashboard, sidebar, login) | shadcn/ui | shadcn-ui org | live site; part of shadcn-ui/ui [U] | https://ui.shadcn.com/blocks |
| Kiranism/next-shadcn-dashboard-starter | blocks / admin template (Next.js; not a Tauri shell — copy components only) | shadcn/ui + Tailwind | single owner (13 contributors) | pushed 2026-11-15; ~7.3k★ [V] | https://github.com/Kiranism/next-shadcn-dashboard-starter |
| shadcnstore/shadcn-dashboard-landing-template | blocks / dashboard template (Vite/Next variants) | shadcn/ui + Tailwind | org (2 contributors) | pushed 2026-10-28; ~1.3k★ [V] | https://github.com/shadcnstore/shadcn-dashboard-landing-template |
| coss ui (cosscom/ui) | component collection / blocks | Base UI + Tailwind (shadcn-style) | org | pushed 2026-11-14; ~10.9k★ [V] | https://github.com/cosscom/ui |
| magicui | animated blocks | shadcn/ui + Tailwind | org | pushed 2026-11-13; ~23k★ [V] | https://github.com/magicuidesign/magicui |
| awesome-shadcn-ui | curated index of blocks/templates | shadcn/ui | single owner, community PRs | pushed 2026-11-16; ~20.8k★ [V] | https://github.com/birobirobiro/awesome-shadcn-ui |
| Base UI | headless design system / primitives | React (shadcn Base UI style, MUI) | org MUI (100+ contributors) | v1.9.0 2026-10-09; pushed 2026-11-16; ~11.4k★ [V] | https://github.com/mui/base-ui |
| Radix Primitives | headless primitives | React | org radix-ui (WorkOS) | pushed 2026-10-14; ~19.5k★ [V] — slower cadence than Base UI | https://github.com/radix-ui/primitives |
| Radix Themes | styled design system | React | radix-ui org | pushed 2026-04-11; ~8.9k★ [V] — stale-ish | https://github.com/radix-ui/themes |
| Radix Colors | theme tool (color scales, a11y steps) | Radix/Tailwind/any | radix-ui org | pushed 2025-12-17; ~1.7k★ [V] — stable, low churn | https://github.com/radix-ui/colors |
| React Aria / React Spectrum (S2) | headless + styled design system | React | org Adobe (many contributors) | react-aria-components 1.22.0 2026-10-09; pushed 2026-11-16; ~16k★ [V] | https://github.com/adobe/react-spectrum |
| Mantine | design system (components, hooks) | React; has no Tauri-specific bits | org mantinedev (100+ contributors) | 9.8.1 2026-11-15; ~32k★ [V] | https://github.com/mantinedev/mantine |
| mantinedev/vite-template | starter template (Vite, official) | Mantine + Vite (base for Tauri frontend) | mantinedev org | pushed 2026-11-15; ~239★ [V] | https://github.com/mantinedev/vite-template |
| ui.mantine.dev | blocks | Mantine | mantinedev org | pushed 2026-11-01; ~3.9k★ [V] | https://github.com/mantinedev/ui.mantine.dev |
| HeroUI | design system | React + Tailwind | org heroui-inc (many contributors) | v3.2.7 2026-10-28; ~31k★ [V] | https://github.com/heroui-inc/heroui |
| heroui-inc/next-app-template | starter template (Next.js only) | HeroUI | org | pushed 2026-10-31; ~392★ [V] | https://github.com/heroui-inc/next-app-template |
| tweakcn | theme tool (visual shadcn/Tailwind theme editor) | shadcn/ui + Tailwind v4 | single owner jnsahaj (30 contributors) | pushed 2026-09-03; ~10.5k★ [V] | https://github.com/jnsahaj/tweakcn |
| Obra shadcn/ui Figma Kit (Community Edition, free) | Figma kit | shadcn/ui | studio (Obra Studio); listed on ui.shadcn.com/docs/figma | v2.0 Aug 2026; 50k+ duplicates [U] | https://shadcn.obra.studio/ |
| shadcndesign.com kit (Matt Wierzbicki) | Figma kit (paid; 2,000+ components) | shadcn/ui | single person; listed on shadcn docs | update Oct 2026 [U] | https://www.shadcndesign.com/ |
| Pretine (Ravn) | Figma kit (community) | Mantine | agency Ravn; no official Mantine kit | status unknown [U] | https://github.com/orgs/mantinedev/discussions/5053 |
| Figma SDS (Simple Design System) | Figma kit + code | React Aria Components | Figma org | pushed 2026-10-07; ~875★ [V] | https://github.com/figma/sds |
| Base UI Figma (Community v0.1) | Figma kit (community, unstyled) | Base UI | community; ~415 days old at search time | stale-ish [U] | https://www.figma.com/community/file/1553396461135994278/base-ui-v0-1-community |
| Untitled UI React | design system + Figma kit (freemium) | React Aria + Tailwind | commercial vendor | synced with Figma kit [U] | https://www.untitledui.com/react |

## Top picks for Android + macOS + Windows
1. **Tauri 2 + create-tauri-app (React) + shadcn/ui** — only mainstream, org-maintained path covering Android/macOS/Windows from one codebase; every shadcn block works in the WebView.
2. **Base UI (or coss ui) under shadcn** — MUI-backed, actively released (v1.9.0), better touch/a11y story than aging Radix cadence.
3. **React Aria Components / Spectrum S2** — strongest touch/keyboard/a11y behavior, Adobe-backed, for adaptive Android-vs-desktop UI.
4. **Mantine (+ official vite-template, ui.mantine.dev blocks)** — most complete batteries-included kit; active; no official Figma.
5. **tweakcn + Obra shadcn Figma kit** — theme and design-handoff pair; Obra is the free kit listed on shadcn docs.

Caveats (Tauri Android maturity, from web sources [U], not official docs): functional/production-usable but less mature than desktop and RN/Flutter; mobile plugin coverage behind desktop (desktop-only plugins like updater break Android builds if in capabilities; dialog plugin lacks folder pick and returns content URIs); needs Android Studio/JDK/NDK + Rust targets; min Android 8 (API 26); WebView needs manual native-feel tuning. 92 open issues labeled Android on tauri-apps/tauri [V]. No popular Tauri 2 + shadcn starter with Android found: all Android-capable ones are tiny single-maintainer repos (<5★) — scaffold with create-tauri-app and add shadcn manually. Tauri 3.0.0-alpha reported published 2026-11-06 [U].
