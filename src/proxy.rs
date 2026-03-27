/// MCP stdio proxy — bidirectional pipe between Claude Code and an MCP server,
/// with interception of tool results for snapshot compression.

use std::io::{self, BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, ChildStderr, Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

use crate::compress::compress_markdown_yaml;

/// Global compression stats.
pub static TOTAL_INPUT: AtomicUsize = AtomicUsize::new(0);
pub static TOTAL_OUTPUT: AtomicUsize = AtomicUsize::new(0);
pub static MESSAGES_COMPRESSED: AtomicUsize = AtomicUsize::new(0);
pub static MESSAGES_TOTAL: AtomicUsize = AtomicUsize::new(0);

/// Spawn the wrapped MCP server as a child process.
pub fn spawn_child(args: &[String]) -> io::Result<Child> {
    if args.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "no command to wrap",
        ));
    }

    Command::new(&args[0])
        .args(&args[1..])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
}

/// Pipe stdin → child stdin (passthrough, no modification).
pub fn pipe_stdin_to_child(mut child_stdin: ChildStdin) {
    let stdin = io::stdin();
    let reader = BufReader::new(stdin.lock());

    for line in reader.lines() {
        match line {
            Ok(line) => {
                if writeln!(child_stdin, "{}", line).is_err() {
                    break; // Child stdin closed
                }
                if child_stdin.flush().is_err() {
                    break;
                }
            }
            Err(_) => break,
        }
    }
}

/// Pipe child stdout → stdout, compressing snapshot YAML in tool results.
pub fn pipe_child_stdout(child_stdout: ChildStdout) {
    let reader = BufReader::new(child_stdout);
    let stdout = io::stdout();
    let mut out = stdout.lock();

    for line in reader.lines() {
        match line {
            Ok(line) => {
                let processed = process_message(&line);
                if writeln!(out, "{}", processed).is_err() {
                    break;
                }
                if out.flush().is_err() {
                    break;
                }
            }
            Err(_) => break,
        }
    }
}

/// Pipe child stderr → stderr (passthrough).
pub fn pipe_child_stderr(child_stderr: ChildStderr) {
    let reader = BufReader::new(child_stderr);
    let stderr = io::stderr();
    let mut err = stderr.lock();

    for line in reader.lines() {
        match line {
            Ok(line) => {
                let _ = writeln!(err, "{}", line);
            }
            Err(_) => break,
        }
    }
}

/// Process a single JSON-RPC message from the MCP server.
/// Compresses snapshot YAML in tool results; passes everything else through.
fn process_message(line: &str) -> String {
    MESSAGES_TOTAL.fetch_add(1, Ordering::Relaxed);

    // Fast path: not JSON? Pass through.
    let trimmed = line.trim();
    if !trimmed.starts_with('{') {
        return line.to_string();
    }

    // Try to parse as JSON
    let mut value: serde_json::Value = match serde_json::from_str(trimmed) {
        Ok(v) => v,
        Err(_) => return line.to_string(), // Not valid JSON, pass through
    };

    // Only process JSON-RPC results (responses from tool calls)
    if !value.get("result").is_some() {
        return line.to_string();
    }

    // Navigate to result.content array
    let content = match value
        .get_mut("result")
        .and_then(|r| r.get_mut("content"))
        .and_then(|c| c.as_array_mut())
    {
        Some(c) => c,
        None => return line.to_string(),
    };

    let mut any_compressed = false;

    // Process each text content block
    for item in content.iter_mut() {
        if item.get("type").and_then(|t| t.as_str()) != Some("text") {
            continue;
        }

        if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
            // Quick check: does this text contain YAML snapshots?
            if !text.contains("```yaml") {
                continue;
            }

            let input_len = text.len();
            let (compressed, saved) = compress_markdown_yaml(text);

            if saved > 0 {
                TOTAL_INPUT.fetch_add(input_len, Ordering::Relaxed);
                TOTAL_OUTPUT.fetch_add(compressed.len(), Ordering::Relaxed);
                item["text"] = serde_json::Value::String(compressed);
                any_compressed = true;
            }
        }
    }

    if any_compressed {
        MESSAGES_COMPRESSED.fetch_add(1, Ordering::Relaxed);
        // Re-serialize with compressed content
        match serde_json::to_string(&value) {
            Ok(s) => s,
            Err(_) => line.to_string(), // Fallback: return original
        }
    } else {
        line.to_string()
    }
}

/// Run the full proxy pipeline. Returns the child's exit code.
pub fn run(args: &[String]) -> io::Result<i32> {
    let mut child = spawn_child(args)?;

    let child_stdin = child.stdin.take().expect("child stdin");
    let child_stdout = child.stdout.take().expect("child stdout");
    let child_stderr = child.stderr.take().expect("child stderr");

    // Three threads: stdin→child, child→stdout (with compression), child stderr→stderr
    let stdin_thread = thread::spawn(move || pipe_stdin_to_child(child_stdin));
    let stdout_thread = thread::spawn(move || pipe_child_stdout(child_stdout));
    let stderr_thread = thread::spawn(move || pipe_child_stderr(child_stderr));

    // Wait for the child process to exit
    let status = child.wait()?;

    // Wait for pipe threads (they'll exit when the child's pipes close)
    let _ = stdin_thread.join();
    let _ = stdout_thread.join();
    let _ = stderr_thread.join();

    Ok(status.code().unwrap_or(1))
}
