//! File discovery (port of `BanditManager.discover_files` and helpers). See
//! docs/spec/core.md §1.2. Status: helpers implemented, `discover_files` stub (M5).

use std::path::Path;

use crate::pycompat::fnmatch::fnmatch;
use crate::pycompat::path;

/// `_matches_glob_list(filename, glob_list)`.
pub fn matches_glob_list(filename: &str, globs: &[String]) -> bool {
    globs.iter().any(|g| fnmatch(filename, g))
}

/// `_is_file_included(path, included_globs, excluded_path_strings, enforce_glob)`:
/// included iff (matches an include glob or `!enforce_glob`) and matches no
/// exclude glob and contains no exclude string as a substring.
pub fn is_file_included(p: &str, included: &[String], excluded: &[String], enforce_glob: bool) -> bool {
    (matches_glob_list(p, included) || !enforce_glob)
        && !matches_glob_list(p, excluded)
        && !excluded.iter().any(|x| p.contains(x.as_str()))
}

/// Result of a discovery.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Discovered {
    /// Sorted, de-duplicated files to scan (`./`-prefixed explicit files,
    /// `-` kept verbatim).
    pub files: Vec<String>,
    /// Sorted, de-duplicated excluded files.
    pub excluded: Vec<String>,
}

/// `_get_files_from_dir(files_dir, included_globs, excluded_path_strings)`:
/// walk recursively (`os.walk`), classify every file with `enforce_glob=true`.
pub fn get_files_from_dir(dir: &str, included: &[String], excluded: &[String]) -> (Vec<String>, Vec<String>) {
    let mut inc = Vec::new();
    let mut exc = Vec::new();
    fn walk(dir: &Path, included: &[String], excluded: &[String], inc: &mut Vec<String>, exc: &mut Vec<String>) {
        let Ok(entries) = std::fs::read_dir(dir) else { return };
        let mut dirs = Vec::new();
        for entry in entries.flatten() {
            let p = entry.path();
            let ps = p.to_string_lossy().into_owned();
            if p.is_dir() {
                dirs.push(p);
            } else if is_file_included(&ps, included, excluded, true) {
                inc.push(ps);
            } else {
                exc.push(ps);
            }
        }
        for d in dirs {
            walk(&d, included, excluded, inc, exc);
        }
    }
    walk(Path::new(dir), included, excluded, &mut inc, &mut exc);
    (inc, exc)
}

/// `discover_files(targets, recursive, excluded_paths)`.
///
/// TODO(M5): `excluded_globs = config exclude_dirs (owned copy) + each
/// comma-separated `excluded_paths` entry (an existing directory becomes
/// `dir/*`)`; `included = config include or ["*.py"]`; per target: directory
/// → recurse when `recursive` else
/// `log_warning!("manager", "Skipping directory ({}), use -r flag to scan contents", t)`;
/// file → `is_file_included(..., enforce_glob=false)` → `"./" + name`
/// (`path::join(".", name)`, `-` kept) else excluded. Sort + dedup both lists.
pub fn discover_files(
    _targets: &[String],
    _recursive: bool,
    _excluded_paths: Option<&str>,
    _config: &crate::core::config::BanditConfig,
) -> Discovered {
    todo!("M5: discover_files")
}

#[allow(dead_code)]
fn _unused() {
    let _ = path::join;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn glob_list_and_inclusion() {
        assert!(matches_glob_list("test", &v(&["*tes*"])));
        assert!(!matches_glob_list("test", &v(&["*fes*"])));
        assert!(is_file_included("a.py", &v(&["*.py"]), &[], true));
        assert!(is_file_included("a.dd", &v(&["*.py"]), &[], false));
        assert!(!is_file_included("a.py", &v(&["*.py"]), &v(&["a.py"]), true));
        assert!(!is_file_included("a.dd", &v(&["*.py"]), &[], true));
        assert!(!is_file_included("x_a.py", &v(&["*.py"]), &v(&["x_*.py"]), true));
        assert!(is_file_included("x.py", &v(&["*.py"]), &v(&["x_*.py"]), true));
        // substring exclusion
        assert!(!is_file_included("./x/y/z.py", &v(&["*.py"]), &v(&["y"]), false));
    }
}
