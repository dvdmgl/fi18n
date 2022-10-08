/*!
fi18n errors
*/

use fluent_syntax::parser::ParserError;
use std::{cmp, fmt, io};

/// Syntax errors
///
/// Wraps [`fluent_syntax::parser::ParserError`] to a single error
#[derive(Debug)]
pub struct FluentResourceError {
    ftl: String,
    origin: Option<String>,
    errs: Vec<ParserError>,
}

impl FluentResourceError {
    pub fn new(ftl: &str, origin: Option<&str>, errs: Vec<ParserError>) -> Self {
        Self {
            ftl: ftl.to_string(),
            origin: origin.map(|s| s.to_string()),
            errs,
        }
    }
}

/// Display shows the syntax errors and lines
impl fmt::Display for FluentResourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "While parsing resource")?;

        if let Some(source) = &self.origin {
            write!(f, " `{source}`")?;
        }
        writeln!(f, ", the following errors where found:")?;
        for ParserError { slice, kind, pos } in self.errs.iter() {
            let rg = slice.clone().unwrap_or_else(|| pos.clone());
            let line_start = self.ftl[..rg.start].matches('\n').count() + 1;
            let part = &self.ftl[rg];
            let line_end = cmp::max(part.matches('\n').count() + line_start - 1, 1);
            writeln!(f, "Lines {line_start} to {line_end} with {kind} {kind:?}")?;
            writeln!(f, "'''")?;
            writeln!(f, "{}", part)?;
            writeln!(f, "'''")?;
        }
        Ok(())
    }
}

impl std::error::Error for FluentResourceError {}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    FluentError(#[from] fluent_bundle::FluentError),
    #[error("Multiple fluent errors {0:#?}")]
    MultipleFluentError(Vec<fluent_bundle::FluentError>),
    #[error(transparent)]
    LanguageIdentifierError(#[from] unic_langid::LanguageIdentifierError),
    #[error("Locale `{0}` does not exist")]
    LocaleUnavailable(unic_langid::LanguageIdentifier),
    #[error(
        r#"Error while parsing resource locale `{locale}` in lines {line_start} to {line_end} \n {part} "#
    )]
    ParseError {
        line_start: usize,
        line_end: usize,
        locale: unic_langid::LanguageIdentifier,
        part: String,
    },
    #[error("probles {0:?}")]
    Overriding(Vec<String>),
    #[error("Unexpected ")]
    Unexpected,
    #[error(transparent)]
    FluentResourceError(#[from] FluentResourceError),
}
