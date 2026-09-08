#!/usr/bin/env python3
"""Print bandit's AST walk as a tab-separated trace.

Usage: dump_walk.py FILE  (run with the reference bandit installed, e.g. the
venv created by scripts/diff_against_python.sh). Compare with
`bandit --dump-walk FILE` (set BANDITRS_PYTHON_COMPAT=3.11 when the reference
interpreter is Python 3.11).
"""
import ast
import io
import sys

from bandit.core import config, meta_ast, metrics, node_visitor as nv, test_set

SKIP = (ast.expr_context, ast.boolop, ast.operator, ast.unaryop, ast.cmpop)


def fmt_lr(lr):
    return f"{min(lr)}-{max(lr)}" if lr else "-"


def install(out):
    orig_pre = nv.BanditNodeVisitor.pre_visit
    orig_call = nv.BanditNodeVisitor.visit_Call
    orig_func = nv.BanditNodeVisitor.visit_FunctionDef
    orig_import = nv.BanditNodeVisitor.visit_Import
    orig_import_from = nv.BanditNodeVisitor.visit_ImportFrom
    orig_str = nv.BanditNodeVisitor.visit_Str
    orig_bytes = nv.BanditNodeVisitor.visit_Bytes

    def pre_visit(self, node):
        r = orig_pre(self, node)
        if isinstance(node, SKIP):
            return r
        ctx = self.context
        parent = getattr(node, "_bandit_parent", None)
        sib = getattr(node, "_bandit_sibling", None)
        sib_line = getattr(sib, "lineno", "-") if sib is not None else "-"
        out.append("\t".join(str(x) for x in [
            node.__class__.__name__,
            ctx.get("lineno", "-"),
            ctx.get("col_offset", "-"),
            getattr(node, "end_lineno", "-"),
            ctx.get("end_col_offset", "-"),
            fmt_lr(ctx["linerange"]),
            parent.__class__.__name__ if parent is not None else "-",
            sib_line,
        ]))
        return r

    def visit_Call(self, node):
        orig_call(self, node)
        out.append(f"CALL\t{self.context['qualname']}\t{self.context['name']}")

    def visit_FunctionDef(self, node):
        orig_func(self, node)
        out.append(f"FUNC\t{self.context['qualname']}\t{self.context['name']}")

    def visit_Import(self, node):
        orig_import(self, node)
        out.append(f"IMPORT\t{self.context['module']}")

    def visit_ImportFrom(self, node):
        orig_import_from(self, node)
        if node.module is not None:
            out.append(f"IMPORTFROM\t{self.context['module']}\t{self.context['name']}")

    def visit_Str(self, node):
        orig_str(self, node)
        if not isinstance(node._bandit_parent, ast.Expr):
            out.append(f"STR\t{node.value!r}\t{fmt_lr(self.context['linerange'])}")

    def visit_Bytes(self, node):
        orig_bytes(self, node)
        if not isinstance(node._bandit_parent, ast.Expr):
            out.append(f"BYTES\t{node.value!r}\t{fmt_lr(self.context['linerange'])}")

    nv.BanditNodeVisitor.pre_visit = pre_visit
    nv.BanditNodeVisitor.visit_Call = visit_Call
    nv.BanditNodeVisitor.visit_FunctionDef = visit_FunctionDef
    nv.BanditNodeVisitor.visit_Import = visit_Import
    nv.BanditNodeVisitor.visit_ImportFrom = visit_ImportFrom
    nv.BanditNodeVisitor.visit_Str = visit_Str
    nv.BanditNodeVisitor.visit_Bytes = visit_Bytes


def main():
    path = sys.argv[1]
    out = []
    install(out)
    b_conf = config.BanditConfig()
    ts = test_set.BanditTestSet(b_conf)
    ts.tests = {}  # do not run plugins, only trace the walk
    with open(path, "rb") as f:
        data = f.read()
    m = metrics.Metrics()
    m.begin(path)
    visitor = nv.BanditNodeVisitor(path, io.BytesIO(data), meta_ast.BanditMetaAst(), ts, False, {}, m)
    visitor.process(data)
    sys.stdout.write("\n".join(out) + "\n")


if __name__ == "__main__":
    main()
