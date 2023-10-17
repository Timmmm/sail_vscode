use chumsky::{Parser, prelude::Rich, extra, IterParser, primitive::{any, just, choice}, select};

use crate::{Spanned, lexer::Token, Span};

// ID is any valid Sail identifier
// OPERATOR is any valid Sail operator, as defined in Operators.
// TYPE_VARIABLE is a valid Sail identifier with a leading single quote '.
// NUMBER is a non-empty sequence of decimal digits [0-9]+.
// HEXADECIMAL_LITERAL is 0x[A-Ba-f0-9_]+
// BINARY_LITERAL is 0b[0-1_]+
// STRING_LITERAL is a Sail string literal.

// ID
pub type Identifier = Spanned<()>;

// OPERATOR
pub type Operator = Spanned<()>;

// ATTRIBUTE
pub type Attribute = Spanned<()>;

// <id> ::= ID
//        | operator OPERATOR
//        | operator -
//        | operator |
//        | operator ^
//        | operator *

pub enum Id {
    Identifier(Identifier),
    Operator(Operator),
    Minus,
    Pipe,
    Caret,
    Star,
}

// <op_no_caret> ::= OPERATOR
//                 | -
//                 | |
//                 | *
//                 | in

pub enum OpNoCaret {
    Operator(Operator),
    Minus,
    Pipe,
    Star,
    In,
}

// <op> ::= OPERATOR
//        | -
//        | |
//        | ^
//        | *
//        | in

pub enum Op {
    Operator(Operator),
    Minus,
    Pipe,
    Caret,
    Star,
    In,
}

// <exp_op> ::= OPERATOR
//            | -
//            | |
//            | @
//            | ::
//            | ^
//            | *

pub enum ExpOp {
    Operator(Operator),
    Minus,
    Pipe,
    At,
    ColonColon,
    Caret,
    Star,
}

// <pat_op> ::= @
//            | ::
//            | ^

pub enum PatOp {
    At,
    ColonColon,
    Caret,
}

// <typ_var> ::= TYPE_VARIABLE

pub type TypVar = Spanned<()>;

// <tyarg> ::= ( <typ_list> )

// <prefix_typ_op> ::= epsilon
//                   | 2^
//                   | -
//                   | *

pub enum PrefixTypOp {
    Epsilon,
    PowerOfTwo,
    Minus,
    Star,
}

// <postfix_typ> ::= <atomic_typ>

pub type PostfixTyp = Box<AtomicTyp>;

// <typ_no_caret> ::= <prefix_typ_op> <postfix_typ> (<op_no_caret> <prefix_typ_op> <postfix_typ>)*

pub struct TypNoCaret {
    pub prefix_typ_op: PrefixTypOp,
    pub postfix_typ: PostfixTyp,
    pub next: Vec<(OpNoCaret, PrefixTypOp, PostfixTyp)>,
}

// <typ> ::= <prefix_typ_op> <postfix_typ> (<op> <prefix_typ_op> <postfix_typ>)*

pub struct Typ {
    pub prefix_typ_op: PrefixTypOp,
    pub postfix_typ: PostfixTyp,
    pub next: Vec<(Op, PrefixTypOp, PostfixTyp)>,
}

// <atomic_typ> ::= <id>
//                | _
//                | <typ_var>
//                | <lit>
//                | dec
//                | inc
//                | <id> <tyarg>
//                | register ( <typ> )
//                | ( <typ> )
//                | ( <typ> , <typ_list> )
//                | { NUMBER (, NUMBER)* }
//                | { <kopt> . <typ> }
//                | { <kopt> , <typ> . <typ> }

pub enum AtomicTyp {
    Id(Id),
    Underscore,
    TypVar(TypVar),
    Lit(Lit),
    Dec,
    Inc,
    IdTy(Id, TypList),
    Register(Typ),
    Typs(Vec<Typ>),
    Numbers(Vec<Number>),
    // BraceNumberList(Vec<Number>),
    // BraceKoptTyp(Kopt, Typ),
    // BraceKoptTypTyp(Kopt, Typ, Typ),
}

// <typ_list> ::= <typ> [,]
//              | <typ> , <typ_list>

pub type TypList = Vec<Typ>;

// <kind> ::= Int
//          | Type
//          | Order
//          | Bool

pub enum Kind {
    Int,
    Type,
    Order,
    Bool,
}

// <kopt> ::= ( constant <typ_var> : <kind> )
//          | ( <typ_var> : <kind> )
//          | <typ_var>

pub enum Kopt {
    Constant(TypVar, Kind),
    NonConstant(TypVar, Option<Kind>),
}

// <quantifier> ::= <kopt> , <typ>
//              | <kopt>

pub struct Quantifier {
    pub kopt: Kopt,
    pub typ: Option<Typ>,
}

// <typschm> ::= <typ> -> <typ>
//             | forall <quantifier> . <typ> -> <typ>
//             | <typ> <-> <typ>
//             | forall <quantifier> . <typ> <-> <typ>

// <pat1> ::= <atomic_pat> (<pat_op> <atomic_pat>)*

pub struct Pat1 {
    pub atomic_pat: AtomicPat,
    pub next: Vec<(PatOp, AtomicPat)>,
}

// <pat> ::= <pat1>
//         | $[ATTRIBUTE] <pat>
//         | <pat1> as <typ>

pub enum Pat {
    Pat1(Pat1),
    Attribute(Attribute, Box<Pat>),
    Pat1Typ(Pat1, Typ),
}

// <pat_list> ::= <pat> [,]
//              | <pat> , <pat_list>

pub type PatList = Vec<Pat>;

// <atomic_pat> ::= _
//                | <lit>
//                | <id>
//                | <typ_var>
//                | <id> ()
//                | <id> [ NUMBER ]
//                | <id> [ NUMBER .. NUMBER ]
//                | <id> ( <pat_list> )
//                | <atomic_pat> : <typ_no_caret>
//                | ( <pat> )
//                | ( <pat> , <pat_list> )
//                | [ <pat_list> ]
//                | [| |]
//                | [| <pat_list> |]
//                | struct { <fpat> (, <fpat>)* }

pub enum AtomicPat {
    Underscore,
    Lit(Lit),
    Id(Id),
    TypVar(TypVar),
    IdUnit(Id),
    IdNumber(Id, Number),
    IdRange(Id, Number, Number),
    // ...
    // IdPatList(Id, PatList),
    // Pat(Pat),
    // PatList(PatList),
    // FpatList(FpatList),
}


// <fpat> ::= <id> = <pat>
//          | <id>
//          | _

pub enum Fpat {
    Assignment(Id, Pat),
    Id(Id),
    Underscore,
}

// <lit> ::= true
//         | false
//         | ()
//         | NUMBER
//         | undefined
//         | bitzero
//         | bitone
//         | BINARY_LITERAL
//         | HEXADECIMAL_LITERAL
//         | STRING_LITERAL

pub type Number = Spanned<()>;
pub type BinaryLiteral = Spanned<()>;
pub type HexadecimalLiteral = Spanned<()>;
pub type StringLiteral = Spanned<()>;

pub enum Lit {
    True,
    False,
    Unit,
    Number(Number),
    Undefined,
    BitZero,
    BitOne,
    BinaryLiteral(BinaryLiteral),
    HexadecimalLiteral(HexadecimalLiteral),
    StringLiteral(StringLiteral),
}

// <exp> ::= <exp0>
//         | $[ATTRIBUTE] <exp>
//         | <exp0> = <exp>
//         | let <letbind> in <exp>
//         | var <atomic_exp> = <exp> in <exp>
//         | { <block> }
//         | return <exp>
//         | throw <exp>
//         | if <exp> then <exp> else <exp>
//         | if <exp> then <exp>
//         | match <exp> { <case_list> }
//         | try <exp> catch { <case_list> }
//         | foreach ( <id> ID <atomic_exp> ID <atomic_exp> by <atomic_exp> in <typ> ) <exp>
//         | foreach ( <id> ID <atomic_exp> ID <atomic_exp> by <atomic_exp> ) <exp>
//         | foreach ( <id> ID <atomic_exp> ID <atomic_exp> ) <exp>
//         | repeat [termination_measure { <exp> }] <exp> until <exp>
//         | while [termination_measure { <exp> }] <exp> do <exp>

pub enum Exp {
    // TODO.
}

// <prefix_op> ::= epsilon
//               | 2^
//               | -
//               | *

pub enum PrefixOp {
    Epsilon,
    PowerOfTwo,
    Minus,
    Star,
}

// <exp0> ::= <prefix_op> <atomic_exp> (<exp_op> <prefix_op> <atomic_exp>)*

pub struct Exp0 {
    pub prefix_op: PrefixOp,
    pub atomic_exp: AtomicExp,
    pub next: Vec<(ExpOp, PrefixOp, AtomicExp)>,
}

// <case> ::= <pat> => <exp>
//          | <pat> if <exp> => <exp>

pub struct Case {
    pub pat: Pat,
    pub guard: Option<Exp>,
    pub exp: Exp,
}

// <case_list> ::= <case>
//               | <case> ,
//               | <case> , <case_list>

pub type CaseList = Vec<Case>;

// <block> ::= <exp> [;]
//           | let <letbind> [;]
//           | let <letbind> ; <block>
//           | var <atomic_exp> = <exp> [;]
//           | var <atomic_exp> = <exp> ; <block>
//           | <exp> ; <block>

pub enum Block {
    Let(LetBind, Option<Box<Block>>),
    Assigment(AtomicExp, Exp, Option<Box<Block>>),
    Exp(Exp, Option<Box<Block>>),
}

// <letbind> ::= <pat> = <exp>

pub struct LetBind {
    pub pat: Pat,
    pub exp: Exp,
}

// <atomic_exp> ::= <atomic_exp> : <atomic_typ>
//                | <lit>
//                | <id> -> <id> ()
//                | <id> -> <id> ( <exp_list> )
//                | <atomic_exp> . <id> ()
//                | <atomic_exp> . <id> ( <exp_list> )
//                | <atomic_exp> . <id>
//                | <id>
//                | <typ_var>
//                | ref <id>
//                | <id> ()
//                | <id> ( <exp_list> )
//                | sizeof ( <typ> )
//                | constraint ( <typ> )
//                | <atomic_exp> [ <exp> ]
//                | <atomic_exp> [ <exp> .. <exp> ]
//                | <atomic_exp> [ <exp> , <exp> ]
//                | struct { <fexp_exp_list> }
//                | { <exp> with <fexp_exp_list> }
//                | [ ]
//                | [ <exp_list> ]
//                | [ <exp> with <vector_update> (, <vector_update>)* ]
//                | [| |]
//                | [| <exp_list> |]
//                | ( <exp> )
//                | ( <exp> , <exp_list> )

pub enum AtomicExp {
    // TODO
}

// <fexp_exp> ::= <atomic_exp> = <exp>
//              | <id>

pub enum FexpExp {
    Assignment(AtomicExp, Exp),
    Id(Id),
}

// <fexp_exp_list> ::= <fexp_exp>
//                   | <fexp_exp> ,
//                   | <fexp_exp> , <fexp_exp_list>

pub type FexpExpList = Vec<FexpExp>;

// <exp_list> ::= <exp> [,]
//              | <exp> , <exp_list>

// <vector_update> ::= <atomic_exp> = <exp>
//                   | <atomic_exp> .. <atomic_exp> = <exp>
//                   | <id>

// <funcl_annotation> ::= $[ATTRIBUTE]

// <funcl_patexp> ::= <pat> = <exp>
//                  | ( <pat> if <exp> ) = <exp>

// <funcl_patexp_typ> ::= <pat> = <exp>
//                      | <pat> -> <typ> = <exp>
//                      | forall <quantifier> . <pat> -> <typ> = <exp>
//                      | ( <pat> if <exp> ) = <exp>
//                      | ( <pat> if <exp> ) -> <typ> = <exp>
//                      | forall <quantifier> . ( <pat> if <exp> ) -> <typ> = <exp>

// <funcl> ::= <funcl_annotation> <id> <funcl_patexp>
//           | <id> <funcl_patexp>

// <funcls> ::= <funcl_annotation> <id> <funcl_patexp_typ>
//            | <id> <funcl_patexp_typ>
//            | <funcl_annotation> <id> <funcl_patexp> and <funcl> (and <funcl>)*
//            | <id> <funcl_patexp> and <funcl> (and <funcl>)*

// <funcl_typ> ::= forall <quantifier> . <typ>
//               | <typ>

// <paren_index_range> ::= ( <paren_index_range> @ <paren_index_range> (@ <paren_index_range>)* )
//                       | <atomic_index_range>

// <atomic_index_range> ::= <typ>
//                        | <typ> .. <typ>
//                        | ( <typ> .. <typ> )

// <r_id_def> ::= <id> : <paren_index_range> (@ <paren_index_range>)*

// <r_def_body> ::= <r_id_def>
//                | <r_id_def> ,
//                | <r_id_def> , <r_def_body>

// <param_kopt> ::= <typ_var> : <kind>
//                | <typ_var>

// <typaram> ::= ( <param_kopt> (, <param_kopt>)* ) , <typ>
//             | ( <param_kopt> (, <param_kopt>)* )

// <type_def> ::= type <id> <typaram> = <typ>
//              | type <id> = <typ>
//              | type <id> <typaram> -> <kind> = <typ>
//              | type <id> : <kind> = <typ>
//              | struct <id> = { <struct_fields> }
//              | struct <id> <typaram> = { <struct_fields> }
//              | enum <id> = <id> (| <id>)*
//              | enum <id> = { <enum_comma> }
//              | enum <id> with <enum_functions> = { <enum_comma> }
//              | union <id> = { <type_unions> }
//              | union <id> <typaram> = { <type_unions> }
//              | bitfield <id> : <typ> = { <r_def_body> }

// <enum_functions> ::= <id> -> <typ> , <enum_functions>
//                    | <id> -> <typ> ,
//                    | <id> -> <typ>

// <enum_comma> ::= <id> [,]
//          | <id> => <exp> [,]
//          | <id> , <enum_comma>
//          | <id> => <exp> , <enum_comma>

// <struct_field> ::= <id> : <typ>

pub struct StructField {
    pub id: Id,
    pub typ: Typ,
}

// <struct_fields> ::= <struct_field>
//                   | <struct_field> ,
//                   | <struct_field> , <struct_fields>

pub type StructFields = Vec<StructField>;

// <type_union> ::= $[ATTRIBUTE] <type_union>
//                | <id> : <typ>
//                | <id> : { <struct_fields> }

pub enum TypeUnion {
    Attribute(Attribute, Box<TypeUnion>),
    IdTyp(Id, Typ),
    IdStructFields(Id, StructFields),
}

// <type_unions> ::= <type_union>
//                 | <type_union> ,
//                 | <type_union> , <type_unions>

pub type TypeUnions = Vec<TypeUnion>;

// <rec_measure> ::= { <pat> => <exp> }

pub struct RecMeasure {
    pub pat: Pat,
    pub exp: Exp,
}

// <fun_def> ::= function [<rec_measure>] <funcls>

// <mpat> ::= <atomic_mpat> (<pat_op> <atomic_mpat>)*
//          | <atomic_mpat> as <id>

pub enum Mpat {
    // TODO
}

// <atomic_mpat> ::= <lit>
//                 | <id>
//                 | <id> [ NUMBER ]
//                 | <id> [ NUMBER .. NUMBER ]
//                 | <id> ()
//                 | <id> ( <mpat> (, <mpat>)* )
//                 | ( <mpat> )
//                 | ( <mpat> , <mpat> (, <mpat>)* )
//                 | [ <mpat> (, <mpat>)* ]
//                 | [| |]
//                 | [| <mpat> (, <mpat>)* |]
//                 | <atomic_mpat> : <typ_no_caret>
//                 | struct { <fmpat> (, <fmpat>)* }

// <fmpat> ::= <id> = <mpat>
//           | <id>

pub struct Fmpat {
    pub id: Id,
    pub mpat: Option<Mpat>,
}

// <mpexp> ::= <mpat>
//           | <mpat> if <exp>

pub struct Mpexp {
    pub mpat: Mpat,
    pub guard: Option<Exp>,
}

// <mapcl> ::= $[ATTRIBUTE] <mapcl>
//           | <mpexp> <-> <mpexp>
//           | <mpexp> => <exp>
//           | forwards <mpexp> => <exp>
//           | backwards <mpexp> => <exp>

pub enum MapCl {
    Attribute(Attribute, Box<MapCl>),
    BiDir(Mpexp, Mpexp),
    Right(Mpexp, Exp),
    Forwards(Mpexp, Exp),
    Backwards(Mpexp, Exp),
}

// <mapcl_list> ::= <mapcl> [,]
//                | <mapcl> , <mapcl_list>

// <map_def> ::= mapping <id> = { <mapcl_list> }
//             | mapping <id> : <typschm> = { <mapcl_list> }

// <let_def> ::= let <letbind>

pub type LetDef = LetBind;

// <pure_opt> ::= monadic
//              | pure

pub enum PureOpt {
    Monadic,
    Pure,
}

// <extern_binding> ::= <id> : STRING_LITERAL
//                    | _ : STRING_LITERAL

pub struct ExternBinding {
    pub id: Id, // TODO: Underscore
    pub string_literal: StringLiteral,
}

// <externs> ::= epsilon
//             | = STRING_LITERAL
//             | = { <extern_binding> (, <extern_binding>)* }
//             | = <pure_opt> STRING_LITERAL
//             | = <pure_opt> { <extern_binding> (, <extern_binding>)* }

// <val_spec_def> ::= val STRING_LITERAL : <typschm>
//                  | val <id> <externs> : <typschm>

pub struct ValSpecDef {
    // TODO:
    // pub string_literal: StringLiteral,
    // pub id: Id,
    // pub externs: Option<Externs>,
    // pub typschm: TypSchm,
}

// <register_def> ::= register <id> : <typ>
//                  | register <id> : <typ> = <exp>

pub struct RegisterDef {
    pub id: Id,
    pub typ: Typ,
    pub init: Option<Exp>,
}

// <default_def> ::= default <kind> inc
//                 | default <kind> dec

pub enum DefaultDef {
    Inc(Kind),
    Dec(Kind),
}

// <scattered_def> ::= scattered enum <id>
//                   | scattered union <id> <typaram>
//                   | scattered union <id>
//                   | scattered function <id>
//                   | scattered mapping <id>
//                   | scattered mapping <id> : <funcl_typ>
//                   | enum clause <id> = <id>
//                   | function clause <funcl>
//                   | union clause <id> = <type_union>
//                   | mapping clause <id> = <mapcl>
//                   | end <id>

pub enum ScatteredDef {
    ScatteredEnum(Identifier),
    ScatteredUnion(Identifier, Option<TypList>),
    ScatteredFunction(Identifier),
    // ScatteredMapping(Identifier, Option<FunclTyp>),
    EnumClause(Identifier, Identifier),
    // FunctionClause(Funcl),
    UnionClause(Identifier, TypeUnion),
    // MappingClause(Identifier, Mapcl),
    End(Identifier),
}

// <loop_measure> ::= until <exp>
//                  | repeat <exp>
//                  | while <exp>

pub enum LoopMeasure {
    Until(Exp),
    Repeat(Exp),
    While(Exp),
}

// <subst> ::= <typ_var> = <typ>
//           | <id> = <id>

pub enum Subst {
    TypVar(TypVar, Typ),
    Id(Id, Id),
}

// <instantiation_def> ::= instantiation <id>
//                       | instantiation <id> with <subst> (, <subst>)*

pub struct InstantiationDef {
    pub id: Identifier,
    pub subst: Vec<Subst>,
}

// <overload_def> ::= overload <id> = { <id> (, <id>)* }
//                  | overload <id> = <id> (| <id>)*

pub struct OverloadDef {
    pub id: Identifier,
    pub overload: Vec<Identifier>,
}

// <def_aux> ::= <fun_def>
//             | <map_def>
//             | FIXITY_DEF
//             | <val_spec_def>
//             | <instantiation_def>
//             | <type_def>
//             | <let_def>
//             | <register_def>
//             | <overload_def>
//             | <scattered_def>
//             | <default_def>
//             | $LINE_DIRECTIVE
//             | termination_measure <id> <pat> = <exp>
//             | termination_measure <id> <loop_measure> (, <loop_measure>)*

pub enum DefAux {
    // FunDef(FunDef),
    // MapDef(MapDef),
    // FixityDef(FixityDef),
    ValSpecDef(ValSpecDef),
    InstantiationDef(InstantiationDef),
    // TypeDef(TypeDef),
    LetDef(LetDef),
    RegisterDef(RegisterDef),
    OverloadDef(OverloadDef),
    ScatteredDef(ScatteredDef),
    DefaultDef(DefaultDef),
    // LineDirective(LineDirective),
    // TerminationMeasure(TerminationMeasure),
}

// <def> ::= $[ATTRIBUTE] <def>
//         | <def_aux>

pub struct Def {
    pub attributes: Vec<Attribute>,
    pub def_aux: DefAux,
}

pub type File = Vec<Def>;
