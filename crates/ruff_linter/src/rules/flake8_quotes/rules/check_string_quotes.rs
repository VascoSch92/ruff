use ruff_diagnostics::{AlwaysFixableViolation, Diagnostic, Edit, Fix};
use ruff_macros::{derive_message_formats, violation};
use ruff_python_ast::StringLike;
use ruff_text_size::{Ranged, TextRange};

use crate::checkers::ast::Checker;

use super::super::settings::Quote;

/// ## What it does
/// Checks for inline strings that use single quotes or double quotes,
/// depending on the value of the [`lint.flake8-quotes.inline-quotes`] option.
///
/// ## Why is this bad?
/// Consistency is good. Use either single or double quotes for inline
/// strings, but be consistent.
///
/// ## Example
/// ```python
/// foo = 'bar'
/// ```
///
/// Assuming `inline-quotes` is set to `double`, use instead:
/// ```python
/// foo = "bar"
/// ```
///
/// ## Options
/// - `lint.flake8-quotes.inline-quotes`
///
/// ## Formatter compatibility
/// We recommend against using this rule alongside the [formatter]. The
/// formatter enforces consistent quotes for inline strings, making the rule
/// redundant.
///
/// [formatter]: https://docs.astral.sh/ruff/formatter
#[violation]
pub struct BadQuotesInlineString {
    preferred_quote: Quote,
}

impl AlwaysFixableViolation for BadQuotesInlineString {
    #[derive_message_formats]
    fn message(&self) -> String {
        let BadQuotesInlineString { preferred_quote } = self;
        match preferred_quote {
            Quote::Double => format!("Single quotes found but double quotes preferred"),
            Quote::Single => format!("Double quotes found but single quotes preferred"),
        }
    }

    fn fix_title(&self) -> String {
        let BadQuotesInlineString { preferred_quote } = self;
        match preferred_quote {
            Quote::Double => "Replace single quotes with double quotes".to_string(),
            Quote::Single => "Replace double quotes with single quotes".to_string(),
        }
    }
}

/// ## What it does
/// Checks for multiline strings that use single quotes or double quotes,
/// depending on the value of the [`lint.flake8-quotes.multiline-quotes`]
/// setting.
///
/// ## Why is this bad?
/// Consistency is good. Use either single or double quotes for multiline
/// strings, but be consistent.
///
/// ## Example
/// ```python
/// foo = '''
/// bar
/// '''
/// ```
///
/// Assuming `multiline-quotes` is set to `double`, use instead:
/// ```python
/// foo = """
/// bar
/// """
/// ```
///
/// ## Options
/// - `lint.flake8-quotes.multiline-quotes`
///
/// ## Formatter compatibility
/// We recommend against using this rule alongside the [formatter]. The
/// formatter enforces double quotes for multiline strings, making the rule
/// redundant.
///
/// [formatter]: https://docs.astral.sh/ruff/formatter
#[violation]
pub struct BadQuotesMultilineString {
    preferred_quote: Quote,
}

impl AlwaysFixableViolation for BadQuotesMultilineString {
    #[derive_message_formats]
    fn message(&self) -> String {
        let BadQuotesMultilineString { preferred_quote } = self;
        match preferred_quote {
            Quote::Double => format!("Single quote multiline found but double quotes preferred"),
            Quote::Single => format!("Double quote multiline found but single quotes preferred"),
        }
    }

    fn fix_title(&self) -> String {
        let BadQuotesMultilineString { preferred_quote } = self;
        match preferred_quote {
            Quote::Double => "Replace single multiline quotes with double quotes".to_string(),
            Quote::Single => "Replace double multiline quotes with single quotes".to_string(),
        }
    }
}

/// ## What it does
/// Checks for docstrings that use single quotes or double quotes, depending
/// on the value of the [`lint.flake8-quotes.docstring-quotes`] setting.
///
/// ## Why is this bad?
/// Consistency is good. Use either single or double quotes for docstring
/// strings, but be consistent.
///
/// ## Example
/// ```python
/// '''
/// bar
/// '''
/// ```
///
/// Assuming `docstring-quotes` is set to `double`, use instead:
/// ```python
/// """
/// bar
/// """
/// ```
///
/// ## Options
/// - `lint.flake8-quotes.docstring-quotes`
///
/// ## Formatter compatibility
/// We recommend against using this rule alongside the [formatter]. The
/// formatter enforces double quotes for docstrings, making the rule
/// redundant.
///
/// [formatter]: https://docs.astral.sh/ruff/formatter
#[violation]
pub struct BadQuotesDocstring {
    preferred_quote: Quote,
}

impl AlwaysFixableViolation for BadQuotesDocstring {
    #[derive_message_formats]
    fn message(&self) -> String {
        let BadQuotesDocstring { preferred_quote } = self;
        match preferred_quote {
            Quote::Double => format!("Single quote docstring found but double quotes preferred"),
            Quote::Single => format!("Double quote docstring found but single quotes preferred"),
        }
    }

    fn fix_title(&self) -> String {
        let BadQuotesDocstring { preferred_quote } = self;
        match preferred_quote {
            Quote::Double => "Replace single quotes docstring with double quotes".to_string(),
            Quote::Single => "Replace double quotes docstring with single quotes".to_string(),
        }
    }
}

const fn good_multiline(quote: Quote) -> &'static str {
    match quote {
        Quote::Double => "\"\"\"",
        Quote::Single => "'''",
    }
}

const fn good_multiline_ending(quote: Quote) -> &'static str {
    match quote {
        Quote::Double => "\"'''",
        Quote::Single => "'\"\"\"",
    }
}

const fn good_docstring(quote: Quote) -> &'static str {
    match quote {
        Quote::Double => "\"",
        Quote::Single => "'",
    }
}

#[derive(Debug)]
struct Trivia<'a> {
    last_quote_char: char,
    prefix: &'a str,
    raw_text: &'a str,
    is_multiline: bool,
}

impl<'a> From<&'a str> for Trivia<'a> {
    fn from(value: &'a str) -> Self {
        // Remove any prefixes (e.g., remove `u` from `u"foo"`).
        let last_quote_char = value.chars().last().unwrap();
        let first_quote_char = value.find(last_quote_char).unwrap();
        let prefix = &value[..first_quote_char];
        let raw_text = &value[first_quote_char..];

        // Determine if the string is multiline-based.
        let is_multiline = if raw_text.len() >= 3 {
            let mut chars = raw_text.chars();
            let first = chars.next().unwrap();
            let second = chars.next().unwrap();
            let third = chars.next().unwrap();
            first == second && second == third
        } else {
            false
        };

        Self {
            last_quote_char,
            prefix,
            raw_text,
            is_multiline,
        }
    }
}

/// Q002
fn docstring(checker: &mut Checker, range: TextRange) {
    let quotes_settings = &checker.settings.flake8_quotes;

    let text = checker.locator().slice(range);
    let trivia: Trivia = text.into();

    if trivia
        .raw_text
        .contains(good_docstring(quotes_settings.docstring_quotes))
    {
        return;
    }

    let mut diagnostic = Diagnostic::new(
        BadQuotesDocstring {
            preferred_quote: quotes_settings.docstring_quotes,
        },
        range,
    );
    let quote_count = if trivia.is_multiline { 3 } else { 1 };
    let string_contents = &trivia.raw_text[quote_count..trivia.raw_text.len() - quote_count];
    let quote = good_docstring(quotes_settings.docstring_quotes).repeat(quote_count);
    let mut fixed_contents =
        String::with_capacity(trivia.prefix.len() + string_contents.len() + quote.len() * 2);
    fixed_contents.push_str(trivia.prefix);
    fixed_contents.push_str(&quote);
    fixed_contents.push_str(string_contents);
    fixed_contents.push_str(&quote);
    diagnostic.set_fix(Fix::safe_edit(Edit::range_replacement(
        fixed_contents,
        range,
    )));
    checker.diagnostics.push(diagnostic);
}

/// Q000, Q001
fn strings(checker: &mut Checker, sequence: &[TextRange]) {
    let quotes_settings = &checker.settings.flake8_quotes;

    let trivia = sequence
        .iter()
        .map(|range| {
            let text = checker.locator().slice(*range);
            let trivia: Trivia = text.into();
            trivia
        })
        .collect::<Vec<_>>();

    // Return `true` if any of the strings are inline strings that contain the quote
    // character in the body.
    let relax_quote = trivia.iter().any(|trivia| {
        if trivia.is_multiline {
            return false;
        }

        if trivia.last_quote_char == quotes_settings.inline_quotes.as_char() {
            return false;
        }

        let string_contents = &trivia.raw_text[1..trivia.raw_text.len() - 1];
        string_contents.contains(quotes_settings.inline_quotes.as_char())
    });

    for (range, trivia) in sequence.iter().zip(trivia) {
        if trivia.is_multiline {
            // If our string is or contains a known good string, ignore it.
            if trivia
                .raw_text
                .contains(good_multiline(quotes_settings.multiline_quotes))
            {
                continue;
            }

            // If our string ends with a known good ending, then ignore it.
            if trivia
                .raw_text
                .ends_with(good_multiline_ending(quotes_settings.multiline_quotes))
            {
                continue;
            }

            let mut diagnostic = Diagnostic::new(
                BadQuotesMultilineString {
                    preferred_quote: quotes_settings.multiline_quotes,
                },
                *range,
            );

            let string_contents = &trivia.raw_text[3..trivia.raw_text.len() - 3];
            let quote = good_multiline(quotes_settings.multiline_quotes);
            let mut fixed_contents = String::with_capacity(
                trivia.prefix.len() + string_contents.len() + quote.len() * 2,
            );
            fixed_contents.push_str(trivia.prefix);
            fixed_contents.push_str(quote);
            fixed_contents.push_str(string_contents);
            fixed_contents.push_str(quote);
            diagnostic.set_fix(Fix::safe_edit(Edit::range_replacement(
                fixed_contents,
                *range,
            )));
            checker.diagnostics.push(diagnostic);
        } else if trivia.last_quote_char != quotes_settings.inline_quotes.as_char()
            // If we're not using the preferred type, only allow use to avoid escapes.
            && !relax_quote
        {
            let mut diagnostic = Diagnostic::new(
                BadQuotesInlineString {
                    preferred_quote: quotes_settings.inline_quotes,
                },
                *range,
            );
            let quote = quotes_settings.inline_quotes.as_char();
            let string_contents = &trivia.raw_text[1..trivia.raw_text.len() - 1];
            let mut fixed_contents =
                String::with_capacity(trivia.prefix.len() + string_contents.len() + 2);
            fixed_contents.push_str(trivia.prefix);
            fixed_contents.push(quote);
            fixed_contents.push_str(string_contents);
            fixed_contents.push(quote);
            diagnostic.set_fix(Fix::safe_edit(Edit::range_replacement(
                fixed_contents,
                *range,
            )));
            checker.diagnostics.push(diagnostic);
        }
    }
}

/// Generate `flake8-quote` diagnostics from a token stream.
pub(crate) fn check_string_quotes(checker: &mut Checker, string_like: StringLike) {
    let ranges: Vec<TextRange> = match string_like {
        StringLike::String(node) => node.value.iter().map(Ranged::range).collect(),
        StringLike::Bytes(node) => node.value.iter().map(Ranged::range).collect(),
        StringLike::FString(node) => node.value.iter().map(Ranged::range).collect(),
    };

    if checker.semantic().in_docstring() {
        for range in ranges {
            docstring(checker, range);
        }
    } else {
        strings(checker, &ranges);
    }
}
