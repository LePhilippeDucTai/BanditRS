//! Blacklist data (port of `bandit/blacklists/calls.py` and
//! `bandit/blacklists/imports.py`) and the builtin `B001` test
//! (`bandit/core/blacklisting.py`).
//!
//! Status: data tables complete (verbatim from upstream). `BlacklistTable`
//! filtering and the `blacklist()` test are stubs — see PLAN.md (M3).
//!
//! Matching rules (`blacklisting.py`):
//! * `Call` nodes: `name = context.call_function_name_qual`, except
//!   `__import__("x")` (first positional str constant, `"UNKNOWN"` if the
//!   first argument is not a str constant, `""` without arguments) and
//!   `importlib.import_module` / `importlib.__import__` (first positional
//!   argument value, else `call_keywords["name"]` — `KeyError` if absent,
//!   `TypeError` when `call_keywords` is `None`). Then for each entry in list
//!   order, for each qualname: **exact equality** → first match wins.
//! * `Import` / `ImportFrom` nodes: `prefix = module + "."` for
//!   `ImportFrom` with a module, else `""`; for each entry, for each alias,
//!   for each qualname: `(prefix + alias.name).startswith(qualname)` → the
//!   reported `{name}` is the bare alias name; first match wins (one issue
//!   per statement).
//! * Issue: severity = entry level, confidence always HIGH, cwe = entry cwe,
//!   text = `message.replace("{name}", name)` (plain substring replacement,
//!   all occurrences), ident = name, test_id = entry id, test = "blacklist".

use std::borrow::Cow;

use rustc_hash::FxHashMap;

use crate::constants::Rank;
use crate::core::issue::{Cwe, IssueDraft};

/// One blacklist datum (`build_conf_dict`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlacklistEntry {
    pub name: Cow<'static, str>,
    pub id: Cow<'static, str>,
    pub cwe: Cwe,
    pub qualnames: Vec<Cow<'static, str>>,
    pub message: Cow<'static, str>,
    /// Severity level name (`"LOW"`, `"MEDIUM"`, `"HIGH"`); legacy configs
    /// may carry other strings, which bandit passes through unchanged.
    pub level: Cow<'static, str>,
}

impl BlacklistEntry {
    const fn new(name: &'static str, id: &'static str, cwe: Cwe, qualnames: &'static [&'static str], message: &'static str, level: &'static str) -> StaticEntry {
        StaticEntry { name, id, cwe, qualnames, message, level }
    }

    /// `report_issue(check, name)`.
    pub fn report_issue(&self, name: &str) -> IssueDraft {
        let severity = Rank::parse(&self.level).unwrap_or(Rank::Medium);
        IssueDraft::new(severity, Rank::High, self.cwe, self.message.replace("{name}", name))
            .with_ident(name)
            .with_test_id(self.id.clone())
    }
}

/// Static form of a datum (converted to [`BlacklistEntry`] on demand).
#[derive(Debug, Clone, Copy)]
pub struct StaticEntry {
    pub name: &'static str,
    pub id: &'static str,
    pub cwe: Cwe,
    pub qualnames: &'static [&'static str],
    pub message: &'static str,
    pub level: &'static str,
}

impl StaticEntry {
    pub fn to_entry(&self) -> BlacklistEntry {
        BlacklistEntry {
            name: Cow::Borrowed(self.name),
            id: Cow::Borrowed(self.id),
            cwe: self.cwe,
            qualnames: self.qualnames.iter().map(|q| Cow::Borrowed(*q)).collect(),
            message: Cow::Borrowed(self.message),
            level: Cow::Borrowed(self.level),
        }
    }
}

const XML_MSG_CALLS: &str = "Using {name} to parse untrusted XML data is known to be vulnerable to XML attacks. Replace {name} with its defusedxml equivalent function or make sure defusedxml.defuse_stdlib() is called";

const XML_MSG_IMPORTS: &str = "Using {name} to parse untrusted XML data is known to be vulnerable to XML attacks. Replace {name} with the equivalent defusedxml package, or make sure defusedxml.defuse_stdlib() is called.";

/// `bandit/blacklists/calls.py::gen_blacklist()["Call"]`, in list order.
pub static BLACKLIST_CALLS: &[StaticEntry] = &[
    BlacklistEntry::new(
        "pickle",
        "B301",
        Cwe::DESERIALIZATION_OF_UNTRUSTED_DATA,
        &[
            "pickle.loads",
            "pickle.load",
            "pickle.Unpickler",
            "dill.loads",
            "dill.load",
            "dill.Unpickler",
            "shelve.open",
            "shelve.DbfilenameShelf",
            "jsonpickle.decode",
            "jsonpickle.unpickler.decode",
            "jsonpickle.unpickler.Unpickler",
            "pandas.read_pickle",
        ],
        "Pickle and modules that wrap it can be unsafe when used to deserialize untrusted data, possible security issue.",
        "MEDIUM",
    ),
    BlacklistEntry::new(
        "marshal",
        "B302",
        Cwe::DESERIALIZATION_OF_UNTRUSTED_DATA,
        &["marshal.load", "marshal.loads"],
        "Deserialization with the marshal module is possibly dangerous.",
        "MEDIUM",
    ),
    BlacklistEntry::new(
        "md5",
        "B303",
        Cwe::BROKEN_CRYPTO,
        &[
            "Crypto.Hash.MD2.new",
            "Crypto.Hash.MD4.new",
            "Crypto.Hash.MD5.new",
            "Crypto.Hash.SHA.new",
            "Cryptodome.Hash.MD2.new",
            "Cryptodome.Hash.MD4.new",
            "Cryptodome.Hash.MD5.new",
            "Cryptodome.Hash.SHA.new",
            "cryptography.hazmat.primitives.hashes.MD5",
            "cryptography.hazmat.primitives.hashes.SHA1",
        ],
        "Use of insecure MD2, MD4, MD5, or SHA1 hash function.",
        "MEDIUM",
    ),
    BlacklistEntry::new(
        "ciphers",
        "B304",
        Cwe::BROKEN_CRYPTO,
        &[
            "Crypto.Cipher.ARC2.new",
            "Crypto.Cipher.ARC4.new",
            "Crypto.Cipher.Blowfish.new",
            "Crypto.Cipher.DES.new",
            "Crypto.Cipher.XOR.new",
            "Cryptodome.Cipher.ARC2.new",
            "Cryptodome.Cipher.ARC4.new",
            "Cryptodome.Cipher.Blowfish.new",
            "Cryptodome.Cipher.DES.new",
            "Cryptodome.Cipher.XOR.new",
            "cryptography.hazmat.primitives.ciphers.algorithms.ARC4",
            "cryptography.hazmat.primitives.ciphers.algorithms.Blowfish",
            "cryptography.hazmat.primitives.ciphers.algorithms.CAST5",
            "cryptography.hazmat.primitives.ciphers.algorithms.IDEA",
            "cryptography.hazmat.primitives.ciphers.algorithms.SEED",
            "cryptography.hazmat.primitives.ciphers.algorithms.TripleDES",
        ],
        "Use of insecure cipher {name}. Replace with a known secure cipher such as AES.",
        "HIGH",
    ),
    BlacklistEntry::new(
        "cipher_modes",
        "B305",
        Cwe::BROKEN_CRYPTO,
        &["cryptography.hazmat.primitives.ciphers.modes.ECB"],
        "Use of insecure cipher mode {name}.",
        "MEDIUM",
    ),
    BlacklistEntry::new(
        "mktemp_q",
        "B306",
        Cwe::INSECURE_TEMP_FILE,
        &["tempfile.mktemp"],
        "Use of insecure and deprecated function (mktemp).",
        "MEDIUM",
    ),
    BlacklistEntry::new(
        "eval",
        "B307",
        Cwe::OS_COMMAND_INJECTION,
        &["eval"],
        "Use of possibly insecure function - consider using safer ast.literal_eval.",
        "MEDIUM",
    ),
    BlacklistEntry::new(
        "mark_safe",
        "B308",
        Cwe::XSS,
        &["django.utils.safestring.mark_safe"],
        "Use of mark_safe() may expose cross-site scripting vulnerabilities and should be reviewed.",
        "MEDIUM",
    ),
    BlacklistEntry::new(
        "urllib_urlopen",
        "B310",
        Cwe::PATH_TRAVERSAL,
        &[
            "urllib.request.urlopen",
            "urllib.request.urlretrieve",
            "urllib.request.URLopener",
            "urllib.request.FancyURLopener",
            "six.moves.urllib.request.urlopen",
            "six.moves.urllib.request.urlretrieve",
            "six.moves.urllib.request.URLopener",
            "six.moves.urllib.request.FancyURLopener",
        ],
        "Audit url open for permitted schemes. Allowing use of file:/ or custom schemes is often unexpected.",
        "MEDIUM",
    ),
    BlacklistEntry::new(
        "random",
        "B311",
        Cwe::INSUFFICIENT_RANDOM_VALUES,
        &[
            "random.Random",
            "random.random",
            "random.randrange",
            "random.randint",
            "random.choice",
            "random.choices",
            "random.uniform",
            "random.triangular",
            "random.randbytes",
            "random.sample",
            "random.randrange",
            "random.getrandbits",
        ],
        "Standard pseudo-random generators are not suitable for security/cryptographic purposes.",
        "LOW",
    ),
    BlacklistEntry::new(
        "telnetlib",
        "B312",
        Cwe::CLEARTEXT_TRANSMISSION,
        &["telnetlib.Telnet"],
        "Telnet-related functions are being called. Telnet is considered insecure. Use SSH or some other encrypted protocol.",
        "HIGH",
    ),
    BlacklistEntry::new(
        "xml_bad_cElementTree",
        "B313",
        Cwe::IMPROPER_INPUT_VALIDATION,
        &[
            "xml.etree.cElementTree.parse",
            "xml.etree.cElementTree.iterparse",
            "xml.etree.cElementTree.fromstring",
            "xml.etree.cElementTree.XMLParser",
        ],
        XML_MSG_CALLS,
        "MEDIUM",
    ),
    BlacklistEntry::new(
        "xml_bad_ElementTree",
        "B314",
        Cwe::IMPROPER_INPUT_VALIDATION,
        &[
            "xml.etree.ElementTree.parse",
            "xml.etree.ElementTree.iterparse",
            "xml.etree.ElementTree.fromstring",
            "xml.etree.ElementTree.XMLParser",
        ],
        XML_MSG_CALLS,
        "MEDIUM",
    ),
    BlacklistEntry::new(
        "xml_bad_expatreader",
        "B315",
        Cwe::IMPROPER_INPUT_VALIDATION,
        &["xml.sax.expatreader.create_parser"],
        XML_MSG_CALLS,
        "MEDIUM",
    ),
    BlacklistEntry::new(
        "xml_bad_expatbuilder",
        "B316",
        Cwe::IMPROPER_INPUT_VALIDATION,
        &["xml.dom.expatbuilder.parse", "xml.dom.expatbuilder.parseString"],
        XML_MSG_CALLS,
        "MEDIUM",
    ),
    BlacklistEntry::new(
        "xml_bad_sax",
        "B317",
        Cwe::IMPROPER_INPUT_VALIDATION,
        &["xml.sax.parse", "xml.sax.parseString", "xml.sax.make_parser"],
        XML_MSG_CALLS,
        "MEDIUM",
    ),
    BlacklistEntry::new(
        "xml_bad_minidom",
        "B318",
        Cwe::IMPROPER_INPUT_VALIDATION,
        &["xml.dom.minidom.parse", "xml.dom.minidom.parseString"],
        XML_MSG_CALLS,
        "MEDIUM",
    ),
    BlacklistEntry::new(
        "xml_bad_pulldom",
        "B319",
        Cwe::IMPROPER_INPUT_VALIDATION,
        &["xml.dom.pulldom.parse", "xml.dom.pulldom.parseString"],
        XML_MSG_CALLS,
        "MEDIUM",
    ),
    BlacklistEntry::new(
        "ftplib",
        "B321",
        Cwe::CLEARTEXT_TRANSMISSION,
        &["ftplib.FTP"],
        "FTP-related functions are being called. FTP is considered insecure. Use SSH/SFTP/SCP or some other encrypted protocol.",
        "HIGH",
    ),
    BlacklistEntry::new(
        "unverified_context",
        "B323",
        Cwe::IMPROPER_CERT_VALIDATION,
        &["ssl._create_unverified_context"],
        "By default, Python will create a secure, verified ssl context for use in such classes as HTTPSConnection. However, it still allows using an insecure context via the _create_unverified_context that  reverts to the previous behavior that does not validate certificates or perform hostname checks.",
        "MEDIUM",
    ),
];

/// `bandit/blacklists/imports.py::gen_blacklist()` — the same list is used
/// for the `Import`, `ImportFrom` and `Call` node types.
pub static BLACKLIST_IMPORTS: &[StaticEntry] = &[
    BlacklistEntry::new(
        "import_telnetlib",
        "B401",
        Cwe::CLEARTEXT_TRANSMISSION,
        &["telnetlib"],
        "A telnet-related module is being imported.  Telnet is considered insecure. Use SSH or some other encrypted protocol.",
        "HIGH",
    ),
    BlacklistEntry::new(
        "import_ftplib",
        "B402",
        Cwe::CLEARTEXT_TRANSMISSION,
        &["ftplib"],
        "A FTP-related module is being imported.  FTP is considered insecure. Use SSH/SFTP/SCP or some other encrypted protocol.",
        "HIGH",
    ),
    BlacklistEntry::new(
        "import_pickle",
        "B403",
        Cwe::DESERIALIZATION_OF_UNTRUSTED_DATA,
        &["pickle", "cPickle", "dill", "shelve"],
        "Consider possible security implications associated with {name} module.",
        "LOW",
    ),
    BlacklistEntry::new(
        "import_subprocess",
        "B404",
        Cwe::OS_COMMAND_INJECTION,
        &["subprocess"],
        "Consider possible security implications associated with the subprocess module.",
        "LOW",
    ),
    BlacklistEntry::new(
        "import_xml_etree",
        "B405",
        Cwe::IMPROPER_INPUT_VALIDATION,
        &["xml.etree.cElementTree", "xml.etree.ElementTree"],
        XML_MSG_IMPORTS,
        "LOW",
    ),
    BlacklistEntry::new("import_xml_sax", "B406", Cwe::IMPROPER_INPUT_VALIDATION, &["xml.sax"], XML_MSG_IMPORTS, "LOW"),
    BlacklistEntry::new(
        "import_xml_expat",
        "B407",
        Cwe::IMPROPER_INPUT_VALIDATION,
        &["xml.dom.expatbuilder"],
        XML_MSG_IMPORTS,
        "LOW",
    ),
    BlacklistEntry::new(
        "import_xml_minidom",
        "B408",
        Cwe::IMPROPER_INPUT_VALIDATION,
        &["xml.dom.minidom"],
        XML_MSG_IMPORTS,
        "LOW",
    ),
    BlacklistEntry::new(
        "import_xml_pulldom",
        "B409",
        Cwe::IMPROPER_INPUT_VALIDATION,
        &["xml.dom.pulldom"],
        XML_MSG_IMPORTS,
        "LOW",
    ),
    BlacklistEntry::new(
        "import_xmlrpclib",
        "B411",
        Cwe::IMPROPER_INPUT_VALIDATION,
        &["xmlrpc"],
        "Using {name} to parse untrusted XML data is known to be vulnerable to XML attacks. Use defusedxml.xmlrpc.monkey_patch() function to monkey-patch xmlrpclib and mitigate XML vulnerabilities.",
        "HIGH",
    ),
    BlacklistEntry::new(
        "import_httpoxy",
        "B412",
        Cwe::IMPROPER_ACCESS_CONTROL,
        &["wsgiref.handlers.CGIHandler", "twisted.web.twcgi.CGIScript", "twisted.web.twcgi.CGIDirectory"],
        "Consider possible security implications associated with {name} module.",
        "HIGH",
    ),
    BlacklistEntry::new(
        "import_pycrypto",
        "B413",
        Cwe::BROKEN_CRYPTO,
        &[
            "Crypto.Cipher",
            "Crypto.Hash",
            "Crypto.IO",
            "Crypto.Protocol",
            "Crypto.PublicKey",
            "Crypto.Random",
            "Crypto.Signature",
            "Crypto.Util",
        ],
        "The pyCrypto library and its module {name} are no longer actively maintained and have been deprecated. Consider using pyca/cryptography library.",
        "HIGH",
    ),
    BlacklistEntry::new(
        "import_pyghmi",
        "B415",
        Cwe::CLEARTEXT_TRANSMISSION,
        &["pyghmi"],
        "An IPMI-related module is being imported. IPMI is considered insecure. Use an encrypted protocol.",
        "HIGH",
    ),
];

/// All blacklist ids (calls then imports), as `extension_loader.MANAGER.blacklist_by_id` would list them.
pub fn all_entries() -> impl Iterator<Item = &'static StaticEntry> {
    BLACKLIST_CALLS.iter().chain(BLACKLIST_IMPORTS.iter())
}

/// `blacklist_by_id`.
pub fn entry_by_id(id: &str) -> Option<&'static StaticEntry> {
    all_entries().find(|e| e.id == id)
}

/// `blacklist_by_name`.
pub fn entry_by_name(name: &str) -> Option<&'static StaticEntry> {
    all_entries().find(|e| e.name == name)
}

/// The blacklist configuration handed to the `B001` test: the entries
/// enabled by the profile, per node type (`extension_loader.MANAGER.blacklist`
/// filtered by `BanditTestSet._load_builtins`).
#[derive(Debug, Clone, Default)]
pub struct BlacklistTable {
    pub call: Vec<BlacklistEntry>,
    pub import: Vec<BlacklistEntry>,
    pub import_from: Vec<BlacklistEntry>,
    /// Exact-match index for `Call` qualnames: qualname → index into `call`
    /// (first entry wins).
    call_index: FxHashMap<String, usize>,
}

impl BlacklistTable {
    /// The full built-in table (`{"Call": calls + imports, "Import": imports, "ImportFrom": imports}`).
    pub fn builtin() -> BlacklistTable {
        let imports: Vec<BlacklistEntry> = BLACKLIST_IMPORTS.iter().map(StaticEntry::to_entry).collect();
        let mut call: Vec<BlacklistEntry> = BLACKLIST_CALLS.iter().map(StaticEntry::to_entry).collect();
        call.extend(imports.iter().cloned());
        let mut t = BlacklistTable { call, import: imports.clone(), import_from: imports, call_index: FxHashMap::default() };
        t.rebuild_index();
        t
    }

    /// Keep only the entries whose id is accepted by `keep`.
    pub fn filtered(&self, keep: impl Fn(&str) -> bool) -> BlacklistTable {
        let mut t = BlacklistTable {
            call: self.call.iter().filter(|e| keep(&e.id)).cloned().collect(),
            import: self.import.iter().filter(|e| keep(&e.id)).cloned().collect(),
            import_from: self.import_from.iter().filter(|e| keep(&e.id)).cloned().collect(),
            call_index: FxHashMap::default(),
        };
        t.rebuild_index();
        t
    }

    pub fn is_empty(&self) -> bool {
        self.call.is_empty() && self.import.is_empty() && self.import_from.is_empty()
    }

    fn rebuild_index(&mut self) {
        self.call_index.clear();
        for (i, e) in self.call.iter().enumerate() {
            for q in &e.qualnames {
                self.call_index.entry(q.to_string()).or_insert(i);
            }
        }
    }

    /// First `Call` entry whose qualnames contain `name` exactly.
    pub fn find_call(&self, name: &str) -> Option<&BlacklistEntry> {
        self.call_index.get(name).map(|&i| &self.call[i])
    }
}

/// The `B001` test: see the module documentation for the matching rules.
/// TODO(M3): implement over `Context` (needs `context.node`, `call_args`,
/// `call_keywords`, alias names of import statements).
pub fn blacklist(
    _ctx: &crate::core::context::Context<'_, '_>,
    _kind: crate::ast::NodeKind,
    _table: &BlacklistTable,
) -> Result<Option<IssueDraft>, crate::ast::literal::PyErr> {
    todo!("M3: port bandit/core/blacklisting.py::blacklist")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_issue_matches_python() {
        let e = BlacklistEntry {
            name: "x".into(),
            id: "B000".into(),
            cwe: Cwe::NOTSET,
            qualnames: vec![],
            message: "test {name}".into(),
            level: "HIGH".into(),
        };
        let d = e.report_issue("name");
        assert_eq!(d.severity, Rank::High);
        assert_eq!(d.confidence, Rank::High);
        assert_eq!(d.cwe, Cwe::NOTSET);
        assert_eq!(d.text, "test name");
        assert_eq!(d.test_id.as_deref(), Some("B000"));
    }

    #[test]
    fn tables() {
        assert_eq!(BLACKLIST_CALLS.len(), 20);
        assert_eq!(BLACKLIST_IMPORTS.len(), 13);
        let t = BlacklistTable::builtin();
        assert_eq!(t.find_call("pickle.loads").map(|e| e.id.as_ref()), Some("B301"));
        assert_eq!(t.find_call("telnetlib").map(|e| e.id.as_ref()), Some("B401"));
        assert!(t.find_call("hashlib.md5").is_none());
        assert_eq!(entry_by_id("B413").unwrap().name, "import_pycrypto");
        assert_eq!(entry_by_name("md5").unwrap().id, "B303");
    }
}
