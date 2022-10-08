use ahash::RandomState;
use std::{collections::HashMap, fs, io, path::Path, sync::Arc};

use crate::{
    machine::MachineBundles, Error, FluentMachineLoader, FluentResource, LanguageIdentifier,
    MachineBundle,
};

#[derive(Debug)]
struct FluentSource {
    source: String,
    ftl: String,
}

/// [`DirectoryLoader`] walks through directories expecting `{global}/{language}-{region}/`
/// format expecting `language` and `region` to be valid tags loading all `ftl` files.
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
/// ### Warning
///
/// While loading does not fail unless a file error, parsing can generate errors
/// displayed at log, to view source use `debug` flag.
#[derive(Debug)]
pub struct DirectoryLoader<'a>(&'a Path);

impl<'a> DirectoryLoader<'a> {
    pub fn new(path: &'a str) -> Self {
        Self(Path::new(path))
    }
}

// Creates bundles from folder
// Loading strategy:
//     1. sorts by name
//     2. if is file and ends with `ftl`:
//         a. extract `tag` from language region
//         b. store if `tag` exist else create new `tag` copying all from parent (without region)
//     3. at the end repeat 1 for all sub directories
#[inline]
fn load_directory(
    p: &Path,
    skip: usize,
    files: &mut HashMap<Option<LanguageIdentifier>, Vec<Arc<FluentSource>>>,
) -> Result<(), Error> {
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
            let lang: Option<LanguageIdentifier> = path
                .iter()
                .nth(skip)
                .and_then(|f| f.to_str())
                .and_then(|f| f.parse().ok());

            let file = Arc::new(FluentSource {
                source: path.to_string_lossy().to_string(),
                ftl: fs::read_to_string(&path)?,
            });

            if files.contains_key(&lang) {
                files.entry(lang).and_modify(|v| v.push(file));
            } else {
                let pre_key = match lang {
                    Some(ref l) if l.region.is_some() => Some(LanguageIdentifier::from_parts(
                        l.language,
                        l.script,
                        None,
                        &[],
                    )),
                    _ => None,
                };
                let lang_prev: Vec<Arc<FluentSource>> = files
                    .get(&pre_key)
                    .map(|s| s.iter().map(Arc::clone).collect())
                    .unwrap_or_else(Vec::new);

                files
                    .entry(lang)
                    .and_modify(|v| v.push(Arc::clone(&file)))
                    .or_insert_with(|| {
                        lang_prev
                            .iter()
                            .cloned()
                            .chain(vec![file])
                            .collect::<Vec<Arc<FluentSource>>>()
                    });
            }
        }
    }
    for d in dirs.iter() {
        load_directory(d, skip, files)?;
    }
    Ok(())
}

impl<'a> FluentMachineLoader for DirectoryLoader<'a> {
    fn load(&self) -> Result<MachineBundles, Error> {
        log::info!(target: "DirectoryLoader", "Loading fluent translations");
        let mut read_files = HashMap::new();

        load_directory(self.0, self.0.iter().count(), &mut read_files)?;

        let mut out: MachineBundles =
            HashMap::with_capacity_and_hasher(read_files.len(), RandomState::new());
        let mut errors_map = HashMap::new();
        for (k, xs) in read_files.iter() {
            if let Some(k) = k {
                let mut bundle: MachineBundle = MachineBundle::new_concurrent(vec![k.clone()]);
                for s in xs {
                    let resource = match FluentResource::try_new(s.ftl.to_string()) {
                        Ok(r) => r,
                        Err((r, errs)) => {
                            if !errors_map.contains_key(&s.source) {
                                errors_map.insert(&s.source, (errs, &s.ftl));
                            }
                            r
                        }
                    };
                    bundle.add_resource_overriding(Arc::new(resource));
                }
                out.insert(k.clone(), bundle);
            }
        }
        if !errors_map.is_empty() {
            log::warn!(
                target: "DirectoryLoader",
                "Problems loading fluent translations"
            );
            for (path, (errors, res)) in errors_map.iter() {
                log::warn!(
                    target: "DirectoryLoader",
                    "{} errors found while parsing the file: `{path}`.", errors.len()
                );
                for error in errors {
                    let rg = error.slice.clone().unwrap_or_else(|| error.pos.clone());
                    let line_start = res[..rg.start].matches('\n').count() + 1;
                    let part = &res[rg];
                    let line_end = part.matches('\n').count() + line_start - 1;
                    log::debug!(
                        target: "DirectoryLoader",
                        "Lines {line_start} to {line_end} with {:?}\n```\n{}```",
                        error.kind,
                        part
                    );
                }
            }
            log::warn!(
                target: "DirectoryLoader",
                "Finish with errors, loading the locales: \n  {}\nwhile it will continue, unexpected behavior will **occur**",
                out.keys()
                    .map(|l| l.to_string())
                    .collect::<Vec<String>>()
                    .join(", ")
            );
        } else {
            log::info!(
                target: "DirectoryLoader",
                "Finish loading the locales: \n{}",
                out.keys()
                    .map(|l| l.to_string())
                    .collect::<Vec<String>>()
                    .join(", ")
            );
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{f_args, FluentMachine};

    #[test]
    fn load_base_lang_order_respected() {
        let mut read_files = HashMap::new();
        let path = Path::new("./examples/locales");
        load_directory(path, path.iter().count(), &mut read_files).expect("failed to load files");
        assert_eq!(
            read_files
                .get(&"en".parse::<LanguageIdentifier>().ok())
                .unwrap()
                .iter()
                .map(|f| &f.source)
                .collect::<Vec<&String>>(),
            vec![
                "./examples/locales/global.ftl",
                "./examples/locales/en/intl.ftl",
                "./examples/locales/en/login.ftl",
                "./examples/locales/en/main.ftl",
                "./examples/locales/en/movie.ftl",
                "./examples/locales/en/sports.ftl",
            ]
        );
    }
    #[test]
    fn load_lang_region_order_respected() {
        let mut read_files = HashMap::new();
        let path = Path::new("./examples/locales");
        load_directory(path, path.iter().count(), &mut read_files).expect("failed to load files");
        assert_eq!(
            read_files
                .get(&"en-US".parse::<LanguageIdentifier>().ok())
                .unwrap()
                .iter()
                .map(|f| &f.source)
                .collect::<Vec<&String>>(),
            vec![
                "./examples/locales/global.ftl",
                "./examples/locales/en/intl.ftl",
                "./examples/locales/en/login.ftl",
                "./examples/locales/en/main.ftl",
                "./examples/locales/en/movie.ftl",
                "./examples/locales/en/sports.ftl",
                "./examples/locales/en-US/intl.ftl",
                "./examples/locales/en-US/overrides.ftl",
            ]
        );
    }

    #[test]
    fn i18n_loader_translate_us() {
        let i18n = FluentMachine::build_loader(DirectoryLoader::new("examples/locales/"))
            .unwrap()
            .set_fallback_locale("en-US")
            .expect("failed to parse locale")
            .finish()
            .expect("failed to create FluentMachine");

        let lang = i18n.negotiate_languages("de-AT;0.9, de-DE;0.8, de;0.7, en-US;0.5");
        assert_eq!(
            lang,
            vec![
                &"en-US".parse::<LanguageIdentifier>().unwrap(),
                &"en".parse::<LanguageIdentifier>().unwrap(),
                &"en-UK".parse::<LanguageIdentifier>().unwrap(),
            ]
        );
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
    fn i18n_loader_translate_pt() {
        let i18n = FluentMachine::build_loader(DirectoryLoader::new("examples/locales/"))
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
        let i18n = FluentMachine::build_loader(DirectoryLoader::new("examples/locales/"))
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
