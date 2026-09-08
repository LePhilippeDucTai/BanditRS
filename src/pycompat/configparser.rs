//! INI subset of `configparser.ConfigParser`: `[section]`, `key = value` /
//! `key: value`, keys lower-cased, full-line `#`/`;` comments, indented
//! continuation lines, `[DEFAULT]` merged, duplicate keys/sections → error,
//! `BasicInterpolation` (`%(k)s`, `%%`). `parse_ini_file(path)` returns the
//! `[bandit]` section or `None` with the warning `Unable to parse config
//! file %s or missing [bandit] section`.

use indexmap::IndexMap;

/// A parsed INI document (`ConfigParser.read`).
#[derive(Debug, Clone, Default)]
pub struct ConfigParser {
    sections: IndexMap<String, IndexMap<String, String>>,
    defaults: IndexMap<String, String>,
}

impl ConfigParser {
    /// `ConfigParser().read_string(text)` (single source, matching `.read(path)`
    /// for one file).
    pub fn read_str(text: &str) -> Result<ConfigParser, String> {
        let mut cp = ConfigParser::default();
        let mut current: Option<String> = None;
        let mut last_key: Option<String> = None;

        for (lineno, raw_line) in text.lines().enumerate() {
            let line = raw_line;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                last_key = None;
                continue;
            }
            if trimmed.starts_with('#') || trimmed.starts_with(';') {
                continue;
            }
            let is_continuation = line.starts_with(' ') || line.starts_with('\t');
            if is_continuation && last_key.is_some() && current.is_some() {
                let section_name = current.clone().unwrap();
                let key = last_key.clone().unwrap();
                let target = if section_name == "DEFAULT" { &mut cp.defaults } else { cp.sections.get_mut(&section_name).unwrap() };
                let entry = target.get_mut(&key).unwrap();
                entry.push('\n');
                entry.push_str(trimmed);
                continue;
            }
            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                let name = trimmed[1..trimmed.len() - 1].to_string();
                if name != "DEFAULT" {
                    if cp.sections.contains_key(&name) {
                        return Err(format!("While reading from '<string>' [line {}]: section '{}' already exists", lineno + 1, name));
                    }
                    cp.sections.insert(name.clone(), IndexMap::new());
                }
                current = Some(name);
                last_key = None;
                continue;
            }
            let sep_pos = trimmed.find(['=', ':']);
            let Some(pos) = sep_pos else {
                return Err(format!("Source contains parsing errors: '<string>' [line {}]: {trimmed:?}", lineno + 1));
            };
            let key = trimmed[..pos].trim().to_ascii_lowercase();
            let value = trimmed[pos + 1..].trim().to_string();
            let Some(section_name) = current.clone() else {
                return Err(format!("File contains no section headers. line {}: {trimmed:?}", lineno + 1));
            };
            let target = if section_name == "DEFAULT" { &mut cp.defaults } else { cp.sections.get_mut(&section_name).unwrap() };
            if target.contains_key(&key) {
                return Err(format!("While reading from '<string>' [line {}]: option {:?} in section {:?} already exists", lineno + 1, key, section_name));
            }
            target.insert(key.clone(), value);
            last_key = Some(key);
        }
        Ok(cp)
    }

    /// `dict(config.items(section))`: section values with `[DEFAULT]`
    /// fallback and `%(key)s`/`%%` interpolation resolved.
    pub fn items(&self, section: &str) -> Option<IndexMap<String, String>> {
        let sect = self.sections.get(section)?;
        let mut merged = self.defaults.clone();
        for (k, v) in sect {
            merged.insert(k.clone(), v.clone());
        }
        let resolved: IndexMap<String, String> = merged.keys().map(|k| (k.clone(), self.interpolate(&merged, k, 0))).collect();
        Some(resolved)
    }

    fn interpolate(&self, values: &IndexMap<String, String>, key: &str, depth: u32) -> String {
        let raw = values.get(key).cloned().unwrap_or_default();
        if depth > 10 {
            return raw;
        }
        let mut out = String::new();
        let bytes: Vec<char> = raw.chars().collect();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == '%' && i + 1 < bytes.len() {
                if bytes[i + 1] == '%' {
                    out.push('%');
                    i += 2;
                    continue;
                }
                if bytes[i + 1] == '(' {
                    if let Some(end) = bytes[i + 2..].iter().position(|c| *c == ')') {
                        let name: String = bytes[i + 2..i + 2 + end].iter().collect();
                        if i + 2 + end + 1 < bytes.len() && bytes[i + 2 + end + 1] == 's' {
                            out.push_str(&self.interpolate(values, &name.to_ascii_lowercase(), depth + 1));
                            i = i + 2 + end + 2;
                            continue;
                        }
                    }
                }
            }
            out.push(bytes[i]);
            i += 1;
        }
        out
    }
}

/// `parse_ini_file(f_loc)`.
pub fn parse_ini_file(path: &str) -> Option<IndexMap<String, String>> {
    let text = std::fs::read_to_string(path).ok()?;
    let cp = ConfigParser::read_str(&text).ok()?;
    cp.items("bandit")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_section() {
        let cp = ConfigParser::read_str("[bandit]\nexclude=/abc,/def\n").unwrap();
        let items = cp.items("bandit").unwrap();
        assert_eq!(items.get("exclude"), Some(&"/abc,/def".to_string()));
    }

    #[test]
    fn missing_section_is_none() {
        let cp = ConfigParser::read_str("[Blabla]\nsomething=something\n").unwrap();
        assert!(cp.items("bandit").is_none());
    }

    #[test]
    fn colon_separator_and_comments_and_default() {
        let cp = ConfigParser::read_str("[DEFAULT]\nlevel: low\n# comment\n[bandit]\nexclude: /a\n").unwrap();
        let items = cp.items("bandit").unwrap();
        assert_eq!(items.get("level"), Some(&"low".to_string()));
        assert_eq!(items.get("exclude"), Some(&"/a".to_string()));
    }

    #[test]
    fn continuation_lines() {
        let cp = ConfigParser::read_str("[bandit]\nexclude = /a,\n  /b,\n  /c\n").unwrap();
        assert_eq!(cp.items("bandit").unwrap().get("exclude"), Some(&"/a,\n/b,\n/c".to_string()));
    }

    #[test]
    fn interpolation() {
        let cp = ConfigParser::read_str("[bandit]\nbase = /tmp\npath = %(base)s/x %% literal\n").unwrap();
        assert_eq!(cp.items("bandit").unwrap().get("path"), Some(&"/tmp/x % literal".to_string()));
    }
}
