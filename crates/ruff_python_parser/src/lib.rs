//! This crate can be used to parse Python source code into an Abstract
//! Syntax Tree.
//!
//! ## Overview:
//!
//! The process by which source code is parsed into an AST can be broken down
//! into two general stages: [lexical analysis] and [parsing].
//!
//! During lexical analysis, the source code is converted into a stream of lexical
//! tokens that represent the smallest meaningful units of the language. For example,
//! the source code `print("Hello world")` would _roughly_ be converted into the following
//! stream of tokens:
//!
//! ```text
//! Name("print"), LeftParen, String("Hello world"), RightParen
//! ```
//!
//! these tokens are then consumed by the `ruff_python_parser`, which matches them against a set of
//! grammar rules to verify that the source code is syntactically valid and to construct
//! an AST that represents the source code.
//!
//! During parsing, the `ruff_python_parser` consumes the tokens generated by the lexer and constructs
//! a tree representation of the source code. The tree is made up of nodes that represent
//! the different syntactic constructs of the language. If the source code is syntactically
//! invalid, parsing fails and an error is returned. After a successful parse, the AST can
//! be used to perform further analysis on the source code. Continuing with the example
//! above, the AST generated by the `ruff_python_parser` would _roughly_ look something like this:
//!
//! ```text
//! node: Expr {
//!     value: {
//!         node: Call {
//!             func: {
//!                 node: Name {
//!                     id: "print",
//!                     ctx: Load,
//!                 },
//!             },
//!             args: [
//!                 node: Constant {
//!                     value: Str("Hello World"),
//!                     kind: None,
//!                 },
//!             ],
//!             keywords: [],
//!         },
//!     },
//! },
//!```
//!
//! Note: The Tokens/ASTs shown above are not the exact tokens/ASTs generated by the `ruff_python_parser`.
//!
//! ## Source code layout:
//!
//! The functionality of this crate is split into several modules:
//!
//! - token: This module contains the definition of the tokens that are generated by the lexer.
//! - [lexer]: This module contains the lexer and is responsible for generating the tokens.
//! - `ruff_python_parser`: This module contains an interface to the `ruff_python_parser` and is responsible for generating the AST.
//!     - Functions and strings have special parsing requirements that are handled in additional files.
//! - mode: This module contains the definition of the different modes that the `ruff_python_parser` can be in.
//!
//! # Examples
//!
//! For example, to get a stream of tokens from a given string, one could do this:
//!
//! ```
//! use ruff_python_parser::{lexer::lex, Mode};
//!
//! let python_source = r#"
//! def is_odd(i):
//!     return bool(i & 1)
//! "#;
//! let mut tokens = lex(python_source, Mode::Module);
//! assert!(tokens.all(|t| t.is_ok()));
//! ```
//!
//! These tokens can be directly fed into the `ruff_python_parser` to generate an AST:
//!
//! ```
//! use ruff_python_parser::{lexer::lex, Mode, parse_tokens};
//!
//! let python_source = r#"
//! def is_odd(i):
//!    return bool(i & 1)
//! "#;
//! let tokens = lex(python_source, Mode::Module);
//! let ast = parse_tokens(tokens, python_source, Mode::Module);
//!
//! assert!(ast.is_ok());
//! ```
//!
//! Alternatively, you can use one of the other `parse_*` functions to parse a string directly without using a specific
//! mode or tokenizing the source beforehand:
//!
//! ```
//! use ruff_python_parser::parse_suite;
//!
//! let python_source = r#"
//! def is_odd(i):
//!   return bool(i & 1)
//! "#;
//! let ast = parse_suite(python_source);
//!
//! assert!(ast.is_ok());
//! ```
//!
//! [lexical analysis]: https://en.wikipedia.org/wiki/Lexical_analysis
//! [parsing]: https://en.wikipedia.org/wiki/Parsing
//! [lexer]: crate::lexer

pub use parser::{
    parse, parse_expression, parse_expression_starts_at, parse_ok_tokens, parse_ok_tokens_lalrpop,
    parse_ok_tokens_new, parse_program, parse_starts_at, parse_suite, parse_tokens, ParsedFile,
};
use ruff_python_ast::{CmpOp, Expr, Mod, PySourceType, Suite};
use ruff_text_size::{Ranged, TextRange, TextSize};
pub use token::{StringKind, Tok, TokenKind};

use crate::lexer::LexResult;

mod context;
mod error;
mod function;
mod helpers;
mod invalid;
pub mod lexer;
mod parser;
mod soft_keywords;
mod string;
mod token;
mod token_set;
pub mod typing;
pub use error::{FStringErrorType, ParseError, ParseErrorType};

/// Collect tokens up to and including the first error.
pub fn tokenize(contents: &str, mode: Mode) -> Vec<LexResult> {
    let mut tokens: Vec<LexResult> = vec![];
    for tok in lexer::lex(contents, mode) {
        let is_err = tok.is_err();
        tokens.push(tok);
        if is_err {
            break;
        }
    }
    tokens
}

/// Parse a full Python program from its tokens.
pub fn parse_program_tokens(
    lxr: Vec<LexResult>,
    source: &str,
    is_jupyter_notebook: bool,
) -> anyhow::Result<Suite, ParseError> {
    let mode = if is_jupyter_notebook {
        Mode::Ipython
    } else {
        Mode::Module
    };
    match parse_tokens(lxr, source, mode)? {
        Mod::Module(m) => Ok(m.body),
        Mod::Expression(_) => unreachable!("Mode::Module doesn't return other variant"),
    }
}

/// Extract all [`CmpOp`] operators from an expression snippet, with appropriate
/// ranges.
///
/// `RustPython` doesn't include line and column information on [`CmpOp`] nodes.
/// `CPython` doesn't either. This method iterates over the token stream and
/// re-identifies [`CmpOp`] nodes, annotating them with valid ranges.
pub fn locate_cmp_ops(expr: &Expr, source: &str) -> Vec<LocatedCmpOp> {
    // If `Expr` is a multi-line expression, we need to parenthesize it to
    // ensure that it's lexed correctly.
    let contents = &source[expr.range()];
    let parenthesized_contents = format!("({contents})");
    let mut tok_iter = lexer::lex(&parenthesized_contents, Mode::Expression)
        .flatten()
        .skip(1)
        .map(|(tok, range)| (tok, range - TextSize::from(1)))
        .filter(|(tok, _)| !matches!(tok, Tok::NonLogicalNewline | Tok::Comment(_)))
        .peekable();

    let mut ops: Vec<LocatedCmpOp> = vec![];

    // Track the bracket depth.
    let mut par_count = 0u32;
    let mut sqb_count = 0u32;
    let mut brace_count = 0u32;

    loop {
        let Some((tok, range)) = tok_iter.next() else {
            break;
        };

        match tok {
            Tok::Lpar => {
                par_count = par_count.saturating_add(1);
            }
            Tok::Rpar => {
                par_count = par_count.saturating_sub(1);
            }
            Tok::Lsqb => {
                sqb_count = sqb_count.saturating_add(1);
            }
            Tok::Rsqb => {
                sqb_count = sqb_count.saturating_sub(1);
            }
            Tok::Lbrace => {
                brace_count = brace_count.saturating_add(1);
            }
            Tok::Rbrace => {
                brace_count = brace_count.saturating_sub(1);
            }
            _ => {}
        }

        if par_count > 0 || sqb_count > 0 || brace_count > 0 {
            continue;
        }

        match tok {
            Tok::Not => {
                if let Some((_, next_range)) = tok_iter.next_if(|(tok, _)| tok.is_in()) {
                    ops.push(LocatedCmpOp::new(
                        TextRange::new(range.start(), next_range.end()),
                        CmpOp::NotIn,
                    ));
                }
            }
            Tok::In => {
                ops.push(LocatedCmpOp::new(range, CmpOp::In));
            }
            Tok::Is => {
                let op = if let Some((_, next_range)) = tok_iter.next_if(|(tok, _)| tok.is_not()) {
                    LocatedCmpOp::new(
                        TextRange::new(range.start(), next_range.end()),
                        CmpOp::IsNot,
                    )
                } else {
                    LocatedCmpOp::new(range, CmpOp::Is)
                };
                ops.push(op);
            }
            Tok::NotEqual => {
                ops.push(LocatedCmpOp::new(range, CmpOp::NotEq));
            }
            Tok::EqEqual => {
                ops.push(LocatedCmpOp::new(range, CmpOp::Eq));
            }
            Tok::GreaterEqual => {
                ops.push(LocatedCmpOp::new(range, CmpOp::GtE));
            }
            Tok::Greater => {
                ops.push(LocatedCmpOp::new(range, CmpOp::Gt));
            }
            Tok::LessEqual => {
                ops.push(LocatedCmpOp::new(range, CmpOp::LtE));
            }
            Tok::Less => {
                ops.push(LocatedCmpOp::new(range, CmpOp::Lt));
            }
            _ => {}
        }
    }
    ops
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocatedCmpOp {
    pub range: TextRange,
    pub op: CmpOp,
}

impl LocatedCmpOp {
    fn new<T: Into<TextRange>>(range: T, op: CmpOp) -> Self {
        Self {
            range: range.into(),
            op,
        }
    }
}

/// Control in the different modes by which a source file can be parsed.
/// The mode argument specifies in what way code must be parsed.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum Mode {
    /// The code consists of a sequence of statements.
    Module,
    /// The code consists of a single expression.
    Expression,
    /// The code consists of a sequence of statements which can include the
    /// escape commands that are part of IPython syntax.
    ///
    /// ## Supported escape commands:
    ///
    /// - [Magic command system] which is limited to [line magics] and can start
    ///   with `?` or `??`.
    /// - [Dynamic object information] which can start with `?` or `??`.
    /// - [System shell access] which can start with `!` or `!!`.
    /// - [Automatic parentheses and quotes] which can start with `/`, `;`, or `,`.
    ///
    /// [Magic command system]: https://ipython.readthedocs.io/en/stable/interactive/reference.html#magic-command-system
    /// [line magics]: https://ipython.readthedocs.io/en/stable/interactive/magics.html#line-magics
    /// [Dynamic object information]: https://ipython.readthedocs.io/en/stable/interactive/reference.html#dynamic-object-information
    /// [System shell access]: https://ipython.readthedocs.io/en/stable/interactive/reference.html#system-shell-access
    /// [Automatic parentheses and quotes]: https://ipython.readthedocs.io/en/stable/interactive/reference.html#automatic-parentheses-and-quotes
    Ipython,
}

impl std::str::FromStr for Mode {
    type Err = ModeParseError;
    fn from_str(s: &str) -> Result<Self, ModeParseError> {
        match s {
            "exec" | "single" => Ok(Mode::Module),
            "eval" => Ok(Mode::Expression),
            "ipython" => Ok(Mode::Ipython),
            _ => Err(ModeParseError),
        }
    }
}

pub trait AsMode {
    fn as_mode(&self) -> Mode;
}

impl AsMode for PySourceType {
    fn as_mode(&self) -> Mode {
        match self {
            PySourceType::Python | PySourceType::Stub => Mode::Module,
            PySourceType::Ipynb => Mode::Ipython,
        }
    }
}

/// Returned when a given mode is not valid.
#[derive(Debug)]
pub struct ModeParseError;

impl std::fmt::Display for ModeParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, r#"mode must be "exec", "eval", "ipython", or "single""#)
    }
}

#[rustfmt::skip]
#[allow(unreachable_pub)]
#[allow(clippy::type_complexity)]
#[allow(clippy::extra_unused_lifetimes)]
#[allow(clippy::needless_lifetimes)]
#[allow(clippy::unused_self)]
#[allow(clippy::cast_sign_loss)]
#[allow(clippy::default_trait_access)]
#[allow(clippy::let_unit_value)]
#[allow(clippy::just_underscores_and_digits)]
#[allow(clippy::no_effect_underscore_binding)]
#[allow(clippy::trivially_copy_pass_by_ref)]
#[allow(clippy::option_option)]
#[allow(clippy::unnecessary_wraps)]
#[allow(clippy::uninlined_format_args)]
#[allow(clippy::cloned_instead_of_copied)]
mod python {

    #[cfg(feature = "lalrpop")]
    include!(concat!(env!("OUT_DIR"), "/src/python.rs"));

    #[cfg(not(feature = "lalrpop"))]
    include!("python.rs");
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use ruff_python_ast::CmpOp;
    use ruff_text_size::TextSize;

    use crate::{locate_cmp_ops, parse_expression, LocatedCmpOp};

    #[test]
    fn extract_cmp_op_location() -> Result<()> {
        let contents = "x == 1";
        let expr = parse_expression(contents)?;
        assert_eq!(
            locate_cmp_ops(&expr, contents),
            vec![LocatedCmpOp::new(
                TextSize::from(2)..TextSize::from(4),
                CmpOp::Eq
            )]
        );

        let contents = "x != 1";
        let expr = parse_expression(contents)?;
        assert_eq!(
            locate_cmp_ops(&expr, contents),
            vec![LocatedCmpOp::new(
                TextSize::from(2)..TextSize::from(4),
                CmpOp::NotEq
            )]
        );

        let contents = "x is 1";
        let expr = parse_expression(contents)?;
        assert_eq!(
            locate_cmp_ops(&expr, contents),
            vec![LocatedCmpOp::new(
                TextSize::from(2)..TextSize::from(4),
                CmpOp::Is
            )]
        );

        let contents = "x is not 1";
        let expr = parse_expression(contents)?;
        assert_eq!(
            locate_cmp_ops(&expr, contents),
            vec![LocatedCmpOp::new(
                TextSize::from(2)..TextSize::from(8),
                CmpOp::IsNot
            )]
        );

        let contents = "x in 1";
        let expr = parse_expression(contents)?;
        assert_eq!(
            locate_cmp_ops(&expr, contents),
            vec![LocatedCmpOp::new(
                TextSize::from(2)..TextSize::from(4),
                CmpOp::In
            )]
        );

        let contents = "x not in 1";
        let expr = parse_expression(contents)?;
        assert_eq!(
            locate_cmp_ops(&expr, contents),
            vec![LocatedCmpOp::new(
                TextSize::from(2)..TextSize::from(8),
                CmpOp::NotIn
            )]
        );

        let contents = "x != (1 is not 2)";
        let expr = parse_expression(contents)?;
        assert_eq!(
            locate_cmp_ops(&expr, contents),
            vec![LocatedCmpOp::new(
                TextSize::from(2)..TextSize::from(4),
                CmpOp::NotEq
            )]
        );

        Ok(())
    }
}
