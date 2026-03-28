# snap

MCP stdio proxy that compresses Playwright accessibility snapshots before they hit the LLM context window.

## why

Every Playwright MCP tool call returns the full page accessibility tree as YAML — 50-80KB per call. Navigation chrome, engagement buttons, cursor annotations, bare images. The LLM doesn't need any of it. On a typical browsing session that's 18MB/week of noise.

snap sits between Claude Code and the Playwright MCP server, strips the noise, keeps the content. ~1ms overhead.

## numbers

```
X.com tweet page:  67KB → 41KB  (40% reduction)
X.com article:     65KB → 60KB  (8% — mostly content, correct)
latency:           ~1ms per call
binary:            454KB
deps:              serde_json only
```

## what gets stripped

- nav sidebars, banners, account menus (full subtree removal)
- engagement buttons (Reply/Repost/Like/Bookmark/Share groups)
- Grok, Subscribe, Share, More buttons
- reply compose areas
- `[cursor=pointer]`, `[active]` annotations
- bare `img` with no alt text
- internal `/url: /path` nav links
- console errors/warnings, `[unchanged]` markers

all text content, element refs, labeled links, images with alt text, main regions, and form elements are preserved.

## install

```bash
cargo install --path .
```

## usage

wrap your Playwright MCP server:

```bash
snap npx -y @playwright/mcp@latest --cdp-endpoint http://localhost:9222
```

### claude code

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

## how it works

```
Claude Code ←stdio→ snap ←stdio→ @playwright/mcp
                      │
              parses JSON-RPC on stdout
              finds ```yaml snapshot blocks
              strips structural noise
              forwards compressed result
```

stdin and stderr pass through unchanged. non-YAML messages pass through unchanged. on exit, prints compression stats to stderr.

## related projects

- **[mcpguard](https://github.com/mark-liu/mcpguard)** — MCP stdio proxy for prompt injection scanning + payload compression. Same proxy architecture, but for security rather than token savings. Use mcpguard for Discord/Telegram/any server returning user-generated content.
- **[webguard-mcp](https://github.com/mark-liu/webguard-mcp)** — prompt injection scanning for web fetches. Different input (HTTP pages vs MCP tool results), same threat model.

## license

MIT
