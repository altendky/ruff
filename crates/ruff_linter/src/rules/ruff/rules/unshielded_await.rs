use crate::checkers::ast::Checker;
use ruff_diagnostics::{Diagnostic, Violation};
use ruff_macros::{derive_message_formats, violation};
use ruff_python_ast::name::QualifiedName;
use ruff_python_ast::visitor::Visitor;
use ruff_python_ast::{self as ast, visitor, Comprehension, ExceptHandler, Expr, ExprCall, Stmt};
use ruff_python_semantic::SemanticModel;
use ruff_text_size::{Ranged, TextRange};

/// ## What it does
/// TODO
///
///
/// ## Why is this bad?
/// TODO
///
/// ## Example
/// TODO
///
/// ## References
/// TODO
#[violation]
pub struct UnshieldedAwait;

impl Violation for UnshieldedAwait {
    #[derive_message_formats]
    fn message(&self) -> String {
        "shield it!".to_string()
    }
}

/// RUF102
pub(crate) fn unshielded_await_for_try(
    checker: &mut Checker,
    handlers: &Vec<ExceptHandler>,
    finalbody: &[Stmt],
) {
    let mut unshielded_await_ranges: Vec<TextRange> = vec![];

    for handler in handlers {
        let ExceptHandler::ExceptHandler(handler) = handler;

        let Some(t) = handler.type_.as_deref() else {
            todo!()
        };

        let types = flattened_tuple(t, checker.semantic());

        if !types.iter().any(|tt| {
            // TODO asyncio.CancelledError vs. CancelledError
            tt.segments() == ["", "Exception"] || tt.segments() == ["asyncio", "CancelledError"]
        }) {
            continue;
        }

        // If there are no awaits then there is nothing to shield
        let mut visitor = PrunedAwaitVisitor {
            semantic: checker.semantic(),
            await_ranges: vec![],
        };
        visitor.visit_body(&handler.body);
        unshielded_await_ranges.extend(visitor.await_ranges);
    }

    // If there are no awaits then there is nothing to shield
    let mut visitor = PrunedAwaitVisitor {
        semantic: checker.semantic(),
        await_ranges: vec![],
    };
    visitor.visit_body(finalbody);
    unshielded_await_ranges.extend(visitor.await_ranges);

    for range in unshielded_await_ranges {
        checker
            .diagnostics
            .push(Diagnostic::new(UnshieldedAwait {}, range));
    }
}

fn flattened_tuple<'a>(t: &'a Expr, semantic: &'a SemanticModel<'a>) -> Vec<QualifiedName<'a>> {
    let mut f = vec![];

    match t {
        Expr::Tuple(t) => {
            for e in t {
                f.append(&mut flattened_tuple(e, semantic));
            }
        }
        Expr::Name(..) | Expr::Attribute(..) => {
            if let Some(name) = semantic.resolve_qualified_name(t) {
                f.push(name);
            } else {
                panic!("inside unable to handle {t:?}");
            };
        }
        _ => panic!("outside unable to handle {t:?}"),
    }

    f
}

/// A [`Visitor`] that detects the presence of `await` expressions in the current scope.
struct PrunedAwaitVisitor<'a> {
    await_ranges: Vec<TextRange>,
    semantic: &'a SemanticModel<'a>,
}

impl Visitor<'_> for PrunedAwaitVisitor<'_> {
    fn visit_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::FunctionDef(_) | Stmt::ClassDef(_) => (),
            Stmt::With(ast::StmtWith { is_async: true, .. }) => {
                self.await_ranges.push(stmt.range());
            }
            Stmt::With(ast::StmtWith {
                is_async: false,
                items,
                ..
            }) => {
                for item in items {
                    if let Expr::Call(ExprCall { ref func, .. }) = item.context_expr {
                        if let Some(name) = self.semantic.resolve_qualified_name(func) {
                            // TODO what about x = y(); with y:? check the shield argument, etc
                            if name.segments() == ["anyio", "CancelScope"] {
                                return;
                            }
                        }
                    }
                }
                visitor::walk_stmt(self, stmt);
            }
            Stmt::For(ast::StmtFor { is_async: true, .. }) => {
                self.await_ranges.push(stmt.range());
            }
            _ => visitor::walk_stmt(self, stmt),
        }
    }

    fn visit_expr(&mut self, expr: &Expr) {
        if let Expr::Await(ast::ExprAwait { .. }) = expr {
            self.await_ranges.push(expr.range());
        } else {
            visitor::walk_expr(self, expr);
        }
    }

    fn visit_comprehension(&mut self, comprehension: &'_ Comprehension) {
        if comprehension.is_async {
            self.await_ranges.push(comprehension.range());
        } else {
            visitor::walk_comprehension(self, comprehension);
        }
    }
}
