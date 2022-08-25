#![doc = include_str!("../README.md")]

mod load;

use std::collections::{hash_map::ValuesMut, HashMap};

pub use fluent_bundle::{FluentArgs, FluentResource, FluentValue};
use fluent_langneg::{negotiate_languages, parse_accepted_languages};

pub use unic_langid::{subtags, LanguageIdentifier};

pub use fluent_langneg::NegotiationStrategy;

#[cfg(feature = "actix-web4")]
use actix_web::{http::header::ACCEPT_LANGUAGE, HttpRequest};
#[cfg(feature = "actix-web4")]
use std::boxed::Box;

pub type FluentBundle<R> =
    fluent_bundle::bundle::FluentBundle<R, intl_memoizer::concurrent::IntlLangMemoizer>;

#[cfg(feature = "actix-web4")]
pub type TranslateFn<'a> = Box<dyn Fn(&'a str, Option<&'a FluentArgs>) -> String + 'a>;

/// A high level loader/api for [`fluent-bundle`]
pub struct I18nStorage {
    bundles: HashMap<LanguageIdentifier, FluentBundle<FluentResource>>,
    pub available: Vec<LanguageIdentifier>,
    fallback: LanguageIdentifier,
    #[cfg(feature = "actix-web4")]
    fallback_string: String,
    strategy: NegotiationStrategy,
}

impl I18nStorage {
    pub fn new(locales_dir: &str, default: String, strategy: NegotiationStrategy) -> Self {
        let bundles = load::load(locales_dir);
        Self::new_from_hm(bundles, default, strategy)
    }

    /// Constructs bundles from strings
    /// ```
    /// use std::collections::HashMap;
    /// use fi18n::{I18nStorage, NegotiationStrategy, LanguageIdentifier};
    ///
    /// let en_string = String::from("
    /// hello = Hi!
    /// goodbye = Bye!
    /// ");
    /// let pt_string = String::from("
    /// hello = Olá!
    /// goodbye = Adeus!
    /// ");
    ///
    /// let mut hm: HashMap<String, Vec<String>> = HashMap::new();
    /// hm.insert("en".into(), vec![en_string]);
    /// hm.insert("pt".into(), vec![pt_string]);
    ///
    /// let i18n = I18nStorage::new_from_hm(hm, "en".into(), NegotiationStrategy::Filtering);
    ///
    /// let en: LanguageIdentifier = "en".parse().unwrap();
    /// assert_eq!(i18n.t(&en, "hello", None), "Hi!");
    /// let pt: LanguageIdentifier = "pt".parse().unwrap();
    /// assert_eq!(i18n.t(&pt, "goodbye", None), "Adeus!");
    /// ```
    pub fn new_from_hm(
        hm: HashMap<String, Vec<String>>,
        default: String,
        strategy: NegotiationStrategy,
    ) -> Self {
        let bundles = hm
            .iter()
            .map(|(k, xs)| {
                let lang: LanguageIdentifier = k.parse().unwrap();
                (lang.clone(), {
                    let mut bundle = FluentBundle::new_concurrent(vec![lang]);
                    for s in xs {
                        bundle.add_resource_overriding(
                            FluentResource::try_new(s.clone())
                                .expect("Failed to parse the resource."),
                        );
                    }
                    if cfg!(feature = "with-title") {
                        bundle
                            .add_function("TITLE", |positional, _named| match positional {
                                [FluentValue::String(s)] => {
                                    let mut c = s.chars();
                                    match c.next() {
                                        None => String::new(),
                                        Some(f) => {
                                            f.to_uppercase().collect::<String>() + c.as_str()
                                        }
                                    }
                                    .into()
                                }
                                _ => FluentValue::Error,
                            })
                            .expect("Failed to add a function to the bundle.");
                    }
                    bundle
                })
            })
            .collect::<HashMap<LanguageIdentifier, FluentBundle<FluentResource>>>();
        let available: Vec<LanguageIdentifier> = bundles.keys().cloned().collect();
        Self {
            bundles,
            available,
            fallback: default.parse().unwrap(),
            #[cfg(feature = "actix-web4")]
            fallback_string: default,
            strategy,
        }
    }

    /// Provides a mutable iterator over bundles
    ///
    /// Example add a function
    /// ```
    /// use std::collections::HashMap;
    /// use fi18n::{I18nStorage, FluentValue, NegotiationStrategy, LanguageIdentifier};
    ///
    /// let ftl_string = String::from("length = { STRLEN(\"12345\") }");
    ///
    /// let mut hm: HashMap<String, Vec<String>> = HashMap::new();
    /// hm.insert("en".into(), vec![ftl_string]);
    ///
    /// let mut i18n = I18nStorage::new_from_hm(hm, "en".into(), NegotiationStrategy::Filtering);
    ///
    /// for bundle in i18n.bundles_mut() {
    ///    bundle
    ///        .add_function("STRLEN", |positional, _named| match positional {
    ///            [FluentValue::String(str)] => str.len().into(),
    ///            _ => FluentValue::Error,
    ///        }).expect("Failed to add a function to the bundle.");
    /// }
    /// let en: LanguageIdentifier = "en".parse().unwrap();
    /// assert_eq!(i18n.t(&en, "length", None), "5");
    /// ```

    pub fn bundles_mut(&mut self) -> ValuesMut<LanguageIdentifier, FluentBundle<FluentResource>> {
        self.bundles.values_mut()
    }

    /// translates message to a locale, if message key has `.` returns the message attribute
    #[inline]
    pub fn t(&self, locale: &LanguageIdentifier, key: &str, args: Option<&FluentArgs>) -> String {
        let bundle = self.bundles.get(locale).unwrap();
        let mut path = key.split('.');
        let message = bundle
            .get_message(path.next().unwrap())
            .expect("Failed to retrieve a message.");

        let pattern = if let Some(attr) = path.next() {
            message.get_attribute(attr).unwrap().value()
        } else {
            message.value().expect("Failed to parse pattern")
        };
        bundle
            .format_pattern(pattern, args, &mut vec![])
            .to_string()
    }

    /// Parses `language-range`, matches with current available languages returning the [`LanguageIdentifier`].
    #[inline]
    pub fn negotiate_languages(&self, requested: &str) -> &LanguageIdentifier {
        negotiate_languages(
            &parse_accepted_languages(requested),
            &self.available,
            Some(&self.fallback),
            self.strategy,
        )
        .first()
        .unwrap()
        // .clone()
    }

    /// Actix helper function to translate to resolved to local
    #[cfg(feature = "actix-web4")]
    #[inline]
    pub fn from_request_tanslate(&self, request: &HttpRequest) -> TranslateFn<'_> {
        let lang = self.negotiate_languages(
            request
                .headers()
                .get(ACCEPT_LANGUAGE)
                .map(|h| h.to_str().unwrap())
                .unwrap_or(&self.fallback_string),
        );
        Box::new(move |key, options| self.t(&lang, key, options))
    }
}

/// A helper macro to simplify creation of FluentArgs.
///
/// # Example
///
/// ```
/// use fi18n::f_args;
///
/// let mut args = f_args![
///     "name" => "John",
///     "emailCount" => 5,
/// ];
#[macro_export]
macro_rules! f_args {
    ( $($key:expr => $value:expr),* $(,)? ) => {
        {
            let mut args: $crate::FluentArgs = $crate::FluentArgs::new();
            $(
                args.set($key, $value);
            )*
            args
        }
    };
}

#[cfg(test)]
mod tests {
    use super::{f_args, I18nStorage, LanguageIdentifier, NegotiationStrategy};
    #[cfg(feature = "actix-web4")]
    use actix_web::test::TestRequest;
    #[test]
    fn i18n_translate_us() {
        let i18n = I18nStorage::new("locales/", "en-Us".into(), NegotiationStrategy::Filtering);

        let lang = i18n.negotiate_languages(&"de-AT;0.9,de-DE;0.8,de;0.7;en-US;0.5");
        assert_eq!(lang, &"en-US".parse::<LanguageIdentifier>().unwrap());
        assert_eq!(
            i18n.t(&lang, "i-am", None),
            "I am a \u{2068}person.\u{2069}"
        );
        assert_eq!(
            i18n.t(
                &lang,
                "i-am",
                Some(&f_args![
                    "gender" => "masculine"
                ])
            ),
            "I am a \u{2068}boy.\u{2069}"
        );

        assert_eq!(
            i18n.t(
                &lang,
                "movie-list",
                Some(&f_args![
                    "movies" => 1,
                    "username" => "Foo"
                ])
            ),
            "\u{2068}Foo\u{2069}, you have \u{2068}one movie\u{2069} to watch in Example ORG."
        );
        assert_eq!(
            i18n.t(
                &lang,
                "movie-list",
                Some(&f_args![
                    "movies" => 10,
                    "username" => "Foo"
                ])
            ),
            "\u{2068}Foo\u{2069}, you have \u{2068}\u{2068}10\u{2069} movies\u{2069} to watch in Example ORG."
        );
    }

    #[test]
    fn i18n_translate_pt() {
        let i18n = I18nStorage::new("locales/", "en-Us".into(), NegotiationStrategy::Filtering);

        let lang = i18n.negotiate_languages("de-AT;0.9,pt-PT;0.8,de;0.7;en-US;0.5");
        assert_eq!(lang, &"pt-PT".parse::<LanguageIdentifier>().unwrap());
        assert_eq!(
            i18n.t(&lang, "i-am", None),
            "Eu sou \u{2068}uma pessoa.\u{2069}"
        );
        assert_eq!(
            i18n.t(
                &lang,
                "i-am",
                Some(&f_args![
                    "gender" => "masculine"
                ])
            ),
            "Eu sou \u{2068}um rapaz.\u{2069}"
        );
        assert_eq!(
            i18n.t(
                &lang,
                "i-am",
                Some(&f_args![
                    "gender" => "feminine"
                ])
            ),
            "Eu sou \u{2068}uma rapariga.\u{2069}"
        );
        assert_eq!(
            i18n.t(&lang, "movie-list", None),
            "Tu tens \u{2068}um filme\u{2069} para ver."
        );
        assert_eq!(
            i18n.t(
                &lang,
                "movie-list",
                Some(&f_args![
                    "movies" => 1
                ])
            ),
            "Tu tens \u{2068}um filme\u{2069} para ver."
        );
        assert_eq!(
            i18n.t(
                &lang,
                "movie-list",
                Some(&f_args![
                    "movies" => 10
                ])
            ),
            "Tu tens \u{2068}\u{2068}10\u{2069} filmes\u{2069} para ver."
        );
        assert_eq!(
            i18n.t(&lang, "login", None),
            "Iniciar sessão em Example ORG"
        );
        assert_eq!(i18n.t(&lang, "login.username", None), "Utilizador");
        assert_eq!(
            i18n.t(&lang, "login.help-text", None),
            "Entre o seu nome de utilizador"
        );
    }

    #[test]
    fn i18n_translate_br() {
        let i18n = I18nStorage::new("locales/", "en-Us".into(), NegotiationStrategy::Filtering);

        let lang = i18n.negotiate_languages(&"pt-BR;0.9,pt-PT;0.8,de;0.7;en-US;0.5");
        assert_eq!(lang, &"pt-BR".parse::<LanguageIdentifier>().unwrap());
        assert_eq!(
            i18n.t(&lang, "i-am", None),
            "Eu sou \u{2068}uma pessoa.\u{2069}"
        );
        assert_eq!(
            i18n.t(
                &lang,
                "i-am",
                Some(&f_args![
                    "gender" => "masculine"
                ])
            ),
            "Eu sou \u{2068}um rapaz.\u{2069}"
        );
        assert_eq!(
            i18n.t(
                &lang,
                "i-am",
                Some(&f_args![
                    "gender" => "feminine"
                ])
            ),
            "Eu sou \u{2068}uma menina.\u{2069}"
        );
        assert_eq!(
            i18n.t(&lang, "movie-list", None),
            "Você tem \u{2068}um filme\u{2069} para ver."
        );
        assert_eq!(
            i18n.t(
                &lang,
                "movie-list",
                Some(&f_args![
                    "movies" => 1
                ])
            ),
            "Você tem \u{2068}um filme\u{2069} para ver."
        );
        assert_eq!(
            i18n.t(
                &lang,
                "movie-list",
                Some(&f_args![
                    "movies" => 10
                ])
            ),
            "Você tem \u{2068}\u{2068}10\u{2069} filmes\u{2069} para ver."
        );
        assert_eq!(
            i18n.t(&lang, "login", None),
            "Iniciar sessão em Example ORG"
        );
        assert_eq!(i18n.t(&lang, "login.username", None), "Usuário");
        assert_eq!(
            i18n.t(&lang, "login.help-text", None),
            "Entre o seu nome de usuário"
        );
    }
    #[cfg(feature = "actix-web4")]
    #[cfg_attr(feature = "actix-web4", actix_web::test)]
    async fn actix_request_tansltate_fn() {
        let i18n = I18nStorage::new("locales/", "en-US".into(), NegotiationStrategy::Filtering);
        let t = i18n.from_request_tanslate(
            &TestRequest::get()
                .insert_header((
                    actix_web::http::header::ACCEPT_LANGUAGE,
                    "en-US;0.9,de-DE;0.8,de;0.7;en-US;0.5",
                ))
                .to_http_request(),
        );
        assert_eq!(t("i-am", None), "I am a \u{2068}person.\u{2069}");
    }
}
