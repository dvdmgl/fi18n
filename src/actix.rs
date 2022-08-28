use super::{Fkey, FluentArgs, FluentMachine};
use actix_web::{http::header::ACCEPT_LANGUAGE, HttpRequest};
use fluent_langneg::{negotiate_languages, parse_accepted_languages};
use std::boxed::Box;

#[cfg(feature = "actix-web4")]
pub type TranslateFn<'a> = Box<dyn Fn(Fkey<'a>, Option<&'a FluentArgs>) -> String + 'a>;

use super::I18nStorage;

impl I18nStorage {
    /// Actix helper function to translate to resolved to local
    /// ## Example with actix-web (requires features = ["actix-web4"]):
    ///
    /// ```no_run
    /// use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer};
    /// use fi18n::{I18nStorage, FluentMachine, NegotiationStrategy, f_args};
    /// use std::io;
    ///
    /// async fn index(req: HttpRequest, i18n: web::Data<I18nStorage>) -> String {
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
    ///     let i18hm = web::Data::new(I18nStorage::new(
    ///         "locales/",
    ///         "en-US".into(),
    ///         NegotiationStrategy::Filtering,
    ///     ));
    ///
    ///     HttpServer::new(move || {
    ///         App::new()
    ///             .app_data(i18hm.clone())
    ///             .service(web::resource("/").to(index))
    ///     })
    ///     .bind("127.0.0.1:8081")?
    ///     .run()
    ///     .await?;
    ///     Ok(())
    /// }
    /// ```
    #[inline]
    pub fn from_request_tanslate(&self, request: &HttpRequest) -> TranslateFn<'_> {
        let langs = negotiate_languages(
            &parse_accepted_languages(
                request
                    .headers()
                    .get(ACCEPT_LANGUAGE)
                    .map(|h| h.to_str().unwrap())
                    .unwrap_or(&self.fallback_string),
            ),
            &self.available,
            Some(&self.fallback),
            self.strategy,
        );

        Box::new(move |key, options| self.t(&langs, key, options))
    }
}

#[cfg(test)]
mod tests {
    use crate::{I18nStorage, NegotiationStrategy};
    use actix_web::test::TestRequest;

    #[cfg(feature = "actix-web4")]
    #[cfg_attr(feature = "actix-web4", actix_web::test)]
    async fn actix_request_tansltate_fn() {
        let i18n = I18nStorage::new("locales/", "en-US".into(), NegotiationStrategy::Filtering);
        let t = i18n.from_request_tanslate(
            &TestRequest::get()
                .insert_header((
                    actix_web::http::header::ACCEPT_LANGUAGE,
                    "en-US;0.9,de-DE;0.8,de;0.7;en-US;0.5,en",
                ))
                .to_http_request(),
        );
        assert_eq!(
            t("i-am".try_into().unwrap(), None),
            "I am a \u{2068}person.\u{2069}"
        );
    }
}
