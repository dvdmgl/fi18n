use std::{
    convert::TryFrom,
    fmt::{Debug, Display},
};

/// Fkey support for fluent message and attribute.
///
/// Converts `str` `"message.attribute".into()` to Fkey
///
/// Example:
/// ```
/// use fi18n::Fkey;
/// assert_eq!(Fkey::try_from("a.b"), Ok(Fkey::new("a", Some("b"))));
/// assert_eq!(Fkey::try_from("a"), Ok(Fkey::new("a", None)));
/// assert_eq!(Fkey::try_from(""), Err("Invalid path: Empty"));
/// assert_eq!(Fkey::try_from("a.b.c"), Err("Invalid path: Has more than one attribute"));
/// ```
#[derive(PartialEq, Hash)]
pub struct Fkey<'a>(pub(super) &'a str, pub(super) Option<&'a str>);

impl<'a> Fkey<'a> {
    pub fn new(message: &'a str, attribute: Option<&'a str>) -> Self {
        Self(message, attribute)
    }

    pub fn message(&self) -> &'a str {
        &self.0
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
    type Error = &'static str;
    /// Converts a str slice into a [`Fkey`] variant.
    /// Path cannot by empty and contains, at most, one attribute.
    #[inline]
    fn try_from(s: &'a str) -> Result<Self, Self::Error> {
        if s.is_empty() {
            return Err("Invalid path: Empty");
        }
        let mut dot_path = s.split('.');
        let out = Self(dot_path.next().unwrap(), dot_path.next());

        if dot_path.next().is_some() {
            Err("Invalid path: Has more than one attribute")
        } else {
            Ok(out)
        }
    }
}
