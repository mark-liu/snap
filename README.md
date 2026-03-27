# snap

MCP stdio proxy that compresses Playwright accessibility snapshots before they enter the LLM context window.

## Problem

Every Playwright MCP tool call (`browser_click`, `browser_navigate`, `browser_snapshot`, etc.) returns the full page accessibility tree as YAML — typically 50-80KB per call. This fills the LLM context window fast, especially on content-heavy sites.

## Solution

`snap` wraps the Playwright MCP server as a transparent stdio proxy. It intercepts JSON-RPC responses, finds YAML snapshot blocks, and strips structural noise while preserving all content the LLM needs for reasoning and interaction.

**What it strips:**
- Navigation sidebars (Home, Explore, Notifications, etc.)
- Banner/header regions
- Account menus
- Engagement button groups (Reply, Repost, Like, Bookmark, Share)
- Grok/Subscribe/Share buttons
- Reply compose areas
- `[cursor=pointer]`, `[active]` annotations
- Bare `img` elements with no alt text
- Internal relative URL lines (`/url: /path`)
- Console error/warning log lines
- `[unchanged]` markers from incremental mode

**What it preserves:**
- All text content
- Element refs (needed for clicking)
- Links with text labels
- Images with alt text
- `main` content regions
- Form elements

## Performance

| Metric | Value |
|--------|-------|
| Latency overhead | ~1ms per tool call |
| Tweet page (X.com) | 67KB → 41KB (40% reduction) |
| Article page (X.com focus) | 65KB → 60KB (8% reduction) |
| Binary size | 454KB |
| Dependencies | `serde_json` only |

## Installation

```bash
cargo install --path .
# or
cargo build --release && cp target/release/snap ~/.cargo/bin/
```

## Usage

Wrap your Playwright MCP server command:

```bash
snap npx -y @playwright/mcp@latest --cdp-endpoint http://localhost:9222
```

### Claude Code config

In `~/.claude.json`, change your Playwright MCP entry:

```json
{
  "mcpServers": {
    "playwright": {
      "command": "snap",
      "args": ["npx", "-y", "@playwright/mcp@latest", "--cdp-endpoint", "http://localhost:9222"]
    }
  }
}
```

## How it works

1. Spawns the wrapped MCP server as a child process
2. Pipes stdin through unchanged (Claude Code → MCP server)
3. Intercepts stdout (MCP server → Claude Code):
   - Parses each line as JSON-RPC
   - For tool results containing ` ```yaml` blocks, applies compression
   - Forwards compressed result
4. Pipes stderr through unchanged
5. On exit, prints compression stats to stderr

## Architecture

```
Claude Code ←stdio→ snap ←stdio→ @playwright/mcp
                      │
              JSON-RPC interception
              YAML snapshot compression
              ~1ms overhead per call
```

Zero-copy passthrough for non-YAML messages. No network layer, no SQLite, no caching — just fast string processing on the stdio pipe.

## License

MIT
