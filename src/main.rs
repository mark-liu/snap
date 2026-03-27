use snap::proxy;

use std::process;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.is_empty() || args[0] == "--help" || args[0] == "-h" {
        eprintln!("snap — MCP stdio proxy that compresses Playwright accessibility snapshots");
        eprintln!();
        eprintln!("Usage: snap <command> [args...]");
        eprintln!();
        eprintln!("Wraps an MCP server, compressing YAML accessibility tree snapshots");
        eprintln!("in tool results before they enter the LLM context window.");
        eprintln!();
        eprintln!("Example:");
        eprintln!("  snap npx -y @playwright/mcp@latest --cdp-endpoint http://localhost:9222");
        eprintln!();
        eprintln!("Claude Code config:");
        eprintln!(r#"  "command": "snap","#);
        eprintln!(r#"  "args": ["npx", "-y", "@playwright/mcp@latest", "--cdp-endpoint", "http://localhost:9222"]"#);
        process::exit(0);
    }

    if args[0] == "--version" || args[0] == "-V" {
        eprintln!("snap {}", env!("CARGO_PKG_VERSION"));
        process::exit(0);
    }

    match proxy::run(&args) {
        Ok(code) => {
            print_stats();
            process::exit(code);
        }
        Err(e) => {
            eprintln!("snap: error: {}", e);
            print_stats();
            process::exit(1);
        }
    }
}

fn print_stats() {
    let total = proxy::MESSAGES_TOTAL.load(std::sync::atomic::Ordering::Relaxed);
    let compressed = proxy::MESSAGES_COMPRESSED.load(std::sync::atomic::Ordering::Relaxed);
    let input = proxy::TOTAL_INPUT.load(std::sync::atomic::Ordering::Relaxed);
    let output = proxy::TOTAL_OUTPUT.load(std::sync::atomic::Ordering::Relaxed);

    if compressed > 0 {
        let saved = input.saturating_sub(output);
        let pct = if input > 0 {
            (saved as f64 / input as f64) * 100.0
        } else {
            0.0
        };
        eprintln!(
            "snap: {} snapshots compressed ({}/{}), {}KB → {}KB ({:.0}% reduction)",
            compressed,
            compressed,
            total,
            input / 1024,
            output / 1024,
            pct,
        );
    }
}
