#![doc = include_str!("../README.md")]

#[cfg(feature = "actix-web4")]
mod actix;
pub mod fkey;
mod load;
pub mod machine;

use std::{
    borrow::Borrow,
    collections::{hash_map::ValuesMut, HashMap},
};

pub use fluent_bundle::{FluentArgs, FluentError, FluentResource, FluentValue};

pub use unic_langid::{subtags, LanguageIdentifier};

pub use fluent_langneg::NegotiationStrategy;

pub use fkey::Fkey;

pub use machine::{FluentBundle, FluentMachine};

/// A high level loader/api for [`fluent-bundle`]
pub struct I18nStorage {
    bundles: HashMap<LanguageIdentifier, FluentBundle<FluentResource>>,
    /// Available language bundles
    pub available: Vec<LanguageIdentifier>,
    fallback: LanguageIdentifier,
    #[cfg(feature = "actix-web4")]
    fallback_string: String,
    strategy: NegotiationStrategy,
}

impl FluentMachine for I18nStorage {
    #[inline]
    fn bundle_by_locale(
        &self,
        locale: &LanguageIdentifier,
    ) -> Option<&FluentBundle<FluentResource>> {
        self.bundles.get(locale)
    }
    #[inline]
    fn get_supported_locales(&self) -> &[LanguageIdentifier] {
        &self.available
    }
    #[inline]
    fn get_fallback_locale(&self) -> &LanguageIdentifier {
        &self.fallback
    }
    #[inline]
    fn get_strategy(&self) -> NegotiationStrategy {
        self.strategy
    }
}

impl I18nStorage {
    /// Create a [`I18nStorage`]
    ///
    /// Recursively walks through `locales_dir` expecting `{global}/{language}/{region}`
    /// format expecting `language` and `region` to be a directory and valid tags.
    /// for a path like:
    /// - `locales/` -- all terms will be available
    ///     - **`global.ftl`** global `messages` and `terms`
    ///     > ```text
    ///     > -company-name = Foo, inc.
    ///     > ```
    ///     - **`en/`** -- `language` directory, all files language contains `messages` and `terms`. The `terms` should be consistent to a region. Resolves to tag `en`.
    ///         - **`sports.ftl`** default US English `terms` and messages
    ///         > ```text
    ///         > -soccer = Soccer
    ///         > -football = Football
    ///         > soccer = { -soccer } is the biggest sport in the world, with UEFA Champions League final 380 million viewers.
    ///         > football = { -football } is the biggest North American sport, with Super Bowl 112.3 million viewers.
    ///         > ```
    ///         - **`UK/`** -- `region` directory. Generates `en-UK`.
    ///             Will override parent language `terms` and `messages` to specific
    ///             regional terms
    ///             - **`overrides.ftl`** will override the previous `en` `terms`
    ///             > ```text
    ///             > -soccer = Football
    ///             > -football = American Football
    ///             > ```
    ///         - **`US/`** -- `region` directory. Generates `en-US`.
    ///             - **`overrides.ftl`** -- blank file to generate `en-US`
    ///     - **`pt/`**
    ///         - `..`
    ///         - **`PT/`**
    ///             - `..`
    ///         - **`BR/`**
    ///             - `..`
    /// Generates `en` as base language, `en-US` and `en-UK` by overriding the
    /// parent language.
    pub fn new(locales_dir: &str, default: String, strategy: NegotiationStrategy) -> Self {
        let bundles = load::load(locales_dir);
        Self::new_from_hm(bundles, default, strategy)
    }

    /// Constructs bundles from strings
    /// ```
    /// use std::collections::HashMap;
    /// use fi18n::{I18nStorage, FluentMachine, NegotiationStrategy, LanguageIdentifier};
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
    /// assert_eq!(i18n.t(&vec![&en], "hello".try_into().unwrap(), None), "Hi!");
    /// let pt: LanguageIdentifier = "pt".parse().unwrap();
    /// assert_eq!(i18n.t(&vec![&pt], "goodbye".try_into().unwrap(), None), "Adeus!");
    /// ```
    pub fn new_from_hm<T>(
        hm: HashMap<String, Vec<T>>,
        default: String,
        strategy: NegotiationStrategy,
    ) -> Self
    where
        T: Borrow<String>,
    {
        let bundles = hm
            .iter()
            .map(|(k, xs)| {
                let lang: LanguageIdentifier = k.parse().unwrap();
                (lang.clone(), {
                    let mut bundle = FluentBundle::new_concurrent(vec![lang]);
                    for s in xs {
                        let res: &String = s.borrow();
                        bundle.add_resource_overriding(
                            FluentResource::try_new(res.into())
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
    /// Example add STRLEN function to all bundles
    /// ```
    /// use std::collections::HashMap;
    /// use fi18n::{I18nStorage, FluentValue, FluentMachine, NegotiationStrategy, LanguageIdentifier};
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
    /// assert_eq!(i18n.t(&vec![&en], "length".try_into().unwrap(), None), "5");
    /// ```
    pub fn bundles_mut(&mut self) -> ValuesMut<LanguageIdentifier, FluentBundle<FluentResource>> {
        self.bundles.values_mut()
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
    use super::{f_args, FluentMachine, I18nStorage, LanguageIdentifier, NegotiationStrategy};
    #[test]
    fn i18n_translate_us() {
        let i18n = I18nStorage::new("locales/", "en-Us".into(), NegotiationStrategy::Filtering);

        let lang = i18n.negotiate_languages(&"de-AT;0.9,de-DE;0.8,de;0.7;en-US;0.5");
        assert_eq!(lang, vec![&"en-US".parse::<LanguageIdentifier>().unwrap()]);
        assert_eq!(
            i18n.t(&lang, "i-am".try_into().unwrap(), None),
            "I am a \u{2068}person.\u{2069}"
        );
        assert_eq!(
            i18n.t(
                &lang,
                "i-am".try_into().unwrap(),
                Some(&f_args![
                    "gender" => "masculine"
                ])
            ),
            "I am a \u{2068}boy.\u{2069}"
        );

        assert_eq!(
            i18n.t(
                &lang,
                "movie-list".try_into().unwrap(),
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
                "movie-list".try_into().unwrap(),
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

        let lang = i18n.negotiate_languages("de-AT;0.9,pt-PT;0.8,de;0.7;en-US;0.5,en");
        assert_eq!(
            lang,
            vec![
                &"pt-PT".parse::<LanguageIdentifier>().unwrap(),
                &"pt".parse::<LanguageIdentifier>().unwrap(),
                &"pt-BR".parse::<LanguageIdentifier>().unwrap(),
                &"en".parse::<LanguageIdentifier>().unwrap(),
                &"en-US".parse::<LanguageIdentifier>().unwrap(),
                &"en-UK".parse::<LanguageIdentifier>().unwrap(),
            ]
        );
        assert_eq!(
            i18n.t(&lang, "i-am".try_into().unwrap(), None),
            "Eu sou \u{2068}uma pessoa.\u{2069}"
        );
        assert_eq!(
            i18n.t(
                &lang,
                "i-am".try_into().unwrap(),
                Some(&f_args![
                    "gender" => "masculine"
                ])
            ),
            "Eu sou \u{2068}um rapaz.\u{2069}"
        );
        assert_eq!(
            i18n.t(
                &lang,
                "i-am".try_into().unwrap(),
                Some(&f_args![
                    "gender" => "feminine"
                ])
            ),
            "Eu sou \u{2068}uma rapariga.\u{2069}"
        );
        assert_eq!(
            i18n.t(&lang, "movie-list".try_into().unwrap(), None),
            "Tu tens \u{2068}um filme\u{2069} para ver."
        );
        assert_eq!(
            i18n.t(
                &lang,
                "movie-list".try_into().unwrap(),
                Some(&f_args![
                    "movies" => 1
                ])
            ),
            "Tu tens \u{2068}um filme\u{2069} para ver."
        );
        assert_eq!(
            i18n.t(
                &lang,
                "movie-list".try_into().unwrap(),
                Some(&f_args![
                    "movies" => 10
                ])
            ),
            "Tu tens \u{2068}\u{2068}10\u{2069} filmes\u{2069} para ver."
        );
        assert_eq!(
            i18n.t(&lang, "login".try_into().unwrap(), None),
            "Iniciar sessão em Example ORG"
        );
        assert_eq!(
            i18n.t(&lang, "login.username".try_into().unwrap(), None),
            "Utilizador"
        );
        assert_eq!(
            i18n.t(&lang, "login.help-text".try_into().unwrap(), None),
            "Entre o seu nome de utilizador"
        );
    }

    #[test]
    fn i18n_translate_br() {
        let i18n = I18nStorage::new("locales/", "en-Us".into(), NegotiationStrategy::Filtering);

        let lang = i18n.negotiate_languages(&"pt-BR;0.9,pt-PT;0.8,de;0.7;en-US");
        assert_eq!(
            lang,
            vec![
                &"pt-BR".parse::<LanguageIdentifier>().unwrap(),
                &"pt".parse::<LanguageIdentifier>().unwrap(),
                &"pt-PT".parse::<LanguageIdentifier>().unwrap(),
                &"en-US".parse::<LanguageIdentifier>().unwrap(),
            ]
        );
        assert_eq!(
            i18n.t(&lang, "i-am".try_into().unwrap(), None),
            "Eu sou \u{2068}uma pessoa.\u{2069}"
        );
        assert_eq!(
            i18n.t(
                &lang,
                "i-am".try_into().unwrap(),
                Some(&f_args![
                    "gender" => "masculine"
                ])
            ),
            "Eu sou \u{2068}um rapaz.\u{2069}"
        );
        assert_eq!(
            i18n.t(
                &lang,
                "i-am".try_into().unwrap(),
                Some(&f_args![
                    "gender" => "feminine"
                ])
            ),
            "Eu sou \u{2068}uma menina.\u{2069}"
        );
        assert_eq!(
            i18n.t(&lang, "movie-list".try_into().unwrap(), None),
            "Você tem \u{2068}um filme\u{2069} para ver."
        );
        assert_eq!(
            i18n.t(
                &lang,
                "movie-list".try_into().unwrap(),
                Some(&f_args![
                    "movies" => 1
                ])
            ),
            "Você tem \u{2068}um filme\u{2069} para ver."
        );
        assert_eq!(
            i18n.t(
                &lang,
                "movie-list".try_into().unwrap(),
                Some(&f_args![
                    "movies" => 10
                ])
            ),
            "Você tem \u{2068}\u{2068}10\u{2069} filmes\u{2069} para ver."
        );
        assert_eq!(
            i18n.t(&lang, "login".try_into().unwrap(), None),
            "Iniciar sessão em Example ORG"
        );
        assert_eq!(
            i18n.t(&lang, "login.username".try_into().unwrap(), None),
            "Usuário"
        );
        assert_eq!(
            i18n.t(&lang, "login.help-text".try_into().unwrap(), None),
            "Entre o seu nome de usuário"
        );
    }
}
