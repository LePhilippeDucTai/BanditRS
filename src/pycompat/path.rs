//! POSIX `os.path` helpers with Python semantics.

use std::path::Path;

/// `os.path.join(a, b)` (POSIX).
pub fn join(a: &str, b: &str) -> String {
    if b.starts_with('/') {
        return b.to_string();
    }
    if a.is_empty() || a.ends_with('/') {
        format!("{a}{b}")
    } else {
        format!("{a}/{b}")
    }
}

/// `os.path.split(p)` → `(head, tail)`.
pub fn split(p: &str) -> (String, String) {
    match p.rfind('/') {
        None => (String::new(), p.to_string()),
        Some(i) => {
            let head = &p[..=i];
            let tail = &p[i + 1..];
            let trimmed = head.trim_end_matches('/');
            let head = if trimmed.is_empty() { head.to_string() } else { trimmed.to_string() };
            (head, tail.to_string())
        }
    }
}

/// `os.path.basename(p)`.
pub fn basename(p: &str) -> String {
    split(p).1
}

/// `os.path.splitext(p)` → `(root, ext)`.
pub fn splitext(p: &str) -> (String, String) {
    let sep_index = p.rfind('/').map(|i| i as isize).unwrap_or(-1);
    let dot_index = p.rfind('.').map(|i| i as isize).unwrap_or(-1);
    if dot_index > sep_index {
        // skip all leading dots
        let mut filename_index = sep_index + 1;
        while filename_index < dot_index {
            if p.as_bytes()[filename_index as usize] != b'.' {
                let d = dot_index as usize;
                return (p[..d].to_string(), p[d..].to_string());
            }
            filename_index += 1;
        }
    }
    (p.to_string(), String::new())
}

/// `os.path.normpath(p)` (POSIX).
pub fn normpath(p: &str) -> String {
    if p.is_empty() {
        return ".".to_string();
    }
    let initial_slashes = if p.starts_with('/') {
        if p.starts_with("//") && !p.starts_with("///") { 2 } else { 1 }
    } else {
        0
    };
    let mut comps: Vec<&str> = Vec::new();
    for comp in p.split('/') {
        if comp.is_empty() || comp == "." {
            continue;
        }
        if comp != ".." || (initial_slashes == 0 && comps.is_empty()) || comps.last() == Some(&"..") {
            comps.push(comp);
        } else if !comps.is_empty() {
            comps.pop();
        }
    }
    let mut out = "/".repeat(initial_slashes);
    out.push_str(&comps.join("/"));
    if out.is_empty() { ".".to_string() } else { out }
}

/// `os.path.isabs(p)`.
pub fn isabs(p: &str) -> bool {
    p.starts_with('/')
}

/// Current working directory as a string (lossy).
pub fn getcwd() -> String {
    std::env::current_dir().map(|p| p.to_string_lossy().into_owned()).unwrap_or_else(|_| ".".to_string())
}

/// `os.path.abspath(p)`.
pub fn abspath(p: &str) -> String {
    if isabs(p) { normpath(p) } else { normpath(&join(&getcwd(), p)) }
}

/// `os.path.relpath(p, start=os.curdir)`.
pub fn relpath(p: &str, start: Option<&str>) -> String {
    let start = start.map(str::to_string).unwrap_or_else(getcwd);
    let start_abs = abspath(&start);
    let start_list: Vec<&str> = start_abs.split('/').filter(|s| !s.is_empty()).collect();
    let path_abs = abspath(p);
    let path_list: Vec<&str> = path_abs.split('/').filter(|s| !s.is_empty()).collect::<Vec<_>>();
    let common = start_list.iter().zip(path_list.iter()).take_while(|(a, b)| a == b).count();
    let mut rel: Vec<&str> = Vec::new();
    for _ in common..start_list.len() {
        rel.push("..");
    }
    rel.extend_from_slice(&path_list[common..]);
    if rel.is_empty() { ".".to_string() } else { rel.join("/") }
}

/// `os.path.isdir(p)`.
pub fn isdir(p: &str) -> bool {
    Path::new(p).is_dir()
}

/// `os.path.isfile(p)`.
pub fn isfile(p: &str) -> bool {
    Path::new(p).is_file()
}

/// `os.path.exists(p)`.
pub fn exists(p: &str) -> bool {
    Path::new(p).exists()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn join_split() {
        assert_eq!(join(".", "a.py"), "./a.py");
        assert_eq!(join(".", "/abs/a.py"), "/abs/a.py");
        assert_eq!(join("a/", "b"), "a/b");
        assert_eq!(join("", "b"), "b");
        assert_eq!(split("/a/b/c.py"), ("/a/b".to_string(), "c.py".to_string()));
        assert_eq!(split("c.py"), (String::new(), "c.py".to_string()));
        assert_eq!(split("/tmp/"), ("/tmp".to_string(), String::new()));
        assert_eq!(split("/a"), ("/".to_string(), "a".to_string()));
        assert_eq!(splitext("a/b.c/d.py"), ("a/b.c/d".to_string(), ".py".to_string()));
        assert_eq!(splitext(".bashrc"), (".bashrc".to_string(), String::new()));
        assert_eq!(splitext("a"), ("a".to_string(), String::new()));
        assert_eq!(normpath("/a/./b/../c"), "/a/c");
        assert_eq!(normpath("a/../../b"), "../b");
        assert_eq!(normpath(""), ".");
        assert_eq!(normpath("//a"), "//a");
        assert_eq!(relpath("/a/b/c", Some("/a")), "b/c");
        assert_eq!(relpath("/a", Some("/a/b")), "..");
        assert_eq!(relpath("/a", Some("/a")), ".");
    }
}
