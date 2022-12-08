use actix_web::{http::header::ACCEPT_LANGUAGE, HttpRequest};
use std::sync::Arc;

use crate::machine::TranslateFnSend;

use super::{machine::TranslateFn, FluentMachine, LanguageIdentifier};

/// Implementation for actix-web
#[cfg(feature = "actix-web4")]
impl FluentMachine {
    /// Returns translate closure, with resolved locales for request.
    ///
    /// # Locale resolution
    /// If is set [`FluentMachineBuilder::set_cookie_name`](crate::builders::FluentMachineBuilder::set_cookie_name)
    /// and is a match to [supported_locales](crate::FluentMachine::get_supported_locales) else
    /// resolves using [`FluentMachine::negotiate_languages`] according to
    /// [`actix_web::http::header::ACCEPT_LANGUAGE`] headers
    ///
    /// ## Example with actix-web (requires features = ["actix-web4"]):
    ///
    /// ```no_run
    /// use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer};
    /// use fi18n::{FluentMachine, NegotiationStrategy, f_args, loaders::DirectoryLoader};
    /// use std::io;
    ///
    /// async fn index(req: HttpRequest, i18n: web::Data<FluentMachine>) -> String {
    ///     let t = i18n.from_request_tanslate(&req);
    ///     t(
    ///         "movie-list".try_into().unwrap(),
    ///         Some(&f_args![
    ///             "movies" => 5,
    ///             "username" => "Foo",
    ///         ])
    ///     )
    /// }
    ///
    /// #[actix_web::main]
    /// async fn main() -> io::Result<()> {
    ///     let i18n = FluentMachine::build_loader(DirectoryLoader::new("examples/locales/"))
    ///         .unwrap()
    ///         .set_fallback_locale("en-US")
    ///         .expect("failed to parse locale")
    ///         .finish()
    ///         .expect("failed to create FluentMachine");
    ///     let machine = web::Data::new(i18n);
    ///     HttpServer::new(move || {
    ///         App::new()
    ///             .app_data(machine.clone())
    ///             .service(web::resource("/").to(index))
    ///     })
    ///     .bind("127.0.0.1:8081")?
    ///     .run()
    ///     .await?;
    ///     Ok(())
    /// }
    /// ```
    #[inline]
    #[cfg_attr(docsrs, doc(cfg(feature = "actix-web4")))]
    pub fn from_request_tanslate(&self, request: &HttpRequest) -> TranslateFn<'_> {
        if let Some(cookie_name) = &self.cookie_name {
            if let Some(lang) = request
                .cookie(&cookie_name)
                .map(|f| String::from(f.value()))
            {
                match lang.parse::<LanguageIdentifier>() {
                    Ok(lang) if self.available.contains(&lang) => {
                        return Box::new(move |key, options| self.t(&[&lang], key, options))
                    }
                    _ => (),
                }
            }
        }
        self.localize_t(
            request
                .headers()
                .get(ACCEPT_LANGUAGE)
                .map(|h| h.to_str().unwrap())
                .unwrap_or(&self.fallback_string),
        )
    }
    #[inline]
    #[cfg_attr(docsrs, doc(cfg(feature = "actix-web4")))]
    /// as [`from_request_tanslate`] but `Send`
    pub fn from_request_tanslate_sync(&self, request: &HttpRequest) -> TranslateFnSend<'_> {
        if let Some(cookie_name) = &self.cookie_name {
            if let Some(lang) = request
                .cookie(&cookie_name)
                .map(|f| String::from(f.value()))
            {
                match lang.parse::<LanguageIdentifier>() {
                    Ok(lang) if self.available.contains(&lang) => {
                        return Arc::new(move |key, options| self.t(&[&lang], key, options))
                    }
                    _ => (),
                }
            }
        }
        self.localize_t_send(
            request
                .headers()
                .get(ACCEPT_LANGUAGE)
                .map(|h| h.to_str().unwrap())
                .unwrap_or(&self.fallback_string),
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::FluentMachine;
    use actix_web::{cookie::Cookie, test::TestRequest};

    #[actix_web::test]
    async fn actix_request_tanslate_fn_headers() {
        let i18n = FluentMachine::build()
            .add_resource_override(
                "en",
                r#"
region = International
missing = Missing on others
"#,
            )
            .expect("failed to add en")
            .add_resource_override(
                "en-UK",
                r#"
region = United Kingdom
"#,
            )
            .expect("failed to add en-UK")
            .add_resource_override(
                "en-US",
                r#"
region = United States
"#,
            )
            .expect("failed to add en-US")
            .finish()
            .unwrap();

        let t_en = i18n.from_request_tanslate(
            &TestRequest::get()
                .insert_header((
                    actix_web::http::header::ACCEPT_LANGUAGE,
                    "en-US;0.9,de-DE;0.8,de;0.7,en;0.5",
                ))
                .to_http_request(),
        );
        assert_eq!(t_en("region".try_into().unwrap(), None), "United States");
        let t_en = i18n.from_request_tanslate(
            &TestRequest::get()
                .insert_header((
                    actix_web::http::header::ACCEPT_LANGUAGE,
                    "de-DE;0.9,en-UK;0.8,de;0.7,en;0.5",
                ))
                .to_http_request(),
        );
        assert_eq!(t_en("region".try_into().unwrap(), None), "United Kingdom");
        let t_en = i18n.from_request_tanslate(
            &TestRequest::get()
                .insert_header((actix_web::http::header::ACCEPT_LANGUAGE, "de-DE;0.9,de;0.7"))
                .to_http_request(),
        );
        assert_eq!(t_en("region".try_into().unwrap(), None), "International");
    }

    #[actix_web::test]
    async fn actix_request_tanslate_invalid_cookie() {
        let i18n = FluentMachine::build()
            .add_resource_override(
                "en",
                r#"
region = International
"#,
            )
            .expect("failed to add en")
            .set_cookie_name("invalid")
            .finish()
            .unwrap();

        let t = i18n.from_request_tanslate(
            &TestRequest::get()
                .insert_header((
                    actix_web::http::header::ACCEPT_LANGUAGE,
                    "en-US;0.9,de-DE;0.8,de;0.7,en;0.5",
                ))
                .to_http_request(),
        );
        assert_eq!(t("region".try_into().unwrap(), None), "International");
    }

    #[actix_web::test]
    async fn actix_request_tanslate_cookie() {
        let i18n = FluentMachine::build()
            .add_resource_override(
                "en",
                r#"
region = International
"#,
            )
            .expect("failed to add en")
            .add_resource_override(
                "pt-BR",
                r#"
region = Brazil
"#,
            )
            .expect("failed to add pt-BR")
            .set_cookie_name("locale")
            .finish()
            .unwrap();

        let t = i18n.from_request_tanslate(
            &TestRequest::get()
                .insert_header((
                    actix_web::http::header::ACCEPT_LANGUAGE,
                    "en-US;0.9,de-DE;0.8,de;0.7,en;0.5",
                ))
                .cookie(Cookie::new("locale", "pt-BR"))
                .to_http_request(),
        );
        assert_eq!(t("region".try_into().unwrap(), None), "Brazil");
    }

    #[actix_web::test]
    async fn actix_request_tanslate_cookie_lang_not_available() {
        let i18n = FluentMachine::build()
            .add_resource_override(
                "en",
                r#"
region = International
"#,
            )
            .expect("failed to add en")
            .add_resource_override(
                "pt-BR",
                r#"
region = Brazil
"#,
            )
            .expect("failed to add pt-BR")
            .set_cookie_name("locale")
            .finish()
            .unwrap();

        let t = i18n.from_request_tanslate(
            &TestRequest::get()
                .insert_header((
                    actix_web::http::header::ACCEPT_LANGUAGE,
                    "en-US;0.9,de-DE;0.8,de;0.7,en;0.5",
                ))
                .cookie(Cookie::new("locale", "jp"))
                .to_http_request(),
        );
        assert_eq!(t("region".try_into().unwrap(), None), "International");
    }
}
