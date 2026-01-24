use otterc_span::Span;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum OtterError {
    #[error("Syntax Error: {0}")]
    SyntaxError(String, Span),
    #[error("Type Error: {0}")]
    TypeError(String, Span),
}
