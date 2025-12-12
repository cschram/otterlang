use chumsky::prelude::Simple;
use otterc_lexer::token::TokenKind;
use otterc_span::Span;
use otterc_utils::errors::{Diagnostic, DiagnosticSeverity};
use std::error::Error;
use std::fmt::Display;

#[derive(Debug, Clone)]
pub struct ParserError {
    pub message: String,
    pub span: Span,
}

impl ParserError {
    pub fn to_diagnostic(&self, source_id: &str) -> Diagnostic {
        let mut diag = Diagnostic::new(
            DiagnosticSeverity::Error,
            source_id,
            self.span,
            self.message.clone(),
        );

        // Add suggestions based on error message
        if self.message.contains("unexpected token") {
            diag = diag.with_suggestion("Check for missing or extra tokens, or syntax errors")
                .with_help("Ensure all statements are properly terminated and parentheses/brackets are balanced.");
        } else if self.message.contains("unexpected end of input") {
            diag = diag
                .with_suggestion("Check for missing closing brackets, parentheses, or quotes")
                .with_help("The parser reached the end of the file while expecting more tokens.");
        }

        diag
    }
}

impl From<Simple<TokenKind>> for ParserError {
    fn from(value: Simple<TokenKind>) -> Self {
        let span_range = value.span();
        let span = Span::new(span_range.start, span_range.end);
        let message = if let Some(found) = value.found() {
            format!("unexpected token: {:?}", found)
        } else {
            "unexpected end of input".to_string()
        };
        Self { message, span }
    }
}

impl Display for ParserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ParserError at {}: {}", self.span, self.message)
    }
}

impl Error for ParserError {}
