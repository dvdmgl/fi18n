use std::{fs, io, path::Path, sync::Arc};

use crate::{
    builders::{FluentMachineInheritanceBuilder, InheritanceSyntaxErrorHandling},
    error::FluentResourceError,
    Error, FluentResource, LanguageIdentifier,
};

#[derive(Debug)]
struct FluentSource {
    source: String,
    ftl: String,
    locale: Option<LanguageIdentifier>,
}

#[inline]
fn load_directory(p: &Path, skip: usize, files: &mut Vec<FluentSource>) -> Result<(), Error> {
    if p.is_file() {
        return Ok(());
    }
    let mut dirs = vec![];

    // need to sort
    let mut entries = fs::read_dir(p)?
        .map(|res| res.map(|e| e.path()))
        .collect::<Result<Vec<_>, io::Error>>()?;
    entries.sort();

    for path in entries {
        if path.is_dir() {
            dirs.push(path);
        } else if path
            .extension()
            .map_or(false, |ext| ext.to_str() == Some("ftl"))
        {
            let locale: Option<LanguageIdentifier> = path
                .iter()
                .nth(skip)
                .and_then(|f| f.to_str())
                .and_then(|f| f.parse().ok());

            files.push(FluentSource {
                source: path.to_string_lossy().to_string(),
                ftl: fs::read_to_string(&path)?,
                locale,
            });
        }
    }
    for d in dirs.iter() {
        load_directory(d, skip, files)?;
    }
    Ok(())
}

impl FluentMachineInheritanceBuilder {
    /// Loads all `ftl` files, expecting `{global}/{language}-{region}/` format with `language`
    /// and `region` to be valid tags.
    ///
    /// For a path like:
    /// - `locales/` -- all terms will be available
    ///     - **`global.ftl`** global `messages` and `terms`
    ///     > ```text
    ///     > -company-name = Foo, inc.
    ///     > ```
    ///     - **`en/`** -- base language directory, all files language contains `messages` and `terms`.
    ///       The `terms` should be consistent to a region. Resolves to tag `en`.
    ///         - **`sports.ftl`** default US English `terms` and messages
    ///         > ```text
    ///         > -soccer-term = Soccer
    ///         > -football-term = Football
    ///         > soccer = { -soccer-term } is the biggest sport in the world, with UEFA Champions League final 380 million viewers.
    ///         > football = { -football-term } is the biggest North American sport, with Super Bowl 112.3 million viewers.
    ///         > ```
    ///     - **`en-UK/`** -- Generates `en-UK`
    ///         Will override parent language `terms` and `messages` to specific
    ///         regional terms
    ///         - **`overrides.ftl`** will override the previous `en` `terms`
    ///         > ```text
    ///         > -soccer-term = Football
    ///         > -football-term = American Football
    ///         > ```
    ///     - **`en-US/`** -- `region` directory. Generates `en-US`.
    ///         - **`overrides.ftl`** -- blank file to generate `en-US`
    ///     - **`pt/`**
    ///         - `..`
    ///     - **`pt-PT/`**
    ///         - `..`
    ///     - **`pt-BR/`**
    ///         - `..`
    /// Generates `en` as base language, `en-US` and `en-UK` by overriding the
    /// base language.
    ///
    /// # Example
    /// ```rust
    /// use fi18n::{
    ///     FluentMachine,
    ///     LanguageIdentifier,
    ///     builders::{FluentMachineInheritanceBuilder, InheritanceSyntaxErrorHandling}
    /// };
    ///
    /// let i18n = FluentMachine::build_with_inheritance(InheritanceSyntaxErrorHandling::AtBuild)
    ///     .load_locales("examples/locales/")
    ///     .unwrap()
    ///     .build_inheritance()
    ///     .unwrap()
    ///     .set_fallback_locale("en-US")
    ///     .expect("failed to parse locale")
    ///     .finish()
    ///     .expect("failed to create FluentMachine");
    ///
    /// let locales = i18n.negotiate_languages("de-AT;0.9, pt-PT;0.8, de;0.7, en-US;0.5, en;0.4");
    /// let locale_pt_pt = "pt-PT".parse::<LanguageIdentifier>().unwrap();
    /// let locale_pt = "pt".parse::<LanguageIdentifier>().unwrap();
    /// let locale_br = "pt-BR".parse::<LanguageIdentifier>().unwrap();
    /// let locale_us = "en-US".parse::<LanguageIdentifier>().unwrap();
    /// let locale_en = "en".parse::<LanguageIdentifier>().unwrap();
    /// let locale_uk = "en-UK".parse::<LanguageIdentifier>().unwrap();
    /// assert_eq!(
    ///     locales,
    ///     vec![
    ///         &locale_pt_pt,
    ///         &locale_pt,
    ///         &locale_br,
    ///         &locale_us,
    ///         &locale_en,
    ///         &locale_uk,
    ///     ]
    /// );
    /// assert_eq!(
    ///     i18n.t(&[&locale_en], "region".try_into().unwrap(), None),
    ///     "International",
    ///     "`region` in locale `en` is International"
    /// );
    /// assert_eq!(
    ///     i18n.t(&[&locale_us], "region".try_into().unwrap(), None),
    ///     "United States",
    ///     "`region` in locale `en-US` is United States"
    /// );
    /// assert_eq!(
    ///     i18n.t(&[&locale_us], "soccer".try_into().unwrap(), None),
    ///     "Soccer is the biggest sport in the world, with UEFA Champions League final 380 million viewers."
    /// );
    /// assert_eq!(
    ///     i18n.t(&[&locale_us], "football".try_into().unwrap(), None),
    ///     "Football is the biggest North American sport, with Super Bowl 112.3 million viewers."
    /// );
    /// assert_eq!(
    ///     i18n.t(&[&locale_uk], "soccer".try_into().unwrap(), None),
    ///     "Football is the biggest sport in the world, with UEFA Champions League final 380 million viewers."
    /// );
    /// assert_eq!(
    ///     i18n.t(&[&locale_uk], "football".try_into().unwrap(), None),
    ///     "American Football is the biggest North American sport, with Super Bowl 112.3 million viewers."
    /// );
    /// assert_eq!(
    ///     i18n.t(&[&locale_pt_pt], "football".try_into().unwrap(), None),
    ///     "Futebol Americano é o maior desporto do Estados Unidos, com os 112.3 milhões de telespectadores do Super Bowl."
    /// );
    /// assert_eq!(
    ///     i18n.t(&[&locale_pt_pt], "soccer".try_into().unwrap(), None),
    ///     "Futebol é o maior desporto do mundo, com os 380 milhões de telespectadores na final da Liga dos Campeões."
    /// );
    /// assert_eq!(
    ///     i18n.t(&[&locale_br], "soccer".try_into().unwrap(), None),
    ///     "Futebol é o maior desporto do mundo, com os 380 milhões de telespectadores na final da Champions League."
    /// );
    /// ```

    pub fn load_locales(mut self, path: &str) -> Result<FluentMachineInheritanceBuilder, Error> {
        let p = Path::new(path);

        log::info!(target: "DirectoryLoader", "Loading fluent translations");
        let mut read_files = Vec::new();

        load_directory(p, p.iter().count(), &mut read_files)?;

        for FluentSource {
            ftl,
            source,
            locale,
        } in read_files.into_iter()
        {
            let rs = match FluentResource::try_new(ftl.to_string()) {
                Ok(rs) => Arc::new(rs),
                Err((rs, exs)) => {
                    let err = FluentResourceError::new(&ftl, Some(&source), exs);
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
        }
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{builders::InheritanceSyntaxErrorHandling, f_args, FluentMachine};

    #[test]
    fn i18n_loader_translate_pt() {
        let i18n = FluentMachine::build_with_inheritance(InheritanceSyntaxErrorHandling::AtBuild)
            .load_locales("examples/locales/")
            .unwrap()
            .build_inheritance()
            .unwrap()
            .set_fallback_locale("en-US")
            .expect("failed to parse locale")
            .finish()
            .expect("failed to create FluentMachine");

        let lang = i18n.negotiate_languages("de-AT;0.9, pt-PT;0.8, de;0.7, en-US;0.5, en;0.4");
        assert_eq!(
            lang,
            vec![
                &"pt-PT".parse::<LanguageIdentifier>().unwrap(),
                &"pt".parse::<LanguageIdentifier>().unwrap(),
                &"pt-BR".parse::<LanguageIdentifier>().unwrap(),
                &"en-US".parse::<LanguageIdentifier>().unwrap(),
                &"en".parse::<LanguageIdentifier>().unwrap(),
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
    fn i18n_loader_translate_br() {
        let i18n = FluentMachine::build_with_inheritance(InheritanceSyntaxErrorHandling::AtBuild)
            .load_locales("examples/locales/")
            .unwrap()
            .build_inheritance()
            .unwrap()
            .set_fallback_locale("en-US")
            .expect("failed to parse locale")
            .finish()
            .expect("failed to create FluentMachine");

        let lang = i18n.negotiate_languages("pt-BR;0.9, pt-PT;0.8, de;0.7, en-US;0.6");
        assert_eq!(
            lang,
            vec![
                &"pt-BR".parse::<LanguageIdentifier>().unwrap(),
                &"pt".parse::<LanguageIdentifier>().unwrap(),
                &"pt-PT".parse::<LanguageIdentifier>().unwrap(),
                &"en-US".parse::<LanguageIdentifier>().unwrap(),
                &"en".parse::<LanguageIdentifier>().unwrap(),
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
