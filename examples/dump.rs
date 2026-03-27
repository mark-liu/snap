use snap::compress::compress_snapshot;
use std::fs;

fn main() {
    let content = fs::read_to_string("/tmp/tweet-snapshot.md").unwrap();
    let result = compress_snapshot(&content);
    // Print first 100 lines to see what remains
    for (i, line) in result.output.lines().enumerate() {
        if i >= 100 { break; }
        println!("{:4} {}", i+1, line);
    }
    eprintln!("---");
    eprintln!("Total: {} lines", result.output.lines().count());
    eprintln!("{}KB -> {}KB", result.input_bytes/1024, result.output_bytes/1024);
}
