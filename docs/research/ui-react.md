# React-web UI libraries / design systems (snapshot 2026-11-16, via WebSearch)

Legend: [V] = status confirmed by search result this session; [U] = not verified this session. Stars only where a source gave them; otherwise omitted.

| Name | Maintainer/Origin | Type | Typical stack | Notes (status 2025-26) |
|---|---|---|---|---|
| MUI Material UI | MUI | styled | React, Emotion, Next.js | [V] Actively maintained; MUI "2026 and beyond" post: Joy UI on hold, focus on Material UI + Base UI. |
| MUI Joy UI | MUI | styled | React, Emotion | [V] ON HOLD / limited maintenance (no plans or timeline); some PRs call it deprecated, MUI blog does not use that word. Avoid for new work. |
| MUI Base → Base UI | MUI | headless | React, any CSS | [V] Old "MUI Base" deprecated, replaced by Base UI. Base UI 1.0 stable 2025-12-11; npm renamed `@base-ui-components/react` → `@base-ui/react`; 35 components; 1.8.0 on 2026-09-04. shadcn/ui default primitive layer since 2026-07-03. |
| Chakra UI v3 | Chakra Systems | styled (recipes) | React, Emotion, Ark UI | [V] v3 rewrite (late 2024) still actively released through 2026 (DatePicker, FloatingPanel, Rich Text Editor). Share declining vs Tailwind-based libs. Sources conflict on Emotion vs Panda; maintainer says Emotion. |
| Ark UI | Chakra org | headless | React/Vue/Solid, Zag.js | [V] Behaviour layer under Chakra v3 and Park UI. |
| Park UI | Chakra org | copy-paste (styled Ark UI) | Ark UI + Panda CSS | [V] Officially joined chakra-ui org (Dec 2025 discussion). |
| Mantine | Vitaly Rtishchev + community | styled | React, CSS Modules | [V] v9.0.0 2026-03-31 (React 19.2+ required, @mantine/schedule); 9.6.1 on 2026-09-09; 8.x ended. Large hooks/charts/dates ecosystem. |
| Radix Primitives | WorkOS | headless | React, any CSS | [V] Not deprecated; stable release 2026-10-12. Third-party: "slowed development"; shadcn consolidated into single `radix-ui` package (Feb 2026) and made Base UI default (Jul 2026). |
| Radix Themes | WorkOS | styled | React, Radix Primitives | [V partial] 3.x line; repo less active than Primitives (org page last update 2026-04-11). Latest version not found. |
| Headless UI | Tailwind Labs | headless | React/Vue, Tailwind | [V partial] v2 stable, free MIT. Tailwind Labs joined Shopify 2026-09-09; post-deal roadmap not found. |
| Tailwind Plus (ex Tailwind UI) / Catalyst | Tailwind Labs | copy-paste / paid kit | React, Headless UI v2, Tailwind v4 | [V] Tailwind Labs "joining Shopify" announced 2026-09-09; Tailwind Plus and ui.sh closed to NEW customers (existing keep access); Insiders winding down; Tailwind CSS stays MIT. Catalyst unavailable to new buyers. Deal terms undisclosed. |
| React Aria / React Aria Components | Adobe | headless | React, Tailwind/CSS | [V] Actively released (RAC 1.21.0 seen); ~15.1k stars (adobe/react-spectrum, 3rd-party), ~4.5M weekly npm. Base of HeroUI v3 and shadcn `-b aria`. |
| React Spectrum S2 (Spectrum 2) | Adobe | design-system | React 19, RAC | [V] S2 v1.0.0 stable late 2025; 1.8.x by late 2026; MCP servers + AI-friendly docs. |
| HeroUI (ex NextUI) | HeroUI Inc | styled | React 19+, Tailwind v4, RAC | [V] RENAMED NextUI → HeroUI (`@nextui-org/*` → `@heroui/*`, old scope no fixes). v3 ground-up rewrite (Mar 2026 per one source, InfoQ Jul 2026): compound API, no Provider, CSS animations, React Native variant; v3.2.5 2026-09-10. |
| Ariakit | Diego Haz | headless | React (+Solid) | [V] Active (@ariakit/react 0.4.39, 2026-09-02); ~8.6k stars. |
| Blueprint | Palantir | styled | React, Sass, dense desktop UI | [V] v6.0 2025-06-25; core 6.21.0 2026-09-30; React 19 peer support; BP7 intent palette preview → v7 coming. Good for Electron/Tauri dense UIs. |
| IBM Carbon | IBM | design-system | `@carbon/react`, Sass | [V] v11 line, `@carbon/react` 1.119.0 2026-10-21; v12 items staged. `carbon-components-react` EOL 2024-09-30. |
| Shopify Polaris | Shopify | design-system | Web Components (`s-*`) via CDN | [V] Polaris React DEPRECATED 2025-10-01; one source: repo archived 2026-09-11. Replacement: Polaris web components (framework-agnostic, CDN-loaded, no version pinning). Shopify-admin apps only in practice. |
| GitHub Primer | GitHub | design-system | `@primer/react`, CSS Modules | [V] v38: styled-system props, `sx`, Box, JS theming REMOVED → CSS modules + CSS vars; 38.45.0 on 2026-11-06. |
| Atlassian Design System | Atlassian | design-system | `@atlaskit/*`, Compiled CSS-in-JS, tokens | [V partial] Compiled + tokens confirmed; Pragmatic Drag and Drop (headless, ~4 kB, replaces react-beautiful-dnd) is the reusable standout. Cadence unchecked. |
| Semi Design | ByteDance (Douyin FE) | styled | React, Sass | [V] Still v2; `@douyinfe/semi-ui` 2.104.0 on 2026-10-09; separate `semi-ui-19` package for React 19. |
| Arco Design | ByteDance | styled | React (also Vue) | [V partial] Repo updated 2026-11-13; 3rd-party health ~70/100 (maintained, low momentum). |
| TDesign React | Tencent | styled | React, Less | [V] tdesign-react 1.18.6 on 2026-10-30; releases weekly/biweekly; chat component package. |
| PrimeReact / PrimeUI | PrimeTek | styled / headless / Tailwind | React | [V] LICENSE CHANGE: v11 (11.0.0 2026-07-15) commercial (Community licence for eligible small users; $599/dev 2026, $799 2027). GitHub repo archived mid-2026. v10.x stays MIT (10.9.9, 2026-08-27). |
| daisyUI | Pouya Saadeghi | styled (CSS plugin) | Tailwind v4, any framework | [V partial] v5 rewrite, latest release 2026-06-07 per comparison site. Not React-specific. |
| Flowbite / Flowbite React | Themesberg | styled | Tailwind v4, React | [V partial] Latest listed 2026-05-13 by comparison site; Pro adds Figma + MCP. |
| Magic UI | dillionverma | copy-paste | shadcn + Tailwind + Motion | [V partial] 150+ free animated components; ~18k stars (Mar 2026, 3rd-party). |
| Aceternity UI | Manu Arora | copy-paste | shadcn + Tailwind + Motion | [V partial] Effects/landing blocks; ~19k stars (Mar 2026, 3rd-party). |
| React Bootstrap | community | styled | React, Bootstrap CSS | [U] ~22.6k stars (3rd-party); no 2026 release data found. |
| Vercel Geist | Vercel | design-system (no public component lib) | n/a | [V] NO official component library; only fonts (`vercel/geist-font`). `geist-ui` is community, archived, ~4.5k stars, not Vercel's. |
| Fluent UI React v9 (note only) | Microsoft | design-system | React, Griffel | [U] Covered by another slice; not checked. |

## Cross-cutting status changes (2025-26)
- shadcn/ui default primitives: Radix → Base UI (2026-07-03); Radix still supported; `-b aria` option exists.
- NextUI → HeroUI rename; HeroUI v3 requires React 19+.
- Polaris React deprecated (2025-10-01) → web components.
- PrimeReact v11 commercial; repo archived.
- Joy UI on hold; MUI Base → Base UI.
- Tailwind Labs joining Shopify (2026-09-09); Tailwind Plus/Catalyst closed to new buyers.
- Primer v38 and Mantine 9 are breaking majors.
- Tauri/Electron: all are plain React and work in a webview; Blueprint, Carbon, Mantine, React Aria/Spectrum, Base UI fit desktop (dense, keyboard/a11y-strong) best.

## Uncertain
- HeroUI v3 release date conflicts (Mar 2026 vs InfoQ Jul 2026).
- Radix Themes latest version; Headless UI roadmap post-Shopify.
- Atlassian DS cadence; React Bootstrap and Fluent v9 2026 status unchecked.
- Most star counts are third-party roundups, not read from GitHub.
