//! In-memory representation of a scanned Python source file.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

/// A decoded source file with a precomputed line index.
///
/// Offsets are byte offsets into `text`; lines are 1-based; columns are byte
/// offsets from the start of the line (CPython's `col_offset` semantics).
#[derive(Debug)]
pub struct SourceFile {
    /// File name as reported by bandit (e.g. `./examples/foo.py` or `<stdin>`).
    pub name: String,
    /// Decoded text (BOM removed, original line terminators preserved).
    pub text: String,
    /// Byte offsets of the start of each line.
    line_starts: Vec<u32>,
    /// `<stdin>` keeps raw line terminators in code snippets (bandit reads the
    /// buffered bytes directly instead of going through `linecache`).
    pub is_stdin: bool,
}

impl SourceFile {
    /// Build a source file from decoded text.
    pub fn new(name: impl Into<String>, text: impl Into<String>) -> SourceFile {
        let name = name.into();
        let text = text.into();
        let line_starts = compute_line_starts(&text);
        let is_stdin = name == "<stdin>";
        SourceFile {
            name,
            text,
            line_starts,
            is_stdin,
        }
    }

    /// Number of lines (a trailing terminator does not start a new line
    /// unless followed by content, matching `str.splitlines`).
    pub fn line_count(&self) -> usize {
        let n = self.line_starts.len();
        if n > 0 && self.line_starts[n - 1] as usize >= self.text.len() && !self.text.is_empty() {
            n - 1
        } else {
            n
        }
    }

    /// 1-based line number containing `offset`.
    pub fn line_index(&self, offset: u32) -> u32 {
        match self.line_starts.binary_search(&offset) {
            Ok(i) => i as u32 + 1,
            Err(i) => i as u32,
        }
    }

    /// Byte offset of the start of `line` (1-based). Lines past the end map
    /// to the end of the text.
    pub fn line_start(&self, line: u32) -> u32 {
        let idx = line.saturating_sub(1) as usize;
        self.line_starts
            .get(idx)
            .copied()
            .unwrap_or(self.text.len() as u32)
    }

    /// `(line, byte column)` of `offset`; line is 1-based, column 0-based.
    pub fn line_col(&self, offset: u32) -> (u32, u32) {
        let line = self.line_index(offset);
        (line, offset - self.line_start(line))
    }

    /// Raw content of `line` (1-based) without its terminator.
    pub fn line_text(&self, line: u32) -> Option<&str> {
        let idx = line.checked_sub(1)? as usize;
        let start = *self.line_starts.get(idx)? as usize;
        if start >= self.text.len() {
            return None;
        }
        let end = self
            .line_starts
            .get(idx + 1)
            .map(|&e| e as usize)
            .unwrap_or(self.text.len());
        let raw = &self.text[start..end];
        Some(raw.trim_end_matches(['\n', '\r']))
    }

    /// Line as returned by Python when building code snippets: for regular
    /// files `linecache` yields every line terminated by `\n` (universal
    /// newlines), for `<stdin>` bandit reads the raw bytes line by line.
    /// Returns an empty string past the end of the file.
    pub fn snippet_line(&self, line: u32) -> String {
        let Some(idx) = line.checked_sub(1) else {
            return String::new();
        };
        let idx = idx as usize;
        let Some(&start) = self.line_starts.get(idx) else {
            return String::new();
        };
        let start = start as usize;
        if start >= self.text.len() {
            return String::new();
        }
        let end = self
            .line_starts
            .get(idx + 1)
            .map(|&e| e as usize)
            .unwrap_or(self.text.len());
        let raw = &self.text[start..end];
        if self.is_stdin {
            raw.to_string()
        } else {
            let mut s = raw.trim_end_matches(['\n', '\r']).to_string();
            s.push('\n');
            s
        }
    }
}

/// Compute the byte offsets of every line start (`\n`, `\r\n` and lone `\r`
/// are line terminators, like CPython's tokenizer and ruff's `LineIndex`).
pub fn compute_line_starts(text: &str) -> Vec<u32> {
    let bytes = text.as_bytes();
    let mut starts = Vec::with_capacity(bytes.len() / 32 + 2);
    starts.push(0u32);
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\n' => starts.push(i as u32 + 1),
            b'\r' => {
                if bytes.get(i + 1) == Some(&b'\n') {
                    i += 1;
                }
                starts.push(i as u32 + 1);
            }
            _ => {}
        }
        i += 1;
    }
    starts
}

/// Process-wide cache of source files keyed by file name, used to render code
/// snippets for issues that do not carry their source (Python's `linecache`).
#[derive(Default)]
pub struct SourceStore {
    files: Mutex<HashMap<String, Arc<SourceFile>>>,
}

static GLOBAL_STORE: OnceLock<SourceStore> = OnceLock::new();

impl SourceStore {
    /// The global store.
    pub fn global() -> &'static SourceStore {
        GLOBAL_STORE.get_or_init(SourceStore::default)
    }

    /// Register a file.
    pub fn insert(&self, file: Arc<SourceFile>) {
        self.files
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(file.name.clone(), file);
    }

    /// Fetch a file, loading it from disk (decoded like Python's `linecache`)
    /// when unknown. Returns `None` when the file cannot be read.
    pub fn get(&self, name: &str) -> Option<Arc<SourceFile>> {
        if let Some(f) = self
            .files
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(name)
        {
            return Some(f.clone());
        }
        let bytes = std::fs::read(name).ok()?;
        let text = crate::pycompat::encoding::decode_source(&bytes).ok()?;
        let file = Arc::new(SourceFile::new(name, text));
        self.insert(file.clone());
        Some(file)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_and_columns() {
        let f = SourceFile::new("x.py", "ab\ncd\r\nef\rgh");
        assert_eq!(f.line_count(), 4);
        assert_eq!(f.line_col(0), (1, 0));
        assert_eq!(f.line_col(4), (2, 1));
        assert_eq!(f.line_col(7), (3, 0));
        assert_eq!(f.line_col(10), (4, 0));
        assert_eq!(f.line_text(2), Some("cd"));
        assert_eq!(f.line_text(4), Some("gh"));
        assert_eq!(f.line_text(5), None);
        assert_eq!(f.snippet_line(2), "cd\n");
        assert_eq!(f.snippet_line(4), "gh\n");
        assert_eq!(f.snippet_line(9), "");
        let f = SourceFile::new("x.py", "ab\n");
        assert_eq!(f.line_count(), 1);
        assert_eq!(f.snippet_line(1), "ab\n");
        assert_eq!(f.snippet_line(2), "");
        let s = SourceFile::new("<stdin>", "ab\r\ncd");
        assert_eq!(s.snippet_line(1), "ab\r\n");
        assert_eq!(s.snippet_line(2), "cd");
        // multibyte columns are byte based
        let f = SourceFile::new("x.py", "é = 1\n");
        assert_eq!(f.line_col(3), (1, 3));
    }
}
