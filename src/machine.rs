/*!

builders to simplify the configuration and generation of [`FluentMachine`](crate::FluentMachine)

*/
use ahash::RandomState;
use fluent_bundle::{FluentArgs, FluentResource};
use fluent_langneg::{negotiate_languages, parse_accepted_languages, NegotiationStrategy};
use std::{boxed::Box, collections::HashMap, sync::Arc};
use unic_langid::LanguageIdentifier;

use crate::{
    builders::{
        FluentMachineBuilder, FluentMachineInheritanceBuilder, InheritanceSyntaxErrorHandling,
    },
    Error, Fkey,
};

/// Concurrent `FluentBlunde`
pub type MachineBundle = fluent_bundle::bundle::FluentBundle<
    Arc<FluentResource>,
    intl_memoizer::concurrent::IntlLangMemoizer,
>;

/// Localized translation function to locale(s)
pub type TranslateFn<'a> =
    Box<dyn Fn(Fkey<'a>, Option<&'_ FluentArgs>) -> String + Send + Sync + 'a>;

pub(crate) type MachineBundles = HashMap<LanguageIdentifier, MachineBundle, RandomState>;

pub trait FluentMachineLoader {
    fn load(&self) -> Result<MachineBundles, Error>;
}

/// Simple Fluent API
pub struct FluentMachine {
    pub(crate) bundles: MachineBundles,
    // stored ordered locales
    pub(crate) available: Vec<LanguageIdentifier>,
    pub(crate) fallback: LanguageIdentifier,
    pub(crate) strategy: NegotiationStrategy,
    #[cfg(feature = "actix-web4")]
    pub(crate) fallback_string: String,
    #[cfg(feature = "actix-web4")]
    pub(crate) cookie_name: Option<String>,
}

impl FluentMachine {
    /// Build an [`FluentMachine`] using [`loaders`](crate::loaders).
    pub fn build_loader<T>(loader: T) -> Result<FluentMachineBuilder, Error>
    where
        T: FluentMachineLoader,
    {
        Ok(FluentMachineBuilder {
            bundles: loader.load()?,
            ..Default::default()
        })
    }

    /// Build an [`FluentMachine`] using [`FluentMachineBuilder`].
    pub fn build() -> FluentMachineBuilder {
        FluentMachineBuilder::default()
    }

    /// Build an [`FluentMachine`] using [`FluentMachineInheritanceBuilder`].
    pub fn build_with_inheritance(
        mode: InheritanceSyntaxErrorHandling,
    ) -> FluentMachineInheritanceBuilder {
        FluentMachineInheritanceBuilder::new(mode)
    }

    /// Returns translation according to the first message found, respecting the
    /// negotiated languages order with available.
    #[inline]
    pub fn t(
        &self,
        negotiated: &[&LanguageIdentifier],
        path: Fkey,
        args: Option<&FluentArgs>,
    ) -> String {
        for locale in negotiated.iter() {
            let bundle = self.bundles.get(locale).unwrap();

            let pattern = match path {
                Fkey(msg_id, None) => bundle
                    .get_message(msg_id)
                    .map(|msg| msg.value().expect("Failed to parse pattern")),
                Fkey(msg_id, Some(attr)) => bundle
                    .get_message(msg_id)
                    .map(|msg| msg.get_attribute(attr).unwrap().value()),
            };
            if let Some(pattern) = pattern {
                let mut errors: Vec<_> = vec![];
                let t = bundle
                    .format_pattern(pattern, args, &mut errors)
                    .to_string();
                if log::log_enabled!(log::Level::Debug) && !errors.is_empty() {
                    log::debug!(
                        "while formatting key {} the following errors where collected: \n {}",
                        path,
                        errors
                            .iter()
                            .map(|x| format!("{x:?}"))
                            .collect::<Vec<String>>()
                            .join("\n")
                    );
                }
                return t;
            }
        }
        log::debug!("missing key {}", path);
        path.to_string()
    }

    /// Localized translation function
    #[inline]
    pub fn localize_t(&self, locales: &str) -> TranslateFn<'_> {
        let langs = self.negotiate_languages(locales);

        Box::new(move |key, options| self.t(&langs, key, options))
    }

    /// Parses `request` language preference filters and sorts with
    /// languages and strategy.
    ///
    /// # Example [`NegotiationStrategy::Filtering`]
    /// ```rust
    /// use fi18n::{FluentMachine, LanguageIdentifier, NegotiationStrategy};
    ///
    /// let i18n = FluentMachine::build()
    ///     .add_resource("en", r#"region = International"#).expect("Should add en")
    ///     .add_resource("en-US", r#"region = United States"#).expect("Should add en-US")
    ///     .add_resource("en-UK", r#"region = United Kingdom"#).expect("Should add en-UK")
    ///     .add_resource("pt", r#"region = Internacional"#).expect("Should add pt")
    ///     .add_resource("pt-PT", r#"region = Portugal"#).expect("Should add pt-PT")
    ///     .add_resource("pt-BR", r#"region = Brazil"#).expect("Should add pt-BR")
    ///     .set_strategy(NegotiationStrategy::Filtering)
    ///     .finish().expect("Should finish");
    /// let locales = ["pt-PT", "pt", "pt-BR", "en-US", "en", "en-UK"].into_iter()
    ///     .map(|s| s.parse::<LanguageIdentifier>().unwrap()).collect::<Vec<_>>();
    /// assert_eq!(
    ///     i18n.negotiate_languages("de-AT;0.9, pt-PT;0.8, de;0.7, en-US;0.5, en;0.4"),
    ///     locales.iter().collect::<Vec<&LanguageIdentifier>>()
    /// );
    /// ```
    ///
    /// # Example [`NegotiationStrategy::Matching`]
    /// ```rust
    /// use fi18n::{FluentMachine, LanguageIdentifier, NegotiationStrategy};
    ///
    /// let i18n = FluentMachine::build()
    ///     .add_resource("en", r#"region = International"#).expect("Should add en")
    ///     .add_resource("en-US", r#"region = United States"#).expect("Should add en-US")
    ///     .add_resource("en-UK", r#"region = United Kingdom"#).expect("Should add en-UK")
    ///     .add_resource("pt", r#"region = Internacional"#).expect("Should add pt")
    ///     .add_resource("pt-PT", r#"region = Portugal"#).expect("Should add pt-PT")
    ///     .add_resource("pt-BR", r#"region = Brazil"#).expect("Should add pt-BR")
    ///     .set_strategy(NegotiationStrategy::Matching)
    ///     .finish().expect("Should finish");
    /// let locales = ["pt-PT", "en-US", "en"].into_iter()
    ///     .map(|s| s.parse::<LanguageIdentifier>().unwrap()).collect::<Vec<_>>();
    /// assert_eq!(
    ///     i18n.negotiate_languages("de-AT;0.9, pt-PT;0.8, de;0.7, en-US;0.5, en;0.4"),
    ///     locales.iter().collect::<Vec<&LanguageIdentifier>>()
    /// );
    /// ```
    ///
    /// # Example [`NegotiationStrategy::Lookup`]
    /// ```rust
    /// use fi18n::{FluentMachine, LanguageIdentifier, NegotiationStrategy};
    ///
    /// let i18n = FluentMachine::build()
    ///     .add_resource("en", r#"region = International"#).expect("Should add en")
    ///     .add_resource("en-US", r#"region = United States"#).expect("Should add en-US")
    ///     .add_resource("en-UK", r#"region = United Kingdom"#).expect("Should add en-UK")
    ///     .add_resource("pt", r#"region = Internacional"#).expect("Should add pt")
    ///     .add_resource("pt-PT", r#"region = Portugal"#).expect("Should add pt-PT")
    ///     .add_resource("pt-BR", r#"region = Brazil"#).expect("Should add pt-BR")
    ///     .set_strategy(NegotiationStrategy::Lookup)
    ///     .finish().expect("Should finish");
    /// let locales = ["pt-PT"].into_iter()
    ///     .map(|s| s.parse::<LanguageIdentifier>().unwrap()).collect::<Vec<_>>();
    /// assert_eq!(
    ///     i18n.negotiate_languages("de-AT;0.9, pt-PT;0.8, de;0.7, en-US;0.5, en;0.4"),
    ///     locales.iter().collect::<Vec<&LanguageIdentifier>>()
    /// );
    /// ```
    ///
    /// For more information how filter is done to check
    /// [negotiate](fluent_langneg::negotiate) from fluent_langeg
    #[inline]
    pub fn negotiate_languages(&self, requested: &str) -> Vec<&LanguageIdentifier> {
        negotiate_languages(
            &parse_accepted_languages(requested),
            &self.available,
            Some(&self.fallback),
            self.strategy,
        )
    }

    /// Returns supported locales.
    #[inline]
    pub fn get_supported_locales(&self) -> &[LanguageIdentifier] {
        &self.available
    }

    /// Returns default locale.
    #[inline]
    pub fn get_fallback_locale(&self) -> &LanguageIdentifier {
        &self.fallback
    }

    /// Returns used [`NegotiationStrategy`].
    #[inline]
    pub fn get_strategy(&self) -> NegotiationStrategy {
        self.strategy
    }
}

#[cfg(test)]
mod tests {
    use super::{FluentMachine, LanguageIdentifier, NegotiationStrategy};
    use crate::{f_args, FluentValue};

    #[test]
    #[should_panic(expected = "LocaleUnavailable")]
    fn test_build_unavailable_locale() {
        FluentMachine::build().finish().unwrap();
    }

    #[test]
    #[should_panic(expected = "MultipleFluentError")]
    fn raises_override_error() {
        FluentMachine::build()
            .add_resource(
                "en",
                r#"
some = is_ok
other = is_ok
"#,
            )
            .expect("Should add en")
            .add_resource(
                "en",
                r#"
some = is not ok
other = is not ok
"#,
            )
            .unwrap();
    }

    #[test]
    fn adds_function_to_bundles() {
        let i18n = FluentMachine::build()
            .add_resource_override("en", r#"length = en { STRLEN("12345") }"#)
            .expect("Should add en")
            .add_resource_override("pt", r#"length = pt { STRLEN("12345") }"#)
            .expect("Should add pt")
            .add_function("STRLEN", &|pos, _| match pos {
                [FluentValue::String(s)] => s.len().into(),
                _ => FluentValue::Error,
            })
            .expect("Should add function")
            .finish()
            .expect("Should build with success");
        let en: LanguageIdentifier = "en".parse().unwrap();
        assert_eq!(
            i18n.t(&[&en], "length".try_into().unwrap(), None),
            "en \u{2068}5\u{2069}"
        );
        let pt: LanguageIdentifier = "pt".parse().unwrap();
        assert_eq!(
            i18n.t(&[&pt], "length".try_into().unwrap(), None),
            "pt \u{2068}5\u{2069}"
        );
    }

    #[test]
    fn supports_attribute() {
        let i18n = FluentMachine::build()
            .add_resource_override(
                "en",
                r#"
login-input = Predefined value
    .placeholder = email@example.com
"#,
            )
            .expect("Should add en")
            .finish()
            .unwrap();
        let en: LanguageIdentifier = "en".parse().unwrap();
        assert_eq!(
            i18n.t(&[&en], "login-input".try_into().unwrap(), None),
            "Predefined value"
        );
        assert_eq!(
            i18n.t(&[&en], "login-input.placeholder".try_into().unwrap(), None),
            "email@example.com"
        );
    }

    #[test]
    fn t_respects_negotiated_languages() {
        let i18n = FluentMachine::build()
            .add_resource_override(
                "en",
                r#"
region = International
missing = Missing on others
"#,
            )
            .expect("Should add en")
            .add_resource_override(
                "en-UK",
                r#"
region = United Kingdom
"#,
            )
            .expect("Should add en-UK")
            .add_resource_override(
                "en-US",
                r#"
region = United States
"#,
            )
            .expect("Should add en-US")
            .finish()
            .unwrap();
        let locales = i18n.negotiate_languages("de-AT;0.9, de-DE;0.8, de;0.7, en-US;0.5");
        assert_eq!(
            locales,
            vec![
                &"en-US".parse::<LanguageIdentifier>().unwrap(),
                &"en".parse::<LanguageIdentifier>().unwrap(),
                &"en-UK".parse::<LanguageIdentifier>().unwrap(),
            ]
        );
        assert_eq!(
            i18n.t(&locales, "region".try_into().unwrap(), None),
            "United States"
        );
        assert_eq!(
            i18n.t(&locales, "missing".try_into().unwrap(), None),
            "Missing on others"
        );
    }

    #[test]
    fn t_respects_negotiated_languages_order_add() {
        let i18n = FluentMachine::build()
            .add_resource_override("en", r#""#)
            .expect("Should add en")
            .add_resource_override("en-UK", r#""#)
            .expect("Should add en-UK")
            .add_resource_override("en-US", r#""#)
            .expect("Should add en-US")
            .add_resource_override("pt-PT", r#""#)
            .expect("Should add pt-PT")
            .finish()
            .unwrap();
        let locales = i18n.negotiate_languages("pt-PT, en-UK;1, en;0.8");
        assert_eq!(
            locales,
            vec![
                &"pt-PT".parse::<LanguageIdentifier>().unwrap(),
                &"en-UK".parse::<LanguageIdentifier>().unwrap(),
                &"en".parse::<LanguageIdentifier>().unwrap(),
                &"en-US".parse::<LanguageIdentifier>().unwrap(),
            ]
        );
    }

    #[test]
    fn t_respects_negotiated_languages_matching() {
        let i18n = FluentMachine::build()
            .add_resource_override(
                "en",
                r#"
region = International
missing = Missing on others
"#,
            )
            .expect("Should add en")
            .add_resource_override(
                "en-UK",
                r#"
region = United Kingdom
"#,
            )
            .expect("Should add en-UK")
            .add_resource_override(
                "en-US",
                r#"
region = United States
"#,
            )
            .expect("Should add en-US")
            .set_strategy(NegotiationStrategy::Matching)
            .finish()
            .unwrap();
        let locales = i18n.negotiate_languages("de-AT;0.9, de-DE;0.8, de;0.7, en-US;0.5");
        assert_eq!(
            locales,
            vec![
                &"en-US".parse::<LanguageIdentifier>().unwrap(),
                &"en".parse::<LanguageIdentifier>().unwrap(),
            ]
        );
        assert_eq!(
            i18n.t(&locales, "region".try_into().unwrap(), None),
            "United States"
        );
        assert_eq!(
            i18n.t(&locales, "missing".try_into().unwrap(), None),
            "Missing on others"
        );
    }
    #[test]
    fn t_respects_negotiated_languages_lookup() {
        let i18n = FluentMachine::build()
            .add_resource_override(
                "en",
                r#"
region = International
missing = Missing on others
"#,
            )
            .expect("Should add en")
            .add_resource_override(
                "en-UK",
                r#"
region = United Kingdom
"#,
            )
            .expect("Should add en-UK")
            .add_resource_override(
                "en-US",
                r#"
region = United States
"#,
            )
            .expect("Should add en-US")
            .set_strategy(NegotiationStrategy::Lookup)
            .finish()
            .unwrap();
        let locales = i18n.negotiate_languages("de-AT;0.9, de-DE;0.8, de;0.7, en-US;0.5");
        assert_eq!(
            locales,
            vec![&"en-US".parse::<LanguageIdentifier>().unwrap(),]
        );
        assert_eq!(
            i18n.t(&locales, "region".try_into().unwrap(), None),
            "United States"
        );
        assert_eq!(
            i18n.t(&locales, "missing".try_into().unwrap(), None),
            "missing"
        );
    }

    #[test]
    fn localize_t_lookup() {
        let i18n = FluentMachine::build()
            .add_resource("en", r#"hello = Hello {$name}."#)
            .expect("Should add en")
            .add_resource("en-US", r#"hello = Hi {$name}."#)
            .expect("Should add en-US")
            .add_resource("pt", r#"hello = Olá {$name}"#)
            .expect("Should add pt")
            .set_strategy(NegotiationStrategy::Filtering)
            .finish()
            .expect("Should finish");
        let en = i18n.localize_t("en");

        assert_eq!(
            en(
                "hello".try_into().unwrap(),
                Some(&f_args![
                    "name" => "Joe",
                ])
            ),
            "Hello \u{2068}Joe\u{2069}."
        );
    }
}
