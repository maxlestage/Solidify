//! Numbers that grow without bound. JavaScript numbers are doubles: `fact(25)` or
//! `fib(100)` lose precision but never fail. An `i64` would overflow, so the integers
//! that grow exponentially are doubles too: those computed from themselves by a
//! product (`p *= 3`, `n * fact(n - 1)`), or by a sum of two of them (`[a, b] = [b,
//! a + b]`, `fib(n - 1) + fib(n - 2)`). A counter (`i += 1`) or a sum of other values
//! (`total += price`) grows linearly and stays an integer.

use std::collections::{HashMap, HashSet};

use super::{Callee, Infer};
use crate::ast::*;
use crate::types::{Flavor, Ty};

/// A value of the program a number flows into: a variable, or what a function returns.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Node {
    Var(NodeId),
    Ret(NodeId),
}

/// `target ← sources`: the sources (with repetitions) of a value, and whether a product
/// combines them.
struct Edge {
    target: Node,
    sources: Vec<Node>,
    product: bool,
}

impl<'m> Infer<'m> {
    /// Makes the integers that grow exponentially doubles (unless something needs an
    /// integer: an index, a length).
    pub(super) fn float_growing_numbers(&mut self) {
        let mut edges = Vec::new();
        for (target, op, value) in self.assigns.clone() {
            self.assign_edges(target, op, value, &mut edges);
        }
        for (b, init) in self.var_inits.clone() {
            let (sources, product) = self.sources(init);
            edges.push(Edge {
                target: Node::Var(b),
                sources,
                product,
            });
        }
        for (f, e) in self.fn_returns.clone() {
            let (sources, product) = self.sources(e);
            edges.push(Edge {
                target: Node::Ret(f),
                sources,
                product,
            });
        }
        // Who feeds whom, to find the cycles.
        let mut preds: HashMap<Node, HashSet<Node>> = HashMap::new();
        for e in &edges {
            preds
                .entry(e.target)
                .or_default()
                .extend(e.sources.iter().copied());
        }
        let mut growing: HashSet<Node> = HashSet::new();
        for e in &edges {
            // The sources that depend on the target itself: on a cycle with it.
            let on_cycle: Vec<&Node> = e
                .sources
                .iter()
                .filter(|s| reaches(&preds, **s, e.target))
                .collect();
            let grows = if e.product {
                !on_cycle.is_empty()
            } else {
                on_cycle.len() >= 2
            };
            if grows {
                growing.insert(e.target);
                for s in on_cycle {
                    growing.insert(*s);
                }
            }
        }
        for n in growing {
            let ty = match n {
                Node::Var(b) => self.vars.get(&b).map(|v| v.ty.clone()),
                Node::Ret(f) => self.funcs.get(&f).map(|f| f.sig.ret.clone()),
            };
            if let Some(Ty::Num(v)) = ty.map(|t| self.types.resolve(&t))
                && (self.types.flavor(v) == Flavor::Unknown || self.types.soft_int(v))
            {
                self.types.set_flavor(v, Flavor::Float);
            }
        }
    }

    fn assign_edges(
        &self,
        target: &AssignTarget,
        op: AssignOp,
        value: &Expr,
        edges: &mut Vec<Edge>,
    ) {
        match target {
            AssignTarget::Expr(t) => {
                let Some(&b) = self.refs.get(&t.id) else {
                    return;
                };
                let (mut sources, mut product) = self.sources(value);
                match op {
                    AssignOp::Assign => {}
                    AssignOp::Add | AssignOp::Sub => sources.push(Node::Var(b)),
                    AssignOp::Mul | AssignOp::Pow => {
                        sources.push(Node::Var(b));
                        product = true;
                    }
                    _ => return,
                }
                edges.push(Edge {
                    target: Node::Var(b),
                    sources,
                    product,
                });
            }
            // `[a, b] = [b, a + b]`: element by element.
            AssignTarget::Pat(Pattern::Array { elems, .. }) => {
                let items: Option<&Vec<ArrayElem>> = match &value.kind {
                    ExprKind::Array(items) => Some(items),
                    _ => None,
                };
                for (i, el) in elems.iter().enumerate() {
                    let Some(el) = el else { continue };
                    let Pattern::Ident { id, .. } = &el.pat else {
                        continue;
                    };
                    let Some(&b) = self.refs.get(id) else {
                        continue;
                    };
                    let (sources, product) = match items.and_then(|it| it.get(i)) {
                        Some(ArrayElem::Expr(x)) => self.sources(x),
                        _ => (Vec::new(), false),
                    };
                    edges.push(Edge {
                        target: Node::Var(b),
                        sources,
                        product,
                    });
                }
            }
            AssignTarget::Pat(_) => {}
        }
    }

    /// The values a number is computed from by sums and products, and whether a product
    /// is among them.
    fn sources(&self, e: &Expr) -> (Vec<Node>, bool) {
        let mut out = Vec::new();
        let mut product = false;
        self.collect_sources(e, &mut out, &mut product);
        (out, product)
    }

    fn collect_sources(&self, e: &Expr, out: &mut Vec<Node>, product: &mut bool) {
        match &e.kind {
            ExprKind::Ident(_) => {
                if let Some(&b) = self.refs.get(&e.id)
                    && matches!(
                        self.vars.get(&b).map(|v| self.types.resolve(&v.ty)),
                        Some(Ty::Num(_))
                    )
                {
                    out.push(Node::Var(b));
                }
            }
            ExprKind::Binary { op, left, right } => match op {
                BinOp::Add | BinOp::Sub => {
                    self.collect_sources(left, out, product);
                    self.collect_sources(right, out, product);
                }
                BinOp::Mul => {
                    let before = out.len();
                    self.collect_sources(left, out, product);
                    self.collect_sources(right, out, product);
                    if out.len() > before {
                        *product = true;
                    }
                }
                _ => {}
            },
            ExprKind::Unary {
                op: UnaryOp::Neg | UnaryOp::Pos,
                arg,
            } => self.collect_sources(arg, out, product),
            ExprKind::Cond { cons, alt, .. } => {
                self.collect_sources(cons, out, product);
                self.collect_sources(alt, out, product);
            }
            ExprKind::Assign { value, .. } => self.collect_sources(value, out, product),
            ExprKind::Call { .. } => match self.calls.get(&e.id) {
                Some(Callee::Fn(b)) => {
                    if let Some(f) = self.fn_bindings.get(b) {
                        out.push(Node::Ret(*f));
                    }
                }
                Some(Callee::Method { fn_id, .. }) => out.push(Node::Ret(*fn_id)),
                Some(Callee::Closure(b)) => {
                    if let Some(f) = self.vars.get(b).and_then(|v| v.closure_fn) {
                        out.push(Node::Ret(f));
                    }
                }
                _ => {}
            },
            ExprKind::As(inner, _) | ExprKind::NonNull(inner) | ExprKind::Await(inner) => {
                self.collect_sources(inner, out, product)
            }
            _ => {}
        }
    }
}

/// Whether `from` depends on `to` (follows the sources of each value).
fn reaches(preds: &HashMap<Node, HashSet<Node>>, from: Node, to: Node) -> bool {
    let mut seen = HashSet::new();
    let mut stack = vec![from];
    while let Some(n) = stack.pop() {
        if n == to {
            return true;
        }
        if !seen.insert(n) {
            continue;
        }
        if let Some(ps) = preds.get(&n) {
            stack.extend(ps.iter().copied());
        }
    }
    false
}
