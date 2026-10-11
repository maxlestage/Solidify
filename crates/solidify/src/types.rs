//! Types, with unification. Numbers carry a *flavor* (integer or float) decided by the
//! literals and operations they meet: `let n = 0` is an `i64`, `n / 2` is an `f64`.

use std::fmt;
use std::rc::Rc;

pub type TyVar = usize;

#[derive(Clone, Debug, PartialEq)]
pub enum Ty {
    /// Not known yet.
    Var(TyVar),
    /// A number whose flavor is the given variable.
    Num(TyVar),
    Str,
    Bool,
    Void,
    /// `null` and `undefined`, before they meet a type and make it an [`Ty::Option`].
    Null,
    Option(Box<Ty>),
    Array(Box<Ty>),
    Tuple(Vec<Ty>),
    /// A `Map`, or (`true`) a `Record` / object used as a dictionary: the same in Rust,
    /// shown differently.
    Map(Box<Ty>, Box<Ty>, bool),
    Set(Box<Ty>),
    /// A struct, an enum or a type parameter, by name.
    Named(String),
    Func(Rc<FuncTy>),
    /// `any` / `unknown`: accepted everywhere, never named in the output.
    Any,
    Never,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FuncTy {
    pub params: Vec<Ty>,
    pub ret: Ty,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Flavor {
    Unknown,
    Int,
    Float,
}

#[derive(Clone, Debug)]
enum Slot<T> {
    Free,
    Link(usize),
    Value(T),
}

/// The unification tables.
#[derive(Clone, Debug, Default)]
pub struct Types {
    vars: Vec<Slot<Ty>>,
    flavors: Vec<Slot<Flavor>>,
}

impl Types {
    pub fn fresh(&mut self) -> Ty {
        self.vars.push(Slot::Free);
        Ty::Var(self.vars.len() - 1)
    }

    pub fn num(&mut self, flavor: Flavor) -> Ty {
        self.flavors.push(if flavor == Flavor::Unknown {
            Slot::Free
        } else {
            Slot::Value(flavor)
        });
        Ty::Num(self.flavors.len() - 1)
    }

    pub fn int(&mut self) -> Ty {
        self.num(Flavor::Int)
    }

    pub fn float(&mut self) -> Ty {
        self.num(Flavor::Float)
    }

    fn find_var(&self, mut v: TyVar) -> TyVar {
        while let Slot::Link(next) = self.vars[v] {
            v = next;
        }
        v
    }

    fn find_flavor(&self, mut v: TyVar) -> TyVar {
        while let Slot::Link(next) = self.flavors[v] {
            v = next;
        }
        v
    }

    pub fn flavor(&self, v: TyVar) -> Flavor {
        match self.flavors[self.find_flavor(v)] {
            Slot::Value(f) => f,
            _ => Flavor::Unknown,
        }
    }

    /// Raises the flavor of `v` (never lowers it).
    pub fn set_flavor(&mut self, v: TyVar, flavor: Flavor) {
        let root = self.find_flavor(v);
        let current = match self.flavors[root] {
            Slot::Value(f) => f,
            _ => Flavor::Unknown,
        };
        if flavor > current {
            self.flavors[root] = Slot::Value(flavor);
        }
    }

    fn union_flavors(&mut self, a: TyVar, b: TyVar) {
        let (ra, rb) = (self.find_flavor(a), self.find_flavor(b));
        if ra == rb {
            return;
        }
        let fa = self.flavor(ra);
        let fb = self.flavor(rb);
        self.flavors[ra] = Slot::Link(rb);
        self.flavors[rb] = Slot::Value(fa.max(fb));
        if fa.max(fb) == Flavor::Unknown {
            self.flavors[rb] = Slot::Free;
        }
    }

    /// Follows variable links one level down.
    pub fn resolve(&self, ty: &Ty) -> Ty {
        match ty {
            Ty::Var(v) => {
                let root = self.find_var(*v);
                match &self.vars[root] {
                    Slot::Value(t) => self.resolve(t),
                    _ => Ty::Var(root),
                }
            }
            Ty::Num(f) => Ty::Num(self.find_flavor(*f)),
            other => other.clone(),
        }
    }

    /// Resolves every variable inside `ty`.
    pub fn zonk(&self, ty: &Ty) -> Ty {
        match self.resolve(ty) {
            Ty::Option(t) => Ty::Option(Box::new(self.zonk(&t))),
            Ty::Array(t) => Ty::Array(Box::new(self.zonk(&t))),
            Ty::Set(t) => Ty::Set(Box::new(self.zonk(&t))),
            Ty::Map(k, v, r) => Ty::Map(Box::new(self.zonk(&k)), Box::new(self.zonk(&v)), r),
            Ty::Tuple(ts) => Ty::Tuple(ts.iter().map(|t| self.zonk(t)).collect()),
            Ty::Func(f) => Ty::Func(Rc::new(FuncTy {
                params: f.params.iter().map(|t| self.zonk(t)).collect(),
                ret: self.zonk(&f.ret),
            })),
            other => other,
        }
    }

    pub fn bind(&mut self, v: TyVar, ty: Ty) {
        let root = self.find_var(v);
        self.vars[root] = Slot::Value(ty);
    }

    fn occurs(&self, v: TyVar, ty: &Ty) -> bool {
        match self.resolve(ty) {
            Ty::Var(w) => w == v,
            Ty::Option(t) | Ty::Array(t) | Ty::Set(t) => self.occurs(v, &t),
            Ty::Map(k, val, _) => self.occurs(v, &k) || self.occurs(v, &val),
            Ty::Tuple(ts) => ts.iter().any(|t| self.occurs(v, t)),
            Ty::Func(f) => f.params.iter().any(|t| self.occurs(v, t)) || self.occurs(v, &f.ret),
            _ => false,
        }
    }

    /// Makes `a` and `b` the same type. `null` lifts the other side to an `Option`, and a
    /// plain value unifies with the content of an `Option` (an assignment of `5` to a
    /// `number | null` is fine).
    pub fn unify(&mut self, a: &Ty, b: &Ty) -> Result<(), String> {
        let (a, b) = (self.resolve(a), self.resolve(b));
        match (&a, &b) {
            (Ty::Var(x), Ty::Var(y)) if x == y => Ok(()),
            (Ty::Var(x), _) => {
                if self.occurs(*x, &b) {
                    return Err("a type that contains itself".into());
                }
                self.bind(*x, b);
                Ok(())
            }
            (_, Ty::Var(y)) => {
                if self.occurs(*y, &a) {
                    return Err("a type that contains itself".into());
                }
                self.bind(*y, a);
                Ok(())
            }
            (Ty::Any, _) | (_, Ty::Any) | (Ty::Never, _) | (_, Ty::Never) => Ok(()),
            (Ty::Num(x), Ty::Num(y)) => {
                self.union_flavors(*x, *y);
                Ok(())
            }
            (Ty::Str, Ty::Str)
            | (Ty::Bool, Ty::Bool)
            | (Ty::Void, Ty::Void)
            | (Ty::Null, Ty::Null) => Ok(()),
            (Ty::Option(x), Ty::Option(y)) => self.unify(x, y),
            (Ty::Option(_), Ty::Null) | (Ty::Null, Ty::Option(_)) => Ok(()),
            (Ty::Option(x), _) => self.unify(x, &b),
            (_, Ty::Option(y)) => self.unify(&a, y),
            (Ty::Null, _) | (_, Ty::Null) => Err(format!(
                "{} cannot be null: declare it as `{} | null`",
                self.describe(if a == Ty::Null { &b } else { &a }),
                self.describe(if a == Ty::Null { &b } else { &a })
            )),
            (Ty::Array(x), Ty::Array(y)) | (Ty::Set(x), Ty::Set(y)) => self.unify(x, y),
            (Ty::Map(k1, v1, _), Ty::Map(k2, v2, _)) => {
                self.unify(k1, k2)?;
                self.unify(v1, v2)
            }
            (Ty::Tuple(xs), Ty::Tuple(ys)) if xs.len() == ys.len() => {
                for (x, y) in xs.iter().zip(ys) {
                    self.unify(x, y)?;
                }
                Ok(())
            }
            (Ty::Tuple(xs), Ty::Array(y)) | (Ty::Array(y), Ty::Tuple(xs)) => {
                for x in xs {
                    self.unify(x, y)?;
                }
                Ok(())
            }
            (Ty::Named(x), Ty::Named(y)) if x == y => Ok(()),
            (Ty::Func(f), Ty::Func(g)) => {
                for (x, y) in f.params.iter().zip(&g.params) {
                    self.unify(x, y)?;
                }
                self.unify(&f.ret, &g.ret)
            }
            _ => Err(format!(
                "{} is not {}",
                self.describe(&a),
                self.describe(&b)
            )),
        }
    }

    /// The type in TypeScript words, for messages.
    pub fn describe(&self, ty: &Ty) -> String {
        match self.resolve(ty) {
            Ty::Var(_) => "an unknown type".into(),
            Ty::Num(f) => match self.flavor(f) {
                Flavor::Float => "number (float)".into(),
                Flavor::Int => "number (integer)".into(),
                Flavor::Unknown => "number".into(),
            },
            Ty::Str => "string".into(),
            Ty::Bool => "boolean".into(),
            Ty::Void => "void".into(),
            Ty::Null => "null".into(),
            Ty::Option(t) => format!("{} | null", self.describe(&t)),
            Ty::Array(t) => format!("{}[]", self.describe(&t)),
            Ty::Tuple(ts) => format!(
                "[{}]",
                ts.iter()
                    .map(|t| self.describe(t))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Ty::Map(k, v, record) => format!(
                "{}<{}, {}>",
                if record { "Record" } else { "Map" },
                self.describe(&k),
                self.describe(&v)
            ),
            Ty::Set(t) => format!("Set<{}>", self.describe(&t)),
            Ty::Named(n) => n,
            Ty::Func(f) => format!(
                "({}) => {}",
                f.params
                    .iter()
                    .map(|t| self.describe(t))
                    .collect::<Vec<_>>()
                    .join(", "),
                self.describe(&f.ret)
            ),
            Ty::Any => "any".into(),
            Ty::Never => "never".into(),
        }
    }

    /// Whether the type copies freely in Rust (numbers, booleans, unit enums).
    pub fn is_copy(&self, ty: &Ty, copy_named: &dyn Fn(&str) -> bool) -> bool {
        match self.resolve(ty) {
            Ty::Num(_) | Ty::Bool | Ty::Void | Ty::Null | Ty::Never => true,
            Ty::Option(t) => self.is_copy(&t, copy_named),
            Ty::Tuple(ts) => ts.iter().all(|t| self.is_copy(t, copy_named)),
            Ty::Named(n) => copy_named(&n),
            _ => false,
        }
    }
}

impl fmt::Display for Ty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flavors_join_upwards() {
        let mut t = Types::default();
        let a = t.int();
        let b = t.num(Flavor::Unknown);
        let c = t.float();
        t.unify(&a, &b).unwrap();
        let Ty::Num(v) = t.resolve(&b) else { panic!() };
        assert_eq!(t.flavor(v), Flavor::Int);
        t.unify(&b, &c).unwrap();
        let Ty::Num(v) = t.resolve(&a) else { panic!() };
        assert_eq!(t.flavor(v), Flavor::Float);
    }

    #[test]
    fn null_lifts_to_option() {
        let mut t = Types::default();
        let x = t.fresh();
        t.unify(&x, &Ty::Null).unwrap();
        let x = Ty::Option(Box::new(t.fresh()));
        t.unify(&x, &Ty::Str).unwrap();
        assert_eq!(t.zonk(&x), Ty::Option(Box::new(Ty::Str)));
        assert!(t.unify(&Ty::Str, &Ty::Bool).is_err());
    }
}
