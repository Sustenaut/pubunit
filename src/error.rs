//! The one error type.

use core::fmt;

use crate::Dimension;

/// Why an expression, a definition or a check failed.
///
/// Each variant carries what a message needs. [`Display`](fmt::Display)
/// writes one line for a person; match on the variant to act on it.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// The text is not a UCUM expression.
    Syntax {
        /// The expression.
        expr: String,
        /// The byte offset where reading stopped.
        position: usize,
        /// What was found there.
        message: String,
    },
    /// ucum-units refused the expression. pubunit accepts nothing it
    /// refuses, so the two never disagree about what is valid UCUM.
    Ucum {
        /// The expression, with declared atoms replaced, as ucum-units saw it.
        expr: String,
        /// ucum-units' message.
        message: String,
    },
    /// An input beyond one of pubunit's limits.
    TooLarge {
        /// What was too large, and the limit.
        what: String,
    },
    /// A symbol that is neither a UCUM atom, a prefixed metric atom, nor a
    /// declared one.
    UnknownSymbol {
        /// The symbol.
        symbol: String,
    },
    /// A brace annotation standing alone, such as `{trays}`, that no
    /// definition declares, under [`Annotations::Strict`](crate::Annotations::Strict).
    UndeclaredAnnotation {
        /// The annotation, with its braces.
        annotation: String,
    },
    /// A special unit (logarithmic, or a temperature with an offset) where
    /// only a linear unit can be used. Temperatures with an offset convert
    /// only when they stand alone.
    NotLinear {
        /// The symbol.
        symbol: String,
    },
    /// An arbitrary unit, such as `[IU]`, which UCUM makes commensurable
    /// with nothing.
    Arbitrary {
        /// The symbol.
        symbol: String,
    },
    /// Two units of different dimensions.
    Incommensurable {
        /// The unit converted from.
        from: String,
        /// Its dimension.
        from_dimension: Dimension,
        /// The unit converted to.
        to: String,
        /// Its dimension.
        to_dimension: Dimension,
    },
    /// A quantity kind no definition declares.
    UnknownKind {
        /// The name asked for.
        kind: String,
    },
    /// A unit whose dimension is not its kind's.
    WrongDimension {
        /// The unit.
        unit: String,
        /// The kind.
        kind: String,
        /// The kind's dimension.
        expected: Dimension,
        /// The unit's.
        found: Dimension,
    },
    /// A unit of the right dimension that its kind does not list.
    NotAllowed {
        /// The unit.
        unit: String,
        /// The kind.
        kind: String,
        /// The units the kind allows.
        allowed: Vec<String>,
    },
    /// A definition that cannot be used.
    Definition {
        /// The symbol or name being defined.
        name: String,
        /// Why.
        message: String,
    },
    /// Text that is not a number.
    Number {
        /// The text.
        text: String,
        /// Why.
        message: String,
    },
    /// Definitions that are not valid JSON of the definitions format.
    Json {
        /// The JSON parser's message.
        message: String,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Syntax {
                expr,
                position,
                message,
            } => write!(f, "`{expr}` is not UCUM: {message} at byte {position}"),
            Error::Ucum { expr, message } => write!(f, "ucum-units refuses `{expr}`: {message}"),
            Error::TooLarge { what } => write!(f, "too large: {what}"),
            Error::UnknownSymbol { symbol } => write!(f, "`{symbol}` is not a known unit"),
            Error::UndeclaredAnnotation { annotation } => write!(
                f,
                "`{annotation}` stands alone but no definition declares it"
            ),
            Error::NotLinear { symbol } => write!(
                f,
                "`{symbol}` is a special unit and cannot be used here; only linear units can"
            ),
            Error::Arbitrary { symbol } => write!(
                f,
                "`{symbol}` is an arbitrary unit, commensurable with nothing"
            ),
            Error::Incommensurable {
                from,
                from_dimension,
                to,
                to_dimension,
            } => write!(
                f,
                "`{from}` ({from_dimension}) and `{to}` ({to_dimension}) are of different dimensions"
            ),
            Error::UnknownKind { kind } => write!(f, "no quantity kind is named `{kind}`"),
            Error::WrongDimension {
                unit,
                kind,
                expected,
                found,
            } => write!(f, "`{unit}` is {found}, but `{kind}` is {expected}"),
            Error::NotAllowed {
                unit,
                kind,
                allowed,
            } => write!(
                f,
                "`{kind}` does not allow `{unit}`; it allows {}",
                allowed.join(", ")
            ),
            Error::Definition { name, message } => write!(f, "definition of `{name}`: {message}"),
            Error::Number { text, message } => write!(f, "`{text}` is not a number: {message}"),
            Error::Json { message } => write!(f, "definitions: {message}"),
        }
    }
}

impl std::error::Error for Error {}
