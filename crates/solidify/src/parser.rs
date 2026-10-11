//! A recursive-descent parser for the supported subset of TypeScript, with precedence
//! climbing for expressions, automatic semicolons and a little backtracking for arrow
//! functions, generic calls and function types.

use crate::ast::*;
use crate::error::{Error, Span};
use crate::lexer::{Comment, Tok, Token, lex};

/// Words that can never be a variable name.
const RESERVED: &[&str] = &[
    "break",
    "case",
    "catch",
    "class",
    "const",
    "continue",
    "debugger",
    "default",
    "delete",
    "do",
    "else",
    "enum",
    "export",
    "extends",
    "false",
    "finally",
    "for",
    "function",
    "if",
    "import",
    "in",
    "instanceof",
    "new",
    "null",
    "return",
    "super",
    "switch",
    "this",
    "throw",
    "true",
    "try",
    "typeof",
    "var",
    "void",
    "while",
    "with",
];

pub fn parse(src: &str) -> Result<Module, Error> {
    parse_at(src, 0..src.len(), 0)
}

/// Parses the part `range` of `src` (one file of a program held in one text): spans point
/// into `src`, node ids start at `first_id`.
pub fn parse_at(
    src: &str,
    range: std::ops::Range<usize>,
    first_id: NodeId,
) -> Result<Module, Error> {
    let base = range.start;
    let shift = |s: Span| Span::new(s.start + base, s.end + base);
    let (mut tokens, mut comments) =
        lex(&src[range]).map_err(|e| Error::new(e.message, shift(e.span)))?;
    for t in &mut tokens {
        t.span = shift(t.span);
    }
    for c in &mut comments {
        c.span = shift(c.span);
    }
    let mut p = Parser {
        src,
        tokens,
        pos: 0,
        comments,
        comment_idx: 0,
        next_id: first_id,
        no_in: false,
        default_export: None,
    };
    let mut body = Vec::new();
    while !p.at_eof() {
        body.push(p.statement()?);
    }
    Ok(Module {
        body,
        node_count: p.next_id,
        default_export: p.default_export,
    })
}

struct Parser<'a> {
    src: &'a str,
    tokens: Vec<Token>,
    pos: usize,
    comments: Vec<Comment>,
    comment_idx: usize,
    next_id: NodeId,
    /// Inside the head of a `for`: `in` is not a binary operator.
    no_in: bool,
    /// The name given to `export default`.
    default_export: Option<String>,
}

type PResult<T> = Result<T, Error>;

const BINARY_PREC: &[(&str, u8)] = &[
    ("??", 1),
    ("||", 2),
    ("&&", 3),
    ("|", 4),
    ("^", 5),
    ("&", 6),
    ("==", 7),
    ("!=", 7),
    ("===", 7),
    ("!==", 7),
    ("<", 8),
    (">", 8),
    ("<=", 8),
    (">=", 8),
    ("instanceof", 8),
    ("in", 8),
    ("as", 8),
    ("satisfies", 8),
    ("<<", 9),
    (">>", 9),
    (">>>", 9),
    ("+", 10),
    ("-", 10),
    ("*", 11),
    ("/", 11),
];

impl<'a> Parser<'a> {
    // ----- tokens -----

    fn id(&mut self) -> NodeId {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.pos.min(self.tokens.len() - 1)]
    }

    fn peek_at(&self, n: usize) -> &Token {
        &self.tokens[(self.pos + n).min(self.tokens.len() - 1)]
    }

    fn next(&mut self) -> Token {
        let t = self.tokens[self.pos.min(self.tokens.len() - 1)].clone();
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
        t
    }

    fn at_eof(&self) -> bool {
        matches!(self.peek().tok, Tok::Eof)
    }

    fn at_punct(&self, p: &str) -> bool {
        matches!(&self.peek().tok, Tok::Punct(q) if *q == p)
    }

    fn at_punct_at(&self, n: usize, p: &str) -> bool {
        matches!(&self.peek_at(n).tok, Tok::Punct(q) if *q == p)
    }

    fn at_word(&self, w: &str) -> bool {
        matches!(&self.peek().tok, Tok::Ident(x) if x == w)
    }

    fn at_word_at(&self, n: usize, w: &str) -> bool {
        matches!(&self.peek_at(n).tok, Tok::Ident(x) if x == w)
    }

    fn eat_punct(&mut self, p: &str) -> bool {
        if self.at_punct(p) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn eat_word(&mut self, w: &str) -> bool {
        if self.at_word(w) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn describe(&self, t: &Token) -> String {
        match &t.tok {
            Tok::Eof => "end of input".into(),
            Tok::Ident(w) => format!("`{w}`"),
            Tok::Private(w) => format!("`#{w}`"),
            Tok::Punct(p) => format!("`{p}`"),
            Tok::Num { raw, .. } => format!("number `{raw}`"),
            Tok::Str(_) => "a string".into(),
            Tok::Template { .. } => "a template literal".into(),
            Tok::Regex { .. } => "a regular expression".into(),
        }
    }

    fn unexpected<T>(&self, expected: &str) -> PResult<T> {
        let t = self.peek();
        Err(Error::new(
            format!("expected {expected}, found {}", self.describe(t)),
            t.span,
        ))
    }

    fn expect_punct(&mut self, p: &str) -> PResult<Span> {
        if self.at_punct(p) {
            Ok(self.next().span)
        } else {
            self.unexpected(&format!("`{p}`"))
        }
    }

    fn expect_word(&mut self, w: &str) -> PResult<Span> {
        if self.at_word(w) {
            Ok(self.next().span)
        } else {
            self.unexpected(&format!("`{w}`"))
        }
    }

    /// An identifier that can name a variable (contextual keywords allowed).
    fn ident(&mut self) -> PResult<(String, Span)> {
        match &self.peek().tok {
            Tok::Ident(w) if !RESERVED.contains(&w.as_str()) => {
                let w = w.clone();
                Ok((w, self.next().span))
            }
            _ => self.unexpected("a name"),
        }
    }

    /// A property name after `.`: any word, keywords included.
    fn property_name(&mut self) -> PResult<(String, Span)> {
        match &self.peek().tok {
            Tok::Ident(w) => {
                let w = w.clone();
                Ok((w, self.next().span))
            }
            Tok::Private(w) => {
                let w = format!("#{w}");
                Ok((w, self.next().span))
            }
            _ => self.unexpected("a property name"),
        }
    }

    fn at_ident(&self) -> bool {
        matches!(&self.peek().tok, Tok::Ident(w) if !RESERVED.contains(&w.as_str()))
    }

    /// `;`, or nothing before `}`, the end of the input or a line break.
    fn semicolon(&mut self) -> PResult<()> {
        if self.eat_punct(";") || self.at_punct("}") || self.at_eof() || self.peek().nl_before {
            Ok(())
        } else {
            self.unexpected("`;`")
        }
    }

    fn last_span(&self) -> Span {
        self.tokens[self.pos.saturating_sub(1)].span
    }

    fn take_comments_before(&mut self, offset: usize) -> Vec<String> {
        let mut out = Vec::new();
        while self.comment_idx < self.comments.len()
            && self.comments[self.comment_idx].span.end <= offset
        {
            out.extend(self.comments[self.comment_idx].lines.iter().cloned());
            self.comment_idx += 1;
        }
        out
    }

    /// Runs `f`; on failure, rewinds and returns `None`.
    fn attempt<T>(&mut self, f: impl FnOnce(&mut Self) -> PResult<T>) -> Option<T> {
        let (pos, id, comments, no_in) = (self.pos, self.next_id, self.comment_idx, self.no_in);
        match f(self) {
            Ok(v) => Some(v),
            Err(_) => {
                self.pos = pos;
                self.next_id = id;
                self.comment_idx = comments;
                self.no_in = no_in;
                None
            }
        }
    }

    // ----- statements -----

    fn statement(&mut self) -> PResult<Stmt> {
        let comments = self.take_comments_before(self.peek().span.start);
        let start = self.peek().span;
        let kind = self.statement_kind()?;
        Ok(Stmt {
            kind,
            span: start.to(self.last_span()),
            comments,
        })
    }

    fn block(&mut self) -> PResult<Block> {
        let start = self.expect_punct("{")?;
        let mut stmts = Vec::new();
        while !self.at_punct("}") {
            if self.at_eof() {
                return self.unexpected("`}`");
            }
            stmts.push(self.statement()?);
        }
        // Comments at the end of the block are dropped.
        let end = self.expect_punct("}")?;
        self.take_comments_before(end.start);
        Ok(Block {
            stmts,
            span: start.to(end),
        })
    }

    fn statement_kind(&mut self) -> PResult<StmtKind> {
        let t = self.peek().clone();
        match &t.tok {
            Tok::Punct("{") => Ok(StmtKind::Block(self.block()?)),
            Tok::Punct(";") => {
                self.next();
                Ok(StmtKind::Empty)
            }
            Tok::Ident(w) => match w.as_str() {
                "var" | "const" | "let"
                    if w != "let"
                        || matches!(self.peek_at(1).tok, Tok::Ident(_) | Tok::Punct("[" | "{")) =>
                {
                    if w == "const" && self.at_word_at(1, "enum") {
                        self.next();
                        return self.enum_decl(false);
                    }
                    let decl = self.var_decl(false)?;
                    self.semicolon()?;
                    Ok(StmtKind::Var(decl))
                }
                "function" => Ok(StmtKind::Func(self.function(false, false)?)),
                "async" if self.at_word_at(1, "function") && !self.peek_at(1).nl_before => {
                    self.next();
                    Ok(StmtKind::Func(self.function(true, false)?))
                }
                "class" => Ok(StmtKind::Class(self.class(false)?)),
                "abstract" if self.at_word_at(1, "class") => {
                    self.next();
                    let mut class = self.class(false)?;
                    class.is_abstract = true;
                    Ok(StmtKind::Class(class))
                }
                "interface"
                    if matches!(self.peek_at(1).tok, Tok::Ident(_))
                        && !self.peek_at(1).nl_before =>
                {
                    Ok(StmtKind::Interface(self.interface(false)?))
                }
                "type"
                    if matches!(self.peek_at(1).tok, Tok::Ident(_))
                        && (self.at_punct_at(2, "=") || self.at_punct_at(2, "<")) =>
                {
                    Ok(StmtKind::TypeAlias(self.type_alias(false)?))
                }
                "enum" => self.enum_decl(false),
                "declare"
                    if !self.peek_at(1).nl_before
                        && matches!(self.peek_at(1).tok, Tok::Ident(_)) =>
                {
                    // Ambient declarations describe what exists elsewhere: nothing to generate.
                    self.next();
                    self.statement_kind()?;
                    Ok(StmtKind::Empty)
                }
                "namespace" | "module"
                    if matches!(self.peek_at(1).tok, Tok::Ident(_) | Tok::Str(_)) =>
                {
                    Err(Error::new("namespaces are not supported", t.span))
                }
                "if" => self.if_stmt(),
                "while" => {
                    self.next();
                    self.expect_punct("(")?;
                    let test = self.expression()?;
                    self.expect_punct(")")?;
                    let body = Box::new(self.statement()?);
                    Ok(StmtKind::While { test, body })
                }
                "do" => {
                    self.next();
                    let body = Box::new(self.statement()?);
                    self.expect_word("while")?;
                    self.expect_punct("(")?;
                    let test = self.expression()?;
                    self.expect_punct(")")?;
                    self.eat_punct(";");
                    Ok(StmtKind::DoWhile { body, test })
                }
                "for" => self.for_stmt(),
                "switch" => self.switch_stmt(),
                "return" => {
                    self.next();
                    let value = if self.at_punct(";")
                        || self.at_punct("}")
                        || self.at_eof()
                        || self.peek().nl_before
                    {
                        None
                    } else {
                        Some(self.expression()?)
                    };
                    self.semicolon()?;
                    Ok(StmtKind::Return(value))
                }
                "break" | "continue" => {
                    let is_break = w == "break";
                    self.next();
                    let label = if self.at_ident() && !self.peek().nl_before {
                        Some(self.ident()?.0)
                    } else {
                        None
                    };
                    self.semicolon()?;
                    Ok(if is_break {
                        StmtKind::Break(label)
                    } else {
                        StmtKind::Continue(label)
                    })
                }
                "throw" => {
                    self.next();
                    if self.peek().nl_before {
                        return Err(Error::new(
                            "`throw` needs its value on the same line",
                            t.span,
                        ));
                    }
                    let value = self.expression()?;
                    self.semicolon()?;
                    Ok(StmtKind::Throw(value))
                }
                "try" => self.try_stmt(),
                "import" if !self.at_punct_at(1, "(") && !self.at_punct_at(1, ".") => self.import(),
                "export" => self.export(),
                "debugger" => {
                    self.next();
                    self.semicolon()?;
                    Ok(StmtKind::Empty)
                }
                "with" => Err(Error::new("`with` is not supported", t.span)),
                _ if self.at_punct_at(1, ":") && !RESERVED.contains(&w.as_str()) => {
                    let (label, _) = self.ident()?;
                    self.expect_punct(":")?;
                    let body = Box::new(self.statement()?);
                    Ok(StmtKind::Labeled { label, body })
                }
                _ => self.expression_statement(),
            },
            _ => self.expression_statement(),
        }
    }

    fn expression_statement(&mut self) -> PResult<StmtKind> {
        let e = self.expression()?;
        self.semicolon()?;
        Ok(StmtKind::Expr(e))
    }

    fn var_decl(&mut self, exported: bool) -> PResult<VarDecl> {
        let kind = match self.next().tok {
            Tok::Ident(w) if w == "var" => VarKind::Var,
            Tok::Ident(w) if w == "let" => VarKind::Let,
            _ => VarKind::Const,
        };
        let mut decls = Vec::new();
        loop {
            let start = self.peek().span;
            let pat = self.pattern()?;
            let ty = if self.eat_punct(":") {
                Some(self.type_ann()?)
            } else {
                None
            };
            let init = if self.eat_punct("=") {
                Some(self.assignment()?)
            } else {
                None
            };
            decls.push(Declarator {
                pat,
                ty,
                init,
                span: start.to(self.last_span()),
            });
            if !self.eat_punct(",") {
                break;
            }
        }
        Ok(VarDecl {
            kind,
            decls,
            exported,
        })
    }

    fn if_stmt(&mut self) -> PResult<StmtKind> {
        self.expect_word("if")?;
        self.expect_punct("(")?;
        let test = self.expression()?;
        self.expect_punct(")")?;
        let then = Box::new(self.statement()?);
        let otherwise = if self.eat_word("else") {
            Some(Box::new(self.statement()?))
        } else {
            None
        };
        Ok(StmtKind::If {
            test,
            then,
            otherwise,
        })
    }

    fn for_stmt(&mut self) -> PResult<StmtKind> {
        let start = self.expect_word("for")?;
        let is_await = self.eat_word("await");
        if is_await {
            return Err(Error::new("`for await` is not supported", start));
        }
        self.expect_punct("(")?;
        let mut init = None;
        if !self.at_punct(";") {
            let is_decl = self.at_word("var")
                || self.at_word("const")
                || (self.at_word("let")
                    && matches!(self.peek_at(1).tok, Tok::Ident(_) | Tok::Punct("[" | "{")));
            let was = std::mem::replace(&mut self.no_in, true);
            let result = if is_decl {
                let decl_start = self.peek().span;
                let decl = self.var_decl(false)?;
                if (self.at_word("of") || self.at_word("in"))
                    && decl.decls.len() == 1
                    && decl.decls[0].init.is_none()
                {
                    let is_in = self.at_word("in");
                    self.next();
                    self.no_in = was;
                    let iter = self.assignment()?;
                    self.expect_punct(")")?;
                    let body = Box::new(self.statement()?);
                    let pat = decl.decls.into_iter().next().unwrap().pat;
                    return Ok(StmtKind::ForOf {
                        kind: decl.kind,
                        pat,
                        iter,
                        body,
                        is_in,
                    });
                }
                Stmt {
                    kind: StmtKind::Var(decl),
                    span: decl_start.to(self.last_span()),
                    comments: Vec::new(),
                }
            } else {
                let e_start = self.peek().span;
                let e = self.expression()?;
                if self.at_word("of") || self.at_word("in") {
                    let is_in = self.at_word("in");
                    self.next();
                    self.no_in = was;
                    let pat = self.expr_to_pattern(e)?;
                    let iter = self.assignment()?;
                    self.expect_punct(")")?;
                    let body = Box::new(self.statement()?);
                    return Ok(StmtKind::ForOf {
                        kind: VarKind::Let,
                        pat,
                        iter,
                        body,
                        is_in,
                    });
                }
                Stmt {
                    kind: StmtKind::Expr(e),
                    span: e_start.to(self.last_span()),
                    comments: Vec::new(),
                }
            };
            self.no_in = was;
            init = Some(Box::new(result));
        }
        self.expect_punct(";")?;
        let test = if self.at_punct(";") {
            None
        } else {
            Some(self.expression()?)
        };
        self.expect_punct(";")?;
        let update = if self.at_punct(")") {
            None
        } else {
            Some(self.expression()?)
        };
        self.expect_punct(")")?;
        let body = Box::new(self.statement()?);
        Ok(StmtKind::For {
            init,
            test,
            update,
            body,
        })
    }

    fn switch_stmt(&mut self) -> PResult<StmtKind> {
        self.expect_word("switch")?;
        self.expect_punct("(")?;
        let disc = self.expression()?;
        self.expect_punct(")")?;
        self.expect_punct("{")?;
        let mut cases = Vec::new();
        while !self.at_punct("}") {
            let start = self.peek().span;
            let test = if self.eat_word("default") {
                None
            } else {
                self.expect_word("case")?;
                Some(self.expression()?)
            };
            self.expect_punct(":")?;
            let mut body = Vec::new();
            while !self.at_word("case") && !self.at_word("default") && !self.at_punct("}") {
                if self.at_eof() {
                    return self.unexpected("`}`");
                }
                body.push(self.statement()?);
            }
            cases.push(Case {
                test,
                body,
                span: start.to(self.last_span()),
            });
        }
        self.expect_punct("}")?;
        Ok(StmtKind::Switch { disc, cases })
    }

    fn try_stmt(&mut self) -> PResult<StmtKind> {
        let start = self.expect_word("try")?;
        let block = self.block()?;
        let catch = if self.eat_word("catch") {
            let param = if self.eat_punct("(") {
                let p = self.pattern()?;
                if self.eat_punct(":") {
                    self.type_ann()?;
                }
                self.expect_punct(")")?;
                Some(p)
            } else {
                None
            };
            Some(Catch {
                param,
                body: self.block()?,
            })
        } else {
            None
        };
        let finally = if self.eat_word("finally") {
            Some(self.block()?)
        } else {
            None
        };
        if catch.is_none() && finally.is_none() {
            return Err(Error::new("`try` needs a `catch` or a `finally`", start));
        }
        Ok(StmtKind::Try {
            block,
            catch,
            finally,
        })
    }

    fn import(&mut self) -> PResult<StmtKind> {
        self.expect_word("import")?;
        let type_only =
            self.at_word("type") && !self.at_word_at(1, "from") && !self.at_punct_at(1, ",");
        if type_only {
            self.next();
        }
        let mut import = Import {
            default: None,
            namespace: None,
            names: Vec::new(),
            from: String::new(),
            type_only,
        };
        if let Tok::Str(s) = &self.peek().tok {
            import.from = s.clone();
            self.next();
            self.semicolon()?;
            return Ok(StmtKind::Import(import));
        }
        if self.at_ident() {
            import.default = Some(self.ident()?.0);
            if !self.eat_punct(",") {
                self.expect_word("from")?;
                import.from = self.string_literal()?;
                self.semicolon()?;
                return Ok(StmtKind::Import(import));
            }
        }
        if self.eat_punct("*") {
            self.expect_word("as")?;
            import.namespace = Some(self.ident()?.0);
        } else {
            self.expect_punct("{")?;
            while !self.at_punct("}") {
                self.eat_word("type");
                let (name, _) = self.property_name()?;
                let alias = if self.eat_word("as") {
                    Some(self.ident()?.0)
                } else {
                    None
                };
                import.names.push((name, alias));
                if !self.eat_punct(",") {
                    break;
                }
            }
            self.expect_punct("}")?;
        }
        self.expect_word("from")?;
        import.from = self.string_literal()?;
        self.semicolon()?;
        Ok(StmtKind::Import(import))
    }

    fn string_literal(&mut self) -> PResult<String> {
        match &self.peek().tok {
            Tok::Str(s) => {
                let s = s.clone();
                self.next();
                Ok(s)
            }
            _ => self.unexpected("a string"),
        }
    }

    fn export(&mut self) -> PResult<StmtKind> {
        let start = self.expect_word("export")?;
        if self.eat_word("default") {
            if self.at_word("function") {
                let mut f = self.function(false, false)?;
                f.exported = true;
                self.default_export = f.name.clone();
                return Ok(StmtKind::Func(f));
            }
            if self.at_word("async") && self.at_word_at(1, "function") {
                self.next();
                let mut f = self.function(true, false)?;
                f.exported = true;
                self.default_export = f.name.clone();
                return Ok(StmtKind::Func(f));
            }
            if self.at_word("class") {
                let c = self.class(true)?;
                self.default_export = Some(c.name.clone());
                return Ok(StmtKind::Class(c));
            }
            let e = self.assignment()?;
            self.semicolon()?;
            return Ok(StmtKind::ExportDefault(e));
        }
        if self.eat_punct("*") {
            let _ = start;
            self.expect_word("from")?;
            let from = self.string_literal()?;
            self.semicolon()?;
            return Ok(StmtKind::ExportFrom {
                names: Vec::new(),
                all: true,
                from,
            });
        }
        if self.at_punct("{") || (self.at_word("type") && self.at_punct_at(1, "{")) {
            self.eat_word("type");
            self.expect_punct("{")?;
            let mut names = Vec::new();
            while !self.at_punct("}") {
                let (name, _) = self.property_name()?;
                let alias = if self.eat_word("as") {
                    Some(self.property_name()?.0)
                } else {
                    None
                };
                names.push((name, alias));
                if !self.eat_punct(",") {
                    break;
                }
            }
            self.expect_punct("}")?;
            if self.eat_word("from") {
                let from = self.string_literal()?;
                self.semicolon()?;
                return Ok(StmtKind::ExportFrom {
                    names,
                    all: false,
                    from,
                });
            }
            self.semicolon()?;
            return Ok(StmtKind::ExportNames(names));
        }
        let t = self.peek().clone();
        match &t.tok {
            Tok::Ident(w) => match w.as_str() {
                "var" | "let" | "const" => {
                    if w == "const" && self.at_word_at(1, "enum") {
                        self.next();
                        return self.enum_decl(true);
                    }
                    let decl = self.var_decl(true)?;
                    self.semicolon()?;
                    Ok(StmtKind::Var(decl))
                }
                "function" => Ok(StmtKind::Func(self.function(false, true)?)),
                "async" => {
                    self.next();
                    Ok(StmtKind::Func(self.function(true, true)?))
                }
                "class" => Ok(StmtKind::Class(self.class(true)?)),
                "abstract" => {
                    self.next();
                    let mut class = self.class(true)?;
                    class.is_abstract = true;
                    Ok(StmtKind::Class(class))
                }
                "interface" => Ok(StmtKind::Interface(self.interface(true)?)),
                "type" => Ok(StmtKind::TypeAlias(self.type_alias(true)?)),
                "enum" => self.enum_decl(true),
                "declare" => {
                    self.next();
                    self.statement_kind()?;
                    Ok(StmtKind::Empty)
                }
                _ => self.unexpected("a declaration after `export`"),
            },
            _ => self.unexpected("a declaration after `export`"),
        }
    }

    // ----- declarations -----

    fn function(&mut self, is_async: bool, exported: bool) -> PResult<Function> {
        let start = self.expect_word("function")?;
        let is_generator = self.eat_punct("*");
        let name = if self.at_ident() {
            Some(self.ident()?.0)
        } else {
            None
        };
        let id = self.id();
        let type_params = self.type_params()?;
        let params = self.params()?;
        let ret = if self.eat_punct(":") {
            Some(self.return_type()?)
        } else {
            None
        };
        if self.at_punct(";") || (!self.at_punct("{") && name.is_some()) {
            // An overload signature: only the implementation matters.
            self.semicolon()?;
            return Ok(Function {
                name,
                type_params,
                params,
                ret,
                body: FnBody::Block(Block {
                    stmts: Vec::new(),
                    span: start,
                }),
                is_async,
                is_arrow: false,
                is_generator,
                exported,
                span: start.to(self.last_span()),
                id: usize::MAX,
            });
        }
        let body = self.block()?;
        Ok(Function {
            name,
            type_params,
            params,
            ret,
            body: FnBody::Block(body),
            is_async,
            is_arrow: false,
            is_generator,
            exported,
            span: start.to(self.last_span()),
            id,
        })
    }

    /// A return type, which may be a type predicate (`x is string`).
    fn return_type(&mut self) -> PResult<TypeAnn> {
        if (self.at_ident() || self.at_word("this")) && self.at_word_at(1, "is") {
            let start = self.next().span;
            self.next();
            self.type_ann()?;
            return Ok(TypeAnn::Named {
                name: "boolean".into(),
                args: Vec::new(),
                span: start.to(self.last_span()),
            });
        }
        if self.eat_word("asserts") {
            let start = self.last_span();
            self.next();
            if self.eat_word("is") {
                self.type_ann()?;
            }
            return Ok(TypeAnn::Named {
                name: "void".into(),
                args: Vec::new(),
                span: start.to(self.last_span()),
            });
        }
        self.type_ann()
    }

    fn type_params(&mut self) -> PResult<Vec<TypeParam>> {
        let mut out = Vec::new();
        if !self.eat_punct("<") {
            return Ok(out);
        }
        loop {
            let start = self.peek().span;
            self.eat_word("const");
            let (name, _) = self.ident()?;
            let constraint = if self.eat_word("extends") {
                Some(self.type_ann()?)
            } else {
                None
            };
            let default = if self.eat_punct("=") {
                Some(self.type_ann()?)
            } else {
                None
            };
            out.push(TypeParam {
                name,
                constraint,
                default,
                span: start.to(self.last_span()),
            });
            if !self.eat_punct(",") {
                break;
            }
            if self.at_punct(">") {
                break;
            }
        }
        self.expect_punct(">")?;
        Ok(out)
    }

    fn params(&mut self) -> PResult<Vec<Param>> {
        self.expect_punct("(")?;
        let mut out = Vec::new();
        while !self.at_punct(")") {
            let start = self.peek().span;
            let mut access = None;
            let mut readonly = false;
            loop {
                let is_mod = matches!(&self.peek().tok, Tok::Ident(w) if matches!(w.as_str(), "public" | "private" | "protected" | "readonly" | "override"))
                    && matches!(self.peek_at(1).tok, Tok::Ident(_) | Tok::Punct("[" | "{"));
                if !is_mod {
                    break;
                }
                let Tok::Ident(w) = self.next().tok else {
                    unreachable!()
                };
                match w.as_str() {
                    "public" => access = Some(Access::Public),
                    "private" => access = Some(Access::Private),
                    "protected" => access = Some(Access::Protected),
                    "readonly" => readonly = true,
                    _ => {}
                }
            }
            let rest = self.eat_punct("...");
            // `this: T` only types `this`.
            if self.at_word("this") && self.at_punct_at(1, ":") {
                self.next();
                self.next();
                self.type_ann()?;
                if !self.eat_punct(",") {
                    break;
                }
                continue;
            }
            let pat = self.pattern()?;
            let optional = self.eat_punct("?");
            let ty = if self.eat_punct(":") {
                Some(self.type_ann()?)
            } else {
                None
            };
            let default = if self.eat_punct("=") {
                Some(self.assignment()?)
            } else {
                None
            };
            out.push(Param {
                pat,
                ty,
                optional,
                default,
                rest,
                access,
                readonly,
                span: start.to(self.last_span()),
            });
            if !self.eat_punct(",") {
                break;
            }
        }
        self.expect_punct(")")?;
        Ok(out)
    }

    fn class(&mut self, exported: bool) -> PResult<Class> {
        let start = self.expect_word("class")?;
        let name = if self.at_ident() {
            self.ident()?.0
        } else {
            String::new()
        };
        let id = self.id();
        let type_params = self.type_params()?;
        let mut extends_args = Vec::new();
        let extends = if self.eat_word("extends") {
            let e = self.call_member(false)?;
            if self.at_punct("<") {
                extends_args = self.type_args()?;
            }
            Some(e)
        } else {
            None
        };
        let mut implements = Vec::new();
        if self.eat_word("implements") {
            loop {
                implements.push(self.type_ann()?);
                if !self.eat_punct(",") {
                    break;
                }
            }
        }
        self.expect_punct("{")?;
        let mut members = Vec::new();
        while !self.at_punct("}") {
            if self.at_eof() {
                return self.unexpected("`}`");
            }
            if self.eat_punct(";") {
                continue;
            }
            if let Some(m) = self.class_member()? {
                members.push(m);
            }
        }
        self.expect_punct("}")?;
        Ok(Class {
            name,
            type_params,
            extends,
            extends_args,
            implements,
            members,
            exported,
            is_abstract: false,
            span: start.to(self.last_span()),
            id,
        })
    }

    #[allow(clippy::while_let_loop)]
    fn class_member(&mut self) -> PResult<Option<ClassMember>> {
        let start = self.peek().span;
        self.take_comments_before(start.start);
        let mut is_static = false;
        let mut access = None;
        let mut readonly = false;
        let mut is_async = false;
        let mut is_abstract = false;
        loop {
            let Tok::Ident(w) = &self.peek().tok else {
                break;
            };
            let is_mod = matches!(
                w.as_str(),
                "static"
                    | "public"
                    | "private"
                    | "protected"
                    | "readonly"
                    | "abstract"
                    | "declare"
                    | "override"
                    | "async"
                    | "accessor"
            );
            let followed = matches!(
                self.peek_at(1).tok,
                Tok::Ident(_)
                    | Tok::Str(_)
                    | Tok::Num { .. }
                    | Tok::Private(_)
                    | Tok::Punct("[" | "*" | "{")
            ) && !(w == "async" && self.peek_at(1).nl_before);
            if !is_mod || !followed {
                break;
            }
            if w == "static" && self.at_punct_at(1, "{") {
                self.next();
                let block = self.block()?;
                return Ok(Some(ClassMember::StaticBlock(block)));
            }
            match w.as_str() {
                "static" => is_static = true,
                "public" => access = Some(Access::Public),
                "private" => access = Some(Access::Private),
                "protected" => access = Some(Access::Protected),
                "readonly" => readonly = true,
                "async" => is_async = true,
                "abstract" => is_abstract = true,
                _ => {}
            }
            self.next();
        }
        let mut kind = MethodKind::Method;
        if (self.at_word("get") || self.at_word("set"))
            && matches!(
                self.peek_at(1).tok,
                Tok::Ident(_) | Tok::Str(_) | Tok::Num { .. } | Tok::Private(_) | Tok::Punct("[")
            )
        {
            kind = if self.at_word("get") {
                MethodKind::Get
            } else {
                MethodKind::Set
            };
            self.next();
        }
        let is_generator = self.eat_punct("*");
        if self.at_punct("[")
            && (self.at_ident_at(1) || self.at_word_at(1, "this"))
            && self.at_punct_at(2, ":")
        {
            // An index signature: `[key: string]: T`.
            self.next();
            self.next();
            self.next();
            self.type_ann()?;
            self.expect_punct("]")?;
            self.expect_punct(":")?;
            self.type_ann()?;
            self.semicolon_in_class();
            return Ok(None);
        }
        let name = match &self.peek().tok {
            Tok::Ident(w) => {
                let w = w.clone();
                self.next();
                w
            }
            Tok::Private(w) => {
                let w = format!("#{w}");
                self.next();
                w
            }
            Tok::Str(s) => {
                let s = s.clone();
                self.next();
                s
            }
            Tok::Num { raw, .. } => {
                let s = raw.clone();
                self.next();
                s
            }
            Tok::Punct("[") => {
                return Err(Error::new("computed member names are not supported", start));
            }
            _ => return self.unexpected("a member name"),
        };
        if name == "constructor" && kind == MethodKind::Method {
            kind = MethodKind::Constructor;
        }
        let optional = self.eat_punct("?");
        self.eat_punct("!");
        if self.at_punct("(") || self.at_punct("<") {
            let id = self.id();
            let type_params = self.type_params()?;
            let params = self.params()?;
            let ret = if self.eat_punct(":") {
                Some(self.return_type()?)
            } else {
                None
            };
            if !self.at_punct("{") {
                // Abstract method or overload signature.
                self.semicolon_in_class();
                if !is_abstract {
                    return Ok(None);
                }
                let span = start.to(self.last_span());
                return Ok(Some(ClassMember::Method(Method {
                    name,
                    kind,
                    func: Function {
                        name: None,
                        type_params,
                        params,
                        ret,
                        body: FnBody::Block(Block {
                            stmts: Vec::new(),
                            span,
                        }),
                        is_async,
                        is_arrow: false,
                        is_generator,
                        exported: false,
                        span,
                        id,
                    },
                    is_static,
                    access,
                    is_abstract: true,
                    span,
                })));
            }
            let body = self.block()?;
            let span = start.to(self.last_span());
            return Ok(Some(ClassMember::Method(Method {
                name,
                kind,
                func: Function {
                    name: None,
                    type_params,
                    params,
                    ret,
                    body: FnBody::Block(body),
                    is_async,
                    is_arrow: false,
                    is_generator,
                    exported: false,
                    span,
                    id,
                },
                is_static,
                access,
                is_abstract: false,
                span,
            })));
        }
        let ty = if self.eat_punct(":") {
            Some(self.type_ann()?)
        } else {
            None
        };
        let init = if self.eat_punct("=") {
            Some(self.assignment()?)
        } else {
            None
        };
        self.semicolon_in_class();
        Ok(Some(ClassMember::Field(Field {
            name,
            ty,
            init,
            is_static,
            access,
            readonly,
            optional,
            span: start.to(self.last_span()),
        })))
    }

    fn at_ident_at(&self, n: usize) -> bool {
        matches!(&self.peek_at(n).tok, Tok::Ident(w) if !RESERVED.contains(&w.as_str()))
    }

    fn semicolon_in_class(&mut self) {
        self.eat_punct(";");
    }

    fn interface(&mut self, exported: bool) -> PResult<Interface> {
        let start = self.expect_word("interface")?;
        let (name, _) = self.ident()?;
        let type_params = self.type_params()?;
        let mut extends = Vec::new();
        if self.eat_word("extends") {
            loop {
                extends.push(self.type_ann()?);
                if !self.eat_punct(",") {
                    break;
                }
            }
        }
        let members = self.type_members()?;
        Ok(Interface {
            name,
            type_params,
            extends,
            members,
            exported,
            span: start.to(self.last_span()),
        })
    }

    /// `{ a: T; b?(): U; [k: string]: V }`.
    fn type_members(&mut self) -> PResult<Vec<TypeMember>> {
        self.expect_punct("{")?;
        let mut members = Vec::new();
        while !self.at_punct("}") {
            if self.at_eof() {
                return self.unexpected("`}`");
            }
            let start = self.peek().span;
            self.take_comments_before(start.start);
            let mut readonly = false;
            if self.at_word("readonly")
                && matches!(
                    self.peek_at(1).tok,
                    Tok::Ident(_) | Tok::Str(_) | Tok::Punct("[")
                )
            {
                self.next();
                readonly = true;
            }
            if self.at_punct("(") || self.at_punct("<") {
                let type_params = self.type_params()?;
                let params = self.params()?;
                let ret = if self.eat_punct(":") {
                    Some(self.type_ann()?)
                } else {
                    None
                };
                let _ = type_params;
                members.push(TypeMember::Call {
                    params,
                    ret,
                    span: start.to(self.last_span()),
                });
            } else if self.at_punct("[") && self.at_ident_at(1) && self.at_punct_at(2, ":") {
                self.next();
                self.next();
                self.next();
                let key = self.type_ann()?;
                self.expect_punct("]")?;
                self.expect_punct(":")?;
                let value = self.type_ann()?;
                members.push(TypeMember::Index {
                    key,
                    value,
                    span: start.to(self.last_span()),
                });
            } else if self.eat_word("new") {
                return Err(Error::new("construct signatures are not supported", start));
            } else {
                if (self.at_word("get") || self.at_word("set"))
                    && matches!(self.peek_at(1).tok, Tok::Ident(_) | Tok::Str(_))
                {
                    self.next();
                }
                let name = match &self.peek().tok {
                    Tok::Ident(w) => {
                        let w = w.clone();
                        self.next();
                        w
                    }
                    Tok::Str(s) => {
                        let s = s.clone();
                        self.next();
                        s
                    }
                    Tok::Num { raw, .. } => {
                        let s = raw.clone();
                        self.next();
                        s
                    }
                    _ => return self.unexpected("a member name"),
                };
                let optional = self.eat_punct("?");
                if self.at_punct("(") || self.at_punct("<") {
                    let type_params = self.type_params()?;
                    let params = self.params()?;
                    let ret = if self.eat_punct(":") {
                        Some(self.type_ann()?)
                    } else {
                        None
                    };
                    members.push(TypeMember::Method {
                        name,
                        type_params,
                        params,
                        ret,
                        optional,
                        span: start.to(self.last_span()),
                    });
                } else {
                    self.expect_punct(":")?;
                    let ty = self.type_ann()?;
                    members.push(TypeMember::Prop {
                        name,
                        ty,
                        optional,
                        readonly,
                        span: start.to(self.last_span()),
                    });
                }
            }
            if !self.eat_punct(";")
                && !self.eat_punct(",")
                && !self.at_punct("}")
                && !self.peek().nl_before
            {
                return self.unexpected("`;` or `,`");
            }
        }
        self.expect_punct("}")?;
        Ok(members)
    }

    fn type_alias(&mut self, exported: bool) -> PResult<TypeAlias> {
        let start = self.expect_word("type")?;
        let (name, _) = self.ident()?;
        let type_params = self.type_params()?;
        self.expect_punct("=")?;
        let ty = self.type_ann()?;
        self.semicolon()?;
        Ok(TypeAlias {
            name,
            type_params,
            ty,
            exported,
            span: start.to(self.last_span()),
        })
    }

    fn enum_decl(&mut self, exported: bool) -> PResult<StmtKind> {
        let start = self.expect_word("enum")?;
        let (name, _) = self.ident()?;
        self.expect_punct("{")?;
        let mut members = Vec::new();
        while !self.at_punct("}") {
            let m_start = self.peek().span;
            self.take_comments_before(m_start.start);
            let name = match &self.peek().tok {
                Tok::Ident(w) => {
                    let w = w.clone();
                    self.next();
                    w
                }
                Tok::Str(s) => {
                    let s = s.clone();
                    self.next();
                    s
                }
                _ => return self.unexpected("an enum member"),
            };
            let init = if self.eat_punct("=") {
                Some(self.assignment()?)
            } else {
                None
            };
            members.push(EnumMember {
                name,
                init,
                span: m_start.to(self.last_span()),
            });
            if !self.eat_punct(",") {
                break;
            }
        }
        self.expect_punct("}")?;
        Ok(StmtKind::Enum(EnumDecl {
            name,
            members,
            exported,
            span: start.to(self.last_span()),
        }))
    }

    // ----- patterns -----

    fn pattern(&mut self) -> PResult<Pattern> {
        let start = self.peek().span;
        if self.eat_punct("[") {
            let mut elems = Vec::new();
            let mut rest = None;
            while !self.at_punct("]") {
                if self.at_punct(",") {
                    self.next();
                    elems.push(None);
                    continue;
                }
                if self.eat_punct("...") {
                    rest = Some(Box::new(self.pattern()?));
                    self.eat_punct(",");
                    break;
                }
                let pat = self.pattern()?;
                let default = if self.eat_punct("=") {
                    Some(self.assignment()?)
                } else {
                    None
                };
                elems.push(Some(PatElem { pat, default }));
                if !self.eat_punct(",") {
                    break;
                }
            }
            self.expect_punct("]")?;
            return Ok(Pattern::Array {
                elems,
                rest,
                span: start.to(self.last_span()),
            });
        }
        if self.eat_punct("{") {
            let mut props = Vec::new();
            let mut rest = None;
            while !self.at_punct("}") {
                let p_start = self.peek().span;
                if self.eat_punct("...") {
                    rest = Some(Box::new(self.pattern()?));
                    self.eat_punct(",");
                    break;
                }
                let key = match &self.peek().tok {
                    Tok::Ident(w) => {
                        let w = w.clone();
                        self.next();
                        w
                    }
                    Tok::Str(s) => {
                        let s = s.clone();
                        self.next();
                        s
                    }
                    _ => return self.unexpected("a property name"),
                };
                let value = if self.eat_punct(":") {
                    self.pattern()?
                } else {
                    Pattern::Ident {
                        name: key.clone(),
                        span: p_start,
                        id: self.id(),
                    }
                };
                let default = if self.eat_punct("=") {
                    Some(self.assignment()?)
                } else {
                    None
                };
                props.push(PatProp {
                    key,
                    value,
                    default,
                    span: p_start.to(self.last_span()),
                });
                if !self.eat_punct(",") {
                    break;
                }
            }
            self.expect_punct("}")?;
            return Ok(Pattern::Object {
                props,
                rest,
                span: start.to(self.last_span()),
            });
        }
        let (name, span) = self.ident()?;
        Ok(Pattern::Ident {
            name,
            span,
            id: self.id(),
        })
    }

    /// Turns an array or object literal on the left of `=` into a pattern.
    fn expr_to_pattern(&mut self, e: Expr) -> PResult<Pattern> {
        let span = e.span;
        match e.kind {
            ExprKind::Ident(name) => Ok(Pattern::Ident {
                name,
                span,
                id: e.id,
            }),
            ExprKind::Array(elems) => {
                let mut out = Vec::new();
                let mut rest = None;
                for el in elems {
                    match el {
                        ArrayElem::Hole => out.push(None),
                        ArrayElem::Spread(e) => rest = Some(Box::new(self.expr_to_pattern(e)?)),
                        ArrayElem::Expr(e) => {
                            let (pat, default) = match e.kind {
                                ExprKind::Assign {
                                    op: AssignOp::Assign,
                                    target: AssignTarget::Expr(t),
                                    value,
                                } => (self.expr_to_pattern(*t)?, Some(*value)),
                                ExprKind::Assign {
                                    op: AssignOp::Assign,
                                    target: AssignTarget::Pat(p),
                                    value,
                                } => (p, Some(*value)),
                                _ => (self.expr_to_pattern(e)?, None),
                            };
                            out.push(Some(PatElem { pat, default }));
                        }
                    }
                }
                Ok(Pattern::Array {
                    elems: out,
                    rest,
                    span,
                })
            }
            ExprKind::Object(props) => {
                let mut out = Vec::new();
                let mut rest = None;
                for p in props {
                    let key = p.key.name().ok_or_else(|| {
                        Error::new("computed keys are not allowed in a pattern", p.span)
                    })?;
                    match p.value {
                        PropValue::Shorthand(id) => out.push(PatProp {
                            key: key.clone(),
                            value: Pattern::Ident {
                                name: key,
                                span: p.span,
                                id,
                            },
                            default: None,
                            span: p.span,
                        }),
                        PropValue::Expr(e) => {
                            let (value, default) = match e.kind {
                                ExprKind::Assign {
                                    op: AssignOp::Assign,
                                    target: AssignTarget::Expr(t),
                                    value,
                                } => (self.expr_to_pattern(*t)?, Some(*value)),
                                _ => (self.expr_to_pattern(e)?, None),
                            };
                            out.push(PatProp {
                                key,
                                value,
                                default,
                                span: p.span,
                            });
                        }
                        PropValue::Spread(e) => rest = Some(Box::new(self.expr_to_pattern(e)?)),
                        _ => return Err(Error::new("invalid assignment target", p.span)),
                    }
                }
                Ok(Pattern::Object {
                    props: out,
                    rest,
                    span,
                })
            }
            _ => Err(Error::new("invalid assignment target", span)),
        }
    }

    // ----- types -----

    fn type_ann(&mut self) -> PResult<TypeAnn> {
        let start = self.peek().span;
        self.eat_punct("|");
        let mut members = vec![self.intersection_type()?];
        while self.eat_punct("|") {
            members.push(self.intersection_type()?);
        }
        let ty = if members.len() == 1 {
            members.pop().unwrap()
        } else {
            TypeAnn::Union(members, start.to(self.last_span()))
        };
        if self.at_word("extends") && !self.peek().nl_before {
            return Err(Error::new(
                "conditional types are not supported",
                self.peek().span,
            ));
        }
        Ok(ty)
    }

    fn intersection_type(&mut self) -> PResult<TypeAnn> {
        let start = self.peek().span;
        self.eat_punct("&");
        let mut members = vec![self.postfix_type()?];
        while self.eat_punct("&") {
            members.push(self.postfix_type()?);
        }
        Ok(if members.len() == 1 {
            members.pop().unwrap()
        } else {
            TypeAnn::Intersection(members, start.to(self.last_span()))
        })
    }

    fn postfix_type(&mut self) -> PResult<TypeAnn> {
        let start = self.peek().span;
        let mut ty = self.primary_type()?;
        while self.at_punct("[") && !self.peek().nl_before {
            self.next();
            if self.eat_punct("]") {
                ty = TypeAnn::Array(Box::new(ty), start.to(self.last_span()));
            } else {
                let index = self.type_ann()?;
                self.expect_punct("]")?;
                ty = TypeAnn::Indexed(Box::new(ty), Box::new(index), start.to(self.last_span()));
            }
        }
        Ok(ty)
    }

    fn primary_type(&mut self) -> PResult<TypeAnn> {
        let t = self.peek().clone();
        let start = t.span;
        match &t.tok {
            Tok::Punct("(") => {
                if let Some(f) = self.attempt(|p| p.function_type()) {
                    return Ok(f);
                }
                self.next();
                let inner = self.type_ann()?;
                self.expect_punct(")")?;
                Ok(inner)
            }
            Tok::Punct("<") => self.function_type(),
            Tok::Punct("{") => {
                let members = self.type_members()?;
                Ok(TypeAnn::Object(members, start.to(self.last_span())))
            }
            Tok::Punct("[") => {
                self.next();
                let mut elems = Vec::new();
                while !self.at_punct("]") {
                    // Named tuple members: `[x: number, y: number]`.
                    if self.at_ident()
                        && (self.at_punct_at(1, ":")
                            || (self.at_punct_at(1, "?") && self.at_punct_at(2, ":")))
                    {
                        self.next();
                        self.eat_punct("?");
                        self.next();
                    }
                    if self.eat_punct("...") {
                        return Err(Error::new(
                            "rest elements in tuple types are not supported",
                            start,
                        ));
                    }
                    let mut el = self.type_ann()?;
                    if self.eat_punct("?") {
                        let span = el.span();
                        el = TypeAnn::Union(vec![el, TypeAnn::Lit(Lit::Undefined, span)], span);
                    }
                    elems.push(el);
                    if !self.eat_punct(",") {
                        break;
                    }
                }
                self.expect_punct("]")?;
                Ok(TypeAnn::Tuple(elems, start.to(self.last_span())))
            }
            Tok::Str(s) => {
                let s = s.clone();
                self.next();
                Ok(TypeAnn::Lit(Lit::Str(s), start))
            }
            Tok::Num { value, .. } => {
                let v = *value;
                self.next();
                Ok(TypeAnn::Lit(Lit::Num(v), start))
            }
            Tok::Punct("-") => {
                self.next();
                match &self.peek().tok {
                    Tok::Num { value, .. } => {
                        let v = -*value;
                        self.next();
                        Ok(TypeAnn::Lit(Lit::Num(v), start.to(self.last_span())))
                    }
                    _ => self.unexpected("a number"),
                }
            }
            Tok::Template { .. } => Err(Error::new(
                "template literal types are not supported",
                start,
            )),
            Tok::Ident(w) => match w.as_str() {
                "true" | "false" => {
                    self.next();
                    Ok(TypeAnn::Lit(Lit::Bool(w == "true"), start))
                }
                "null" => {
                    self.next();
                    Ok(TypeAnn::Lit(Lit::Null, start))
                }
                "undefined" => {
                    self.next();
                    Ok(TypeAnn::Lit(Lit::Undefined, start))
                }
                "typeof" => {
                    self.next();
                    let (name, _) = self.ident()?;
                    let mut full = name;
                    while self.eat_punct(".") {
                        full.push('.');
                        full.push_str(&self.property_name()?.0);
                    }
                    Ok(TypeAnn::TypeOf(full, start.to(self.last_span())))
                }
                "keyof" => {
                    self.next();
                    let inner = self.postfix_type()?;
                    Ok(TypeAnn::KeyOf(Box::new(inner), start.to(self.last_span())))
                }
                "readonly" => {
                    self.next();
                    let inner = self.postfix_type()?;
                    Ok(TypeAnn::Readonly(
                        Box::new(inner),
                        start.to(self.last_span()),
                    ))
                }
                "infer" => Err(Error::new("`infer` is not supported", start)),
                "new" => Err(Error::new("constructor types are not supported", start)),
                "this" | "void" => {
                    self.next();
                    Ok(TypeAnn::Named {
                        name: w.clone(),
                        args: Vec::new(),
                        span: start,
                    })
                }
                _ => {
                    let (mut name, _) = self.ident()?;
                    while self.at_punct(".") && matches!(self.peek_at(1).tok, Tok::Ident(_)) {
                        self.next();
                        name.push('.');
                        name.push_str(&self.property_name()?.0);
                    }
                    let args = if self.at_punct("<") && !self.peek().nl_before {
                        self.type_args()?
                    } else {
                        Vec::new()
                    };
                    Ok(TypeAnn::Named {
                        name,
                        args,
                        span: start.to(self.last_span()),
                    })
                }
            },
            _ => self.unexpected("a type"),
        }
    }

    fn type_args(&mut self) -> PResult<Vec<TypeAnn>> {
        self.expect_punct("<")?;
        let mut args = Vec::new();
        while !self.at_punct(">") {
            args.push(self.type_ann()?);
            if !self.eat_punct(",") {
                break;
            }
        }
        // `>>` closes two levels at once: split it.
        if self.at_punct(">>")
            || self.at_punct(">>>")
            || self.at_punct(">=")
            || self.at_punct(">>=")
        {
            let t = self.peek().clone();
            let Tok::Punct(p) = t.tok else { unreachable!() };
            let rest: &'static str = match p {
                ">>" => ">",
                ">>>" => ">>",
                ">=" => "=",
                _ => ">=",
            };
            self.tokens[self.pos] = Token {
                tok: Tok::Punct(rest),
                span: Span::new(t.span.start + 1, t.span.end),
                nl_before: false,
            };
            return Ok(args);
        }
        self.expect_punct(">")?;
        Ok(args)
    }

    /// `(a: T) => U`, `<T>(a: T) => T`.
    fn function_type(&mut self) -> PResult<TypeAnn> {
        let start = self.peek().span;
        let type_params = self.type_params()?;
        let params = self.params()?;
        self.expect_punct("=>")?;
        let ret = self.type_ann()?;
        Ok(TypeAnn::Func {
            type_params,
            params,
            ret: Box::new(ret),
            span: start.to(self.last_span()),
        })
    }

    // ----- expressions -----

    fn expression(&mut self) -> PResult<Expr> {
        let start = self.peek().span;
        let first = self.assignment()?;
        if !self.at_punct(",") {
            return Ok(first);
        }
        let mut items = vec![first];
        while self.eat_punct(",") {
            items.push(self.assignment()?);
        }
        Ok(Expr {
            kind: ExprKind::Seq(items),
            span: start.to(self.last_span()),
            id: self.id(),
        })
    }

    fn expr(&mut self, kind: ExprKind, start: Span) -> Expr {
        Expr {
            kind,
            span: start.to(self.last_span()),
            id: self.id(),
        }
    }

    fn assignment(&mut self) -> PResult<Expr> {
        let start = self.peek().span;
        if let Some(arrow) = self.try_arrow()? {
            return Ok(arrow);
        }
        if self.at_word("yield") {
            self.next();
            let delegate = self.eat_punct("*");
            let ends = self.at_punct(";")
                || self.at_punct(")")
                || self.at_punct("]")
                || self.at_punct("}")
                || self.at_punct(",")
                || self.at_eof()
                || self.peek().nl_before;
            let value = if ends && !delegate {
                None
            } else {
                Some(Box::new(self.assignment()?))
            };
            return Ok(self.expr(ExprKind::Yield { value, delegate }, start));
        }
        let left = self.conditional()?;
        let op = match &self.peek().tok {
            Tok::Punct(p) => match *p {
                "=" => AssignOp::Assign,
                "+=" => AssignOp::Add,
                "-=" => AssignOp::Sub,
                "*=" => AssignOp::Mul,
                "/=" => AssignOp::Div,
                "%=" => AssignOp::Mod,
                "**=" => AssignOp::Pow,
                "<<=" => AssignOp::Shl,
                ">>=" => AssignOp::Shr,
                ">>>=" => AssignOp::UShr,
                "&=" => AssignOp::BitAnd,
                "|=" => AssignOp::BitOr,
                "^=" => AssignOp::BitXor,
                "&&=" => AssignOp::And,
                "||=" => AssignOp::Or,
                "??=" => AssignOp::Nullish,
                _ => return Ok(left),
            },
            _ => return Ok(left),
        };
        self.next();
        let target = match (&left.kind, op) {
            (ExprKind::Array(_) | ExprKind::Object(_), AssignOp::Assign) => {
                AssignTarget::Pat(self.expr_to_pattern(left)?)
            }
            (
                ExprKind::Ident(_)
                | ExprKind::Member { .. }
                | ExprKind::Index { .. }
                | ExprKind::NonNull(_),
                _,
            ) => AssignTarget::Expr(Box::new(left)),
            _ => return Err(Error::new("invalid assignment target", left.span)),
        };
        let value = Box::new(self.assignment()?);
        Ok(self.expr(ExprKind::Assign { op, target, value }, start))
    }

    /// An arrow function, if one starts here.
    fn try_arrow(&mut self) -> PResult<Option<Expr>> {
        let start = self.peek().span;
        let is_async = self.at_word("async")
            && !self.peek_at(1).nl_before
            && (self.at_ident_at(1) && self.at_punct_at(2, "=>")
                || self.at_punct_at(1, "(")
                || self.at_punct_at(1, "<"));
        let offset = usize::from(is_async);
        // `x => …`
        if self.at_ident_at(offset)
            && self.at_punct_at(offset + 1, "=>")
            && !self.peek_at(offset + 1).nl_before
        {
            if is_async {
                self.next();
            }
            let (name, span) = self.ident()?;
            let id = self.id();
            let param = Param {
                pat: Pattern::Ident { name, span, id },
                ty: None,
                optional: false,
                default: None,
                rest: false,
                access: None,
                readonly: false,
                span,
            };
            self.expect_punct("=>")?;
            return Ok(Some(self.arrow_body(
                vec![],
                vec![param],
                None,
                is_async,
                start,
            )?));
        }
        if !(self.at_punct_at(offset, "(") || self.at_punct_at(offset, "<")) {
            return Ok(None);
        }
        let head = self.attempt(|p| {
            if is_async {
                p.next();
            }
            let type_params = p.type_params()?;
            let params = p.params()?;
            let ret = if p.eat_punct(":") {
                Some(p.return_type()?)
            } else {
                None
            };
            if p.peek().nl_before {
                return p.unexpected("`=>`");
            }
            p.expect_punct("=>")?;
            Ok((type_params, params, ret))
        });
        match head {
            Some((type_params, params, ret)) => Ok(Some(self.arrow_body(
                type_params,
                params,
                ret,
                is_async,
                start,
            )?)),
            None => Ok(None),
        }
    }

    fn arrow_body(
        &mut self,
        type_params: Vec<TypeParam>,
        params: Vec<Param>,
        ret: Option<TypeAnn>,
        is_async: bool,
        start: Span,
    ) -> PResult<Expr> {
        let id = self.id();
        let body = if self.at_punct("{") {
            let was = std::mem::replace(&mut self.no_in, false);
            let block = self.block();
            self.no_in = was;
            FnBody::Block(block?)
        } else {
            FnBody::Expr(Box::new(self.assignment()?))
        };
        let func = Function {
            name: None,
            type_params,
            params,
            ret,
            body,
            is_async,
            is_arrow: true,
            is_generator: false,
            exported: false,
            span: start.to(self.last_span()),
            id,
        };
        Ok(self.expr(ExprKind::Arrow(Box::new(func)), start))
    }

    fn conditional(&mut self) -> PResult<Expr> {
        let start = self.peek().span;
        let test = self.binary(0)?;
        if !self.at_punct("?") {
            return Ok(test);
        }
        self.next();
        let was = std::mem::replace(&mut self.no_in, false);
        let cons = self.assignment()?;
        self.no_in = was;
        self.expect_punct(":")?;
        let alt = self.assignment()?;
        Ok(self.expr(
            ExprKind::Cond {
                test: Box::new(test),
                cons: Box::new(cons),
                alt: Box::new(alt),
            },
            start,
        ))
    }

    fn binary_op(&self) -> Option<(&'static str, u8)> {
        match &self.peek().tok {
            Tok::Punct(p) => BINARY_PREC
                .iter()
                .find(|(s, _)| s == p)
                .copied()
                .or(match *p {
                    "%" => Some(("%", 11)),
                    "**" => Some(("**", 12)),
                    _ => None,
                }),
            Tok::Ident(w) => match w.as_str() {
                "instanceof" => Some(("instanceof", 8)),
                "in" if !self.no_in => Some(("in", 8)),
                "as" | "satisfies" if !self.peek().nl_before => {
                    Some((if w == "as" { "as" } else { "satisfies" }, 8))
                }
                _ => None,
            },
            _ => None,
        }
    }

    #[allow(clippy::while_let_loop)]
    fn binary(&mut self, min_prec: u8) -> PResult<Expr> {
        let start = self.peek().span;
        let mut left = self.unary()?;
        loop {
            let Some((op, prec)) = self.binary_op() else {
                break;
            };
            if prec < min_prec {
                break;
            }
            self.next();
            match op {
                "as" => {
                    let ty = if self.at_word("const") {
                        let span = self.next().span;
                        TypeAnn::Named {
                            name: "const".into(),
                            args: Vec::new(),
                            span,
                        }
                    } else {
                        self.type_ann()?
                    };
                    left = self.expr(ExprKind::As(Box::new(left), ty), start);
                    continue;
                }
                "satisfies" => {
                    self.type_ann()?;
                    continue;
                }
                _ => {}
            }
            let right = if op == "**" {
                self.binary(prec)?
            } else {
                self.binary(prec + 1)?
            };
            let kind = match op {
                "&&" => ExprKind::Logical {
                    op: LogicalOp::And,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                "||" => ExprKind::Logical {
                    op: LogicalOp::Or,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                "??" => ExprKind::Logical {
                    op: LogicalOp::Nullish,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                _ => {
                    let op = match op {
                        "+" => BinOp::Add,
                        "-" => BinOp::Sub,
                        "*" => BinOp::Mul,
                        "/" => BinOp::Div,
                        "%" => BinOp::Mod,
                        "**" => BinOp::Pow,
                        "==" => BinOp::Eq,
                        "!=" => BinOp::NotEq,
                        "===" => BinOp::StrictEq,
                        "!==" => BinOp::StrictNotEq,
                        "<" => BinOp::Lt,
                        ">" => BinOp::Gt,
                        "<=" => BinOp::LtEq,
                        ">=" => BinOp::GtEq,
                        "<<" => BinOp::Shl,
                        ">>" => BinOp::Shr,
                        ">>>" => BinOp::UShr,
                        "&" => BinOp::BitAnd,
                        "|" => BinOp::BitOr,
                        "^" => BinOp::BitXor,
                        "in" => BinOp::In,
                        _ => BinOp::InstanceOf,
                    };
                    ExprKind::Binary {
                        op,
                        left: Box::new(left),
                        right: Box::new(right),
                    }
                }
            };
            left = self.expr(kind, start);
        }
        Ok(left)
    }

    fn unary(&mut self) -> PResult<Expr> {
        let start = self.peek().span;
        let t = self.peek().clone();
        let op = match &t.tok {
            Tok::Punct("!") => Some(UnaryOp::Not),
            Tok::Punct("~") => Some(UnaryOp::BitNot),
            Tok::Punct("+") => Some(UnaryOp::Pos),
            Tok::Punct("-") => Some(UnaryOp::Neg),
            _ => None,
        };
        if let Some(op) = op {
            self.next();
            let arg = Box::new(self.unary()?);
            return Ok(self.expr(ExprKind::Unary { op, arg }, start));
        }
        if self.at_punct("++") || self.at_punct("--") {
            let inc = self.at_punct("++");
            self.next();
            let arg = Box::new(self.unary()?);
            return Ok(self.expr(
                ExprKind::Update {
                    inc,
                    prefix: true,
                    arg,
                },
                start,
            ));
        }
        if let Tok::Ident(w) = &t.tok {
            match w.as_str() {
                "typeof" => {
                    self.next();
                    let arg = Box::new(self.unary()?);
                    return Ok(self.expr(ExprKind::TypeOf(arg), start));
                }
                "void" => {
                    self.next();
                    let arg = Box::new(self.unary()?);
                    return Ok(self.expr(ExprKind::Void(arg), start));
                }
                "delete" => {
                    self.next();
                    let arg = Box::new(self.unary()?);
                    return Ok(self.expr(ExprKind::Delete(arg), start));
                }
                "await"
                    if !self.peek_at(1).nl_before
                        && !matches!(
                            self.peek_at(1).tok,
                            Tok::Punct(")" | ";" | "," | "]" | "}" | "=" | "=>")
                        ) =>
                {
                    self.next();
                    let arg = Box::new(self.unary()?);
                    return Ok(self.expr(ExprKind::Await(arg), start));
                }
                _ => {}
            }
        }
        self.postfix()
    }

    fn postfix(&mut self) -> PResult<Expr> {
        let start = self.peek().span;
        let mut e = self.call_member(true)?;
        loop {
            if (self.at_punct("++") || self.at_punct("--")) && !self.peek().nl_before {
                let inc = self.at_punct("++");
                self.next();
                e = self.expr(
                    ExprKind::Update {
                        inc,
                        prefix: false,
                        arg: Box::new(e),
                    },
                    start,
                );
            } else if self.at_punct("!") && !self.peek().nl_before {
                self.next();
                e = self.expr(ExprKind::NonNull(Box::new(e)), start);
                // Members may follow: `a!.b`.
                e = self.member_suffixes(e, start, true)?;
            } else {
                break;
            }
        }
        Ok(e)
    }

    fn call_member(&mut self, allow_call: bool) -> PResult<Expr> {
        let start = self.peek().span;
        let e = if self.at_word("new") {
            self.new_expr()?
        } else {
            self.primary()?
        };
        self.member_suffixes(e, start, allow_call)
    }

    fn member_suffixes(&mut self, mut e: Expr, start: Span, allow_call: bool) -> PResult<Expr> {
        loop {
            match &self.peek().tok {
                Tok::Punct(".") => {
                    self.next();
                    let (prop, _) = self.property_name()?;
                    e = self.expr(
                        ExprKind::Member {
                            obj: Box::new(e),
                            prop,
                            optional: false,
                        },
                        start,
                    );
                }
                Tok::Punct("?.") => {
                    self.next();
                    if self.at_punct("(") {
                        let args = self.arguments()?;
                        e = self.expr(
                            ExprKind::Call {
                                callee: Box::new(e),
                                args,
                                optional: true,
                                type_args: Vec::new(),
                            },
                            start,
                        );
                    } else if self.eat_punct("[") {
                        let was = std::mem::replace(&mut self.no_in, false);
                        let index = self.expression()?;
                        self.no_in = was;
                        self.expect_punct("]")?;
                        e = self.expr(
                            ExprKind::Index {
                                obj: Box::new(e),
                                index: Box::new(index),
                                optional: true,
                            },
                            start,
                        );
                    } else {
                        let (prop, _) = self.property_name()?;
                        e = self.expr(
                            ExprKind::Member {
                                obj: Box::new(e),
                                prop,
                                optional: true,
                            },
                            start,
                        );
                    }
                }
                Tok::Punct("[") => {
                    self.next();
                    let was = std::mem::replace(&mut self.no_in, false);
                    let index = self.expression()?;
                    self.no_in = was;
                    self.expect_punct("]")?;
                    e = self.expr(
                        ExprKind::Index {
                            obj: Box::new(e),
                            index: Box::new(index),
                            optional: false,
                        },
                        start,
                    );
                }
                Tok::Punct("(") if allow_call => {
                    let args = self.arguments()?;
                    e = self.expr(
                        ExprKind::Call {
                            callee: Box::new(e),
                            args,
                            optional: false,
                            type_args: Vec::new(),
                        },
                        start,
                    );
                }
                Tok::Punct("<") if allow_call => {
                    let Some(type_args) = self.attempt(|p| {
                        let args = p.type_args()?;
                        if !p.at_punct("(") {
                            return p.unexpected("`(`");
                        }
                        Ok(args)
                    }) else {
                        break;
                    };
                    let args = self.arguments()?;
                    e = self.expr(
                        ExprKind::Call {
                            callee: Box::new(e),
                            args,
                            optional: false,
                            type_args,
                        },
                        start,
                    );
                }
                Tok::Template { .. } => {
                    let Tok::Template { quasis, exprs } = self.next().tok else {
                        unreachable!()
                    };
                    let exprs = self.template_exprs(exprs)?;
                    e = self.expr(
                        ExprKind::Tagged {
                            tag: Box::new(e),
                            quasis,
                            exprs,
                        },
                        start,
                    );
                }
                _ => break,
            }
        }
        Ok(e)
    }

    fn new_expr(&mut self) -> PResult<Expr> {
        let start = self.expect_word("new")?;
        if self.at_punct(".") {
            return Err(Error::new("`new.target` is not supported", start));
        }
        let callee = self.call_member(false)?;
        let type_args = if self.at_punct("<") {
            self.attempt(|p| p.type_args()).unwrap_or_default()
        } else {
            Vec::new()
        };
        let args = if self.at_punct("(") {
            self.arguments()?
        } else {
            Vec::new()
        };
        Ok(self.expr(
            ExprKind::New {
                callee: Box::new(callee),
                args,
                type_args,
            },
            start,
        ))
    }

    fn arguments(&mut self) -> PResult<Vec<Arg>> {
        self.expect_punct("(")?;
        let was = std::mem::replace(&mut self.no_in, false);
        let mut args = Vec::new();
        while !self.at_punct(")") {
            let spread = self.eat_punct("...");
            let expr = self.assignment()?;
            args.push(Arg { expr, spread });
            if !self.eat_punct(",") {
                break;
            }
        }
        self.no_in = was;
        self.expect_punct(")")?;
        Ok(args)
    }

    fn template_exprs(&mut self, exprs: Vec<Vec<Token>>) -> PResult<Vec<Expr>> {
        let mut out = Vec::new();
        for tokens in exprs {
            let end = tokens.last().map_or(0, |t| t.span.end);
            let mut tokens = tokens;
            tokens.push(Token {
                tok: Tok::Eof,
                span: Span::new(end, end),
                nl_before: false,
            });
            let mut sub = Parser {
                src: self.src,
                tokens,
                pos: 0,
                comments: Vec::new(),
                comment_idx: 0,
                next_id: self.next_id,
                no_in: false,
                default_export: None,
            };
            let e = sub.expression()?;
            if !sub.at_eof() {
                return sub.unexpected("`}`");
            }
            self.next_id = sub.next_id;
            out.push(e);
        }
        Ok(out)
    }

    fn primary(&mut self) -> PResult<Expr> {
        let t = self.peek().clone();
        let start = t.span;
        match t.tok {
            Tok::Num { value, raw, int } => {
                self.next();
                Ok(self.expr(ExprKind::Num { value, raw, int }, start))
            }
            Tok::Str(s) => {
                self.next();
                Ok(self.expr(ExprKind::Str(s), start))
            }
            Tok::Template { quasis, exprs } => {
                self.next();
                let exprs = self.template_exprs(exprs)?;
                Ok(self.expr(ExprKind::Template { quasis, exprs }, start))
            }
            Tok::Regex { pattern, flags } => {
                self.next();
                Ok(self.expr(ExprKind::Regex { pattern, flags }, start))
            }
            Tok::Punct("(") => {
                self.next();
                let was = std::mem::replace(&mut self.no_in, false);
                let e = self.expression()?;
                self.no_in = was;
                self.expect_punct(")")?;
                Ok(e)
            }
            Tok::Punct("[") => self.array_literal(),
            Tok::Punct("{") => self.object_literal(),
            Tok::Ident(w) => match w.as_str() {
                "function" => {
                    let f = self.function(false, false)?;
                    Ok(self.expr(ExprKind::Func(Box::new(f)), start))
                }
                "async" if self.at_word_at(1, "function") && !self.peek_at(1).nl_before => {
                    self.next();
                    let f = self.function(true, false)?;
                    Ok(self.expr(ExprKind::Func(Box::new(f)), start))
                }
                "class" => {
                    let c = self.class(false)?;
                    Ok(self.expr(ExprKind::Class(Box::new(c)), start))
                }
                "this" => {
                    self.next();
                    Ok(self.expr(ExprKind::This, start))
                }
                "super" => {
                    self.next();
                    Ok(self.expr(ExprKind::Super, start))
                }
                "null" => {
                    self.next();
                    Ok(self.expr(ExprKind::Null, start))
                }
                "undefined" => {
                    self.next();
                    Ok(self.expr(ExprKind::Undefined, start))
                }
                "true" | "false" => {
                    self.next();
                    Ok(self.expr(ExprKind::Bool(w == "true"), start))
                }
                "new" => self.new_expr(),
                _ if RESERVED.contains(&w.as_str()) => self.unexpected("an expression"),
                _ => {
                    self.next();
                    Ok(self.expr(ExprKind::Ident(w), start))
                }
            },
            _ => self.unexpected("an expression"),
        }
    }

    fn array_literal(&mut self) -> PResult<Expr> {
        let start = self.expect_punct("[")?;
        let was = std::mem::replace(&mut self.no_in, false);
        let mut elems = Vec::new();
        while !self.at_punct("]") {
            if self.at_punct(",") {
                self.next();
                elems.push(ArrayElem::Hole);
                continue;
            }
            if self.eat_punct("...") {
                elems.push(ArrayElem::Spread(self.assignment()?));
            } else {
                elems.push(ArrayElem::Expr(self.assignment()?));
            }
            if !self.eat_punct(",") {
                break;
            }
        }
        self.no_in = was;
        self.expect_punct("]")?;
        Ok(self.expr(ExprKind::Array(elems), start))
    }

    fn object_literal(&mut self) -> PResult<Expr> {
        let start = self.expect_punct("{")?;
        let was = std::mem::replace(&mut self.no_in, false);
        let mut props = Vec::new();
        while !self.at_punct("}") {
            let p_start = self.peek().span;
            if self.eat_punct("...") {
                let e = self.assignment()?;
                props.push(Prop {
                    key: PropKey::Ident(String::new()),
                    value: PropValue::Spread(e),
                    span: p_start.to(self.last_span()),
                });
            } else {
                let mut is_async = false;
                let mut accessor = None;
                let key_follows = |p: &Self, n: usize| {
                    matches!(
                        p.peek_at(n).tok,
                        Tok::Ident(_) | Tok::Str(_) | Tok::Num { .. } | Tok::Punct("[" | "*")
                    )
                };
                if self.at_word("async") && key_follows(self, 1) && !self.peek_at(1).nl_before {
                    self.next();
                    is_async = true;
                }
                if (self.at_word("get") || self.at_word("set")) && key_follows(self, 1) {
                    accessor = Some(self.at_word("get"));
                    self.next();
                }
                let is_generator = self.eat_punct("*");
                let key = match &self.peek().tok {
                    Tok::Ident(w) => {
                        let w = w.clone();
                        self.next();
                        PropKey::Ident(w)
                    }
                    Tok::Str(s) => {
                        let s = s.clone();
                        self.next();
                        PropKey::Str(s)
                    }
                    Tok::Num { value, .. } => {
                        let v = *value;
                        self.next();
                        PropKey::Num(v)
                    }
                    Tok::Punct("[") => {
                        self.next();
                        let e = self.assignment()?;
                        self.expect_punct("]")?;
                        PropKey::Computed(e)
                    }
                    _ => return self.unexpected("a property name"),
                };
                let value = if self.at_punct("(") || self.at_punct("<") {
                    let id = self.id();
                    let type_params = self.type_params()?;
                    let params = self.params()?;
                    let ret = if self.eat_punct(":") {
                        Some(self.type_ann()?)
                    } else {
                        None
                    };
                    let body = self.block()?;
                    let func = Function {
                        name: key.name(),
                        type_params,
                        params,
                        ret,
                        body: FnBody::Block(body),
                        is_async,
                        is_arrow: false,
                        is_generator,
                        exported: false,
                        span: p_start.to(self.last_span()),
                        id,
                    };
                    match accessor {
                        Some(true) => PropValue::Get(func),
                        Some(false) => PropValue::Set(func),
                        None => PropValue::Method(func),
                    }
                } else if self.eat_punct(":") {
                    PropValue::Expr(self.assignment()?)
                } else if let PropKey::Ident(name) = &key {
                    if RESERVED.contains(&name.as_str()) {
                        return self.unexpected("`:`");
                    }
                    let id = self.id();
                    if self.at_punct("=") {
                        // `{ x = 1 }` only makes sense as a pattern with a default.
                        self.next();
                        let default = self.assignment()?;
                        let target = Expr {
                            kind: ExprKind::Ident(name.clone()),
                            span: p_start,
                            id,
                        };
                        let assign = self.expr(
                            ExprKind::Assign {
                                op: AssignOp::Assign,
                                target: AssignTarget::Expr(Box::new(target)),
                                value: Box::new(default),
                            },
                            p_start,
                        );
                        PropValue::Expr(assign)
                    } else {
                        PropValue::Shorthand(id)
                    }
                } else {
                    return self.unexpected("`:`");
                };
                props.push(Prop {
                    key,
                    value,
                    span: p_start.to(self.last_span()),
                });
            }
            if !self.eat_punct(",") {
                break;
            }
        }
        self.no_in = was;
        self.expect_punct("}")?;
        Ok(self.expr(ExprKind::Object(props), start))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(src: &str) -> Module {
        match parse(src) {
            Ok(m) => m,
            Err(e) => panic!("{}", e.pretty(src)),
        }
    }

    #[test]
    fn parses_declarations() {
        let m = ok(
            "export function f<T>(a: T, b?: number[] = [], ...rest: string[]): T | null { return a; }\n\
                    class A extends B implements C { private x = 1; static y: string; constructor(public z: number) {} get v() { return 1 } m(): void {} }\n\
                    interface I extends J { a: number; b?(x: string): void; readonly c: string[] }\n\
                    type U = 'a' | 'b' | { x: number }\n\
                    enum E { A, B = 2 }\n\
                    const [a, , b = 1, ...c] = xs, { d, e: f = 2 } = o;",
        );
        assert_eq!(m.body.len(), 6);
        assert!(matches!(
            m.body[0].kind,
            StmtKind::Func(Function { exported: true, .. })
        ));
        assert!(matches!(&m.body[1].kind, StmtKind::Class(c) if c.members.len() == 5));
    }

    #[test]
    fn arrow_functions_and_calls() {
        let m = ok(
            "const f = (a, b: number): number => a + b; const g = async x => await x; h(1)(2).k?.[3]; f<T>(1); x = (a) ? (b) : c;",
        );
        assert_eq!(m.body.len(), 5);
        let StmtKind::Var(v) = &m.body[0].kind else {
            panic!()
        };
        assert!(matches!(
            v.decls[0].init.as_ref().unwrap().kind,
            ExprKind::Arrow(_)
        ));
        let StmtKind::Expr(e) = &m.body[4].kind else {
            panic!()
        };
        let ExprKind::Assign { value, .. } = &e.kind else {
            panic!()
        };
        assert!(matches!(value.kind, ExprKind::Cond { .. }));
    }

    #[test]
    fn asi_and_control_flow() {
        let m = ok(
            "let x = 1\nx++\nfor (const a of b) {}\nfor (let i = 0; i < n; i++) continue\nswitch (x) { case 1: break; default: }\ntry { f() } catch (e) { } finally { }\nif (a) b\nelse c\nlabel: while (true) break label\nreturn_ = 3",
        );
        assert_eq!(m.body.len(), 9);
    }

    #[test]
    fn templates_and_objects() {
        let m = ok(
            "const s = `a${b + `c${d}`}e`; const o = { a, b: 1, [c]: 2, m() {}, get g() { return 1 }, ...r };",
        );
        assert_eq!(m.body.len(), 2);
    }

    #[test]
    fn types() {
        ok(
            "let a: Array<Map<string, number[]>>; let b: (x: number) => void; let c: [number, string?]; let d: typeof x; let e: keyof T; let f: { [k: string]: number }; let g: readonly string[]; let h: <T>(x: T) => T; let i: A.B<C>; let j: (A | B)[]",
        );
    }

    #[test]
    fn errors_point_at_the_problem() {
        let e = parse("let x = ;").unwrap_err();
        assert_eq!(e.span.start, 8);
        assert!(e.message.contains("expected an expression"));
    }
}
