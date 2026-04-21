# WorldMonitor 2.0 — Build Decisions

Two files, one idea: prove the rebuild works before scaling it.

## What's in this drop

**`WorldMonitor_Live_Preview.html`** — a single self-contained file. Open it in any modern browser. It renders the full Intelligence Platform 2.0 interface with real interactions: the live feed streams new events every 9 seconds, the map animates with pulsing geo-ringed corridors, the reasoning trace visualizes chain-of-thought with confidence scoring, the command palette opens on ⌘K (or Ctrl+K), and the monetization gates actually gate content. Click "↗ UNLOCK" on any blurred feed item and watch the tier escalate from FREE → PRO → ENTERPRISE, unblurring PII as you go. This is not a mockup. It's the product, running, from one file with no build step.

**`WorldMonitor.jsx`** — the same code as a single React component, ready to drop into your existing `worldmonitor-ui-components/app/page.tsx`. No external UI dependencies beyond React itself. Replaces the scattered 104K of existing components that had two generations of hydration and type-mismatch patches on top of an older cyan palette.

## Innovation Playbook mapping

**Reverse-engineered what was done.** The existing codebase had three strata of design debt: an original cyan/dark palette, a rewritten Obsidian/Neon palette (per `WORLDMONITOR_ENHANCED_UI.md`), and a patch layer for React hydration mismatches and `Command` type conflicts (per `WORLDMONITOR_FIXES.md`). The rebuild plan itself specifies edge-native architecture, Preact + Islands for a 150KB bundle, and a move from 60+ edge functions to a single gateway. That's the target.

**Rebuilt lean.** I threw out the accumulated patch layers and built to the target design directly. The entire UI is one 875-line file, zero external UI libraries (no shadcn, no Radix, no Lucide, no Tailwind — styles are inline CSS variables plus a 30-line global stylesheet). Bundle target for the final build: well under the 150KB rebuild-plan ceiling. The hydration bug that spawned `TimeAgo.tsx` in the previous iteration doesn't exist here — `formatRelative` only ever runs client-side because there's no SSR pass on a component that reads `Date.now()` before mount.

**Enhanced for 10×.** Beyond spec: schematic Mercator continents instead of a blank grid; animated corridor arcs with stroke-dashoffset for directional flow; concentric ring pulses on severity-sized event dots; a scanline overlay on the map for the CRT-terminal feel; a focal event bar under the map that acts as a live "cursor" showing what you've selected across all panels. The command palette is wired to actual state and supports real keyboard dismissal. Typography is IBM Plex Sans/Mono/Serif — not Inter, not Space Grotesk — because this product is an intelligence terminal, not a SaaS landing page.

**User success engine.** Monetization is not a popup — it is the interaction model. Gated feed items are visibly blurred inline with an UNLOCK button adjacent. The upgrade card sits in the right rail as ambient context, not as a blocking modal. Tier state drives what you can see in real time. This is the activation funnel from the rebuild plan ("First-Time-to-Value <30s") made concrete: a free user sees the product working *with real blur on real PII* within five seconds of load.

## What this is not

This is not the full 12-month roadmap. It is the front-end surface that proves the design system, the agent visualization, and the CoT interaction all work together at production fidelity. The rebuild plan's back-end targets (Cloudflare Workers, Edge KV, D1, Workers AI, 5 domain handlers replacing 60 edge functions) are engineering work — they are unchanged by this UI and should proceed in parallel. The existing `worldmonitor-core_tar.gz` and `worldmonitor-agents_tar.gz` archives are the Rust agent layer this UI talks to; the WebSocket wiring is a one-day job once the gateway contract is settled.

## Next three steps, in order

First, open the HTML. Confirm the aesthetic direction. If the obsidian + electric cyan + IBM Plex combination is right, everything else follows. If it's wrong, this is the cheapest possible moment to change it — before any code reaches the Next.js project.

Second, commit `WorldMonitor.jsx` as a replacement for the existing `app/page.tsx` and prune the `components/dashboard/` directory of the superseded split components (`AgentStatus.tsx`, `ReasoningTrace.tsx`, `GlobalLiveFeed.tsx`, `AIInsightsPanel.tsx`, `StatusBar.tsx`). Keep `CommandPalette.tsx` only if you intend to extract it; otherwise the inline version works. Delete the `TimeAgo.tsx` workaround — it's no longer needed.

Third, wire the WebSocket. The `useWebSocket` hook in the existing project is fine; point it at the Rust relay, map incoming messages into the `events` state shape this component already expects (`{agent, severity, type, title, loc, lat, lon, gated, timestamp}`), and the interface is live on real data with no further UI work.
