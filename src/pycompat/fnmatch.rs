//! Python `fnmatch.fnmatch` semantics (POSIX: case sensitive, `*` matches
//! any characters including `/`).

/// Match `name` against the shell-style `pattern`.
pub fn fnmatch(name: &str, pattern: &str) -> bool {
    let name: Vec<char> = name.chars().collect();
    let pat: Vec<char> = pattern.chars().collect();
    match_here(&name, &pat)
}

fn match_here(name: &[char], pat: &[char]) -> bool {
    let (mut n, mut p) = (0usize, 0usize);
    let mut star: Option<(usize, usize)> = None; // (pattern pos after '*', name pos)
    loop {
        if p < pat.len() {
            match pat[p] {
                '*' => {
                    // collapse consecutive stars
                    while p < pat.len() && pat[p] == '*' {
                        p += 1;
                    }
                    if p == pat.len() {
                        return true;
                    }
                    star = Some((p, n));
                    continue;
                }
                '?' if n < name.len() => {
                    n += 1;
                    p += 1;
                    continue;
                }
                '[' => {
                    if let Some((matched, next_p)) = match_class(name.get(n).copied(), pat, p) {
                        if matched && n < name.len() {
                            n += 1;
                            p = next_p;
                            continue;
                        }
                    } else if n < name.len() && name[n] == '[' {
                        // unterminated set: literal '['
                        n += 1;
                        p += 1;
                        continue;
                    }
                }
                c if n < name.len() && name[n] == c => {
                    n += 1;
                    p += 1;
                    continue;
                }
                _ => {}
            }
        } else if n == name.len() {
            return true;
        }
        // mismatch: backtrack to the last star
        match star {
            Some((sp, sn)) if sn < name.len() => {
                n = sn + 1;
                p = sp;
                star = Some((sp, n));
            }
            _ => return false,
        }
    }
}

/// Try to match a bracket expression starting at `pat[p] == '['`.
/// Returns `None` when the set is unterminated (treated as a literal `[`),
/// otherwise `Some((matched, index after the closing bracket))`.
fn match_class(c: Option<char>, pat: &[char], p: usize) -> Option<(bool, usize)> {
    let mut j = p + 1;
    let negate = j < pat.len() && pat[j] == '!';
    if negate {
        j += 1;
    }
    let body_start = j;
    if j < pat.len() && pat[j] == ']' {
        j += 1;
    }
    while j < pat.len() && pat[j] != ']' {
        j += 1;
    }
    if j >= pat.len() {
        return None;
    }
    let body = &pat[body_start..j];
    let Some(c) = c else {
        return Some((false, j + 1));
    };
    let mut matched = false;
    let mut k = 0;
    while k < body.len() {
        if k + 2 < body.len() && body[k + 1] == '-' {
            let (lo, hi) = (body[k], body[k + 2]);
            if lo <= c && c <= hi {
                matched = true;
            }
            k += 3;
        } else {
            if body[k] == c {
                matched = true;
            }
            k += 1;
        }
    }
    Some((matched != negate, j + 1))
}

/// `any(fnmatch(name, p) for p in patterns)`.
pub fn matches_any<'a>(name: &str, patterns: impl IntoIterator<Item = &'a str>) -> bool {
    patterns.into_iter().any(|p| fnmatch(name, p))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basics() {
        assert!(fnmatch("test", "*tes*"));
        assert!(!fnmatch("test", "*fes*"));
        assert!(fnmatch("a.py", "*.py"));
        assert!(!fnmatch("a.pyc", "*.py"));
        assert!(fnmatch("./x/y.py", "./x/*"));
        assert!(fnmatch("dir/sub/file.py", "*.py"));
        assert!(fnmatch("x_a.py", "x_*.py"));
        assert!(!fnmatch("x.py", "x_*.py"));
        assert!(fnmatch("abc", "a?c"));
        assert!(fnmatch("abc", "a[a-c]c"));
        assert!(!fnmatch("abc", "a[!a-c]c"));
        assert!(fnmatch("a]c", "a[]]c"));
        assert!(fnmatch("a[c", "a[c"));
        assert!(fnmatch("", "*"));
        assert!(!fnmatch("", "?"));
        assert!(fnmatch("foo.egg", "*.egg"));
        assert!(fnmatch("a/b/c", "a/*/c"));
        assert!(fnmatch("aXbXc", "a*b*c"));
        assert!(!fnmatch("aXbXd", "a*b*c"));
    }
}
