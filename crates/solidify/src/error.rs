//! Errors with a position in the source.

use std::fmt;

/// Byte offsets into the source, `start..end`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Span {
        Span { start, end }
    }

    pub fn to(self, other: Span) -> Span {
        Span {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }
}

/// A syntax or type error, with the place it was found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error {
    pub message: String,
    pub span: Span,
}

impl Error {
    pub fn new(message: impl Into<String>, span: Span) -> Error {
        Error {
            message: message.into(),
            span,
        }
    }

    /// Line and column (1-based) of the error in `src`.
    pub fn position(&self, src: &str) -> (usize, usize) {
        position(src, self.span.start)
    }

    /// `line:col: message`.
    pub fn display(&self, src: &str) -> String {
        let (line, col) = self.position(src);
        format!("{line}:{col}: {}", self.message)
    }

    /// The error with the offending line and a caret under it.
    pub fn pretty(&self, src: &str) -> String {
        let (line, col) = self.position(src);
        let text = src.lines().nth(line - 1).unwrap_or("");
        let width = (self.span.end.saturating_sub(self.span.start)).clamp(1, text.len().max(1));
        format!(
            "error: {}\n  --> {line}:{col}\n   |\n{line:>3}| {text}\n   | {}{}",
            self.message,
            " ".repeat(col - 1),
            "^".repeat(width.min(text.len().saturating_sub(col - 1).max(1)))
        )
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for Error {}

/// Line and column (1-based) of a byte offset.
pub fn position(src: &str, offset: usize) -> (usize, usize) {
    let offset = offset.min(src.len());
    let before = &src[..offset];
    let line = before.matches('\n').count() + 1;
    let col = before.rfind('\n').map_or(before.chars().count(), |nl| {
        before[nl + 1..].chars().count()
    }) + 1;
    (line, col)
}
