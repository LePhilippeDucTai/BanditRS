//! Port of the helpers in `bandit/core/utils.py` that are not tied to the
//! AST layer.

use std::fmt;

use crate::pycompat::path;
use crate::pycompat::unicode_escape;

/// Raised for a path from which no module name can be derived.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidModulePath(pub String);

impl fmt::Display for InvalidModulePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for InvalidModulePath {}

/// `get_module_qualname_from_path(path)`: the dotted module name, walking up
/// through directories that contain an `__init__.py`.
pub fn get_module_qualname_from_path(p: &str) -> Result<String, InvalidModulePath> {
    let (mut head, tail) = path::split(p);
    if head.is_empty() || tail.is_empty() {
        return Err(InvalidModulePath(format!(
            "Invalid python file path: \"{p}\" Missing path or file name"
        )));
    }
    let mut qname = vec![path::splitext(&tail).0];
    while head != "/" && head != "." && !head.is_empty() {
        if path::isfile(&path::join(&head, "__init__.py")) {
            let (h, t) = path::split(&head);
            qname.insert(0, t);
            head = h;
        } else {
            break;
        }
    }
    Ok(qname.join("."))
}

/// `namespace_path_join(base, name)`.
pub fn namespace_path_join(base: &str, name: &str) -> String {
    format!("{base}.{name}")
}

/// `namespace_path_split(path)`: `path.rsplit(".", 1)` (a single element
/// when there is no dot).
pub fn namespace_path_split(p: &str) -> (String, Option<String>) {
    match p.rsplit_once('.') {
        Some((a, b)) => (a.to_string(), Some(b.to_string())),
        None => (p.to_string(), None),
    }
}

/// `escaped_bytes_representation(b)`: decode with `unicode_escape` and
/// re-encode.
pub fn escaped_bytes_representation(b: &[u8]) -> Result<Vec<u8>, String> {
    Ok(unicode_escape::encode(&unicode_escape::decode(b)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn module_qualname() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_str().unwrap().to_string();
        let good = format!("{root}/good/a/b/c");
        fs::create_dir_all(&good).unwrap();
        for d in ["good", "good/a", "good/a/b", "good/a/b/c"] {
            fs::write(format!("{root}/{d}/__init__.py"), "").unwrap();
        }
        fs::write(format!("{good}/test_typical.py"), "").unwrap();
        assert_eq!(
            get_module_qualname_from_path(&format!("{good}/test_typical.py")).unwrap(),
            "good.a.b.c.test_typical"
        );
        assert_eq!(
            get_module_qualname_from_path("./__init__.py").unwrap(),
            "__init__"
        );
        // missing middle __init__.py
        let mid = format!("{root}/missingmid/a/b/c");
        fs::create_dir_all(&mid).unwrap();
        for d in ["missingmid", "missingmid/a/b", "missingmid/a/b/c"] {
            fs::write(format!("{root}/{d}/__init__.py"), "").unwrap();
        }
        fs::write(format!("{mid}/test_missingmid.py"), "").unwrap();
        assert_eq!(
            get_module_qualname_from_path(&format!("{mid}/test_missingmid.py")).unwrap(),
            "b.c.test_missingmid"
        );
        // missing end __init__.py
        let end = format!("{root}/missingend/a/b/c");
        fs::create_dir_all(&end).unwrap();
        for d in ["missingend", "missingend/a", "missingend/a/b"] {
            fs::write(format!("{root}/{d}/__init__.py"), "").unwrap();
        }
        fs::write(format!("{end}/test_missingend.py"), "").unwrap();
        assert_eq!(
            get_module_qualname_from_path(&format!("{end}/test_missingend.py")).unwrap(),
            "test_missingend"
        );
        // symlinks are not resolved
        let syms = format!("{root}/syms/a");
        fs::create_dir_all(&syms).unwrap();
        for d in ["syms", "syms/a"] {
            fs::write(format!("{root}/{d}/__init__.py"), "").unwrap();
        }
        std::os::unix::fs::symlink(format!("{root}/good/a/b"), format!("{syms}/bsym")).unwrap();
        assert_eq!(
            get_module_qualname_from_path(&format!("{syms}/bsym/c/test_typical.py")).unwrap(),
            "syms.a.bsym.c.test_typical"
        );
        // relative paths
        let cwd = std::env::current_dir().unwrap();
        std::env::set_current_dir(&root).unwrap();
        assert_eq!(
            get_module_qualname_from_path("good/a/b/c/test_typical.py").unwrap(),
            "good.a.b.c.test_typical"
        );
        assert_eq!(
            get_module_qualname_from_path("missingmid/a/b/c/test_missingmid.py").unwrap(),
            "b.c.test_missingmid"
        );
        assert_eq!(
            get_module_qualname_from_path("syms/a/bsym/c/test_typical.py").unwrap(),
            "syms.a.bsym.c.test_typical"
        );
        std::env::set_current_dir(cwd).unwrap();
        assert_eq!(get_module_qualname_from_path("/a/b/c/d/e.py").unwrap(), "e");
        assert_eq!(
            get_module_qualname_from_path("/usr/lib/python3.11/os.py").unwrap(),
            "os"
        );
        assert!(get_module_qualname_from_path("/tmp/").is_err());
    }

    #[test]
    fn namespace_helpers() {
        assert_eq!(
            namespace_path_join("base1.base2", "name"),
            "base1.base2.name"
        );
        assert_eq!(
            namespace_path_split("base1.base2.name"),
            ("base1.base2".into(), Some("name".into()))
        );
        assert_eq!(namespace_path_split("name"), ("name".into(), None));
    }

    #[test]
    fn escaped_representation() {
        assert_eq!(escaped_bytes_representation(b"ascii").unwrap(), b"ascii");
        assert_eq!(escaped_bytes_representation(b"\\u0000").unwrap(), b"\\x00");
        assert_eq!(
            escaped_bytes_representation(b"\\uffff").unwrap(),
            b"\\uffff"
        );
        assert_eq!(
            escaped_bytes_representation(b"ascii\\u0000\\uffff").unwrap(),
            b"ascii\\x00\\uffff"
        );
    }
}
