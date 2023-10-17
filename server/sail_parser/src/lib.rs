use chumsky::span::SimpleSpan;

pub mod cst;
pub mod lexer;
pub mod node;
pub mod parser;

pub type Span = SimpleSpan<usize>;
pub type Spanned<T> = (T, Span);
