/*!
Key split message and attribute
*/

use std::{
    convert::TryFrom,
    fmt::{Debug, Display},
};

#[derive(thiserror::Error, Debug, PartialEq, Eq)]
pub enum ParserError {
    #[error("The given key is empty")]
    Empty,
    #[error("The given key `{0}` has invalid chars")]
    InvalidChars(String),
    #[error("The given key `{0}` has more than one attribute")]
    ToManyAttributes(String),
}

/// Fkey support for fluent message and attribute.
///
/// Example:
/// ```
/// use fi18n::{Fkey, fkey::ParserError};
/// assert_eq!(Fkey::try_from("a.b"), Ok(Fkey::new("a", Some("b"))));
/// assert_eq!(Fkey::try_from("a"), Ok(Fkey::new("a", None)));
/// assert_eq!(Fkey::try_from(""), Err(ParserError::Empty));
/// assert_eq!(Fkey::try_from("a.b.c"), Err(ParserError::ToManyAttributes("a.b.c".into())));
/// ```
#[derive(PartialEq, Eq, Hash)]
pub struct Fkey<'a>(pub(super) &'a str, pub(super) Option<&'a str>);

impl<'a> Fkey<'a> {
    pub fn new(message: &'a str, attribute: Option<&'a str>) -> Self {
        Self(message, attribute)
    }

    pub fn message(&self) -> &'a str {
        self.0
    }

    pub fn attribute(&self) -> Option<&'a str> {
        self.1
    }
}

impl<'a> Display for Fkey<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.1 {
            Some(v) => write!(f, "{}.{}", self.0, v),
            None => write!(f, "{}", self.0),
        }
    }
}

impl<'a> Debug for Fkey<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Fkey message: {}, attribute: {}",
            self.0,
            self.1.unwrap_or("None")
        )
    }
}

impl<'a> TryFrom<&'a str> for Fkey<'a> {
    type Error = ParserError;
    /// Converts a str slice into a [`Fkey`] variant.
    /// Path cannot by empty and contains, at most, one attribute.
    #[inline]
    fn try_from(s: &'a str) -> Result<Self, Self::Error> {
        if s.is_empty() {
            return Err(ParserError::Empty);
        } else if !s
            .chars()
            .all(|c| c.is_alphanumeric() || c == '.' || c == '-')
        {
            return Err(ParserError::InvalidChars(s.into()));
        }
        let mut dot_path = s.split('.');
        let out = Self::new(dot_path.next().unwrap(), dot_path.next());

        if dot_path.next().is_some() {
            Err(ParserError::ToManyAttributes(s.into()))
        } else {
            Ok(out)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Fkey;

    #[test]
    fn converts_from_str() {
        let expected = Fkey::new("key", None);
        let k: Fkey = "key".try_into().unwrap();
        assert_eq!(expected, k);
        let expected = Fkey::new("key", Some("attribute"));
        let k: Fkey = "key.attribute".try_into().unwrap();
        assert_eq!(expected, k);
    }

    #[test]
    #[should_panic(expected = "Empty")]
    fn empty_str_error() {
        let _: Fkey = "".try_into().unwrap();
    }

    #[test]
    #[should_panic(expected = r#"InvalidChars("invalid path")"#)]
    fn empty_str_error_invalid_chars() {
        let _: Fkey = "invalid path".try_into().unwrap();
    }

    #[test]
    #[should_panic(expected = r#"ToManyAttributes("key.attribute.invalid")"#)]
    fn empty_str_error_more_than_one_dot() {
        let _: Fkey = "key.attribute.invalid".try_into().unwrap();
    }
}
