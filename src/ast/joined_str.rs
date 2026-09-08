//! CPython-style view of f-strings and t-strings.
//!
//! CPython's `JoinedStr.values` alternates `Constant(str)` and
//! `FormattedValue` nodes, with adjacent literal segments merged into a single
//! constant (even across implicitly concatenated parts) and empty segments
//! dropped. ruff keeps the individual parts and elements; this module builds
//! the merged view once per string and stores it in an arena so that
//! [`super::VNode`] can reference it cheaply.

use ruff_python_ast::{self as ast, Expr};
use ruff_text_size::{Ranged, TextRange, TextSize};
use typed_arena::Arena;

use super::PyCompat;

/// What the view was built from.
#[derive(Clone, Copy)]
pub enum JoinedOwner<'a> {
    FString(&'a ast::ExprFString),
    TString(&'a ast::ExprTString),
    Spec(&'a ast::InterpolatedStringFormatSpec),
}

/// One value of the `JoinedStr`.
pub enum JoinedPart<'a> {
    /// Merged literal segments.
    Constant { text: Box<str>, range: TextRange },
    /// `FormattedValue` / `Interpolation`.
    Formatted {
        element: &'a ast::InterpolatedElement,
        range: TextRange,
        spec: Option<&'a JoinedStrView<'a>>,
    },
}

impl JoinedPart<'_> {
    pub fn range(&self) -> TextRange {
        match self {
            JoinedPart::Constant { range, .. } | JoinedPart::Formatted { range, .. } => *range,
        }
    }

    pub fn constant_text(&self) -> Option<&str> {
        match self {
            JoinedPart::Constant { text, .. } => Some(text),
            JoinedPart::Formatted { .. } => None,
        }
    }

    pub fn is_constant(&self) -> bool {
        matches!(self, JoinedPart::Constant { .. })
    }
}

/// The merged view of a joined string.
pub struct JoinedStrView<'a> {
    pub owner: JoinedOwner<'a>,
    /// CPython range of the `JoinedStr` node itself.
    pub range: TextRange,
    pub values: Vec<JoinedPart<'a>>,
}

/// Arena holding the views built while walking one file.
pub type ViewArena<'a> = Arena<JoinedStrView<'a>>;

struct Builder<'a, 'b> {
    arena: &'a ViewArena<'a>,
    source: &'b str,
    compat: PyCompat,
    values: Vec<JoinedPart<'a>>,
    pending: Option<(String, TextRange)>,
}

impl<'a, 'b> Builder<'a, 'b> {
    fn new(arena: &'a ViewArena<'a>, source: &'b str, compat: PyCompat) -> Self {
        Builder {
            arena,
            source,
            compat,
            values: Vec::new(),
            pending: None,
        }
    }

    fn literal(&mut self, text: &str, range: TextRange) {
        if text.is_empty() {
            return;
        }
        match &mut self.pending {
            Some((buf, r)) => {
                buf.push_str(text);
                *r = TextRange::new(r.start(), range.end());
            }
            None => self.pending = Some((text.to_string(), range)),
        }
    }

    fn flush(&mut self) {
        if let Some((text, range)) = self.pending.take() {
            self.values.push(JoinedPart::Constant {
                text: text.into_boxed_str(),
                range,
            });
        }
    }

    /// `part_range` is the range of the enclosing f-string token (used by
    /// the Python 3.11 position policy for format specs).
    fn elements(&mut self, elements: &'a ast::InterpolatedStringElements, part_range: TextRange) {
        for element in elements {
            match element {
                ast::InterpolatedStringElement::Literal(lit) => {
                    self.literal(&lit.value, lit.range())
                }
                ast::InterpolatedStringElement::Interpolation(e) => {
                    if let Some(debug) = &e.debug_text {
                        // `f"{x=}"`: CPython emits the debug text as a constant.
                        let range =
                            TextRange::new(e.start() + TextSize::from(1), e.expression.end());
                        let text = format!(
                            "{}{}{}",
                            debug.leading(),
                            debug.expression(),
                            debug.trailing()
                        );
                        self.literal(&text, range);
                    }
                    self.flush();
                    let spec = e.format_spec.as_deref().map(|spec| {
                        let spec_range = match self.compat {
                            PyCompat::Py311 => part_range,
                            PyCompat::Py312 => spec.range(),
                        };
                        let mut b = Builder::new(self.arena, self.source, self.compat);
                        b.elements(&spec.elements, part_range);
                        b.flush();
                        let view = JoinedStrView {
                            owner: JoinedOwner::Spec(spec),
                            range: spec_range,
                            values: b.values,
                        };
                        &*self.arena.alloc(finish(view, self.compat, spec_range))
                    });
                    self.values.push(JoinedPart::Formatted {
                        element: e,
                        range: e.range(),
                        spec,
                    });
                }
            }
        }
    }
}

/// Apply the position policy: under the 3.11 policy every value takes the
/// range of the joined string.
fn finish<'a>(
    mut view: JoinedStrView<'a>,
    compat: PyCompat,
    whole: TextRange,
) -> JoinedStrView<'a> {
    if compat == PyCompat::Py311 {
        for v in &mut view.values {
            match v {
                JoinedPart::Constant { range, .. } | JoinedPart::Formatted { range, .. } => {
                    *range = whole
                }
            }
        }
    }
    view
}

impl<'a> JoinedStrView<'a> {
    /// Build the view of an f-string expression.
    pub fn build_fstring(
        expr: &'a ast::ExprFString,
        arena: &'a ViewArena<'a>,
        source: &str,
        compat: PyCompat,
    ) -> &'a JoinedStrView<'a> {
        let mut b = Builder::new(arena, source, compat);
        for part in expr.value.as_slice() {
            match part {
                ast::FStringPart::Literal(lit) => b.literal(lit.as_str(), lit.range()),
                ast::FStringPart::FString(f) => b.elements(&f.elements, f.range()),
            }
        }
        b.flush();
        let view = JoinedStrView {
            owner: JoinedOwner::FString(expr),
            range: expr.range(),
            values: b.values,
        };
        arena.alloc(finish(view, compat, expr.range()))
    }

    /// Build the view of a t-string expression (Python 3.14 `TemplateStr`).
    pub fn build_tstring(
        expr: &'a ast::ExprTString,
        arena: &'a ViewArena<'a>,
        source: &str,
        compat: PyCompat,
    ) -> &'a JoinedStrView<'a> {
        let mut b = Builder::new(arena, source, compat);
        for t in expr.value.iter() {
            b.elements(&t.elements, t.range());
        }
        b.flush();
        let view = JoinedStrView {
            owner: JoinedOwner::TString(expr),
            range: expr.range(),
            values: b.values,
        };
        arena.alloc(finish(view, compat, expr.range()))
    }

    /// Build the view for an f-string or t-string expression.
    pub fn build(
        expr: &'a Expr,
        arena: &'a ViewArena<'a>,
        source: &str,
        compat: PyCompat,
    ) -> Option<&'a JoinedStrView<'a>> {
        match expr {
            Expr::FString(f) => Some(JoinedStrView::build_fstring(f, arena, source, compat)),
            Expr::TString(t) => Some(JoinedStrView::build_tstring(t, arena, source, compat)),
            _ => None,
        }
    }

    pub fn is_template(&self) -> bool {
        matches!(self.owner, JoinedOwner::TString(_))
    }

    /// Index of the first constant value, if any.
    pub fn first_constant_index(&self) -> Option<u32> {
        self.values
            .iter()
            .position(JoinedPart::is_constant)
            .map(|i| i as u32)
    }

    /// Concatenation of all constant values (`"".join(str(c.value) for c in substrings)`).
    pub fn constants_joined(&self) -> String {
        let mut s = String::new();
        for v in &self.values {
            if let Some(t) = v.constant_text() {
                s.push_str(t);
            }
        }
        s
    }
}
