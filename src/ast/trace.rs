//! Walk tracing used to compare the traversal with Python bandit
//! (`scripts/dump_walk.py` prints the same lines from the reference
//! implementation).

use ruff_python_ast::ModModule;

use super::joined_str::ViewArena;
use super::literal::repr_str;
use super::walker::{TestRunner, Walker};
use super::{NodeKind, PyCompat};
use crate::core::context::Context;
use crate::core::metrics::Scores;
use crate::source::SourceFile;

/// Collects one line per visited node plus one line per visitor method.
#[derive(Default)]
pub struct TraceRunner {
    pub lines: Vec<String>,
}

fn opt<T: std::fmt::Display>(v: Option<T>) -> String {
    v.map(|x| x.to_string()).unwrap_or_else(|| "-".to_string())
}

impl<'a> TestRunner<'a> for TraceRunner {
    fn wants(&self, _kind: NodeKind) -> bool {
        true
    }

    fn trace_all(&self) -> bool {
        true
    }

    fn pre_visit(&mut self, ctx: &Context<'a, '_>) {
        let node = ctx.node.expect("pre_visit always has a node");
        let parent = ctx.parent();
        let sibling_line = ctx
            .sibling
            .and_then(|s| super::positions::lineno(s, parent, ctx.file));
        let lr = if ctx.linerange.is_empty() {
            "-".to_string()
        } else {
            format!("{}-{}", ctx.linerange.start, ctx.linerange.end)
        };
        self.lines.push(format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            node.class_name(),
            opt(ctx.pos.map(|p| p.lineno)),
            opt(ctx.pos.map(|p| p.col_offset)),
            opt(ctx.pos.map(|p| p.end_lineno)),
            opt(ctx.pos.map(|p| p.end_col_offset)),
            lr,
            opt(parent.map(|p| p.class_name())),
            opt(sibling_line),
        ));
    }

    fn run_tests(&mut self, ctx: &Context<'a, '_>, kind: NodeKind) -> Scores {
        match kind {
            NodeKind::Call => {
                self.lines
                    .push(format!("CALL\t{}\t{}", opt(ctx.qualname), opt(ctx.name)))
            }
            NodeKind::FunctionDef => {
                self.lines
                    .push(format!("FUNC\t{}\t{}", opt(ctx.qualname), opt(ctx.name)))
            }
            NodeKind::Import => self.lines.push(format!("IMPORT\t{}", opt(ctx.module))),
            NodeKind::ImportFrom => self.lines.push(format!(
                "IMPORTFROM\t{}\t{}",
                opt(ctx.module),
                opt(ctx.name)
            )),
            NodeKind::Str => self.lines.push(format!(
                "STR\t{}\t{}-{}",
                repr_str(ctx.string_val().unwrap_or("")),
                ctx.linerange.start,
                ctx.linerange.end
            )),
            NodeKind::Bytes => self.lines.push(format!(
                "BYTES\t{}\t{}-{}",
                super::literal::repr_bytes(ctx.bytes_val().unwrap_or(&[])),
                ctx.linerange.start,
                ctx.linerange.end
            )),
            _ => {}
        }
        Scores::default()
    }
}

/// Trace the walk of an already parsed module.
pub fn trace_module(file: &SourceFile, module: &ModModule, compat: PyCompat) -> Vec<String> {
    let arena = ViewArena::new();
    let mut runner = TraceRunner::default();
    {
        let mut walker = Walker::new(file, &arena, compat, &mut runner);
        walker.process(module);
    }
    runner.lines
}

/// Read, decode, parse and trace a file.
pub fn dump_walk(path: &str, compat: PyCompat) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let text = crate::pycompat::encoding::decode_source(&bytes).map_err(|e| e.to_string())?;
    let file = SourceFile::new(path, text);
    let parsed =
        crate::source::parse::parse_module(&file.text, compat).map_err(|e| e.to_string())?;
    let lines = trace_module(&file, parsed.syntax(), compat);
    Ok(lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trace(src: &str) -> Vec<String> {
        let file = SourceFile::new("./x/mod.py", src);
        let parsed = crate::source::parse::parse_module(&file.text, PyCompat::Py312).unwrap();
        trace_module(&file, parsed.syntax(), PyCompat::Py312)
    }

    #[test]
    fn calls_imports_and_strings() {
        let lines = trace(
            "import os as o\nfrom a import b\no.system(b('x'))\n\"doc\"\ndef f(p='/tmp'):\n    pass\n",
        );
        assert!(lines.contains(&"IMPORT\tos".to_string()));
        assert!(lines.contains(&"IMPORTFROM\ta\tb".to_string()));
        assert!(lines.contains(&"CALL\tos.system\tsystem".to_string()));
        assert!(lines.contains(&"CALL\ta.b\tb".to_string()));
        assert!(lines.contains(&"STR\t'x'\t3-3".to_string()));
        assert!(!lines.iter().any(|l| l.starts_with("STR\t'doc'")));
        assert!(lines.contains(&"FUNC\tmod.f\tf".to_string()));
        // default value: parent is `arguments` (no position) -> range of its children
        assert!(lines.contains(&"STR\t'/tmp'\t5-5".to_string()));
        assert!(
            lines
                .iter()
                .any(|l| l.starts_with("arguments\t-\t-\t-\t-\t5-5\tFunctionDef\t-"))
        );
    }

    #[test]
    fn decorated_def_position_and_elif() {
        let lines = trace(
            "@dec\n@dec2\ndef f():\n    pass\nif a:\n    pass\nelif b:\n    pass\nelse:\n    pass\n",
        );
        assert!(
            lines
                .iter()
                .any(|l| l.starts_with("FunctionDef\t3\t0\t4\t8\t3-4\tModule\t5"))
        );
        assert!(
            lines
                .iter()
                .any(|l| l.starts_with("If\t7\t0\t10\t8\t7-10\tIf\t-"))
        );
    }

    #[test]
    fn fstring_constants_are_merged() {
        let lines = trace("x = f'a{b}c' 'd'\n");
        let strs: Vec<&String> = lines.iter().filter(|l| l.starts_with("STR")).collect();
        assert_eq!(strs, vec!["STR\t'a'\t1-1", "STR\t'cd'\t1-1"]);
        assert!(
            lines
                .iter()
                .any(|l| l.starts_with("JoinedStr\t1\t4\t1\t16"))
        );
        assert!(lines.iter().any(|l| l.starts_with("FormattedValue\t")));
    }
}
