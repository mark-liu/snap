use snap::compress::{compress_snapshot, compress_markdown_yaml};
use std::fs;
use std::time::Instant;

fn main() {
    for (name, path) in [("tweet", "/tmp/tweet-snapshot.md"), ("article", "/tmp/article-snapshot.md")] {
        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => { eprintln!("skip {}: {}", path, e); continue; }
        };

        // Benchmark raw YAML compression
        let start = Instant::now();
        let iterations = 1000;
        let mut result = None;
        for _ in 0..iterations {
            result = Some(compress_snapshot(&content));
        }
        let elapsed = start.elapsed();
        let r = result.unwrap();

        let pct = ((r.input_bytes - r.output_bytes) as f64 / r.input_bytes as f64) * 100.0;
        
        eprintln!(
            "{}: {}KB -> {}KB ({:.1}% reduction, {:.0}KB saved) | {:.2}ms/call ({} iterations in {:.0}ms)",
            name,
            r.input_bytes / 1024,
            r.output_bytes / 1024,
            pct,
            (r.input_bytes - r.output_bytes) as f64 / 1024.0,
            elapsed.as_micros() as f64 / iterations as f64 / 1000.0,
            iterations,
            elapsed.as_millis(),
        );

        // Also test JSON-RPC wrapping
        let md = format!("### Snapshot\n```yaml\n{}\n```\n", content);
        let start2 = Instant::now();
        for _ in 0..iterations {
            let _ = compress_markdown_yaml(&md);
        }
        let elapsed2 = start2.elapsed();
        eprintln!(
            "  json-rpc path: {:.2}ms/call",
            elapsed2.as_micros() as f64 / iterations as f64 / 1000.0,
        );
    }
}
