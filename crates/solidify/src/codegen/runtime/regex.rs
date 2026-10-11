/// A regular expression (`/pattern/flags`): parsed once, matched by backtracking on the
/// characters of the text, with JavaScript's syntax and semantics.
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct JsRegex {
    root: Vec<Vec<RxNode>>,
    groups: usize,
    names: Vec<(String, usize)>,
    ignore_case: bool,
    multiline: bool,
    dot_all: bool,
    global: bool,
    source: String,
    flags: String,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
enum RxNode {
    Char(char),
    Any,
    /// Ranges of characters, and whether the class is negated.
    Class(Vec<(char, char)>, bool),
    Start,
    End,
    /// `\b` (true) or `\B` (false).
    WordBoundary(bool),
    /// Alternatives, and the capture index.
    Group(Vec<Vec<RxNode>>, Option<usize>),
    /// Lookahead: alternatives, and whether it is negative.
    Look(Vec<Vec<RxNode>>, bool),
    Repeat(Box<RxNode>, usize, Option<usize>, bool),
    BackRef(usize),
    /// `\p{…}` (negated: `\P{…}`): a Unicode property, by name.
    Prop(String, bool),
}

/// A match: the character index, the text, the full match and the groups (`None` for a
/// group that took no part).
#[derive(Clone, Debug, PartialEq)]
#[allow(dead_code)]
pub struct JsMatch {
    pub index: i64,
    pub input: String,
    pub groups: Vec<Option<String>>,
}

#[allow(dead_code)]
impl JsMatch {
    /// `m[i]`: a group, empty when it took no part.
    pub fn get(&self, i: i64) -> String {
        self.groups
            .get(i as usize)
            .cloned()
            .flatten()
            .unwrap_or_default()
    }

    pub fn len(&self) -> usize {
        self.groups.len()
    }

    pub fn is_empty(&self) -> bool {
        self.groups.is_empty()
    }
}

type RxCaps = Vec<Option<(usize, usize)>>;

#[allow(dead_code)]
impl JsRegex {
    /// Parses `source` with `flags`; a bad pattern is a panic (the translator checks it).
    pub fn new(source: &str, flags: &str) -> JsRegex {
        match JsRegex::parse(source, flags) {
            Ok(re) => re,
            Err(e) => panic!("invalid regular expression /{source}/{flags}: {e}"),
        }
    }

    pub fn parse(source: &str, flags: &str) -> Result<JsRegex, String> {
        let chars: Vec<char> = source.chars().collect();
        let mut p = RxParser {
            chars: &chars,
            pos: 0,
            groups: 0,
            names: Vec::new(),
        };
        let root = p.alternatives()?;
        if p.pos < chars.len() {
            return Err(format!("unexpected `{}`", chars[p.pos]));
        }
        for f in flags.chars() {
            if !"gimsuyd".contains(f) {
                return Err(format!("unknown flag `{f}`"));
            }
        }
        Ok(JsRegex {
            root,
            groups: p.groups,
            names: p.names,
            ignore_case: flags.contains('i'),
            multiline: flags.contains('m'),
            dot_all: flags.contains('s'),
            global: flags.contains('g'),
            source: source.to_string(),
            flags: flags.to_string(),
        })
    }

    fn same(&self, a: char, b: char) -> bool {
        a == b || (self.ignore_case && a.to_lowercase().eq(b.to_lowercase()))
    }

    fn in_class(&self, c: char, ranges: &[(char, char)], negated: bool) -> bool {
        let mut hit = ranges.iter().any(|&(lo, hi)| lo <= c && c <= hi);
        if !hit && self.ignore_case {
            let lower = c.to_lowercase().next().unwrap_or(c);
            let upper = c.to_uppercase().next().unwrap_or(c);
            hit = ranges
                .iter()
                .any(|&(lo, hi)| (lo <= lower && lower <= hi) || (lo <= upper && upper <= hi));
        }
        hit != negated
    }

    fn is_word(c: char) -> bool {
        c == '_' || c.is_ascii_alphanumeric()
    }

    /// The Unicode properties understood: letters, numbers, cases, spaces, punctuation.
    fn has_property(c: char, name: &str) -> bool {
        match name {
            "L" | "Letter" | "Alphabetic" | "Alpha" => c.is_alphabetic(),
            "Lu" | "Uppercase_Letter" | "Uppercase" | "Upper" => c.is_uppercase(),
            "Ll" | "Lowercase_Letter" | "Lowercase" | "Lower" => c.is_lowercase(),
            "N" | "Number" | "Nd" | "Decimal_Number" | "digit" => c.is_numeric(),
            "White_Space" | "space" | "Zs" => c.is_whitespace(),
            "P" | "Punctuation" | "punct" => {
                c.is_ascii_punctuation()
                    || (!c.is_alphanumeric()
                        && !c.is_whitespace()
                        && !c.is_control()
                        && (c as u32) > 127
                        && !c.is_ascii())
            }
            "S" | "Symbol" => "$+<=>^`|~".contains(c),
            "Cc" | "Control" => c.is_control(),
            _ => false,
        }
    }

    /// Matches `seq` at `pos`, then calls `k` with where it ended.
    fn seq(
        &self,
        text: &[char],
        seq: &[RxNode],
        pos: usize,
        caps: &mut RxCaps,
        k: &mut dyn FnMut(usize, &mut RxCaps) -> bool,
    ) -> bool {
        let Some((node, rest)) = seq.split_first() else {
            return k(pos, caps);
        };
        match node {
            RxNode::Char(c) => {
                pos < text.len()
                    && self.same(text[pos], *c)
                    && self.seq(text, rest, pos + 1, caps, k)
            }
            RxNode::Any => {
                pos < text.len()
                    && (self.dot_all || text[pos] != '\n')
                    && self.seq(text, rest, pos + 1, caps, k)
            }
            RxNode::Class(ranges, negated) => {
                pos < text.len()
                    && self.in_class(text[pos], ranges, *negated)
                    && self.seq(text, rest, pos + 1, caps, k)
            }
            RxNode::Prop(name, negated) => {
                pos < text.len()
                    && (Self::has_property(text[pos], name) != *negated)
                    && self.seq(text, rest, pos + 1, caps, k)
            }
            RxNode::Start => {
                (pos == 0 || (self.multiline && text[pos - 1] == '\n'))
                    && self.seq(text, rest, pos, caps, k)
            }
            RxNode::End => {
                (pos == text.len() || (self.multiline && text[pos] == '\n'))
                    && self.seq(text, rest, pos, caps, k)
            }
            RxNode::WordBoundary(expected) => {
                let before = pos > 0 && Self::is_word(text[pos - 1]);
                let after = pos < text.len() && Self::is_word(text[pos]);
                ((before != after) == *expected) && self.seq(text, rest, pos, caps, k)
            }
            RxNode::Group(alts, cap) => {
                for alt in alts {
                    let saved = caps.clone();
                    let matched = self.seq(text, alt, pos, caps, &mut |end, caps| {
                        if let Some(i) = cap {
                            caps[*i] = Some((pos, end));
                        }
                        self.seq(text, rest, end, caps, k)
                    });
                    if matched {
                        return true;
                    }
                    *caps = saved;
                }
                false
            }
            RxNode::Look(alts, negative) => {
                let saved = caps.clone();
                let found = alts
                    .iter()
                    .any(|alt| self.seq(text, alt, pos, caps, &mut |_, _| true));
                if found == *negative {
                    *caps = saved;
                    return false;
                }
                if *negative {
                    *caps = saved;
                }
                self.seq(text, rest, pos, caps, k)
            }
            RxNode::Repeat(inner, min, max, greedy) => {
                self.repeat(text, inner, *min, *max, *greedy, 0, pos, caps, rest, k)
            }
            RxNode::BackRef(i) => {
                let Some(Some((a, b))) = caps.get(*i).copied() else {
                    return self.seq(text, rest, pos, caps, k);
                };
                let len = b - a;
                if pos + len > text.len() {
                    return false;
                }
                for j in 0..len {
                    if !self.same(text[pos + j], text[a + j]) {
                        return false;
                    }
                }
                self.seq(text, rest, pos + len, caps, k)
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn repeat(
        &self,
        text: &[char],
        inner: &RxNode,
        min: usize,
        max: Option<usize>,
        greedy: bool,
        count: usize,
        pos: usize,
        caps: &mut RxCaps,
        rest: &[RxNode],
        k: &mut dyn FnMut(usize, &mut RxCaps) -> bool,
    ) -> bool {
        let can_more = max.is_none_or(|m| count < m);
        let more = |this: &Self,
                    caps: &mut RxCaps,
                    k: &mut dyn FnMut(usize, &mut RxCaps) -> bool|
         -> bool {
            can_more
                && this.seq(
                    text,
                    std::slice::from_ref(inner),
                    pos,
                    caps,
                    &mut |end, caps| {
                        // An empty iteration would loop forever.
                        if end == pos && count >= min {
                            return false;
                        }
                        this.repeat(text, inner, min, max, greedy, count + 1, end, caps, rest, k)
                    },
                )
        };
        if greedy {
            if more(self, caps, k) {
                return true;
            }
            count >= min && self.seq(text, rest, pos, caps, k)
        } else {
            if count >= min && self.seq(text, rest, pos, caps, k) {
                return true;
            }
            more(self, caps, k)
        }
    }

    /// The first match at or after character `from`.
    fn find_from(&self, text: &[char], input: &str, from: usize) -> Option<JsMatch> {
        for start in from..=text.len() {
            let mut caps: RxCaps = vec![None; self.groups + 1];
            let mut found: Option<usize> = None;
            let matched = self.seq(
                text,
                &[RxNode::Group(self.root.clone(), Some(0))],
                start,
                &mut caps,
                &mut |end, _| {
                    found = Some(end);
                    true
                },
            );
            if matched {
                let groups = caps
                    .iter()
                    .map(|c| c.map(|(a, b)| text[a..b].iter().collect::<String>()))
                    .collect();
                let _ = found;
                return Some(JsMatch {
                    index: start as i64,
                    input: input.to_string(),
                    groups,
                });
            }
        }
        None
    }

    fn chars(input: &str) -> Vec<char> {
        input.chars().collect()
    }

    /// `re.test(s)`.
    pub fn test(&self, input: &str) -> bool {
        self.find_from(&Self::chars(input), input, 0).is_some()
    }

    /// `re.exec(s)` / `s.match(re)` without the `g` flag.
    pub fn exec(&self, input: &str) -> Option<JsMatch> {
        self.find_from(&Self::chars(input), input, 0)
    }

    /// Every match, as `s.matchAll(re)` gives them.
    pub fn find_all(&self, input: &str) -> Vec<JsMatch> {
        let text = Self::chars(input);
        let mut out = Vec::new();
        let mut from = 0;
        while from <= text.len() {
            let Some(m) = self.find_from(&text, input, from) else {
                break;
            };
            let len = m.groups[0].as_ref().map_or(0, |g| g.chars().count());
            from = m.index as usize + len.max(1);
            out.push(m);
        }
        out
    }

    /// `s.match(re)` with the `g` flag: the full matches.
    pub fn match_all(&self, input: &str) -> Option<Vec<String>> {
        let all: Vec<String> = self.find_all(input).into_iter().map(|m| m.get(0)).collect();
        if all.is_empty() { None } else { Some(all) }
    }

    /// `s.search(re)`: the index of the first match, or -1.
    pub fn search(&self, input: &str) -> i64 {
        self.exec(input).map_or(-1, |m| m.index)
    }

    /// `s.replace(re, text)`: `$1`, `$&`, `$$` and `$<name>` are expanded; every match when
    /// the flag `g` is set.
    pub fn replace(&self, input: &str, replacement: &str) -> String {
        self.replace_with(input, |m| self.expand(replacement, m))
    }

    /// `s.replace(re, (match, ...groups) => text)`.
    pub fn replace_with(&self, input: &str, mut f: impl FnMut(&JsMatch) -> String) -> String {
        let text = Self::chars(input);
        let matches = if self.global {
            self.find_all(input)
        } else {
            self.exec(input).into_iter().collect()
        };
        let mut out = String::new();
        let mut last = 0;
        for m in &matches {
            let start = m.index as usize;
            let end = start + m.groups[0].as_ref().map_or(0, |g| g.chars().count());
            out.extend(text[last..start].iter());
            out.push_str(&f(m));
            last = end;
        }
        out.extend(text[last..].iter());
        out
    }

    fn expand(&self, replacement: &str, m: &JsMatch) -> String {
        let mut out = String::new();
        let mut chars = replacement.chars().peekable();
        while let Some(c) = chars.next() {
            if c != '$' {
                out.push(c);
                continue;
            }
            match chars.peek().copied() {
                Some('$') => {
                    chars.next();
                    out.push('$');
                }
                Some('&') => {
                    chars.next();
                    out.push_str(&m.get(0));
                }
                Some(d) if d.is_ascii_digit() => {
                    chars.next();
                    let mut n = d.to_digit(10).unwrap_or(0) as usize;
                    if let Some(d2) = chars.peek().copied().filter(|c| c.is_ascii_digit())
                        && n * 10 + d2.to_digit(10).unwrap_or(0) as usize <= self.groups
                    {
                        chars.next();
                        n = n * 10 + d2.to_digit(10).unwrap_or(0) as usize;
                    }
                    if n >= 1 && n <= self.groups {
                        out.push_str(&m.get(n as i64));
                    } else {
                        out.push('$');
                        out.push_str(&n.to_string());
                    }
                }
                Some('<') => {
                    chars.next();
                    let name: String = chars.by_ref().take_while(|c| *c != '>').collect();
                    if let Some((_, i)) = self.names.iter().find(|(n, _)| *n == name) {
                        out.push_str(&m.get(*i as i64));
                    }
                }
                _ => out.push('$'),
            }
        }
        out
    }

    /// `s.split(re)`.
    pub fn split(&self, input: &str) -> Vec<String> {
        let text = Self::chars(input);
        if text.is_empty() {
            return if self.exec("").is_some() {
                Vec::new()
            } else {
                vec![String::new()]
            };
        }
        let mut out = Vec::new();
        let mut last = 0;
        let mut from = 0;
        while from < text.len() {
            let Some(m) = self.find_from(&text, input, from) else {
                break;
            };
            let start = m.index as usize;
            let len = m.groups[0].as_ref().map_or(0, |g| g.chars().count());
            if len == 0 {
                if start >= text.len() {
                    break;
                }
                if start == last {
                    from = start + 1;
                    if start == 0 {
                        continue;
                    }
                }
            }
            if start >= text.len() {
                break;
            }
            out.push(text[last..start].iter().collect());
            for g in m.groups.iter().skip(1) {
                out.push(g.clone().unwrap_or_default());
            }
            last = start + len;
            from = if len == 0 { start + 1 } else { last };
        }
        out.push(text[last..].iter().collect());
        out
    }

    /// `re.source`.
    pub fn source(&self) -> String {
        self.source.clone()
    }

    /// `re.flags`.
    pub fn flags(&self) -> String {
        self.flags.clone()
    }
}

impl std::fmt::Display for JsRegex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "/{}/{}", self.source, self.flags)
    }
}

struct RxParser<'a> {
    chars: &'a [char],
    pos: usize,
    groups: usize,
    names: Vec<(String, usize)>,
}

impl RxParser<'_> {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn eat(&mut self, c: char) -> bool {
        if self.peek() == Some(c) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn alternatives(&mut self) -> Result<Vec<Vec<RxNode>>, String> {
        let mut alts = vec![self.sequence()?];
        while self.eat('|') {
            alts.push(self.sequence()?);
        }
        Ok(alts)
    }

    fn sequence(&mut self) -> Result<Vec<RxNode>, String> {
        let mut out = Vec::new();
        while let Some(c) = self.peek() {
            if c == '|' || c == ')' {
                break;
            }
            let atom = self.atom()?;
            let atom = self.quantified(atom)?;
            out.push(atom);
        }
        Ok(out)
    }

    fn quantified(&mut self, atom: RxNode) -> Result<RxNode, String> {
        let (min, max) = match self.peek() {
            Some('*') => {
                self.pos += 1;
                (0, None)
            }
            Some('+') => {
                self.pos += 1;
                (1, None)
            }
            Some('?') => {
                self.pos += 1;
                (0, Some(1))
            }
            Some('{') => {
                let save = self.pos;
                self.pos += 1;
                let min = self.number();
                let (min, max) = match (min, self.peek()) {
                    (Some(n), Some('}')) => (n, Some(n)),
                    (Some(n), Some(',')) => {
                        self.pos += 1;
                        if self.peek() == Some('}') {
                            (n, None)
                        } else {
                            match self.number() {
                                Some(m) => (n, Some(m)),
                                None => {
                                    self.pos = save;
                                    return Ok(atom);
                                }
                            }
                        }
                    }
                    _ => {
                        self.pos = save;
                        return Ok(atom);
                    }
                };
                if !self.eat('}') {
                    self.pos = save;
                    return Ok(atom);
                }
                (min, max)
            }
            _ => return Ok(atom),
        };
        if matches!(
            atom,
            RxNode::Start | RxNode::End | RxNode::WordBoundary(_) | RxNode::Look(..)
        ) {
            return Err("nothing to repeat".into());
        }
        let greedy = !self.eat('?');
        Ok(RxNode::Repeat(Box::new(atom), min, max, greedy))
    }

    fn number(&mut self) -> Option<usize> {
        let start = self.pos;
        while self.peek().is_some_and(|c| c.is_ascii_digit()) {
            self.pos += 1;
        }
        if start == self.pos {
            return None;
        }
        self.chars[start..self.pos]
            .iter()
            .collect::<String>()
            .parse()
            .ok()
    }

    fn atom(&mut self) -> Result<RxNode, String> {
        let c = self.peek().ok_or("unexpected end")?;
        self.pos += 1;
        Ok(match c {
            '.' => RxNode::Any,
            '^' => RxNode::Start,
            '$' => RxNode::End,
            '(' => {
                let mut cap = None;
                let mut look = None;
                if self.eat('?') {
                    match self.peek() {
                        Some(':') => {
                            self.pos += 1;
                        }
                        Some('=') => {
                            self.pos += 1;
                            look = Some(false);
                        }
                        Some('!') => {
                            self.pos += 1;
                            look = Some(true);
                        }
                        Some('<') => {
                            self.pos += 1;
                            if self.peek() == Some('=') || self.peek() == Some('!') {
                                return Err("lookbehind is not supported".into());
                            }
                            let name: String = std::iter::from_fn(|| {
                                let c = self.peek()?;
                                if c == '>' {
                                    None
                                } else {
                                    self.pos += 1;
                                    Some(c)
                                }
                            })
                            .collect();
                            if !self.eat('>') {
                                return Err("unterminated group name".into());
                            }
                            self.groups += 1;
                            self.names.push((name, self.groups));
                            cap = Some(self.groups);
                        }
                        _ => return Err("unknown group kind".into()),
                    }
                } else {
                    self.groups += 1;
                    cap = Some(self.groups);
                }
                let alts = self.alternatives()?;
                if !self.eat(')') {
                    return Err("missing `)`".into());
                }
                match look {
                    Some(negative) => RxNode::Look(alts, negative),
                    None => RxNode::Group(alts, cap),
                }
            }
            ')' => return Err("unmatched `)`".into()),
            '[' => self.class()?,
            '\\' => self.escape(false)?,
            '*' | '+' | '?' => return Err("nothing to repeat".into()),
            c => RxNode::Char(c),
        })
    }

    fn class(&mut self) -> Result<RxNode, String> {
        let negated = self.eat('^');
        let mut ranges = Vec::new();
        let mut first = true;
        loop {
            let c = self.peek().ok_or("unterminated character class")?;
            if c == ']' && !first {
                self.pos += 1;
                break;
            }
            if c == ']' {
                self.pos += 1;
                break;
            }
            first = false;
            self.pos += 1;
            let lo = if c == '\\' {
                match self.escape(true)? {
                    RxNode::Char(c) => c,
                    RxNode::Class(mut r, false) => {
                        ranges.append(&mut r);
                        continue;
                    }
                    RxNode::Class(r, true) => {
                        // A negated class inside a class: its complement.
                        ranges.extend(complement(&r));
                        continue;
                    }
                    _ => return Err("unexpected escape in a class".into()),
                }
            } else {
                c
            };
            if self.peek() == Some('-') && self.chars.get(self.pos + 1).is_some_and(|c| *c != ']') {
                self.pos += 1;
                let hi_c = self.peek().ok_or("unterminated character class")?;
                self.pos += 1;
                let hi = if hi_c == '\\' {
                    match self.escape(true)? {
                        RxNode::Char(c) => c,
                        _ => return Err("bad range".into()),
                    }
                } else {
                    hi_c
                };
                if hi < lo {
                    return Err("range out of order".into());
                }
                ranges.push((lo, hi));
            } else {
                ranges.push((lo, lo));
            }
        }
        Ok(RxNode::Class(ranges, negated))
    }

    fn escape(&mut self, in_class: bool) -> Result<RxNode, String> {
        let c = self.peek().ok_or("trailing backslash")?;
        self.pos += 1;
        let digit = vec![('0', '9')];
        let word = vec![('0', '9'), ('A', 'Z'), ('_', '_'), ('a', 'z')];
        let space = vec![
            (' ', ' '),
            ('\t', '\t'),
            ('\n', '\n'),
            ('\r', '\r'),
            ('\u{b}', '\u{c}'),
            ('\u{a0}', '\u{a0}'),
            ('\u{feff}', '\u{feff}'),
            ('\u{2028}', '\u{2029}'),
        ];
        Ok(match c {
            'd' => RxNode::Class(digit, false),
            'D' => RxNode::Class(digit, true),
            'w' => RxNode::Class(word, false),
            'W' => RxNode::Class(word, true),
            's' => RxNode::Class(space, false),
            'S' => RxNode::Class(space, true),
            'b' if in_class => RxNode::Char('\u{8}'),
            'b' => RxNode::WordBoundary(true),
            'B' => RxNode::WordBoundary(false),
            'n' => RxNode::Char('\n'),
            't' => RxNode::Char('\t'),
            'r' => RxNode::Char('\r'),
            'f' => RxNode::Char('\u{c}'),
            'v' => RxNode::Char('\u{b}'),
            '0' => RxNode::Char('\0'),
            'p' | 'P' if !in_class => {
                if !self.eat('{') {
                    return Err("`\\p` needs `{…}`".into());
                }
                let name: String = std::iter::from_fn(|| {
                    let c = self.peek()?;
                    if c == '}' {
                        None
                    } else {
                        self.pos += 1;
                        Some(c)
                    }
                })
                .collect();
                if !self.eat('}') {
                    return Err("unterminated `\\p{…}`".into());
                }
                let name = name.rsplit('=').next().unwrap_or(&name).to_string();
                if !matches!(
                    name.as_str(),
                    "L" | "Letter"
                        | "Alphabetic"
                        | "Alpha"
                        | "Lu"
                        | "Uppercase_Letter"
                        | "Uppercase"
                        | "Upper"
                        | "Ll"
                        | "Lowercase_Letter"
                        | "Lowercase"
                        | "Lower"
                        | "N"
                        | "Number"
                        | "Nd"
                        | "Decimal_Number"
                        | "digit"
                        | "White_Space"
                        | "space"
                        | "Zs"
                        | "P"
                        | "Punctuation"
                        | "punct"
                        | "S"
                        | "Symbol"
                        | "Cc"
                        | "Control"
                ) {
                    return Err(format!("the Unicode property `{name}` is not supported"));
                }
                RxNode::Prop(name, c == 'P')
            }
            'x' => RxNode::Char(self.hex(2)?),
            'u' => {
                if self.eat('{') {
                    let start = self.pos;
                    while self.peek().is_some_and(|c| c != '}') {
                        self.pos += 1;
                    }
                    let hex: String = self.chars[start..self.pos].iter().collect();
                    self.eat('}');
                    let code = u32::from_str_radix(&hex, 16).map_err(|_| "bad \\u{…}")?;
                    RxNode::Char(char::from_u32(code).ok_or("bad code point")?)
                } else {
                    RxNode::Char(self.hex(4)?)
                }
            }
            '1'..='9' if !in_class => {
                let mut n = c.to_digit(10).unwrap_or(0) as usize;
                while let Some(d) = self.peek().and_then(|c| c.to_digit(10)) {
                    self.pos += 1;
                    n = n * 10 + d as usize;
                }
                RxNode::BackRef(n)
            }
            'k' if !in_class => {
                if !self.eat('<') {
                    return Err("bad named reference".into());
                }
                let name: String = std::iter::from_fn(|| {
                    let c = self.peek()?;
                    if c == '>' {
                        None
                    } else {
                        self.pos += 1;
                        Some(c)
                    }
                })
                .collect();
                self.eat('>');
                let (_, i) = self
                    .names
                    .iter()
                    .find(|(n, _)| *n == name)
                    .ok_or("unknown group name")?;
                RxNode::BackRef(*i)
            }
            c => RxNode::Char(c),
        })
    }

    fn hex(&mut self, n: usize) -> Result<char, String> {
        let hex: String = self.chars[self.pos..(self.pos + n).min(self.chars.len())]
            .iter()
            .collect();
        if hex.len() != n {
            return Err("bad escape".into());
        }
        self.pos += n;
        let code = u32::from_str_radix(&hex, 16).map_err(|_| "bad escape")?;
        char::from_u32(code).ok_or_else(|| "bad escape".into())
    }
}

/// The ranges outside the given ones.
fn complement(ranges: &[(char, char)]) -> Vec<(char, char)> {
    let mut sorted: Vec<(char, char)> = ranges.to_vec();
    sorted.sort();
    let mut out = Vec::new();
    let mut next = '\0';
    for (lo, hi) in sorted {
        if lo > next {
            out.push((next, char::from_u32(lo as u32 - 1).unwrap_or(next)));
        }
        if let Some(c) = char::from_u32(hi as u32 + 1) {
            next = c;
        } else {
            return out;
        }
    }
    out.push((next, char::MAX));
    out
}
