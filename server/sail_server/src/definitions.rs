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
            (&Token::KwOverload, &Token::Id(ref ident)) => {
                definitions.insert(ident.clone(), token_1.1.start);
            }
            (&Token::KwBitfield, &Token::Id(ref ident)) => {
                definitions.insert(ident.clone(), token_1.1.start);
                // Auto-generated Mk_ functions.
                definitions.insert(format!("Mk_{}", ident), token_1.1.start);
            }
            _ => {}
        }
    }

    // "Parse" enums of the form `enum Foo = { Bar, Baz, ... }`
    let mut token_iter = tokens.iter();
    while let Some(next) = token_iter.next() {
        if matches!(next.0, Token::KwEnum | Token::KwOverload) {
            add_enum_definition(&mut token_iter, definitions);
        }
    }
}

fn add_enum_definition(token_iter: &mut std::slice::Iter<(Token, Span)>, definitions: &mut HashMap<String, usize>) {
    if let Some((Token::Id(ref ident), span)) = token_iter.next() {
        definitions.insert(ident.clone(), span.start);
        if let Some((Token::Equal, _)) = token_iter.next() {
            if let Some((Token::LeftCurlyBracket, _)) = token_iter.next() {
                while let Some((Token::Id(ident), span)) = token_iter.next() {
                    definitions.insert(ident.clone(), span.start);
                    if let Some((Token::Comma, _)) = token_iter.next() {
                        // Ok
                    } else {
                        break;
                    }
                }
            }
        }
    }
}
