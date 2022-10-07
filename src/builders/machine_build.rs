use fluent_bundle::{FluentArgs, FluentResource, FluentValue};
use std::{
    collections::{hash_map::Entry, HashMap},
    fmt::Debug,
    sync::Arc,
};
use unic_langid::LanguageIdentifier;

use crate::{machine::MachineBundles, Error, FluentMachine, MachineBundle, NegotiationStrategy};

/// Configure [`FluentMachine`] options negotiation strategy, default locale, add functions
/// to locales and add a resource to a specific locale.
///
/// # Example
/// ```rust
/// use fi18n::{f_args, FluentMachine, FluentValue, LanguageIdentifier, NegotiationStrategy};
///
/// let i18n = FluentMachine::build()
///     .add_resource("en", r#"
/// login-input = Predefined value
///     .placeholder = email@example.com"#)
///     .expect("Should have added language en")
///     .add_resource("en-US", r#"
/// login-input = Predefined value
///     .placeholder = email@example.com"#)
///     .expect("Should have added locale en-US")
///     .add_resource_override("en-UK", r#"
/// login-input = Predefined value
///     .placeholder = email@example.com"#)
///     .expect("Should have added locale en-UK")
///     .set_strategy(NegotiationStrategy::Filtering)
///     .set_fallback_locale("en-UK")
///     .expect("Should set default fallback to United Kingdom region")
///     .finish()
///     .expect("Should have have finished creating of FluentMachine");
///
/// assert_eq!(
///     i18n.get_supported_locales(),
///     vec![
///         "en".parse::<LanguageIdentifier>().unwrap(),
///         "en-UK".parse::<LanguageIdentifier>().unwrap(),
///         "en-US".parse::<LanguageIdentifier>().unwrap(),
///     ]
/// );
///
/// assert_eq!(
///     i18n.get_fallback_locale(),
///     &"en-UK".parse::<LanguageIdentifier>().unwrap(),
/// );
///
/// let negotiated = i18n.negotiate_languages("en-US");
/// assert_eq!(
///     i18n.t(&negotiated, "login-input".try_into().unwrap(), None),
///     "Predefined value"
/// );
/// assert_eq!(
///     i18n.t(&negotiated, "login-input.placeholder".try_into().unwrap(), None),
///     "email@example.com"
/// );
///
/// ```

pub struct FluentMachineBuilder {
    pub(crate) bundles: MachineBundles,
    pub(crate) strategy: NegotiationStrategy,
    pub(crate) fallback: LanguageIdentifier,
    #[cfg(feature = "actix-web4")]
    pub(crate) fallback_string: String,
    #[cfg(feature = "actix-web4")]
    pub(crate) cookie_name: Option<String>,
}

impl Debug for FluentMachineBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FluentMachineBuilder")
            .field(
                "bundles",
                &self
                    .bundles
                    .keys()
                    .map(|locale| locale.to_string())
                    .collect::<Vec<String>>()
                    .join(","),
            )
            .field("strategy", &self.strategy)
            .field("fallback", &self.fallback)
            .finish()
    }
}

impl Default for FluentMachineBuilder {
    fn default() -> Self {
        Self {
            bundles: HashMap::default(),
            strategy: NegotiationStrategy::Filtering,
            fallback: "en".parse::<LanguageIdentifier>().unwrap(),
            #[cfg(feature = "actix-web4")]
            cookie_name: None,
            #[cfg(feature = "actix-web4")]
            fallback_string: "en".into(),
        }
    }
}

impl FluentMachineBuilder {
    /// Set [`FluentMachine::negotiate_languages`] [`NegotiationStrategy`].
    ///
    /// # Arguments:
    /// * `strategy` strategy applied to negotiate locales
    ///
    /// Default [`NegotiationStrategy::Filtering`]
    pub fn set_strategy(mut self, strategy: NegotiationStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    /// Set default locale.
    ///
    /// # Arguments:
    /// * `locale` valid locale with optional 2 regional letter code
    ///
    /// Default `en`
    #[cfg_attr(docsrs, doc(cfg(feature = "actix-web4")))]
    #[cfg(feature = "actix-web4")]
    pub fn set_fallback_locale(mut self, locale: &str) -> Result<Self, Error> {
        self.fallback = locale.parse::<LanguageIdentifier>()?;
        self.fallback_string = String::from(locale);
        Ok(self)
    }

    /// Set default locale.
    ///
    /// # Arguments:
    /// * `locale` valid locale with optional 2 regional letter code
    ///
    /// Default `en`
    #[cfg(not(feature = "actix-web4"))]
    pub fn set_fallback_locale(mut self, locale: &str) -> Result<Self, Error> {
        self.fallback = locale.parse::<LanguageIdentifier>()?;
        Ok(self)
    }

    /// Set default locale.
    ///
    /// # Arguments:
    /// * `locale` valid locale with optional 2 regional letter code
    ///
    /// Default `en`
    #[cfg(feature = "actix-web4")]
    pub fn set_cookie_name(mut self, name: &str) -> Self {
        self.cookie_name = Some(name.to_string());
        self
    }

    /// Finish building and returns [`FluentMachine`].
    ///
    /// ### Errors
    ///
    /// Returns [`Error::LocaleUnavailable`] if fallback locale is not present.
    pub fn finish(mut self) -> Result<FluentMachine, Error> {
        let mut available: Vec<LanguageIdentifier> = self.bundles.keys().cloned().collect();
        if !available.contains(&self.fallback) {
            return Err(Error::LocaleUnavailable(self.fallback));
        }

        if cfg!(feature = "with-title") {
            for bundle in self.bundles.values_mut() {
                bundle
                    .add_function("TITLE", |positional, _named| match positional {
                        [FluentValue::String(s)] => {
                            let mut c = s.chars();
                            match c.next() {
                                None => String::new(),
                                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                            }
                            .into()
                        }
                        _ => FluentValue::Error,
                    })
                    .expect("Failed to add a function to the bundle.");
            }
        }

        available.sort();
        available.shrink_to_fit();
        self.bundles.shrink_to_fit();

        Ok(FluentMachine {
            bundles: self.bundles,
            available,
            fallback: self.fallback,
            #[cfg(feature = "actix-web4")]
            fallback_string: self.fallback_string,
            #[cfg(feature = "actix-web4")]
            cookie_name: self.cookie_name,
            strategy: self.strategy,
        })
    }

    /// Add a function to bundles.
    ///
    /// # Arguments:
    /// * `name` function name id
    /// * `func` FTL function
    ///
    /// _For more information how to add function, check [`FluentBundle::add_function`](crate::fluent-bundle::bundle::FluentBundle::add_function`)_.
    pub fn add_function<F>(mut self, name: &str, func: &'static F) -> Result<Self, Error>
    where
        F: for<'a> Fn(&[FluentValue<'a>], &FluentArgs) -> FluentValue<'a> + Sync + Send + 'static,
    {
        for bundle in self.bundles.values_mut() {
            bundle.add_function(name, func)?;
        }

        Ok(self)
    }

    /// Add resource to a specific locale, overrides existing entries in case of collusion.
    /// If locale doesn't exist, will be added.
    ///
    /// # Arguments:
    /// * `locale` valid locale with optional 2 regional letter code
    /// * `resource` Fluent resource
    ///
    /// _For more information how overrides works, check [`FluentBundle::add_resource_overriding`](crate::fluent-bundle::bundle::FluentBundle::add_resource_override)_.
    pub fn add_resource_override(mut self, locale: &str, resource: &str) -> Result<Self, Error> {
        let locale = locale.parse::<LanguageIdentifier>()?;
        let r = Arc::new(
            FluentResource::try_new(resource.to_string()).map_err(|err| {
                let e = err.1.first().unwrap();
                let rg = e.slice.clone().unwrap_or_else(|| e.pos.clone());
                let line_start = resource[..rg.start].matches('\n').count() + 1;
                let part = &resource[rg];
                let line_end = part.matches('\n').count() + line_start - 1;
                Error::ParseError {
                    locale: locale.clone(),
                    part: part.into(),
                    line_end,
                    line_start,
                }
            })?,
        );
        if let Entry::Vacant(e) = self.bundles.entry(locale.clone()) {
            let mut bundle: MachineBundle = MachineBundle::new_concurrent(vec![locale]);
            bundle.add_resource_overriding(r);
            e.insert(bundle);
        } else {
            self.bundles
                .entry(locale)
                .and_modify(|b| b.add_resource_overriding(r));
        }

        Ok(self)
    }

    /// Add resource to a specific locale, in case of collusion of existing entries raises error.
    /// If locale doesn't exist, will be added.
    ///
    /// # Arguments:
    /// * `locale` a valid locale with optional 2 regional letter code
    /// * `resource` a Fluent resource
    ///
    /// _For more information, check [`FluentBundle::add_resource`](crate::fluent-bundle::bundle::FluentBundle::add_resource)_.
    pub fn add_resource(mut self, locale: &str, resource: &str) -> Result<Self, Error> {
        let locale = locale.parse::<LanguageIdentifier>()?;
        let r = FluentResource::try_new(resource.to_string()).map_err(|err| {
            let e = err.1.first().unwrap();
            let rg = e.slice.clone().unwrap_or_else(|| e.pos.clone());
            let line_start = resource[..rg.start].matches('\n').count() + 1;
            let part = &resource[rg];
            let line_end = part.matches('\n').count() + line_start - 1;
            Error::ParseError {
                locale: locale.clone(),
                part: part.into(),
                line_end,
                line_start,
            }
        })?;

        if !self.bundles.contains_key(&locale) {
            self.bundles.insert(
                locale.clone(),
                MachineBundle::new_concurrent(vec![locale.clone()]),
            );
        }
        let bundle = self.bundles.get_mut(&locale).unwrap();

        match bundle.add_resource(Arc::new(r)) {
            Ok(()) => Ok(self),
            Err(errs) => Err(Error::MultipleFluentError(errs)),
        }
    }
}
