use chumsky::{Parser, prelude::Rich, extra, IterParser, primitive::{any, just, choice}, select};

use crate::{Spanned, lexer::{Token, Span}};

// Loosly based on `src/lib/parse_ast.ml` in the Sail compiler.

pub trait Node {
    fn span(&self) -> Span;
    fn child_at_pos(&self, pos: usize) -> Option<&dyn Node>;
}

pub struct Extern {
    pub pure: bool,
    pub bindings: Vec<(String, String)>,
}

pub struct Identifier {
    pub name: String,
}

// Ticked identifier, e.g. 'a
pub struct KindIdentifier {
    pub name: String,
}

pub struct InfixIdentifier {
    pub name: String,
}

pub enum Kind {
    Type,
    Int,
    Order,
    Bool,
}

pub enum Literal {
    Unit, // ()
    Zero,
    One,
    True,
    False,
    Num(String), // Arbitrary precision integer
    Hex(String), // 0xF00 bit vector
    Bin(String), // 0b1010 bit vector
    Undef, // Undefined value
    String(String),
    Real(String),
}

// `atyp` in the Sail code.
pub enum Type {
    Identifier(Identifier),
    KindIdentifier(KindIdentifier),
    Literal(Literal),
    Set, // TODO
    Times(Box<Type>, Box<Type>),
    Plus(Box<Type>, Box<Type>),
    Minus(Box<Type>, Box<Type>),
    Exp(Box<Type>), // Exponential
    Neg(Box<Type>), // Negate (internal?)
    Increasing,
    Decreasing,
    FunctionType(Box<Type>, Box<Type>),
    MappingType(Box<Type>, Box<Type>),
    Wildcard,
    TupleType(Vec<Type>),
    TypeConstructorApplication(Identifier, Vec<Type>),
    Exist, // TODO
    Parentheses(Box<Type>),
}

// Kind annotated variable?
struct KindedId {
    ident: Option<Identifier>,
    kind_ids: Vec<KindIdentifier>,
    kind: Option<Kind>,
}

enum QuantifierItem {
    Id(KindedId),
    Constraint(Type),
}

// enum TypeQuantifier {
//     Forall(Vec<QuantifierItem>),
//     Exists(Vec<QuantifierItem>),
// }

// TODO: more complex type stuff.

pub enum Pattern {
    Literal(Literal),
    Wildcard,
    Typed((Type, Box<Pattern>)),
    Identifier(Identifier),
    BindVar((Box<Pattern>, Type)), // Bind pattern to type variable.
    UnionConstructor((Identifier, Vec<Pattern>)),
    // TODO: There's more.
}

pub enum FieldPattern {
    Field((Identifier, Pattern)),
    Wildcard,
}

pub enum Expression {
    Block(Vec<Expression>),
    Identifier(Identifier),
    Reference(Identifier),
    Dereference(Identifier),
    Literal(Literal),
    TypeCast((Type, Box<Expression>)),
    FunctionApplication((Identifier, Vec<Expression>)),
    InfixFunctionApplication((Box<Expression>, Identifier, Box<Expression>)),
    Tuple(Vec<Expression>),
    // TODO: Loads more.
}

// Optional default value for indexed vectors.
pub type OptDefault = Option<Expression>;

// Pattern match
pub enum PatternMatchExpression {
    Expression((Pattern, Expression)),
    When((Pattern, Expression, Expression)),
}

pub struct LetBind {
    pattern: Pattern,
    expression: Expression,
}

// ...

enum ScatteredDef {
    // TODO: Add data.
    Function,
    FunctionClause,
    Enum,
    EnumClause,
    Union,
    UnionClause,
    Mapping,
    MappingClause,
    End,
}

pub struct OverloadDef {
    pub overload_id: Spanned<Identifier>,
    pub target_ids: Vec<Spanned<Identifier>>,
}

// Top level definitions.
pub enum Def {
    Type,
    Fundef,
    Mapdef,
    Impl,
    Let,
    Val,
    Outcome,
    Instantiation,
    Fixity,
    Overload(OverloadDef),
    Default,
    Scattered(ScatteredDef),
    Measure,
    LoopMeasures,
    Register,
    // InternalMutrec,
    Pragma,
}
