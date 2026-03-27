/// Integration test: compress real Playwright snapshots captured from X.com
use std::fs;

// Import from the crate
use snap::compress::{compress_snapshot, compress_markdown_yaml};

#[test]
fn test_real_tweet_snapshot() {
    let path = "/tmp/tweet-snapshot.md";
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => {
            eprintln!("Skipping: {} not found (run tweet-ingestion first)", path);
            return;
        }
    };

    let result = compress_snapshot(&content);
    let pct = ((result.input_bytes - result.output_bytes) as f64 / result.input_bytes as f64) * 100.0;

    eprintln!(
        "tweet-snapshot: {}KB → {}KB ({:.0}% reduction)",
        result.input_bytes / 1024,
        result.output_bytes / 1024,
        pct,
    );

    // Should achieve at least 10% reduction on real X.com pages
    assert!(pct > 10.0, "Expected >10% reduction, got {:.1}%", pct);
    // Should not destroy content
    assert!(result.output.contains("witcheer"));
}

#[test]
fn test_real_article_snapshot() {
    let path = "/tmp/article-snapshot.md";
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => {
            eprintln!("Skipping: {} not found (run tweet-ingestion first)", path);
            return;
        }
    };

    let result = compress_snapshot(&content);
    let pct = ((result.input_bytes - result.output_bytes) as f64 / result.input_bytes as f64) * 100.0;

    eprintln!(
        "article-snapshot: {}KB → {}KB ({:.0}% reduction)",
        result.input_bytes / 1024,
        result.output_bytes / 1024,
        pct,
    );

    // Article page has less chrome, but still some savings
    assert!(pct > 5.0, "Expected >5% reduction, got {:.1}%", pct);
    // Must preserve article content
    assert!(result.output.contains("Living With an AI agent"));
    assert!(result.output.contains("Mac Mini"));
}

#[test]
fn test_real_jsonrpc_message() {
    let path = "/tmp/tweet-snapshot.md";
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => {
            eprintln!("Skipping: {} not found", path);
            return;
        }
    };

    // Simulate a real MCP JSON-RPC message
    let markdown = format!("### Snapshot\n```yaml\n{}\n```\n", content);
    let msg = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "result": {
            "content": [
                {"type": "text", "text": markdown}
            ]
        }
    });

    let input = serde_json::to_string(&msg).unwrap();
    let input_len = input.len();

    // Extract text, compress, rebuild
    let (compressed_text, saved) = compress_markdown_yaml(&markdown);
    assert!(saved > 0, "Should compress something");

    let output_msg = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "result": {
            "content": [
                {"type": "text", "text": compressed_text}
            ]
        }
    });
    let output = serde_json::to_string(&output_msg).unwrap();
    let output_len = output.len();

    let pct = ((input_len - output_len) as f64 / input_len as f64) * 100.0;
    eprintln!(
        "JSON-RPC message: {}KB → {}KB ({:.0}% reduction)",
        input_len / 1024,
        output_len / 1024,
        pct,
    );

    // Verify the output is still valid JSON-RPC
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert!(parsed.get("result").is_some());
    assert!(parsed.get("jsonrpc").unwrap().as_str() == Some("2.0"));
}
