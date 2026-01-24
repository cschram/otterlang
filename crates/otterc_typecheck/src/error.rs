use log::debug;
use otterc_span::Span;
use otterc_symbol::SymbolError;
use otterc_utils::errors::{Diagnostic, DiagnosticSeverity};

/// This result is used to track when an error has occurred.
/// Errors are collected separately in `TypeChecker`'s state, but we want to be able to
/// short-circuit certain operations when an error occurs.
pub type TypeCheckResult<T> = Result<T, ()>;

/// Type checking error
#[derive(Debug, Clone)]
pub struct TypeError {
    pub message: String,
    pub hint: Option<String>,
    pub help: Option<String>,
    pub suggestion: Option<String>,
    pub span: Option<Span>,
}

impl TypeError {
    pub fn new(message: String) -> Self {
        debug!("TypeError created: {}", message);
        Self {
            message,
            hint: None,
            help: None,
            suggestion: None,
            span: None,
        }
    }

    pub fn with_hint(mut self, hint: String) -> Self {
        self.hint = Some(hint);
        self
    }

    pub fn with_help(mut self, help: String) -> Self {
        self.help = Some(help);
        self
    }

    pub fn with_suggestion(mut self, suggestion: String) -> Self {
        self.suggestion = Some(suggestion);
        self
    }

    pub fn with_span(mut self, span: Span) -> Self {
        self.span = Some(span);
        self
    }

    pub fn with_optional_span(mut self, span: Option<Span>) -> Self {
        self.span = span;
        self
    }

    pub fn to_diagnostic(&self, source_id: &str, source: &str) -> Diagnostic {
        let span = self.span.unwrap_or_else(|| guess_span(self, source));
        let mut diagnostic = Diagnostic::new(
            DiagnosticSeverity::Error,
            source_id.to_string(),
            span,
            self.message.clone(),
        );

        if let Some(suggestion) = &self.suggestion {
            diagnostic = diagnostic.with_suggestion(suggestion.clone());
        }

        match (&self.hint, &self.help) {
            (Some(hint), Some(help)) => {
                diagnostic = diagnostic.with_help(format!("{}\n{}", hint, help));
            }
            (Some(hint), None) => {
                diagnostic = diagnostic.with_help(hint.clone());
            }
            (None, Some(help)) => {
                diagnostic = diagnostic.with_help(help.clone());
            }
            (None, None) => {}
        }

        diagnostic
    }
}

impl From<SymbolError> for TypeError {
    fn from(err: SymbolError) -> Self {
        Self::new(format!("SymbolError: {}", err))
    }
}

impl std::fmt::Display for TypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)?;
        if let Some(hint) = &self.hint {
            write!(f, "\nSuggestion: {}", hint)?;
        }
        if let Some(help) = &self.help {
            write!(f, "\n{}", help)?;
        }
        Ok(())
    }
}

impl std::error::Error for TypeError {}

/// Convert type checker errors into rich diagnostics with span guessing and suggestions.
pub fn from_type_errors(errors: &[TypeError], source_id: &str, source: &str) -> Vec<Diagnostic> {
    errors
        .iter()
        .map(|error| error.to_diagnostic(source_id, source))
        .collect()
}

fn guess_span(error: &TypeError, source: &str) -> Span {
    let candidates = extract_candidates(&error.message);

    for candidate in candidates {
        if let Some(span) = find_identifier_span(source, candidate) {
            return span;
        }
    }

    Span::new(0, 0)
}

fn extract_candidates(message: &str) -> Vec<&str> {
    let mut candidates = Vec::new();

    // Backtick enclosed identifiers
    candidates.extend(
        message
            .split('`')
            .skip(1)
            .step_by(2)
            .filter(|segment| !segment.trim().is_empty()),
    );

    // Single-quoted identifiers
    candidates.extend(
        message
            .split('\'')
            .skip(1)
            .step_by(2)
            .filter(|segment| !segment.trim().is_empty()),
    );

    // After colon (e.g., "undefined variable: foo")
    if let Some(idx) = message.find(':') {
        let candidate = message[idx + 1..]
            .split_whitespace()
            .next()
            .map(|token| token.trim_matches(|c: char| !c.is_alphanumeric() && c != '_'));
        if let Some(candidate) = candidate
            && !candidate.is_empty()
        {
            candidates.push(candidate);
        }
    }

    candidates
}

fn find_identifier_span(source: &str, needle: &str) -> Option<Span> {
    if needle.is_empty() {
        return None;
    }

    let bytes = source.as_bytes();
    let needle_bytes = needle.as_bytes();
    let len = needle_bytes.len();
    let mut byte_index = 0usize;

    while byte_index + len <= bytes.len() {
        if &bytes[byte_index..byte_index + len] == needle_bytes
            && is_word_boundary(bytes, byte_index, len)
        {
            return Some(Span::new(byte_index, byte_index + len));
        }

        // Advance by one character (respect UTF-8)
        if let Some(ch) = source[byte_index..].chars().next() {
            byte_index += ch.len_utf8();
        } else {
            break;
        }
    }

    None
}

fn is_word_boundary(bytes: &[u8], start: usize, len: usize) -> bool {
    let is_ident_char = |b: u8| -> bool {
        let ch = b as char;
        ch.is_alphanumeric() || ch == '_'
    };

    let start_ok = if start == 0 {
        true
    } else {
        !is_ident_char(bytes[start - 1])
    };

    let end_index = start + len;
    let end_ok = if end_index >= bytes.len() {
        true
    } else {
        !is_ident_char(bytes[end_index])
    };

    start_ok && end_ok
}
