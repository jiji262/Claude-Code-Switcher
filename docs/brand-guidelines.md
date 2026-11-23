# Brand Guidelines — Claude Code Switcher

## Brand Essence
- **Name**: Claude Code Switcher
- **Role**: Precision Environment & Provider Switchboard for Claude Code
- **Voice & Tone**: Calm, technical, direct, trustworthy. Controls state exact outcomes ("切换并激活", "智能合并 · 保留 Hooks 与权限") rather than vague labels ("设为默认").

## Color Palette
| Token | Obsidian Dark | Warm Parchment Light | Usage |
| :--- | :--- | :--- | :--- |
| `--bg-canvas` | `#131311` | `#FAF8F4` | Primary workspace canvas |
| `--bg-rail` | `#1A1917` | `#F1EEE7` | Left profile switchboard rail |
| `--bg-surface` | `#21201D` | `#FFFFFF` | Cards, form groups, code panes |
| `--text-primary` | `#F2EFE9` | `#181715` | Primary copy & headings |
| `--text-secondary` | `#B2ACA0` | `#57534A` | Labels, metadata, descriptions |
| `--accent` | `#D97757` | `#C65D3B` | Primary CTAs, active switch, focus rings |
| `--signal-live` | `#3ECF8E` | `#1F845A` | Live `~/.claude/settings.json` sync indicator |
| `--signal-warn` | `#E6A23C` | `#B56A10` | External drift / unsaved edits |
| `--signal-danger` | `#F06A63` | `#C43B33` | Destructive actions & invalid JSON |

## Typography & Numerals
- **Headings**: Optical tracking `-0.02em`, weight `600`, balanced wrapping.
- **Metrics & Latencies**: `font-variant-numeric: tabular-nums` mandatory on latency (`ms`), backup timestamps, line numbers, and character/key counts.
- **Secrets**: Masked by default (`sk-ant-••••••9a2f`) in monospace with explicit reveal/copy controls.
