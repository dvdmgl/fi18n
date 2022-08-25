fi18n provides [`I18nStorage`] a small a high level api/loader to make simple and easy way how translations can be done using [Fluent Syntax](https://projectfluent.org/).

The crate builds on top of:
* [`fluent-bundle`](https://crates.io/crates/fluent-bundle) - Fluent bundle
* [`fluent-langneg`](https://crates.io/crates/fluent-langneg) - Perform language negotiation
* [`unic-langid`](https://crates.io/crates/unic-langid) - Language Identifier

Takes advantage of fluent [`overriding`](https://docs.rs/fluent-bundle/0.15.2/fluent_bundle/bundle/struct.FluentBundle.html#method.add_resource_overriding)
to construct a more DRYer natural translations, using isolation of resources in a
path structure `{global}/{language}/{region}/`.

- `global` - are used in as shared resource
- `language` - base language that should be a consistent language, `terms` that can be overridden in region.
- `region` - specific `terms` or messages overridden for this region of the parent language.

## Example folder and file structure

```text
locales/
    - global.ftl
    en/
        - login.ftl
        - movie.ftl
        US/
            - overrides.ftl
        UK/
            - overrides.ftl
    pt/
        ..
        BR/
            ..
        PT/
            ..
```

<!-- using <pre> as rust doc removes fluent comments -->

- `locales/global.ftl`

Global messages used in all translations, ex: brand-name

<pre>
brand-name = Example ORG
</pre>

- `locales/en/movie.ftl`

Should contain the language messages to default language without region, with terms that will be overridden in region specification as example:

<pre>
# base en default language US English
# private term, referenced in other messages, US `movie` will be overridden to UK `film`.
-movie = movie

movie-list = { $username }, you have { $movies ->
       *[one] one { -movie }
        [other] { $movies } { -movie }s
    } to watch in { brand-name }.
    .title = { TITLE(-movie) }s list
</pre>

- `locales/en/UK/overrides.ftl`

Specific language terms to a specific region

<pre>
# overrides the previous `-movie` term from US English to UK English.
-movie = film
</pre>

- `locales/en/US/overrides.ftl`
<pre>
# keep blank to generate en-US region
</pre>


```
use fi18n::{f_args, FluentValue, I18nStorage, NegotiationStrategy, LanguageIdentifier};

// read locales from folder
let mut i18n = I18nStorage::new("locales/", "en-Us".into(), NegotiationStrategy::Filtering);

// add a function to bundles
for bundle in i18n.bundles_mut() {
    bundle
        .add_function("STRLEN", |positional, _named| match positional {
            [FluentValue::String(str)] => str.len().into(),
            _ => FluentValue::Error,
        }).expect("Failed to add a function to the bundle.");
}

// en-US
let en_us = "en-US".parse::<LanguageIdentifier>().unwrap();
assert_eq!(
    i18n.t(
        &en_us,
        "movie-list",
        Some(&f_args![
            "movies" => 1,
            "username" => "Foo",
        ])
    ),
    "\u{2068}Foo\u{2069}, you have \u{2068}one movie\u{2069} to watch in Example ORG."
);
// accessing title attribute and use TITLE function
assert_eq!(
    i18n.t(
        &en_us,
        // you can access the title attribute using `.`
        "movie-list.title",
        None
    ),
    "\u{2068}Movie\u{2069}s list"
);

// en-UK
let en_uk = "en-UK".parse::<LanguageIdentifier>().unwrap();
assert_eq!(
    i18n.t(
        &en_uk,
        "movie-list",
        Some(&f_args![
            "movies" => 5,
            "username" => "Foo",
        ])
    ),
    "\u{2068}Foo\u{2069}, you have \u{2068}\u{2068}5\u{2069} films\u{2069} to watch in Example ORG."
);
// accessing title attribute and use TITLE function
assert_eq!(
    i18n.t(
        &en_uk,
        // you can access the title attribute using `.`
        "movie-list.title",
        None
    ),
    "\u{2068}Film\u{2069}s list"
);
```

## Example with actix-web (requires features = ["actix-web4"]):

```no_run
use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer};
use fi18n::{I18nStorage, NegotiationStrategy, f_args};
use std::io;

async fn index(req: HttpRequest, i18n: web::Data<I18nStorage>) -> String {
    let t = i18n.from_request_tanslate(&req);
    t("movie-list", Some(&f_args![
            "movies" => 5,
            "username" => "Foo",
        ]))
}

#[actix_web::main]
async fn main() -> io::Result<()> {
    let i18hm = web::Data::new(I18nStorage::new(
        "locales/",
        "en-US".into(),
        NegotiationStrategy::Filtering,
    ));

    HttpServer::new(move || {
        App::new()
            .app_data(i18hm.clone())
            .service(web::resource("/").to(index))
    })
    .bind("127.0.0.1:8081")?
    .run()
    .await?;
    Ok(())
}
```
