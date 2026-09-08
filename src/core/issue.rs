//! Issues reported by tests (port of `bandit/core/issue.py`).

use std::borrow::Cow;
use std::fmt;
use std::sync::Arc;

use serde_json::{Map, Value};

use crate::constants::Rank;
use crate::source::{SourceFile, SourceStore};

/// A Common Weakness Enumeration identifier (`0` = not set).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Cwe(pub u32);

#[allow(dead_code)]
impl Cwe {
    pub const NOTSET: Cwe = Cwe(0);
    pub const IMPROPER_INPUT_VALIDATION: Cwe = Cwe(20);
    pub const PATH_TRAVERSAL: Cwe = Cwe(22);
    pub const OS_COMMAND_INJECTION: Cwe = Cwe(78);
    pub const XSS: Cwe = Cwe(79);
    pub const BASIC_XSS: Cwe = Cwe(80);
    pub const SQL_INJECTION: Cwe = Cwe(89);
    pub const CODE_INJECTION: Cwe = Cwe(94);
    pub const IMPROPER_WILDCARD_NEUTRALIZATION: Cwe = Cwe(155);
    pub const HARD_CODED_PASSWORD: Cwe = Cwe(259);
    pub const IMPROPER_ACCESS_CONTROL: Cwe = Cwe(284);
    pub const IMPROPER_CERT_VALIDATION: Cwe = Cwe(295);
    pub const CLEARTEXT_TRANSMISSION: Cwe = Cwe(319);
    pub const INADEQUATE_ENCRYPTION_STRENGTH: Cwe = Cwe(326);
    pub const BROKEN_CRYPTO: Cwe = Cwe(327);
    pub const INSUFFICIENT_RANDOM_VALUES: Cwe = Cwe(330);
    pub const INSECURE_TEMP_FILE: Cwe = Cwe(377);
    pub const UNCONTROLLED_RESOURCE_CONSUMPTION: Cwe = Cwe(400);
    pub const DOWNLOAD_OF_CODE_WITHOUT_INTEGRITY_CHECK: Cwe = Cwe(494);
    pub const DESERIALIZATION_OF_UNTRUSTED_DATA: Cwe = Cwe(502);
    pub const MULTIPLE_BINDS: Cwe = Cwe(605);
    pub const IMPROPER_CHECK_OF_EXCEPT_COND: Cwe = Cwe(703);
    pub const INCORRECT_PERMISSION_ASSIGNMENT: Cwe = Cwe(732);
    pub const INAPPROPRIATE_ENCODING_FOR_OUTPUT_CONTEXT: Cwe = Cwe(838);

    pub const MITRE_URL_PATTERN: &'static str = "https://cwe.mitre.org/data/definitions/%s.html";

    /// Numeric identifier.
    pub const fn id(self) -> u32 {
        self.0
    }

    /// Whether the CWE is set.
    pub const fn is_set(self) -> bool {
        self.0 != 0
    }

    /// Link to the MITRE definition, empty when not set.
    pub fn link(self) -> String {
        if self.is_set() {
            format!("https://cwe.mitre.org/data/definitions/{}.html", self.0)
        } else {
            String::new()
        }
    }

    /// `{"id": .., "link": ..}` or `{}` when not set.
    pub fn as_dict(self) -> Value {
        if self.is_set() {
            let mut m = Map::new();
            m.insert("id".into(), Value::from(self.0));
            m.insert("link".into(), Value::from(self.link()));
            Value::Object(m)
        } else {
            Value::Object(Map::new())
        }
    }
}

impl fmt::Display for Cwe {
    /// `"CWE-78 (https://...)"` or the empty string when not set.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_set() {
            write!(f, "CWE-{} ({})", self.0, self.link())
        } else {
            Ok(())
        }
    }
}

/// An inclusive, contiguous range of line numbers (bandit's `linerange`
/// list). `end < start` encodes the empty list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LineRange {
    pub start: u32,
    pub end: u32,
}

impl LineRange {
    /// The empty range (`[]`).
    pub const EMPTY: LineRange = LineRange { start: 1, end: 0 };

    /// `list(range(start, end + 1))`.
    pub const fn new(start: u32, end: u32) -> LineRange {
        LineRange { start, end }
    }

    /// A single line.
    pub const fn single(line: u32) -> LineRange {
        LineRange {
            start: line,
            end: line,
        }
    }

    pub const fn is_empty(self) -> bool {
        self.end < self.start
    }

    /// Number of lines (`len(linerange)`).
    pub const fn len(self) -> usize {
        if self.end < self.start {
            0
        } else {
            (self.end - self.start + 1) as usize
        }
    }

    /// Iterate over the line numbers.
    pub fn iter(self) -> impl Iterator<Item = u32> {
        self.start..=self.end
    }

    /// Materialise as a list.
    pub fn to_vec(self) -> Vec<u32> {
        self.iter().collect()
    }

    /// Materialise as a JSON array.
    pub fn to_json(self) -> Value {
        Value::Array(self.iter().map(Value::from).collect())
    }
}

impl Default for LineRange {
    fn default() -> LineRange {
        LineRange::EMPTY
    }
}

/// The issue returned by a plugin before the tester fills in location data.
/// Fields left as `None` are taken from the scanning context.
#[derive(Debug, Clone, PartialEq)]
pub struct IssueDraft {
    pub severity: Rank,
    pub confidence: Rank,
    pub cwe: Cwe,
    pub text: String,
    pub ident: Option<String>,
    pub lineno: Option<u32>,
    pub col_offset: Option<u32>,
    pub linerange: Option<LineRange>,
    pub test_id: Option<Cow<'static, str>>,
}

impl IssueDraft {
    /// A draft with the given ranks and text; everything else defaults.
    pub fn new(severity: Rank, confidence: Rank, cwe: Cwe, text: impl Into<String>) -> IssueDraft {
        IssueDraft {
            severity,
            confidence,
            cwe,
            text: text.into(),
            ident: None,
            lineno: None,
            col_offset: None,
            linerange: None,
            test_id: None,
        }
    }

    pub fn with_lineno(mut self, lineno: Option<u32>) -> IssueDraft {
        self.lineno = lineno;
        self
    }

    pub fn with_col_offset(mut self, col: u32) -> IssueDraft {
        self.col_offset = Some(col);
        self
    }

    pub fn with_linerange(mut self, range: LineRange) -> IssueDraft {
        self.linerange = Some(range);
        self
    }

    pub fn with_ident(mut self, ident: impl Into<String>) -> IssueDraft {
        self.ident = Some(ident.into());
        self
    }

    pub fn with_test_id(mut self, test_id: impl Into<Cow<'static, str>>) -> IssueDraft {
        self.test_id = Some(test_id.into());
        self
    }
}

/// A fully populated issue.
#[derive(Debug, Clone)]
pub struct Issue {
    pub severity: Rank,
    pub confidence: Rank,
    pub cwe: Cwe,
    pub text: String,
    pub ident: Option<String>,
    pub fname: String,
    pub test: Cow<'static, str>,
    pub test_id: Cow<'static, str>,
    pub lineno: u32,
    pub col_offset: u32,
    pub end_col_offset: u32,
    pub linerange: LineRange,
    /// Source text used to render code snippets (falls back to the global
    /// [`SourceStore`] when absent).
    pub source: Option<Arc<SourceFile>>,
}

impl Issue {
    /// Construct an issue with explicit location data (used by tests and by
    /// the tester once a draft has been decorated).
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        severity: Rank,
        confidence: Rank,
        cwe: Cwe,
        text: impl Into<String>,
        fname: impl Into<String>,
        test: impl Into<Cow<'static, str>>,
        test_id: impl Into<Cow<'static, str>>,
        lineno: u32,
    ) -> Issue {
        Issue {
            severity,
            confidence,
            cwe,
            text: text.into(),
            ident: None,
            fname: fname.into(),
            test: test.into(),
            test_id: test_id.into(),
            lineno,
            col_offset: 0,
            end_col_offset: 0,
            linerange: LineRange::EMPTY,
            source: None,
        }
    }

    /// Python's `Issue.__eq__`: text, severity, cwe, confidence, file name,
    /// test name and test id (line numbers are ignored).
    pub fn same_signature(&self, other: &Issue) -> bool {
        self.text == other.text
            && self.severity == other.severity
            && self.cwe == other.cwe
            && self.confidence == other.confidence
            && self.fname == other.fname
            && self.test == other.test
            && self.test_id == other.test_id
    }

    /// Compare with an issue loaded from a baseline report.
    pub fn matches_baseline(&self, other: &BaselineIssue) -> bool {
        other.text.as_deref() == Some(self.text.as_str())
            && other.severity.as_deref() == Some(self.severity.as_str())
            && other.cwe == self.cwe
            && other.confidence.as_deref() == Some(self.confidence.as_str())
            && other.fname.as_deref() == Some(self.fname.as_str())
            && other.test.as_deref() == Some(&*self.test)
            && other.test_id.as_deref() == Some(&*self.test_id)
    }

    /// Whether the issue meets the severity and confidence thresholds.
    pub fn filter(&self, severity: Rank, confidence: Rank) -> bool {
        self.severity >= severity && self.confidence >= confidence
    }

    /// Code snippet around the issue, formatted as `"<lineno> <line>"` per
    /// line (or tab separated when `tabbed`).
    pub fn get_code(&self, max_lines: i64, tabbed: bool) -> String {
        let max_lines = max_lines.max(1);
        let lmin = (self.lineno as i64 - max_lines / 2).max(1);
        let lmax = lmin + self.linerange.len() as i64 + max_lines - 1;
        let source = match &self.source {
            Some(s) => Some(s.clone()),
            None => SourceStore::global().get(&self.fname),
        };
        let mut out = String::new();
        let Some(source) = source else {
            return out;
        };
        for line in lmin..lmax {
            let text = source.snippet_line(line as u32);
            if text.is_empty() {
                break;
            }
            if tabbed {
                out.push_str(&format!("{line}\t{text}"));
            } else {
                out.push_str(&format!("{line} {text}"));
            }
        }
        out
    }

    /// Dictionary representation used by the machine-readable formatters.
    pub fn as_dict(&self, with_code: bool, max_lines: i64) -> Value {
        let mut m = Map::new();
        m.insert("filename".into(), Value::from(self.fname.as_str()));
        m.insert("test_name".into(), Value::from(&*self.test));
        m.insert("test_id".into(), Value::from(&*self.test_id));
        m.insert("issue_severity".into(), Value::from(self.severity.as_str()));
        m.insert("issue_cwe".into(), self.cwe.as_dict());
        m.insert(
            "issue_confidence".into(),
            Value::from(self.confidence.as_str()),
        );
        m.insert("issue_text".into(), Value::from(self.text.as_str()));
        m.insert("line_number".into(), Value::from(self.lineno));
        m.insert("line_range".into(), self.linerange.to_json());
        m.insert("col_offset".into(), Value::from(self.col_offset));
        m.insert("end_col_offset".into(), Value::from(self.end_col_offset));
        if with_code {
            m.insert("code".into(), Value::from(self.get_code(max_lines, false)));
        }
        Value::Object(m)
    }
}

impl fmt::Display for Issue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Issue: '{}' from {}:{}: CWE: {}, Severity: {} Confidence: {} at {}:{}:{}",
            self.text,
            self.test_id,
            self.ident.as_deref().unwrap_or(&self.test),
            self.cwe,
            self.severity,
            self.confidence,
            self.fname,
            self.lineno,
            self.col_offset
        )
    }
}

/// An issue loaded from a JSON baseline report (`issue_from_dict`). Only the
/// fields taking part in issue equality are typed; the remaining ones are kept
/// verbatim.
#[derive(Debug, Clone, PartialEq)]
pub struct BaselineIssue {
    pub text: Option<String>,
    pub severity: Option<String>,
    pub cwe: Cwe,
    pub confidence: Option<String>,
    pub fname: Option<String>,
    pub test: Option<String>,
    pub test_id: Option<String>,
    pub code: Value,
    pub line_number: Value,
    pub line_range: Value,
    pub col_offset: Value,
    pub end_col_offset: Value,
}

/// Error raised while loading a baseline issue (a missing key or an invalid
/// CWE id in Python terms).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaselineError(pub String);

impl fmt::Display for BaselineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

fn required<'a>(data: &'a Map<String, Value>, key: &str) -> Result<&'a Value, BaselineError> {
    data.get(key)
        .ok_or_else(|| BaselineError(format!("'{key}'")))
}

fn as_opt_string(v: &Value) -> Option<String> {
    v.as_str().map(str::to_string)
}

/// `cwe_from_dict`: `int(data["id"])` when present, `NOTSET` otherwise.
pub fn cwe_from_dict(data: &Value) -> Result<Cwe, BaselineError> {
    let Some(obj) = data.as_object() else {
        return Err(BaselineError("argument of type is not iterable".into()));
    };
    match obj.get("id") {
        None => Ok(Cwe::NOTSET),
        Some(Value::Number(n)) => {
            if let Some(i) = n.as_u64() {
                Ok(Cwe(i as u32))
            } else if let Some(f) = n.as_f64() {
                Ok(Cwe(f.trunc().max(0.0) as u32))
            } else {
                Err(BaselineError("invalid literal for int()".into()))
            }
        }
        Some(Value::String(s)) => {
            s.trim().parse::<u32>().map(Cwe).map_err(|_| {
                BaselineError(format!("invalid literal for int() with base 10: '{s}'"))
            })
        }
        Some(Value::Bool(b)) => Ok(Cwe(*b as u32)),
        Some(_) => Err(BaselineError(
            "int() argument must be a string or a number".into(),
        )),
    }
}

impl BaselineIssue {
    /// `issue_from_dict(data)`.
    pub fn from_dict(data: &Value) -> Result<BaselineIssue, BaselineError> {
        let obj = data
            .as_object()
            .ok_or_else(|| BaselineError("issue is not a dictionary".into()))?;
        Ok(BaselineIssue {
            code: required(obj, "code")?.clone(),
            fname: as_opt_string(required(obj, "filename")?),
            severity: as_opt_string(required(obj, "issue_severity")?),
            cwe: cwe_from_dict(required(obj, "issue_cwe")?)?,
            confidence: as_opt_string(required(obj, "issue_confidence")?),
            text: as_opt_string(required(obj, "issue_text")?),
            test: as_opt_string(required(obj, "test_name")?),
            test_id: as_opt_string(required(obj, "test_id")?),
            line_number: required(obj, "line_number")?.clone(),
            line_range: required(obj, "line_range")?.clone(),
            col_offset: obj.get("col_offset").cloned().unwrap_or(Value::from(0)),
            end_col_offset: obj.get("end_col_offset").cloned().unwrap_or(Value::from(0)),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn issue() -> Issue {
        let mut i = Issue::new(
            Rank::Medium,
            Rank::Medium,
            Cwe::MULTIPLE_BINDS,
            "Test issue",
            "code.py",
            "bandit_plugin",
            "B999",
            1,
        );
        i.col_offset = 8;
        i.end_col_offset = 16;
        i
    }

    #[test]
    fn issue_str() {
        let expected = "Issue: 'Test issue' from B999:bandit_plugin: CWE: CWE-605 \
                        (https://cwe.mitre.org/data/definitions/605.html), Severity: MEDIUM \
                        Confidence: MEDIUM at code.py:1:8";
        assert_eq!(issue().to_string(), expected);
    }

    #[test]
    fn issue_as_dict() {
        let d = issue().as_dict(false, 3);
        assert_eq!(d["filename"], "code.py");
        assert_eq!(d["test_name"], "bandit_plugin");
        assert_eq!(d["test_id"], "B999");
        assert_eq!(d["issue_severity"], "MEDIUM");
        assert_eq!(d["issue_cwe"]["id"], 605);
        assert_eq!(
            d["issue_cwe"]["link"],
            "https://cwe.mitre.org/data/definitions/605.html"
        );
        assert_eq!(d["issue_confidence"], "MEDIUM");
        assert_eq!(d["issue_text"], "Test issue");
        assert_eq!(d["line_number"], 1);
        assert_eq!(d["line_range"], Value::Array(vec![]));
        assert_eq!(d["col_offset"], 8);
        assert_eq!(d["end_col_offset"], 16);
        assert!(d.get("code").is_none());
        assert_eq!(Cwe::NOTSET.as_dict(), Value::Object(Map::new()));
        assert_eq!(Cwe::NOTSET.to_string(), "");
    }

    #[test]
    fn issue_filter() {
        let i = issue();
        assert!(i.filter(Rank::Undefined, Rank::Undefined));
        assert!(i.filter(Rank::Low, Rank::Low));
        assert!(i.filter(Rank::Medium, Rank::Medium));
        assert!(!i.filter(Rank::High, Rank::Medium));
        assert!(!i.filter(Rank::Medium, Rank::High));
    }

    #[test]
    fn matches_issue() {
        let a = issue();
        let mut b = issue();
        b.lineno = 2;
        assert!(a.same_signature(&b));
        b.severity = Rank::High;
        assert!(!a.same_signature(&b));
        let mut c = issue();
        c.text = "other".into();
        assert!(!a.same_signature(&c));
        let mut d = issue();
        d.fname = "other.py".into();
        assert!(!a.same_signature(&d));
        let mut e = issue();
        e.test = "other".into();
        assert!(!a.same_signature(&e));
    }

    #[test]
    fn get_code_with_control_chars() {
        let mut i = issue();
        i.source = Some(Arc::new(SourceFile::new(
            "code.py",
            "\x08\x30\nsecond\nthird\n",
        )));
        i.linerange = LineRange::single(1);
        assert_eq!(i.get_code(3, false), "1 \x08\x30\n2 second\n3 third\n");
        assert_eq!(i.get_code(-1, true), "1\t\x08\x30\n");
        i.lineno = 3;
        i.linerange = LineRange::single(3);
        assert_eq!(i.get_code(3, false), "2 second\n3 third\n");
    }

    #[test]
    fn baseline_roundtrip() {
        let data = serde_json::json!({
            "code": "x", "filename": "f.py", "issue_severity": "low",
            "issue_cwe": {"id": 605, "link": "https://cwe.mitre.org/data/definitions/605.html"},
            "issue_confidence": "low", "issue_text": "t", "test_name": "n", "test_id": "B1",
            "line_number": "n", "line_range": "n-m"
        });
        let b = BaselineIssue::from_dict(&data).unwrap();
        assert_eq!(b.cwe, Cwe(605));
        assert_eq!(b.severity.as_deref(), Some("low"));
        assert!(BaselineIssue::from_dict(&serde_json::json!({"data": "bad"})).is_err());
        assert_eq!(cwe_from_dict(&serde_json::json!({})).unwrap(), Cwe::NOTSET);
        assert_eq!(
            cwe_from_dict(&serde_json::json!({"id": "78"})).unwrap(),
            Cwe(78)
        );
    }
}
