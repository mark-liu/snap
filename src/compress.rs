/// Playwright accessibility snapshot YAML compressor.
///
/// Strips structural noise from the YAML accessibility tree that Playwright MCP
/// returns with every tool result. Operates on text, not parsed YAML — simple
/// line-by-line state machine.
///
/// Safe transforms only — no information loss for content the LLM needs.

/// Result of compressing a snapshot.
pub struct CompressResult {
    pub output: String,
    pub input_bytes: usize,
    pub output_bytes: usize,
}

/// Compress a Playwright accessibility snapshot YAML string.
pub fn compress_snapshot(yaml: &str) -> CompressResult {
    let input_bytes = yaml.len();
    let mut output = String::with_capacity(yaml.len());
    let mut skip_depth: Option<usize> = None;

    for line in yaml.lines() {
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();

        // Block skip: if we're inside a skipped block, check if we've dedented
        if let Some(block_indent) = skip_depth {
            if indent > block_indent {
                continue; // Still inside skipped block
            }
            // Dedented — check if this line is a continuation at same level
            // that's still part of the block (sibling items)
            if indent == block_indent && !trimmed.starts_with("- ") && !trimmed.is_empty() {
                continue;
            }
            skip_depth = None; // Exited the block
        }

        // Block removal: detect blocks to skip entirely
        if should_skip_block(trimmed) {
            skip_depth = Some(indent);
            continue;
        }

        // Line removal: skip entire lines that are pure noise
        if should_remove_line(trimmed) {
            continue;
        }

        // Inline stripping: remove noise substrings but keep the line
        let cleaned = strip_inline_noise(line);

        // Skip lines that became empty after stripping
        let cleaned_trimmed = cleaned.trim();
        if cleaned_trimmed.is_empty() {
            continue;
        }

        output.push_str(&cleaned);
        output.push('\n');
    }

    let output_bytes = output.len();
    CompressResult {
        output,
        input_bytes,
        output_bytes,
    }
}

/// Detect blocks that should be skipped entirely (nav, banner, account menu).
fn should_skip_block(trimmed: &str) -> bool {
    // Navigation sidebar (Home, Explore, Notifications, etc.)
    if trimmed.contains("navigation \"Primary\"") {
        return true;
    }
    // Banner regions (site header with logo)
    if trimmed.starts_with("- banner ") || trimmed.starts_with("banner ") {
        return true;
    }
    // Account menu
    if trimmed.contains("button \"Account menu\"") {
        return true;
    }
    // "Skip to" accessibility buttons
    if trimmed.contains("button \"Skip to ") {
        return true;
    }
    // Keyboard shortcuts heading
    if trimmed.contains("heading \"To view keyboard shortcuts") {
        return true;
    }
    // "See new posts" status bar (nested inside status/button/generic)
    if trimmed.contains("See new posts") {
        return true;
    }
    // "Want to publish your own Article?" promo
    if trimmed.contains("Want to publish your own") {
        return true;
    }
    // Engagement button groups: "N replies, N reposts, N likes, N bookmarks, N views"
    if trimmed.starts_with("- group \"") && trimmed.contains("replies,") && trimmed.contains("likes,") {
        return true;
    }
    if trimmed.starts_with("group \"") && trimmed.contains("replies,") && trimmed.contains("likes,") {
        return true;
    }
    // "Grok actions" buttons
    if trimmed.contains("button \"Grok actions\"") {
        return true;
    }
    // "Subscribe to @" buttons
    if trimmed.contains("button \"Subscribe to @") {
        return true;
    }
    // Reply compose area
    if trimmed.contains("textbox \"Post text\"") {
        return true;
    }
    // "Post your reply" prompt
    if trimmed.contains("Post your reply") {
        return true;
    }
    // "Relevant" / "View quotes" buttons after engagement
    if trimmed.contains("button \"Relevant\"") || trimmed.contains("link \"View quotes\"") {
        return true;
    }
    // View post analytics links
    if trimmed.contains("View post analytics") {
        return true;
    }
    false
}

/// Detect lines that should be removed entirely.
fn should_remove_line(trimmed: &str) -> bool {
    // Bare img with only a ref — no alt text, no useful info
    // Pattern: "- img [ref=eNNN]" or "- img [ref=eNNN] [cursor=pointer]"
    if trimmed.starts_with("- img [ref=e") && !trimmed.contains('"') {
        return true;
    }
    // Relative URL lines under nav links (internal site navigation)
    if trimmed.starts_with("- /url: /") {
        return true;
    }
    // Unchanged markers in incremental snapshot mode
    if trimmed.starts_with("- ref=e") && trimmed.ends_with("[unchanged]") {
        return true;
    }
    // Bare ref= unchanged lines
    if trimmed.starts_with("ref=e") && trimmed.ends_with("[unchanged]") {
        return true;
    }
    // Standalone "More" overflow menu buttons (per-tweet)
    if trimmed == "- button \"More\":" || trimmed.starts_with("- button \"More\" [ref=e") {
        return true;
    }
    if trimmed == "button \"More\":" || trimmed.starts_with("button \"More\" [ref=e") {
        return true;
    }
    // "Share post" buttons
    if trimmed.starts_with("- button \"Share post\"") || trimmed.starts_with("button \"Share post\"") {
        return true;
    }
    // Console error/warning log references
    if trimmed.starts_with("- [ERROR]") || trimmed.starts_with("- [WARNING]") {
        return true;
    }
    // "New console entries" event lines
    if trimmed.starts_with("- New console entries:") {
        return true;
    }
    false
}

/// Strip noise substrings from a line without removing it.
fn strip_inline_noise(line: &str) -> String {
    let mut s = line.to_string();

    // Strip cursor annotations — never used by the LLM
    for pattern in &[
        " [cursor=pointer]",
        " [cursor=text]",
        " [cursor=default]",
        " [cursor=grab]",
        " [cursor=auto]",
    ] {
        // Replace all occurrences (a line could theoretically have multiple)
        while let Some(pos) = s.find(pattern) {
            s.replace_range(pos..pos + pattern.len(), "");
        }
    }

    // Strip [active] markers
    if let Some(pos) = s.find(" [active]") {
        s.replace_range(pos..pos + 9, "");
    }

    s
}

/// Find YAML code blocks in markdown text and compress them.
/// Returns the text with compressed YAML blocks, plus total bytes saved.
pub fn compress_markdown_yaml(text: &str) -> (String, usize) {
    let mut result = String::with_capacity(text.len());
    let mut total_saved: usize = 0;
    let mut pos = 0;
    let bytes = text.as_bytes();

    while pos < bytes.len() {
        // Look for ```yaml or ```\nyaml pattern
        if let Some(fence_start) = find_yaml_fence(text, pos) {
            // Copy everything before the fence
            result.push_str(&text[pos..fence_start.content_start]);

            // Find the closing fence
            if let Some(fence_end) = find_closing_fence(text, fence_start.content_start) {
                let yaml_content = &text[fence_start.content_start..fence_end];
                let compressed = compress_snapshot(yaml_content);
                total_saved += compressed.input_bytes.saturating_sub(compressed.output_bytes);
                result.push_str(&compressed.output);
                // Skip past the closing fence
                let after_fence = skip_past_closing_fence(text, fence_end);
                pos = after_fence;
            } else {
                // No closing fence — compress the rest as YAML
                let yaml_content = &text[fence_start.content_start..];
                let compressed = compress_snapshot(yaml_content);
                total_saved += compressed.input_bytes.saturating_sub(compressed.output_bytes);
                result.push_str(&compressed.output);
                pos = bytes.len();
            }
        } else {
            // No more YAML blocks — copy the rest
            result.push_str(&text[pos..]);
            pos = bytes.len();
        }
    }

    (result, total_saved)
}

struct FenceLocation {
    content_start: usize, // byte offset where YAML content begins (after the fence line)
}

/// Find the next ```yaml fence starting from `from`.
fn find_yaml_fence(text: &str, from: usize) -> Option<FenceLocation> {
    let search = &text[from..];

    // Look for ```yaml\n or ```yaml\r\n
    for pattern in &["```yaml\n", "```yaml\r\n"] {
        if let Some(idx) = search.find(pattern) {
            return Some(FenceLocation {
                content_start: from + idx + pattern.len(),
            });
        }
    }

    None
}

/// Find the closing ``` fence for a YAML block.
fn find_closing_fence(text: &str, from: usize) -> Option<usize> {
    let search = &text[from..];
    // Look for \n``` at the start of a line
    for pattern in &["\n```\n", "\n```\r\n", "\n``` \n"] {
        if let Some(idx) = search.find(pattern) {
            return Some(from + idx);
        }
    }
    // Also check for ``` at end of string
    if search.ends_with("\n```") {
        return Some(from + search.len() - 3);
    }
    None
}

/// Skip past the closing fence (``` plus its newline).
fn skip_past_closing_fence(text: &str, fence_pos: usize) -> usize {
    let rest = &text[fence_pos..];
    // fence_pos points to \n before ```, skip \n```\n
    for pattern in &["\n```\n", "\n```\r\n", "\n``` \n"] {
        if rest.starts_with(pattern) {
            return fence_pos + pattern.len();
        }
    }
    if rest == "\n```" {
        return fence_pos + 4;
    }
    // Fallback: skip past the ```
    fence_pos + 4
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_cursor() {
        let input = r#"- link "Home" [ref=e36] [cursor=pointer]:"#;
        let result = strip_inline_noise(input);
        assert_eq!(result, r#"- link "Home" [ref=e36]:"#);
    }

    #[test]
    fn test_strip_active() {
        let input = r#"- generic [active] [ref=e1]:"#;
        let result = strip_inline_noise(input);
        assert_eq!(result, r#"- generic [ref=e1]:"#);
    }

    #[test]
    fn test_remove_bare_img() {
        assert!(should_remove_line("- img [ref=e39]"));
        assert!(should_remove_line("- img [ref=e39] [cursor=pointer]"));
        // Should NOT remove img with alt text
        assert!(!should_remove_line(r#"- img "Profile photo" [ref=e39]"#));
    }

    #[test]
    fn test_remove_relative_url() {
        assert!(should_remove_line("- /url: /home"));
        assert!(should_remove_line("- /url: /explore"));
        // Should NOT remove absolute URLs
        assert!(!should_remove_line("- /url: https://example.com"));
    }

    #[test]
    fn test_remove_unchanged() {
        assert!(should_remove_line("- ref=e36 [unchanged]"));
        assert!(should_remove_line("ref=e36 [unchanged]"));
    }

    #[test]
    fn test_skip_navigation_block() {
        let yaml = r#"- generic [ref=e18]:
  - navigation "Primary" [ref=e35]:
    - link "Home" [ref=e36] [cursor=pointer]:
      - /url: /home
      - generic [ref=e37]:
        - img [ref=e39]
        - generic [ref=e42]: Home
    - link "Explore" [ref=e43] [cursor=pointer]:
      - /url: /explore
  - main [ref=e141]:
    - generic: actual content"#;

        let result = compress_snapshot(yaml);
        assert!(result.output.contains("main [ref=e141]"));
        assert!(!result.output.contains("navigation"));
        assert!(!result.output.contains("Home"));
        assert!(!result.output.contains("Explore"));
        assert!(result.output.contains("actual content"));
    }

    #[test]
    fn test_compress_markdown_yaml() {
        let md = "### Snapshot\n```yaml\n- link \"Home\" [ref=e36] [cursor=pointer]:\n  - /url: /home\n```\nDone.";
        let (result, saved) = compress_markdown_yaml(md);
        assert!(result.contains("### Snapshot\n```yaml\n"));
        assert!(!result.contains("[cursor=pointer]"));
        assert!(!result.contains("/url: /home"));
        assert!(result.contains("Done."));
        assert!(saved > 0);
    }

    #[test]
    fn test_passthrough_non_yaml() {
        let text = "No YAML here, just plain text.";
        let (result, saved) = compress_markdown_yaml(text);
        assert_eq!(result, text);
        assert_eq!(saved, 0);
    }
}
