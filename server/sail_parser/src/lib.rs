use chumsky::span::SimpleSpan;

pub mod analyser;
pub mod cst;
pub mod lexer;
pub mod parser;

pub type Span = SimpleSpan<usize>;
pub type Spanned<T> = (T, Span);
