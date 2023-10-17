use chumsky::{Parser, prelude::Rich, extra, IterParser, primitive::{any, just, choice}, select};

use crate::{Span, Spanned, cst::{Def, Identifier, OverloadDef, DefAux}, lexer::Token};


// Input to the parser is tokens with spans `&[(Token, Span)]` from the lexer.
// `SpannedInput` which 'splits' it apart into its constituent parts, tokens and spans, for chumsky
// to understand.
type ParserInput<'tokens, 'src> =
    chumsky::input::SpannedInput<Token<'src>, Span, &'tokens [(Token<'src>, Span)]>;
type ParserOutput = Vec<Spanned<DefAux>>; // TODO: Parse attributes and set this to Def.


pub fn parse_file<'tokens, 'src: 'tokens>() -> impl Parser<
    'tokens,
    ParserInput<'tokens, 'src>,
    ParserOutput,
    extra::Err<Rich<'tokens, Token<'src>, Span>>,
> + Clone {
    parse_def().repeated().collect()
}

fn parse_def<'tokens, 'src: 'tokens>() -> impl Parser<
    'tokens,
    ParserInput<'tokens, 'src>,
    Spanned<DefAux>,
    extra::Err<Rich<'tokens, Token<'src>, Span>>,
> + Clone {
    // TODO: Parse attributes.
    choice((
        // parse_type(),
        // parse_bitfield(),
        // parse_fundef(),
        // parse_mapdef(),
        // parse_let(),
        // parse_val(),
        // parse_outcome(),
        // parse_instantiation(),
        // parse_fixity(),
        parse_overload(),
        // parse_default(),
        // parse_scattered(),
        // parse_measure(),
        // parse_loop_measure(),
        // parse_register(),
        // parse_pragma(),
    ))
}

fn parse_identifier<'tokens, 'src: 'tokens>() -> impl Parser<
    'tokens,
    ParserInput<'tokens, 'src>,
    Identifier,
    extra::Err<Rich<'tokens, Token<'src>, Span>>,
> + Clone {
    select! { Token::Id(ident) => ident.to_owned() }.labelled("identifier").map_with(|_name, e| ((), e.span()))
}

fn parse_overload<'tokens, 'src: 'tokens>() -> impl Parser<
    'tokens,
    ParserInput<'tokens, 'src>,
    Spanned<DefAux>,
    extra::Err<Rich<'tokens, Token<'src>, Span>>,
> + Clone {
    // overload = { id, id, id }
    // You can also apparently do
    // overload = id | id | id
    // but I haven't seen that anywhere so maybe it is old syntax?

    let id_list_pipe = parse_identifier().separated_by(just(Token::Or)).at_least(1).collect::<Vec<_>>().boxed();

    let ident_list_comma = parse_identifier().separated_by(just(Token::Comma)).at_least(1).collect::<Vec<_>>().boxed();
    let id_list_comma = just(Token::LeftCurlyBracket)
        .ignore_then(ident_list_comma)
        .then_ignore(just(Token::RightCurlyBracket)).boxed();

    just(Token::KwOverload)
    .ignore_then(parse_identifier())
    .then_ignore(just(Token::Equal))
    .then(id_list_comma.or(id_list_pipe))
    .map_with(|(id, overload), e| (DefAux::OverloadDef(OverloadDef { id, overload }), e.span()))
}


// fn parse_register<'tokens, 'src: 'tokens>() -> impl Parser<
//     'tokens,
//     ParserInput<'tokens, 'src>,
//     Spanned<DefAux>,
//     extra::Err<Rich<'tokens, Token<'src>, Span>>,
// > + Clone {
//     // register_def:
//     // | Register id Colon typ
//     //   { mk_reg_dec (DEC_reg ($4, $2, None)) $startpos $endpos }
//     // | Register id Colon typ Eq exp
//     //   { mk_reg_dec (DEC_reg ($4, $2, Some $6)) $startpos $endpos }

//     // There's also stuff about effects and 'configuration' which I think is
//     // also to do with effects, but they aren't used anymore.

//     just(Token::KwRegister)
//     .ignore_then(parse_identifier())
//     .then_ignore(just(Token::Colon))
//     .then(parse_type())
//     .then_maybe(just(Token::Equal).ignore_then(parse_expression()))
//     .map_with_span(|_, span| (DefAux::RegisterDef(, span))
// }


// fn parse_type<'tokens, 'src: 'tokens>() -> impl Parser<
//     'tokens,
//     ParserInput<'tokens, 'src>,
//     Spanned<Type>,
//     extra::Err<Rich<'tokens, Token<'src>, Span>>,
// > + Clone {
//     todo!()
// }

// fn parse_expression<'tokens, 'src: 'tokens>() -> impl Parser<
//     'tokens,
//     ParserInput<'tokens, 'src>,
//     Spanned<Expression>,
//     extra::Err<Rich<'tokens, Token<'src>, Span>>,
// > + Clone {
//     todo!()
// }
