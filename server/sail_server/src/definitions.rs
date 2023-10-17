use itertools::Itertools;
use std::collections::HashMap;

use sail_parser::{Span, Token};

pub fn add_definitions(
    tokens: &[(Token, Span)],
    _text: &str,
    definitions: &mut HashMap<String, usize>,
) {
    // For now we'll do something stupidly simple. Look for `KwFunction`
    // followed by `Id(...)`. That is a function definitions.

    // Then when we go-to-definition on an `Id()` we look those up.

    // Go-to-definition is a bit tricky because of scattered functions.
    for (token_0, token_1) in tokens.iter().tuple_windows() {
        match (&token_0.0, &token_1.0) {
            (&Token::KwFunction, &Token::Id(ref ident)) => {
                definitions.insert(ident.clone(), token_1.1.start);
            }
            (&Token::KwRegister, &Token::Id(ref ident)) => {
                definitions.insert(ident.clone(), token_1.1.start);
            }
            (&Token::KwMapping, &Token::Id(ref ident)) => {
                definitions.insert(ident.clone(), token_1.1.start);
            }
            (&Token::KwUnion, &Token::Id(ref ident)) => {
                definitions.insert(ident.clone(), token_1.1.start);
            }
            (&Token::KwStruct, &Token::Id(ref ident)) => {
                definitions.insert(ident.clone(), token_1.1.start);
            }
            (&Token::KwType, &Token::Id(ref ident)) => {
                definitions.insert(ident.clone(), token_1.1.start);
            }
            (&Token::KwEnum, &Token::Id(ref ident)) => {
                definitions.insert(ident.clone(), token_1.1.start);
            }
            (&Token::KwBitfield, &Token::Id(ref ident)) => {
                definitions.insert(ident.clone(), token_1.1.start);
            }
            _ => {}
        }
    }
}
