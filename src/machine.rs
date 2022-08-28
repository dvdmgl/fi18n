use fluent_bundle::{FluentArgs, FluentResource};
use fluent_langneg::NegotiationStrategy;
use fluent_langneg::{negotiate_languages, parse_accepted_languages};
use unic_langid::LanguageIdentifier;

use crate::Fkey;

pub type FluentBundle<R> =
    fluent_bundle::bundle::FluentBundle<R, intl_memoizer::concurrent::IntlLangMemoizer>;

/// Simple Fluent API
pub trait FluentMachine {
    /// Returns translation according to the first message found, respecting the
    /// negotiated languages order with available.
    #[inline]
    fn t(
        &self,
        negotiated: &[&LanguageIdentifier],
        path: Fkey,
        args: Option<&FluentArgs>,
    ) -> String {
        for locale in negotiated.iter() {
            let bundle = self.bundle_by_locale(locale).unwrap();

            let pattern = match path {
                Fkey(msg_id, None) => bundle
                    .get_message(msg_id)
                    .map(|msg| msg.value().expect("Failed to parse pattern")),
                Fkey(msg_id, Some(attr)) => bundle
                    .get_message(msg_id)
                    .map(|msg| msg.get_attribute(attr).unwrap().value()),
            };
            match pattern {
                Some(pattern) => {
                    let mut errors: Vec<_> = vec![];
                    let t = bundle
                        .format_pattern(pattern, args, &mut errors)
                        .to_string();
                    if !errors.is_empty() {}
                    return t;
                }
                None => continue,
            }
        }
        path.to_string()
    }

    /// Parses `request` language preference filters and sorts with
    /// languages and strategy.
    ///
    /// For more information how filter is done to check
    /// [negotiate](fluent_langneg::negotiate) from fluent_langeg
    #[inline]
    fn negotiate_languages(&self, requested: &str) -> Vec<&LanguageIdentifier> {
        negotiate_languages(
            &parse_accepted_languages(requested),
            &self.get_supported_locales(),
            Some(&self.get_fallback_locale()),
            self.get_strategy(),
        )
    }

    /// Returns bundle by [`LanguageIdentifier`]
    fn bundle_by_locale(&self, lang: &LanguageIdentifier) -> Option<&FluentBundle<FluentResource>>;

    /// Returns supported locales
    fn get_supported_locales(&self) -> &[LanguageIdentifier];

    /// Returns defined fallback [`LanguageIdentifier`]
    fn get_fallback_locale(&self) -> &LanguageIdentifier;

    /// Returns defined negotiation strategy
    fn get_strategy(&self) -> NegotiationStrategy;
}
