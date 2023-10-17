use itertools::Itertools;
use std::collections::HashMap;

use sail_parser::{Span, lexer::Token};

pub fn add_definitions(
    tokens: &[(Token, Span)],
    _text: &str,
    definitions: &mut HashMap<String, usize>,
) {
    todo!()
}
