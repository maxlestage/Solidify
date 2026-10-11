//! Inference on small programs: types and analyses come out as expected.

use solidify::infer::{Role, infer};
use solidify::parser::parse;
use solidify::types::Ty;

fn program(src: &str) -> solidify::infer::Program {
    let module = parse(src).unwrap_or_else(|e| panic!("{}", e.pretty(src)));
    infer(&module, src, &[]).unwrap_or_else(|e| panic!("{}", e.pretty(src)))
}

fn var_ty(p: &solidify::infer::Program, name: &str) -> String {
    let v = p
        .vars
        .values()
        .find(|v| v.name == name)
        .unwrap_or_else(|| panic!("no variable {name}"));
    p.types.describe(&v.ty)
}

#[test]
fn numbers_are_integers_until_proven_otherwise() {
    let p = program(
        "function fib(n: number): number { if (n < 2) return n; return fib(n - 1) + fib(n - 2); }\nlet total = 0; for (let i = 0; i < 10; i++) total += fib(i);\nconst avg = total / 10;\nlet f = 1; f = 2.5;\nlet count = 0; for (const x of [1, 2]) count += x;",
    );
    // `fib` grows exponentially: it returns a double, as in JavaScript, where it cannot
    // overflow; its parameter stays an integer.
    assert_eq!(var_ty(&p, "n"), "number (integer)");
    assert_eq!(var_ty(&p, "total"), "number (float)");
    assert_eq!(var_ty(&p, "count"), "number (integer)");
    assert_eq!(var_ty(&p, "i"), "number (integer)");
    assert_eq!(var_ty(&p, "avg"), "number (float)");
    assert_eq!(var_ty(&p, "f"), "number (float)");
    let total = p.vars.values().find(|v| v.name == "total").unwrap();
    assert!(total.reassigned);
    assert_eq!(total.role, Role::TopLevel);
    assert!(p.has_main);
}

#[test]
fn classes_interfaces_and_options() {
    let p = program(
        "interface Point { x: number; y: number; label?: string }\n\
         class Counter { count = 0; constructor(public name: string) {} inc(): void { this.count++; } get label(): string { return `${this.name}: ${this.count}`; } }\n\
         function describe(p: Point): string { if (p.label !== null && p.label !== undefined) { return p.label; } return `${p.x},${p.y}`; }\n\
         const c = new Counter(\"a\"); c.inc(); const pt: Point = { x: 1, y: 2 }; console.log(describe(pt), c.label);\n\
         let found: Point | null = null; for (const q of [pt]) { if (q.x > 0) found = q; }\n\
         if (found) { console.log(found.x); }",
    );
    assert_eq!(var_ty(&p, "c"), "Counter");
    assert_eq!(var_ty(&p, "found"), "Point | null");
    let c = p.vars.values().find(|v| v.name == "c").unwrap();
    assert!(c.mutated, "inc mutates the counter");
    let inc = p
        .structs
        .iter()
        .find(|s| s.name == "Counter")
        .unwrap()
        .method("inc", solidify::ast::MethodKind::Method)
        .unwrap()
        .fn_id;
    assert!(p.funcs[&inc].mutates_self);
    assert!(p.narrow_tests.values().any(|n| n.path == ["label"]));
    assert!(
        p.narrow_tests
            .values()
            .any(|n| n.path.is_empty() && n.name == "found")
    );
}

#[test]
fn throws_propagate_and_arrays_infer() {
    let p = program(
        "function parse(s: string): number { const n = parseInt(s); if (isNaN(n)) throw new Error(`bad ${s}`); return n; }\n\
         function sum(xs: string[]): number { let t = 0; for (const x of xs) t += parse(x); return t; }\n\
         function safe(xs: string[]): number { try { return sum(xs); } catch (e) { console.log(e.message); return -1; } }\n\
         const words = [\"1\", \"2\"]; const doubled = words.map((w) => parse(w) * 2); const names = doubled.filter((d) => d > 2).map((d) => `n${d}`);",
    );
    let f = |name: &str| {
        p.funcs
            .values()
            .find(|f| f.name.as_deref() == Some(name))
            .unwrap()
    };
    assert!(f("parse").throws);
    assert!(f("sum").throws);
    assert!(!f("safe").throws);
    assert_eq!(var_ty(&p, "doubled"), "number (integer)[]");
    assert_eq!(var_ty(&p, "names"), "string[]");
}

#[test]
fn literal_unions_become_enums_and_objects_get_names() {
    let p = program(
        "type Dir = \"up\" | \"down\";\nfunction step(d: Dir): number { return d === \"up\" ? 1 : -1; }\nconst user = { name: \"a\", age: 3 }; const users = [user, { name: \"b\", age: 4 }];\nconsole.log(step(\"down\"), users.length);",
    );
    assert!(p.enum_info("Dir").is_some());
    assert_eq!(p.enum_literal.len(), 2);
    assert_eq!(var_ty(&p, "user"), "User");
    assert_eq!(var_ty(&p, "users"), "User[]");
}

#[test]
fn unconstrained_parameters_are_generic() {
    let p = program("function first(xs) { return xs[0]; }\nfunction id(x) { return x; }");
    let f = |name: &str| {
        p.funcs
            .values()
            .find(|f| f.name.as_deref() == Some(name))
            .unwrap()
            .clone()
    };
    assert_eq!(f("id").type_params, vec!["T".to_string()]);
    assert!(matches!(f("first").sig.ret, Ty::Named(..)));
}

#[test]
fn errors_are_precise() {
    let src = "let x = 5;\nx = \"a\";";
    let module = parse(src).unwrap();
    let Err(err) = infer(&module, src, &[]) else {
        panic!("expected an error")
    };
    assert_eq!(err.position(src), (2, 5));
    let src = "const p = { x: 1 };\nconsole.log(p.y);";
    let module = parse(src).unwrap();
    let Err(err) = infer(&module, src, &[]) else {
        panic!("expected an error")
    };
    assert!(err.message.contains("no property `y`"), "{}", err.message);
}
