fi18n combines:

* [`fluent-bundle`](https://crates.io/crates/fluent-bundle) - Fluent bundle
* [`fluent-langneg`](https://crates.io/crates/fluent-langneg) - Perform language negotiation
* [`unic-langid`](https://crates.io/crates/unic-langid) - Language Identifier

into a small a high level api/loader to simplify the usage of [Project Fluent](https://projectfluent.org/).

Takes advantage of fluent [`overriding`](fluent_bundle::FluentBundle::add_resource_overriding)
to construct a more DRYer natural translations, using isolation of resources in a
path structure `{global}/{language}/{region}/`.

### Features:
- **with-title**: TITLE function
- **actix-web4**: [`actix-web`](actix-web) support [`I18nStorage::from_request_tanslate`]


- `global` - is used in as shared resource.
- `language` - base language that should be a consistent language, `terms` that can be overridden in region.
Should contain the language messages to default language without region, with `terms` that will be overridden in region specification.
- `region` - specific `terms` or `messages` for the region, overwrites the parent language.

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

<pre>
brand-name = Example ORG
</pre>

Global `messages` and `terms` used in all translations, ex: `brand-name`.

- `locales/en/movie.ftl`

<pre>
# base language: en-US
-movie = movie

movie-list = { $username }, you have { $movies ->
       *[one] one { -movie }
        [other] { $movies } { -movie }s
    } to watch in { brand-name }.
    .title = { TITLE(-movie) }s list
</pre>

Declare `-movie` as a private `term` in US English is _movie_, as in UK English is _film_.

- `locales/en/UK/overrides.ftl`

<pre>
-movie = film
</pre>

Specific language `terms` to a UK English, overwrites the previous `en` `-movie` `term` to UK English _film_.


- `locales/en/US/overrides.ftl`
<pre>
# kept blank to generate en-US region, there are no need for overwrites, as the `en` languages is in en-US
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
        &vec![&en_us],
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
        &vec![&en_us],
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
        &vec![&en_uk],
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
        &vec![&en_uk],
        // you can access the title attribute using `.`
        "movie-list.title",
        None
    ),
    "\u{2068}Film\u{2069}s list"
);
```
