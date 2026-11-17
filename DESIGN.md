# Claude Code Switcher — Design System & Product Specification (v2.0)

## 1. Product Vision & Re-Architecture

**Claude Code Switcher** is an environment, gateway, and model routing switchboard for **Claude Code** (`~/.claude/settings.json`).

### Core Product Shifts (v1 → v2)

| Dimension | Legacy v0.1 (`egui` JSON Notepad) | Redesigned v2.0 ("Editorial Switchboard") |
| :--- | :--- | :--- |
| **Mental Model** | Raw file list (`新配置_1.json`) + text box | Provider-aware Profiles with presets, metadata, model routing & live status |
| **Sync Safety** | Destructive `fs::write` overwriting entire `~/.claude/settings.json` | **Smart Merge (Default)** preserves `hooks`, `permissions`, `enabledPlugins`, `statusLine` while cleanly swapping provider `env` & `model`; optional **Full Replace** |
| **Drift Awareness** | Blind overwrite on startup/switch | Real-time **Live Sync / Drift Detection** against `~/.claude/settings.json` with 1-click import or re-apply |
| **Authoring** | Manual raw JSON typing with broken scroll gutter | **Tri-Mode Studio**: Visual Form + Synchronized JSON Code Editor + Live Effective Diff |
| **Verification** | None | Built-in **Endpoint Reachability & Latency Probe** (`ms` + TLS/HTTP status) |
| **Safety Net** | None | **Automatic Time-Machine Snapshots** (`~/.claude-code-switcher/backups/`) before every write |
| **Ergonomics** | Mouse-only `[R]` / `[-]` / `[*]` bracket buttons | `⌘K` Command Palette, `⌘↵` Activate, `⌘S` Save, `⌘N` Preset Wizard, CLI `--use <profile>` |

---

## 2. Visual Direction: Editorial Warm Precision

Inspired by **Anthropic's warm editorial humanism** paired with **Braun / Dieter Rams tactile instrument clarity**:

- **Surface Philosophy**: Warm mineral neutrals instead of cold blue-gray SaaS defaults or purple-gradient AI clichés.
  - **Obsidian Studio (Dark)**: Warm carbon-ink canvas (`#131311`), graphite rail (`#1A1917`), elevated stone cards (`#21201D`), warm bone ink (`#F2EFE9`).
  - **Warm Parchment (Light)**: Archival paper canvas (`#FAF8F4`), warm sand rail (`#F1EEE7`), crisp ivory surface (`#FFFFFF`), deep walnut-charcoal ink (`#181715`).
- **Primary Accent**: **Claude Terracotta** (`#C96442` in Light / `#D97757` in Dark) — reserved strictly for primary actions, active focus rings, and brand identity.
- **Instrument Signal Indicators**:
  - **Live / Synced**: Phosphor Emerald (`#238656` Light / `#3ECF8E` Dark)
  - **Drifted / Unsaved**: Ochre Amber (`#B76E12` Light / `#E6A23C` Dark)
  - **Error / Destructive**: Vermilion (`#C43B33` Light / `#F06A63` Dark)
- **Elevation Discipline**: Single-declaration depth — crisp `1px` hairlines (`--border-subtle`) on operational panes; soft offset shadow (`0 12px 32px rgba(0,0,0,0.24)`) reserved exclusively for floating overlays (`⌘K` palette, modals, toasts).

---

## 3. Three-Layer Token Architecture

All UI styles resolve strictly through `ui/styles/tokens.css` (`Primitive → Semantic → Component`):

### Layer 1: Primitives
- **Color**: `--stone-950`..`--stone-50`, `--clay-600`..`--clay-300`, `--emerald-600`..`--emerald-400`, `--amber-600`..`--amber-400`, `--crimson-600`..`--crimson-400`
- **Spacing**: `4px` base unit (`--space-1: 4px` through `--space-10: 40px`)
- **Radius**: `--radius-xs: 5px`, `--radius-sm: 7px`, `--radius-md: 10px`, `--radius-lg: 14px`, `--radius-pill: 999px`
- **Typography**:
  - `--font-sans`: `-apple-system, BlinkMacSystemFont, "SF Pro Text", "PingFang SC", "Hiragino Sans GB", "Microsoft YaHei UI", sans-serif`
  - `--font-serif`: `"Iowan Old Style", "Newsreader", "Georgia", "Songti SC", serif`
  - `--font-mono`: `"SF Mono", "JetBrains Mono", "Menlo", "Consolas", ui-monospace, monospace`

### Layer 2: Semantics (Theme-Adaptive)
- Surfaces: `--bg-canvas`, `--bg-rail`, `--bg-surface`, `--bg-elevated`, `--bg-inset`, `--bg-hover`, `--bg-selected`
- Borders: `--border-subtle`, `--border-default`, `--border-strong`, `--border-focus`
- Text: `--text-primary` (≥12:1), `--text-secondary` (≥6:1, warm-tinted), `--text-tertiary` (≥4.5:1), `--text-on-accent`
- Signals: `--accent`, `--accent-hover`, `--accent-soft`, `--signal-live`, `--signal-live-soft`, `--signal-warn`, `--signal-warn-soft`, `--signal-danger`, `--signal-danger-soft`

### Layer 3: Components
- `--rail-width: 284px`, `--titlebar-height: 46px`, `--statusbar-height: 30px`
- `--control-height-sm: 28px`, `--control-height-md: 34px`, `--control-height-lg: 38px`
- `--ease-out-expo: cubic-bezier(0.16, 1, 0.3, 1)`

---

## 4. Iconography & Asset System

- **Zero Emoji Policy**: Every control, provider badge, empty state, and status indicator uses authored `1.5px`-stroke vector SVGs (`ui/scripts/icons.js`).
- **App Icon (`resources/icons/icon.svg`)**: macOS squircle geometry (`1024×1024`) with warm obsidian housing, brushed dark mineral inner bezel, and interlocking terracotta/ivory switchboard rails forming code chevrons.
