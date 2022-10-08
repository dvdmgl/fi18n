use crate::{
    error::FluentResourceError, machine::MachineBundles, Error, FluentResource, LanguageIdentifier,
    MachineBundle,
};
use ahash::RandomState;
use std::{collections::HashMap, sync::Arc};

use super::FluentMachineBuilder;

/// a DRYer builder by _"inheritance"_, overwrites terms and messages from the base language
/// to region specific, when calling [`build_inheritance`](FluentMachineInheritanceBuilder::build_inheritance)
/// returns [`FluentMachineBuilder`] to continue configuration.
///
/// ## Example
/// Creating `en-US` locale inheriting from `en` message `language` and overriding `region`,
/// having global `locale`
/// ```
/// use fi18n::{
///     FluentMachine,
///     builders::{FluentMachineInheritanceBuilder, InheritanceSyntaxErrorHandling}
/// };
///
/// let bundles = FluentMachine::build_with_inheritance(InheritanceSyntaxErrorHandling::AtBuild)
///     // add `company` and `locale` message that will use `region` and `language` messages to global
///     .add_source("", r#"
/// company = Example, inc.
/// locale = {region} {language}"#,
///     Some("global"))
///     .expect("Should be added global messages")
///     // base English language with `region` and `language` messages
///     .add_source("en", r#"
/// -soccer-term = Soccer
/// -football-term = Football
/// language = English
/// region = International
/// soccer = { -soccer-term } is the biggest sport in the world, with UEFA Champions League final 380 million viewers.
/// football = { -football-term } is the biggest North American sport, with Super Bowl 112.3 million viewers."#,
///         Some("international English"),
///     )
///     .expect("Should add to base language English")
///     // united states language with `region` message, will "inherit" language from base English
///     .add_source("en-US", r#"
/// region = United States"#,
///         Some("united states English"),
///     )
///     .expect("Should add overrides to English to united States region")
///     .add_source("en-UK", r#"
/// -soccer-term = Football
/// -football-term = American Football
/// region = United Kingdom"#,
///         Some("united states English"),
///     )
///     .expect("Should add overrides to English to united kingdom region")
///     .build_inheritance()
///     .expect("Build should create en, en-US and en-UK locales")
///     .finish()
///     .expect("Should build with default locale `en`");
///
/// let locale_en = bundles.negotiate_languages("en");
/// let locale_us = bundles.negotiate_languages("en-US");
/// let locale_uk = bundles.negotiate_languages("en-UK");
///
/// for locale in [&locale_en, &locale_us, &locale_uk] {
///     assert_eq!(
///         bundles.t(locale, "company".try_into().unwrap(), None),
///         "Example, inc.",
///         "`company` message is present in all locales"
///     );
///     assert_eq!(
///         bundles.t(locale, "language".try_into().unwrap(), None),
///         "English",
///         "`language` message is present in all locales"
///     );
/// }
/// assert_eq!(
///     bundles.t(&locale_en, "region".try_into().unwrap(), None),
///     "International",
///     "`region` in locale `en` is International"
/// );
/// assert_eq!(
///     bundles.t(&locale_en, "locale".try_into().unwrap(), None),
///     "International English",
///     "`locale` resolves by referencing in `language` and `region` to `en`"
/// );
/// assert_eq!(
///     bundles.t(&locale_us, "region".try_into().unwrap(), None),
///     "United States",
///     "`region` in locale `en-US` is United States"
/// );
/// assert_eq!(
///     bundles.t(&locale_us, "locale".try_into().unwrap(), None),
///     "United States English",
///     "`locale` resolves by referencing in `language` and `region` to `en-US`"
/// );
/// assert_eq!(
///     bundles.t(&locale_uk, "locale".try_into().unwrap(), None),
///     "United Kingdom English",
///     "`locale` resolves by referencing in `language` and `region` to `en-UK`"
/// );
/// assert_eq!(
///     bundles.t(&locale_us, "soccer".try_into().unwrap(), None),
///     "Soccer is the biggest sport in the world, with UEFA Champions League final 380 million viewers."
/// );
/// assert_eq!(
///     bundles.t(&locale_us, "football".try_into().unwrap(), None),
///     "Football is the biggest North American sport, with Super Bowl 112.3 million viewers."
/// );
/// assert_eq!(
///     bundles.t(&locale_uk, "soccer".try_into().unwrap(), None),
///     "Football is the biggest sport in the world, with UEFA Champions League final 380 million viewers."
/// );
/// assert_eq!(
///     bundles.t(&locale_uk, "football".try_into().unwrap(), None),
///     "American Football is the biggest North American sport, with Super Bowl 112.3 million viewers."
/// );
/// ```
///
/// ### Note how syntax errors are handled
///
/// Syntax errors are handled according to [`InheritanceSyntaxErrorHandling`] choice.
pub struct FluentMachineInheritanceBuilder {
    pub(crate) sources: HashMap<Option<LanguageIdentifier>, Vec<Arc<FluentResource>>>,
    pub(crate) errors: Vec<FluentResourceError>,
    pub(crate) mode: InheritanceSyntaxErrorHandling,
}

/// Mode of processing [`FluentMachineInheritanceBuilder`] syntax errors
pub enum InheritanceSyntaxErrorHandling {
    /// Retuns errors at [`FluentMachineInheritanceBuilder::add_source`]
    ///
    /// # Example:
    /// ```
    /// use fi18n::builders::{FluentMachineInheritanceBuilder, InheritanceSyntaxErrorHandling};
    ///
    /// let mut building = FluentMachineInheritanceBuilder::new(InheritanceSyntaxErrorHandling::AtAddSource);
    /// assert!(building.add_source("en", r"bad-message = ", None).is_err());
    /// ```
    AtAddSource,
    /// Retuns errors at [`FluentMachineInheritanceBuilder::build_inheritance`]
    ///
    /// # Example:
    /// ```
    /// use fi18n::builders::{FluentMachineInheritanceBuilder, InheritanceSyntaxErrorHandling, FluentMachineBuilder};
    ///
    /// let mut adding_source = FluentMachineInheritanceBuilder::new(InheritanceSyntaxErrorHandling::AtBuild);
    /// let adding_source = adding_source.add_source("en", r#"
    /// ok = I'm OK
    /// bad-message = "#, None);
    /// assert!(adding_source.is_ok());
    /// let building = adding_source.unwrap().build_inheritance();
    /// assert!(building.is_err());
    /// let machine_build: FluentMachineBuilder = match building {
    ///     Ok(build) => build,
    ///     Err((build, errors)) => {
    ///         assert_eq!(errors.len(), 1);
    ///         build
    ///     }
    /// };
    /// let fi18n = machine_build.finish().expect("Should build as en is default");
    /// let langs = fi18n.negotiate_languages("en");
    /// assert_eq!(fi18n.t(&langs, "ok".try_into().unwrap(), None), "I'm OK");
    /// assert_eq!(fi18n.t(&langs, "bad-message".try_into().unwrap(), None), "bad-message");
    /// ```
    AtBuild,
}

/// Syntax errors can be returned at two points, every time at [`add_source`] or when [`build_inheritance`]
/// When using [`build_inheritance`], the [`Result::Error`] contains the bundle and a list of errors
pub type BuildInheritanceError = (FluentMachineBuilder, Vec<FluentResourceError>);

impl FluentMachineInheritanceBuilder {
    /// Set how syntax errors are handled, check [`InheritanceSyntaxErrorHandling`] for more information.
    pub fn new(mode: InheritanceSyntaxErrorHandling) -> Self {
        Self {
            mode,
            sources: HashMap::default(),
            errors: Vec::new(),
        }
    }

    /// Add resource to builder
    ///
    /// # Arguments:
    /// * `locale` a valid locale with optional 2 regional letter code or an empty
    ///     `str` to be shared between all locales
    /// * `ftl` a Fluent resource
    /// * `origin` optional, used to format error with the origin
    ///
    /// # Errors
    /// Returns [`Error::LanguageIdentifierError`] if locale is invalid
    ///
    /// If [`FluentMachineInheritanceBuilder`] is created with [`InheritanceSyntaxErrorHandling::AtAddSource`]
    /// Returns [`Error::FluentResourceError`]
    pub fn add_source(
        mut self,
        locale: &str,
        ftl: &str,
        origin: Option<&str>,
    ) -> Result<Self, Error> {
        let locale = if locale.is_empty() {
            None
        } else {
            Some(locale.parse::<LanguageIdentifier>()?)
        };

        let rs = match FluentResource::try_new(ftl.to_string()) {
            Ok(rs) => Arc::new(rs),
            Err((rs, exs)) => {
                let err = FluentResourceError::new(ftl, origin, exs);
                match self.mode {
                    InheritanceSyntaxErrorHandling::AtAddSource => return Err(err.into()),
                    InheritanceSyntaxErrorHandling::AtBuild => {
                        self.errors.push(err);
                        Arc::new(rs)
                    }
                }
            }
        };

        self.sources
            .entry(locale)
            .and_modify(|v| v.push(rs.clone()))
            .or_insert_with(|| vec![rs]);

        Ok(self)
    }

    /// Groups and builds bundles according to provided sources, passing to the [`FluentMachineBuilder`]
    ///
    /// # Errors
    ///
    /// According to [`InheritanceSyntaxErrorHandling`]
    ///
    /// Returns [`Error::FluentResourceError`] if [`FluentMachineInheritanceBuilder`] uses [`InheritanceSyntaxErrorHandling::AtBuild`]
    pub fn build_inheritance(self) -> Result<FluentMachineBuilder, BuildInheritanceError> {
        let mut available: Vec<LanguageIdentifier> =
            self.sources.keys().flatten().cloned().collect();

        let mut bundles: MachineBundles =
            HashMap::with_capacity_and_hasher(available.len(), RandomState::new());
        available.sort();

        for lang in available.into_iter() {
            let mut bundle: MachineBundle = MachineBundle::new_concurrent(vec![lang.clone()]);

            if let Some(global) = self.sources.get(&None) {
                for src in global {
                    bundle.add_resource_overriding(src.to_owned());
                }
            }

            if lang.region.is_some() {
                if let Some(base_lang) = self.sources.get(&Some(LanguageIdentifier::from_parts(
                    lang.language,
                    lang.script,
                    None,
                    &[],
                ))) {
                    for src in base_lang {
                        bundle.add_resource_overriding(src.to_owned());
                    }
                }
            }
            if let Some(global) = self.sources.get(&Some(lang.clone())) {
                for src in global {
                    bundle.add_resource_overriding(src.to_owned());
                }
            }
            bundles.insert(lang, bundle);
        }

        if self.errors.is_empty() {
            Ok(FluentMachineBuilder {
                bundles,
                ..Default::default()
            })
        } else {
            Err((
                FluentMachineBuilder {
                    bundles,
                    ..Default::default()
                },
                self.errors,
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::FluentMachine;

    use super::{
        FluentMachineInheritanceBuilder, InheritanceSyntaxErrorHandling, LanguageIdentifier,
    };
    #[test]
    fn groups_by_locales() {
        let builder =
            FluentMachineInheritanceBuilder::new(InheritanceSyntaxErrorHandling::AtAddSource)
                .add_source("", "gloval0=gloval0", None)
                .expect("Should add global")
                .add_source("en-UK", "region=United Kingdom", None)
                .expect("Should add en-uk")
                .add_source("en", "lang=English", None)
                .expect("Should add en")
                .add_source("", "global1=global1", None)
                .expect("Should add global");
        assert!(
            builder.sources.get(&None).map(|xs| xs.len() == 2).unwrap(),
            "groups global"
        );
        assert!(
            builder
                .sources
                .get(&Some("en".parse::<LanguageIdentifier>().unwrap()))
                .map(|xs| xs.len() == 1)
                .unwrap(),
            "groups en"
        );
        assert!(
            builder
                .sources
                .get(&Some("en-UK".parse::<LanguageIdentifier>().unwrap()))
                .map(|xs| xs.len() == 1)
                .unwrap(),
            "groups en-UK"
        );
    }

    #[test]
    fn raises_error_with_localization() {
        let src = r#"
this-is-ok = this is ok
bad = {$some ->
    [one] 1
    [two] 2
}
bad-two = {$some ->
    [one] 1
   *[two]
}
"#;
        assert!(
            FluentMachineInheritanceBuilder::new(InheritanceSyntaxErrorHandling::AtBuild)
                .add_source("", src, Some("badstring"))
                .unwrap()
                .build_inheritance()
                .is_err()
        );
    }

    #[test]
    fn builder_shows_errors_at_add_source() {
        let src = r#"
not-ok =
"#;
        assert!(
            FluentMachineInheritanceBuilder::new(InheritanceSyntaxErrorHandling::AtAddSource)
                .add_source("", src, Some("badstring"))
                .is_err()
        );
    }

    #[test]
    fn builder_shows_errors_at_add_source_when_invalid_locale() {
        let src = r#"
ok = ok
"#;
        assert!(
            FluentMachineInheritanceBuilder::new(InheritanceSyntaxErrorHandling::AtBuild)
                .add_source("in-va-lid", src, Some("okstring"))
                .is_err()
        );
    }

    #[test]
    fn build_inheritance_integration() {
        let bundles =
            FluentMachine::build_with_inheritance(InheritanceSyntaxErrorHandling::AtBuild)
                .add_source("", r#"company = Example, inc."#, Some("global"))
                .expect("Should add to global")
                .add_source(
                    "",
                    r#"locale = {region} {language}"#,
                    Some("locale to global"),
                )
                .expect("Should add to global")
                .add_source(
                    "en",
                    r#"
language = English
region = International
            "#,
                    Some("international English"),
                )
                .expect("Should add to en")
                .add_source(
                    "en-US",
                    r#"
region = United States
            "#,
                    Some("united states English"),
                )
                .expect("Should add to en-us")
                .build_inheritance()
                .expect("Should pass to BuildMachine")
                .finish()
                .unwrap();
        let locale_en = bundles.negotiate_languages("en");
        let locale_us = bundles.negotiate_languages("en-US");

        assert_eq!(
            bundles.t(&locale_en, "company".try_into().unwrap(), None),
            "Example, inc."
        );
        assert_eq!(
            bundles.t(&locale_en, "language".try_into().unwrap(), None),
            "English"
        );
        assert_eq!(
            bundles.t(&locale_en, "region".try_into().unwrap(), None),
            "International"
        );
        assert_eq!(
            bundles.t(&locale_en, "locale".try_into().unwrap(), None),
            "International English"
        );

        assert_eq!(
            bundles.t(&locale_us, "company".try_into().unwrap(), None),
            "Example, inc."
        );
        assert_eq!(
            bundles.t(&locale_us, "language".try_into().unwrap(), None),
            "English"
        );
        assert_eq!(
            bundles.t(&locale_us, "region".try_into().unwrap(), None),
            "United States"
        );
        assert_eq!(
            bundles.t(&locale_us, "locale".try_into().unwrap(), None),
            "United States English"
        );
    }
    #[test]
    fn build_inheritance_integration_terms() {
        let bundles = FluentMachine::build_with_inheritance(InheritanceSyntaxErrorHandling::AtBuild)
     .add_source(
         "en",
         r#"
-soccer-term = Soccer
-football-term = Football
soccer = { -soccer-term } is the biggest sport in the world, with UEFA Champions League final 380 million viewers.
football = { -football-term } is the biggest North American sport, with Super Bowl 112.3 million viewers.
         "#,
         Some("international sport"),
     )
     .expect("Should add en")
     .add_source(
             "en-UK",
             r#"
-soccer-term = Football
-football-term = American Football
         "#,
         Some("Overrides UK to sport"),
     )
     .expect("Should add en-uk")
     .add_source(
             "en-US",
             r#""#,
         Some("adds us locale"),
     )
     .expect("Should add en-us")
     .build_inheritance()
     .expect("Should pass to BuildMachine")
     .finish()
     .unwrap();

        let locale_en = bundles.negotiate_languages("en");
        let locale_us = bundles.negotiate_languages("en-US");
        let locale_uk = bundles.negotiate_languages("en-UK");

        assert_eq!(
            bundles.t(&locale_en, "soccer".try_into().unwrap(), None),
            "Soccer is the biggest sport in the world, with UEFA Champions League final 380 million viewers."
        );
        assert_eq!(
            bundles.t(&locale_en, "football".try_into().unwrap(), None),
            "Football is the biggest North American sport, with Super Bowl 112.3 million viewers."
        );
        assert_eq!(
            bundles.t(&locale_us, "soccer".try_into().unwrap(), None),
            "Soccer is the biggest sport in the world, with UEFA Champions League final 380 million viewers."
        );
        assert_eq!(
            bundles.t(&locale_us, "football".try_into().unwrap(), None),
            "Football is the biggest North American sport, with Super Bowl 112.3 million viewers."
        );
        assert_eq!(
            bundles.t(&locale_uk, "soccer".try_into().unwrap(), None),
            "Football is the biggest sport in the world, with UEFA Champions League final 380 million viewers."
        );
        assert_eq!(
            bundles.t(&locale_uk, "football".try_into().unwrap(), None),
            "American Football is the biggest North American sport, with Super Bowl 112.3 million viewers."
        );
    }
}
