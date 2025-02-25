use memchr::{memchr, memchr_iter};

use ruff_diagnostics::{Diagnostic, Violation};
use ruff_macros::{derive_message_formats, ViolationMetadata};
use ruff_source_file::Line;
use ruff_text_size::{TextRange, TextSize};

/// ## What it does
/// Checks for form feed characters preceded by either a space or a tab.
///
/// ## Why is this bad?
/// [The language reference][lexical-analysis-indentation] states:
///
/// > A formfeed character may be present at the start of the line;
/// > it will be ignored for the indentation calculations above.
/// > Formfeed characters occurring elsewhere in the leading whitespace
/// > have an undefined effect (for instance, they may reset the space count to zero).
///
/// ## Example
///
/// ```python
/// if foo():\n    \fbar()
/// ```
///
/// Use instead:
///
/// ```python
/// if foo():\n    bar()
/// ```
///
/// [lexical-analysis-indentation]: https://docs.python.org/3/reference/lexical_analysis.html#indentation
#[derive(ViolationMetadata)]
pub(crate) struct IndentedFormFeed;

impl Violation for IndentedFormFeed {
    #[derive_message_formats]
    fn message(&self) -> String {
        "Indented form feed".to_string()
    }

    fn fix_title(&self) -> Option<String> {
        Some("Remove form feed".to_string())
    }
}

const FORM_FEED: u8 = b'\x0c';
const SPACE: u8 = b' ';
const TAB: u8 = b'\t';
const NEW_LINE: u8 = b'\n';

/// RUF054
pub(crate) fn indented_form_feed(line: &Line) -> Option<Diagnostic> {
    println!("{:?}", line);
    let line_as_byte = line.as_bytes();
    for (i, a) in line.split(|char| char == '\n').enumerate() {
        println!("{:?}", a);
        println!("{} ----", i);
    }

    for (i, logical_line) in line_as_byte.split(|&b| b == NEW_LINE).enumerate() {
        for j in 1..logical_line.len() {
            if logical_line[j] == FORM_FEED {
                if (i == 0 && (logical_line[j - 1] == SPACE || logical_line[j - 1] == TAB))
                    || logical_line[j - 1] == TAB
                {
                    let relative_index = u32::try_from(i + j).ok()?;
                    let absolute_index = line.start() + TextSize::new(relative_index);
                    let range = TextRange::at(absolute_index, 1.into());

                    return Some(Diagnostic::new(IndentedFormFeed, range));
                }
            }
        }
    }
    None
}
