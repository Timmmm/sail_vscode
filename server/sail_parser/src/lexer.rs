//! Sail parser using Chumsky.
use chumsky::{
    combinator::Repeated,
    error::Error,
    extra::ParserExtra,
    input::{StrInput, ValueInput},
    prelude::*,
    text::Char,
    util::MaybeRef,
    Parser,
};
use std::fmt;

pub type Span = SimpleSpan<usize>;

// TODO: Make tokens zero copy &str when we have a parser as well as a lexer.
// For now they are String to keep things simple.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Token {
    // Identifiers
    Id(String),
    TyVal(String), // 'identifier (the ' is discarded)

    // Number literals.
    Bin(String),  // 0b010101 (the 0b is discarded)
    Hex(String),  // 0xDEAD32 (the 0x is discarded)
    Num(String),  // -123
    Real(String), //-034.432

    // String literal.
    String(String),

    // Operators and control characters.
    Dollar,
    LeftBracket,        // (
    RightBracket,       // )
    LeftSquareBracket,  // [
    RightSquareBracket, // ]
    LeftCurlyBracket,   // {
    RightCurlyBracket,  // }
    RightArrow,         // ->
    LeftArrow,          // <-
    FatRightArrow,      // =>
    DoubleArrow,        // <->
    Comma,
    Colon,
    Semicolon,
    Dot,
    Caret, // ^
    At,    // @
    LessThan,
    GreaterThan,
    LessThanOrEqualTo,
    GreaterThanOrEqualTo,
    Modulus,    // %
    Multiply,   // *
    Divide,     // /
    Equal,      // =
    EqualTo,    // ==
    NotEqualTo, // !=
    And,        // &
    Or,         // |
    Scope,      // ::
    Plus,
    Minus,
    LeftCurlyBar,   // {|
    RightCurlyBar,  // |}
    LeftSquareBar,  // [|
    RightSquareBar, // |]
    Underscore,     // _
    Unit,           // ()

    // Keywords.
    KwAnd,
    KwAs,
    KwAssert,
    KwBackwards,
    KwBarr,
    KwBitfield,
    KwBitone,
    KwBitzero,
    KwBool,
    KwBy,
    KwCast,
    KwCatch,
    KwClause,
    KwConfiguration,
    KwConstant,
    KwConstraint,
    KwDec,
    KwDefault,
    KwDepend,
    KwDo,
    KwEamem,
    KwEffect,
    KwElse,
    KwEnd,
    KwEnum,
    KwEscape,
    KwExit,
    KwExmem,
    KwFalse,
    KwForall,
    KwForeach,
    KwForwards,
    KwFunction,
    KwIf,
    KwImpl,
    KwIn,
    KwInc,
    KwInfix,
    KwInfixl,
    KwInfixr,
    KwInstantiation,
    KwInt,
    KwLet,
    KwMapping,
    KwMatch,
    KwMonadic,
    KwMutual,
    KwMwv,
    KwNewtype,
    KwNondet,
    KwOrder,
    KwOutcome,
    KwOverload,
    KwPure,
    KwRef,
    KwRegister,
    KwRepeat,
    KwReturn,
    KwRmem,
    KwRreg,
    KwScattered,
    KwSizeof,
    KwStruct,
    KwTerminationMeasure,
    KwThen,
    KwThrow,
    KwTrue,
    KwTry,
    KwType,      // type
    KwTypeUpper, // Type
    KwUndef,
    KwUndefined,
    KwUnion,
    KwUnspec,
    KwUntil,
    KwVal,
    KwVar,
    KwWhile,
    KwWith,
    KwWmem,
    KwWreg,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            // Identifiers.
            Token::Id(s) => write!(f, "{}", s),
            Token::TyVal(s) => write!(f, "{}", s),

            // Numbers literals.
            Token::Bin(s) => write!(f, "{}", s),
            Token::Hex(s) => write!(f, "{}", s),
            Token::Num(s) => write!(f, "{}", s),
            Token::Real(s) => write!(f, "{}", s),

            // String literal.
            Token::String(s) => write!(f, "{}", s),

            // Operators and other control characters.
            Token::Dollar => write!(f, "$"),
            Token::LeftBracket => write!(f, "("),
            Token::RightBracket => write!(f, ")"),
            Token::LeftSquareBracket => write!(f, "["),
            Token::RightSquareBracket => write!(f, "]"),
            Token::LeftCurlyBracket => write!(f, "{{"),
            Token::RightCurlyBracket => write!(f, "}}"),
            Token::RightArrow => write!(f, "->"),
            Token::LeftArrow => write!(f, "<-"),
            Token::FatRightArrow => write!(f, "=>"),
            Token::DoubleArrow => write!(f, "<->"),
            Token::Comma => write!(f, ","),
            Token::Colon => write!(f, ":"),
            Token::Semicolon => write!(f, ";"),
            Token::Dot => write!(f, "."),
            Token::Caret => write!(f, "^"),
            Token::At => write!(f, "@"),
            Token::LessThan => write!(f, "<"),
            Token::GreaterThan => write!(f, ">"),
            Token::LessThanOrEqualTo => write!(f, "<="),
            Token::GreaterThanOrEqualTo => write!(f, ">="),
            Token::Modulus => write!(f, "%"),
            Token::Multiply => write!(f, "*"),
            Token::Divide => write!(f, "/"),
            Token::Equal => write!(f, "="),
            Token::EqualTo => write!(f, "=="),
            Token::NotEqualTo => write!(f, "!="),
            Token::And => write!(f, "&"),
            Token::Or => write!(f, "|"),
            Token::Scope => write!(f, "::"),
            Token::Plus => write!(f, "+"),
            Token::Minus => write!(f, "-"),
            Token::LeftCurlyBar => write!(f, "{{|"),
            Token::RightCurlyBar => write!(f, "|}}"),
            Token::LeftSquareBar => write!(f, "[|"),
            Token::RightSquareBar => write!(f, "|]"),
            Token::Underscore => write!(f, "_"),
            Token::Unit => write!(f, "()"),

            // Keywords.
            Token::KwAnd => write!(f, "and"),
            Token::KwAs => write!(f, "as"),
            Token::KwAssert => write!(f, "assert"),
            Token::KwBackwards => write!(f, "backwards"),
            Token::KwBarr => write!(f, "barr"),
            Token::KwBitfield => write!(f, "bitfield"),
            Token::KwBitone => write!(f, "bitone"),
            Token::KwBitzero => write!(f, "bitzero"),
            Token::KwBool => write!(f, "Bool"),
            Token::KwBy => write!(f, "by"),
            Token::KwCast => write!(f, "cast"),
            Token::KwCatch => write!(f, "catch"),
            Token::KwClause => write!(f, "clause"),
            Token::KwConfiguration => write!(f, "configuration"),
            Token::KwConstant => write!(f, "constant"),
            Token::KwConstraint => write!(f, "constraint"),
            Token::KwDec => write!(f, "dec"),
            Token::KwDefault => write!(f, "default"),
            Token::KwDepend => write!(f, "depend"),
            Token::KwDo => write!(f, "do"),
            Token::KwEamem => write!(f, "eamem"),
            Token::KwEffect => write!(f, "effect"),
            Token::KwElse => write!(f, "else"),
            Token::KwEnd => write!(f, "end"),
            Token::KwEnum => write!(f, "enum"),
            Token::KwEscape => write!(f, "escape"),
            Token::KwExit => write!(f, "exit"),
            Token::KwExmem => write!(f, "exmem"),
            Token::KwFalse => write!(f, "false"),
            Token::KwForall => write!(f, "forall"),
            Token::KwForeach => write!(f, "foreach"),
            Token::KwForwards => write!(f, "forwards"),
            Token::KwFunction => write!(f, "function"),
            Token::KwIf => write!(f, "if"),
            Token::KwImpl => write!(f, "impl"),
            Token::KwIn => write!(f, "in"),
            Token::KwInc => write!(f, "inc"),
            Token::KwInfix => write!(f, "infix"),
            Token::KwInfixl => write!(f, "infixl"),
            Token::KwInfixr => write!(f, "infixr"),
            Token::KwInstantiation => write!(f, "instantiation"),
            Token::KwInt => write!(f, "Int"),
            Token::KwLet => write!(f, "let"),
            Token::KwMapping => write!(f, "mapping"),
            Token::KwMatch => write!(f, "match"),
            Token::KwMonadic => write!(f, "monadic"),
            Token::KwMutual => write!(f, "mutual"),
            Token::KwMwv => write!(f, "mwv"),
            Token::KwNewtype => write!(f, "newtype"),
            Token::KwNondet => write!(f, "nondet"),
            Token::KwOrder => write!(f, "Order"),
            Token::KwOutcome => write!(f, "outcome"),
            Token::KwOverload => write!(f, "overload"),
            Token::KwPure => write!(f, "pure"),
            Token::KwRef => write!(f, "ref"),
            Token::KwRegister => write!(f, "register"),
            Token::KwRepeat => write!(f, "repeat"),
            Token::KwReturn => write!(f, "return"),
            Token::KwRmem => write!(f, "rmem"),
            Token::KwRreg => write!(f, "rreg"),
            Token::KwScattered => write!(f, "scattered"),
            Token::KwSizeof => write!(f, "sizeof"),
            Token::KwStruct => write!(f, "struct"),
            Token::KwTerminationMeasure => write!(f, "termination_measure"),
            Token::KwThen => write!(f, "then"),
            Token::KwThrow => write!(f, "throw"),
            Token::KwTrue => write!(f, "true"),
            Token::KwTry => write!(f, "try"),
            Token::KwType => write!(f, "type"),
            Token::KwTypeUpper => write!(f, "Type"),
            Token::KwUndef => write!(f, "undef"),
            Token::KwUndefined => write!(f, "undefined"),
            Token::KwUnion => write!(f, "union"),
            Token::KwUnspec => write!(f, "unspec"),
            Token::KwUntil => write!(f, "until"),
            Token::KwVal => write!(f, "val"),
            Token::KwVar => write!(f, "var"),
            Token::KwWhile => write!(f, "while"),
            Token::KwWith => write!(f, "with"),
            Token::KwWmem => write!(f, "wmem"),
            Token::KwWreg => write!(f, "wreg"),
        }
    }
}

/// Same as C identifiers but ? is allowed and ' is allowed after the first character.
/// Also '~' is allowed as a special identifier.
#[must_use]
pub fn ident<'a, I: ValueInput<'a> + StrInput<'a, char>, E: ParserExtra<'a, I>>(
) -> impl Parser<'a, I, &'a str, E> + Copy + Clone {
    any()
        // Use try_map over filter to get a better error on failure
        .try_map(|c: char, span| {
            if c.is_ascii_alphabetic() || c == '_' || c == '?' {
                Ok(c)
            } else {
                Err(Error::expected_found([], Some(MaybeRef::Val(c)), span))
            }
        })
        .then(
            any()
                // This error never appears due to `repeated` so can use `filter`
                .filter(|&c: &char| c.is_ascii_alphanumeric() || c == '_' || c == '?' || c == '\'')
                .repeated(),
        )
        .ignored()
        .or(just('~').ignored())
        .slice()
}

/// Like digits() but an exact number of then.
#[must_use]
pub fn n_digits<'a, C, I, E>(
    radix: u32,
    count: usize,
) -> Repeated<impl Parser<'a, I, C, E> + Copy + Clone, C, I, E>
where
    C: Char,
    I: ValueInput<'a> + Input<'a, Token = C>,
    E: ParserExtra<'a, I>,
{
    any()
        // Use try_map over filter to get a better error on failure
        .try_map(move |c: C, span| {
            if c.is_digit(radix) {
                Ok(c)
            } else {
                Err(Error::expected_found([], Some(MaybeRef::Val(c)), span))
            }
        })
        .repeated()
        .exactly(count)
}

pub fn lexer<'src>(
) -> impl Parser<'src, &'src str, Vec<(Token, Span)>, extra::Err<Rich<'src, char, Span>>> {
    // Arbitrary length positive or negative integer.
    let num = just('-')
        .or_not()
        .then(text::digits(10))
        .map_slice(|s: &str| Token::Num(s.to_owned()))
        .boxed();

    // Real number.
    let real = just('-')
        .or_not()
        .then(text::digits(10))
        .then(just('.'))
        .then(text::digits(10))
        .map_slice(|s: &str| Token::Real(s.to_owned()))
        .boxed();

    // Hex number.
    let hex = just("0x")
        .ignore_then(text::digits(16))
        .map_slice(|s: &str| Token::Hex(s.to_owned()))
        .boxed();

    // Binary number.
    let bin = just("0b")
        .ignore_then(text::digits(2))
        .map_slice(|s: &str| Token::Bin(s.to_owned()))
        .boxed();

    // Strings.
    let escape = just('\\')
        .ignore_then(choice((
            just('\\'),
            just('"'),
            just('\''),
            just('n').to('\n'),
            just('t').to('\t'),
            just('b').to('\x08'),
            just('r').to('\r'),
            just('\n').to(' '), // TODO: Handle this properly.
            just('d').ignore_then(n_digits(10, 3).slice().try_map(|digits: &str, span| {
                char::from_u32(u32::from_str_radix(&digits, 10).unwrap())
                    .ok_or_else(|| Rich::custom(span, "invalid decimal unicode value"))
            })),
            just('x').ignore_then(n_digits(16, 2).slice().try_map(|digits: &str, span| {
                char::from_u32(u32::from_str_radix(&digits, 16).unwrap())
                    .ok_or_else(|| Rich::custom(span, "invalid hex unicode value"))
            })),
        )))
        .boxed();

    let string = just('"')
        .ignore_then(none_of(&['\\', '"']).or(escape).repeated())
        .then_ignore(just('"'))
        .map_slice(|s: &str| Token::String(s.to_owned()))
        .boxed();

    // The order of these is important, e.g. <= must come before < otherwise
    // <= will be parsed as <, =.
    // Have to split it into two choices because there's more than 26 and
    // they are different types.
    let op = choice((
        just("|}").to(Token::RightCurlyBar),
        just("|]").to(Token::RightSquareBar),
        just(">=").to(Token::GreaterThanOrEqualTo),
        just("=>").to(Token::FatRightArrow),
        just("==").to(Token::EqualTo),
        just("<=").to(Token::LessThanOrEqualTo),
        just("<->").to(Token::DoubleArrow),
        just("<-").to(Token::LeftArrow),
        just("{|").to(Token::LeftCurlyBar),
        just("[|").to(Token::LeftSquareBar),
        just("()").to(Token::Unit),
        just("!=").to(Token::NotEqualTo),
        just("::").to(Token::Scope),
        just("->").to(Token::RightArrow),
    ))
    .or(choice((
        just('$').to(Token::Dollar),
        just('|').to(Token::Or),
        just('>').to(Token::GreaterThan),
        just('=').to(Token::Equal),
        just('<').to(Token::LessThan),
        just('+').to(Token::Plus),
        just('^').to(Token::Caret),
        just('%').to(Token::Modulus),
        just('&').to(Token::And),
        just('/').to(Token::Divide),
        just('*').to(Token::Multiply),
        just('@').to(Token::At),
        just('}').to(Token::RightCurlyBracket),
        just('{').to(Token::LeftCurlyBracket),
        just(']').to(Token::RightSquareBracket),
        just('[').to(Token::LeftSquareBracket),
        just(')').to(Token::RightBracket),
        just('(').to(Token::LeftBracket),
        just('.').to(Token::Dot),
        just(':').to(Token::Colon),
        just(';').to(Token::Semicolon),
        just(',').to(Token::Comma),
        just('-').to(Token::Minus),
        just('_').to(Token::Underscore),
    )))
    .boxed();

    // TyVar
    let tyvar = just('\'')
        .ignore_then(ident())
        .map_slice(|s: &str| Token::TyVal(s.to_owned()))
        .boxed();

    // A parser for identifiers and keywords.
    // '~' is a specially allowed identifier.
    let ident = ident()
        .map(|ident: &str| match ident {
            "and" => Token::KwAnd,
            "as" => Token::KwAs,
            "assert" => Token::KwAssert,
            "backwards" => Token::KwBackwards,
            "barr" => Token::KwBarr,
            "bitfield" => Token::KwBitfield,
            "bitone" => Token::KwBitone,
            "bitzero" => Token::KwBitzero,
            "Bool" => Token::KwBool,
            "by" => Token::KwBy,
            "cast" => Token::KwCast,
            "catch" => Token::KwCatch,
            "clause" => Token::KwClause,
            "configuration" => Token::KwConfiguration,
            "constant" => Token::KwConstant,
            "constraint" => Token::KwConstraint,
            "dec" => Token::KwDec,
            "default" => Token::KwDefault,
            "depend" => Token::KwDepend,
            "do" => Token::KwDo,
            "eamem" => Token::KwEamem,
            "effect" => Token::KwEffect,
            "else" => Token::KwElse,
            "end" => Token::KwEnd,
            "enum" => Token::KwEnum,
            "escape" => Token::KwEscape,
            "exit" => Token::KwExit,
            "exmem" => Token::KwExmem,
            "false" => Token::KwFalse,
            "forall" => Token::KwForall,
            "foreach" => Token::KwForeach,
            "forwards" => Token::KwForwards,
            "function" => Token::KwFunction,
            "if" => Token::KwIf,
            "impl" => Token::KwImpl,
            "in" => Token::KwIn,
            "inc" => Token::KwInc,
            "infix" => Token::KwInfix,
            "infixl" => Token::KwInfixl,
            "infixr" => Token::KwInfixr,
            "instantiation" => Token::KwInstantiation,
            "Int" => Token::KwInt,
            "let" => Token::KwLet,
            "mapping" => Token::KwMapping,
            "match" => Token::KwMatch,
            "monadic" => Token::KwMonadic,
            "mutual" => Token::KwMutual,
            "mwv" => Token::KwMwv,
            "newtype" => Token::KwNewtype,
            "nondet" => Token::KwNondet,
            "Order" => Token::KwOrder,
            "outcome" => Token::KwOutcome,
            "overload" => Token::KwOverload,
            "pure" => Token::KwPure,
            "ref" => Token::KwRef,
            "register" => Token::KwRegister,
            "repeat" => Token::KwRepeat,
            "return" => Token::KwReturn,
            "rmem" => Token::KwRmem,
            "rreg" => Token::KwRreg,
            "scattered" => Token::KwScattered,
            "sizeof" => Token::KwSizeof,
            "struct" => Token::KwStruct,
            "termination_measure" => Token::KwTerminationMeasure,
            "then" => Token::KwThen,
            "throw" => Token::KwThrow,
            "true" => Token::KwTrue,
            "try" => Token::KwTry,
            "type" => Token::KwType,
            "Type" => Token::KwTypeUpper,
            "undef" => Token::KwUndef,
            "undefined" => Token::KwUndefined,
            "union" => Token::KwUnion,
            "unspec" => Token::KwUnspec,
            "until" => Token::KwUntil,
            "val" => Token::KwVal,
            "var" => Token::KwVar,
            "while" => Token::KwWhile,
            "with" => Token::KwWith,
            "wmem" => Token::KwWmem,
            "wreg" => Token::KwWreg,
            _ => Token::Id(ident.to_string()),
        })
        .boxed();

    // A single token can be one of the above
    let token = choice((tyvar, hex, bin, real, num, string, ident, op))
        .recover_with(skip_then_retry_until(any().ignored(), end()))
        .boxed();

    let line_comment = just("//").then(none_of('\n').repeated()).padded().ignored();
    let block_comment = just("/*")
        .then(any().and_is(just("*/").not()).repeated())
        .then(just("*/"))
        .padded()
        .ignored();

    let comment = line_comment.or(block_comment);

    token
        .map_with_span(|tok, span| (tok, span))
        .padded_by(comment.repeated())
        .padded()
        .repeated()
        .collect()
        .then_ignore(end())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_basic() {
        let code = r#"
/* This is a slightly arbitrary limit on the maximum number of bytes
   in a memory access.  It helps to generate slightly better C code
   because it means width argument can be fast native integer. It
   would be even better if it could be <= 8 bytes so that data can
   also be a 64-bit int but CHERI needs 128-bit accesses for
   capabilities and SIMD / vector instructions will also need more. */
type max_mem_access : Int = 16

val write_ram = {lem: "write_ram", coq: "write_ram"} : forall 'n, 0 < 'n <= max_mem_access . (write_kind, xlenbits, atom('n), bits(8 * 'n), mem_meta) -> bool effect {wmv, wmvt}
function write_ram(wk, addr, width, data, meta) = {
  /* Write out metadata only if the value write succeeds.
   * It is assumed for now that this write always succeeds;
   * there is currently no return value.
   * FIXME: We should convert the external API for all backends
   * (not just for Lem) to consume the value along with the
   * metadata to ensure atomicity.
   */
  let ret : bool = __write_mem(wk, sizeof(xlen), addr, width, data);
  if ret then __WriteRAM_Meta(addr, width, meta);
  ret
}

val write_ram_ea : forall 'n, 0 < 'n <= max_mem_access . (write_kind, xlenbits, atom('n)) -> unit effect {eamem}
function write_ram_ea(wk, addr, width) =
  __write_mem_ea(wk, sizeof(xlen), addr, width)

val read_ram = {lem: "read_ram", coq: "read_ram"} : forall 'n, 0 < 'n <= max_mem_access .  (read_kind, xlenbits, atom('n), bool) -> (bits(8 * 'n), mem_meta) effect {rmem, rmemt}
function read_ram(rk, addr, width, read_meta) =
  let meta = if read_meta then __ReadRAM_Meta(addr, width) else default_meta in
  (__read_mem(rk, sizeof(xlen), addr, width), meta)

val __TraceMemoryWrite : forall 'n 'm. (atom('n), bits('m), bits(8 * 'n)) -> unit
val __TraceMemoryRead  : forall 'n 'm. (atom('n), bits('m), bits(8 * 'n)) -> unit
"#;
        let result = lexer().parse(code);
        dbg!(result);
    }

    #[test]
    fn test_span_bytes() {
        // Check that the span is in bytes and works with unicode characters.
        let code = "/* 😊 */ foo";
        let result = lexer().parse(code);
        dbg!(result);
    }
}
