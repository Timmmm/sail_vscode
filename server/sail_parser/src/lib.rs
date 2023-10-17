mod cst;
mod lexer;
mod parser;
pub use lexer::*;

pub type Spanned<T> = (T, Span);
