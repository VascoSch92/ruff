use ruff_diagnostics::{Applicability, Diagnostic, Edit, Fix, FixAvailability, Violation};
use ruff_macros::{derive_message_formats, ViolationMetadata};
use ruff_python_ast::{self as ast, Expr, ExprCall, ExprName, };
use ruff_python_semantic::analyze::type_inference::{NumberLike, PythonType, ResolvedPythonType};
use ruff_text_size::Ranged;

use crate::checkers::ast::Checker;
use crate::fix::snippet::SourceCodeSnippet;

/// ## What it does
/// Checks for uses of `bin(...)[2:]` (or `hex`, or `oct`) to convert
/// an integer into a string.
///
/// ## Why is this bad?
/// When converting an integer to a baseless binary, hexadecimal, or octal
/// string, using f-strings is more concise and readable than using the
/// `bin`, `hex`, or `oct` functions followed by a slice.
///
/// ## Example
/// ```python
/// print(bin(1337)[2:])
/// ```
///
/// Use instead:
/// ```python
/// print(f"{1337:b}")
/// ```
#[derive(ViolationMetadata)]
pub(crate) struct FStringNumberFormat {
    replacement: Option<SourceCodeSnippet>,
    base: Base,
}

impl Violation for FStringNumberFormat {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Sometimes;

    #[derive_message_formats]
    fn message(&self) -> String {
        let FStringNumberFormat { replacement, base } = self;
        let function_name = base.function_name();

        if let Some(display) = replacement
            .as_ref()
            .and_then(SourceCodeSnippet::full_display)
        {
            format!("Replace `{function_name}` call with `{display}`")
        } else {
            format!("Replace `{function_name}` call with f-string")
        }
    }

    fn fix_title(&self) -> Option<String> {
        if let Some(display) = self
            .replacement
            .as_ref()
            .and_then(SourceCodeSnippet::full_display)
        {
            Some(format!("Replace with `{display}`"))
        } else {
            Some("Replace with f-string".to_string())
        }
    }
}

/// FURB116
pub(crate) fn fstring_number_format(checker: &Checker, subscript: &ast::ExprSubscript) {
    // The slice must be exactly `[2:]`.
    let Expr::Slice(ast::ExprSlice {
        lower: Some(lower),
        upper: None,
        step: None,
        ..
    }) = subscript.slice.as_ref()
    else {
        return;
    };

    let Expr::NumberLiteral(ast::ExprNumberLiteral {
        value: ast::Number::Int(int),
        ..
    }) = lower.as_ref()
    else {
        return;
    };

    if *int != 2 {
        return;
    }

    // The call must be exactly `hex(...)`, `bin(...)`, or `oct(...)`.
    let Expr::Call(ExprCall {
        func, arguments, ..
    }) = subscript.value.as_ref()
    else {
        return;
    };

    if !arguments.keywords.is_empty() {
        return;
    }

    let [arg] = &*arguments.args else {
        return;
    };

    // float and complex number literal must be ignored by the rule
    if matches!(
        ResolvedPythonType::from(arg),
        ResolvedPythonType::Atom(PythonType::Number(NumberLike::Float | NumberLike::Complex))) {
        return;
    } else if matches!(
        arg,
        Expr::Call(ExprCall{
            func: Expr::Name(ExprName {
                id: ExprNameId::Name(name),
                ..
            }),
            ..
        }) if name == "complex"
    ){

    }
    println!("{:?}", checker.semantic().resolve_qualified_name());
    let Some(id) = checker.semantic().resolve_builtin_symbol(func) else {
        return;
    };

    let Some(base) = Base::from_str(id) else {
        return;
    };

    // Generate a replacement, if possible.
    let replacement = if matches!(
        arg,
        Expr::NumberLiteral(_) | Expr::Name(_) | Expr::Attribute(_)
    ) {
        let inner_source = checker.locator().slice(arg);

        let quote = checker.stylist().quote();
        let shorthand = base.shorthand();

        Some(format!("f{quote}{{{inner_source}:{shorthand}}}{quote}"))
    } else {
        None
    };

    let applicability: Applicability = match ResolvedPythonType::from(arg) {
        ResolvedPythonType::Atom(PythonType::Number(NumberLike::Integer)) => {
            // if we can assert that the integer is positive, then the fix is safe
            Applicability::Safe
        }
        ResolvedPythonType::Atom(PythonType::Number(NumberLike::Bool)) => Applicability::Safe,
        _ => Applicability::Unsafe,
    };

    let replacement_range = subscript.range();
    let mut diagnostic = Diagnostic::new(
        FStringNumberFormat {
            replacement: replacement.as_deref().map(SourceCodeSnippet::from_str),
            base,
        },
        replacement_range,
    );

    if let Some(replacement) = replacement {
        diagnostic.set_fix(Fix::applicable_edit(
            Edit::range_replacement(
                replacement,
                replacement_range,
            ),
            applicability,
        ));
    }

    checker.report_diagnostic(diagnostic);
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum Base {
    Hex,
    Bin,
    Oct,
}

impl Base {
    /// Returns the shorthand for the base.
    fn shorthand(self) -> &'static str {
        match self {
            Base::Hex => "x",
            Base::Bin => "b",
            Base::Oct => "o",
        }
    }

    /// Returns the builtin function name for the base.
    fn function_name(self) -> &'static str {
        match self {
            Base::Hex => "hex",
            Base::Bin => "bin",
            Base::Oct => "oct",
        }
    }

    /// Parses the base from a string.
    fn from_str(s: &str) -> Option<Self> {
        match s {
            "hex" => Some(Base::Hex),
            "bin" => Some(Base::Bin),
            "oct" => Some(Base::Oct),
            _ => None,
        }
    }
}
