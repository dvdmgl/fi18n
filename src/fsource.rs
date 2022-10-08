use std::{cmp, fmt, rc::Rc};

use crate::{errors::FluentResourceError, FluentResource};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FluentSource<'a> {
    ftl: &'a str,
    source: Rc<Option<&'a str>>,
}

impl<'a> FluentSource<'a> {
    pub fn new(ftl: &'a str, source: Option<&'a str>) -> Self {
        Self {
            ftl,
            source: Rc::new(source),
        }
    }
}

impl<'a> TryFrom<FluentSource<'a>> for FluentResource {
    type Error = (FluentResource, FluentResourceError);

    fn try_from(value: FluentSource) -> Result<Self, Self::Error> {
        match FluentResource::try_new(value.ftl.to_string()) {
            Ok(r) => Ok(r),
            Err((r, errs)) => Err((
                r,
                FluentResourceError {
                    ftl: value.ftl.to_string(),
                    origin: value.source.map(|s| s.to_string()),
                    errs,
                },
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{FluentResource, FluentResourceError, FluentSource};

    #[test]
    fn fluent_source_conversion() {
        let resp: Result<FluentResource, (FluentResource, FluentResourceError)> =
            FluentSource::new("i-am-ok = OK", None).try_into();
        assert!(resp.is_ok());
        let resp: Result<FluentResource, (FluentResource, FluentResourceError)> =
            FluentSource::new("i-am-ok =", None).try_into();
        assert!(resp.is_err());
    }
    #[test]
    fn fluent_source_simple_error_display() {
        let resp: Result<FluentResource, (FluentResource, FluentResourceError)> =
            FluentSource::new("i-am-ok =", None).try_into();
        let exp = r#"While parsing resource, the following errors where found:
Lines 1 to 1 with Expected a message field for "i-am-ok" ExpectedMessageField { entry_id: "i-am-ok" }
'''
i-am-ok =
'''
"#;
        if let Err(e) = resp {
            assert_eq!(e.1.to_string(), exp);
        }
    }

    #[test]
    fn fluent_source_multiple_error_display() {
        let resp: Result<FluentResource, (FluentResource, FluentResourceError)> =
            FluentSource::new(
                r#"
i-am-ok =
bad = {$some ->
    [one] 1
    [two] 2
}
"#,
                None,
            )
            .try_into();
        let exp = r#"While parsing resource, the following errors where found:
Lines 2 to 2 with Expected a message field for "i-am-ok" ExpectedMessageField { entry_id: "i-am-ok" }
'''
i-am-ok =

'''
Lines 3 to 6 with The select expression must have a default variant MissingDefaultVariant
'''
bad = {$some ->
    [one] 1
    [two] 2
}

'''
"#;
        if let Err(e) = resp {
            assert_eq!(e.1.to_string(), exp);
        }
    }
}
