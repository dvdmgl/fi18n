fi18n is a work in progress to simplify [Project Fluent](https://projectfluent.org/) ecosystem into
a simple easy to use API with [`builders`] and [`loaders`]:

* [`fluent-bundle`](https://crates.io/crates/fluent-bundle) - Fluent bundle
* [`fluent-langneg`](https://crates.io/crates/fluent-langneg) - Perform language negotiation
* [`unic-langid`](https://crates.io/crates/unic-langid) - Language Identifier

into an high level api [`FluentMachine`], with 2 builders and a directory loader to simplify the usage of [Project Fluent](https://projectfluent.org/).

# Example
```rust
use fi18n::{f_args, FluentMachine, loaders::DirectoryLoader};

let i18n = FluentMachine::build_loader(DirectoryLoader::new("examples/locales/"))
    .unwrap()
    .set_fallback_locale("en-US")
    .expect("failed to parse locale")
    .finish()
    .expect("failed to create FluentMachine");

let locale_us = i18n.localize_t("en-US");
let locale_uk = i18n.localize_t("en-UK");
let locale_pt = i18n.localize_t("pt-PT");
let locale_br = i18n.localize_t("pt-BR");
let locale_jp = i18n.localize_t("jp");

assert_eq!(
    locale_uk("football".try_into().unwrap(), None),
    "American Football is the biggest North American sport, with Super Bowl 112.3 million viewers.",
    "let's call American Football, whatever that is",
);
assert_eq!(
    locale_jp("soccer".try_into().unwrap(), None),
    "Soccer is the biggest sport in the world, with UEFA Champions League final 380 million viewers.",
    "fallback is en-US",
);    
assert_eq!(
    locale_us("soccer".try_into().unwrap(), None),
    "Soccer is the biggest sport in the world, with UEFA Champions League final 380 million viewers.",
    "Americans call football soccer",
);
assert_eq!(
    locale_uk("soccer".try_into().unwrap(), None),
    "Football is the biggest sport in the world, with UEFA Champions League final 380 million viewers.",
    "English call football, football",
);
assert_eq!(
    locale_pt("soccer".try_into().unwrap(), None),
    "Futebol é o maior desporto do mundo, com os 380 milhões de telespectadores na final da Liga dos Campeões.",
    "Champions League in Portuguese is Liga dos Campeões",
);
assert_eq!(
    locale_br("soccer".try_into().unwrap(), None),
    "Futebol é o maior desporto do mundo, com os 380 milhões de telespectadores na final da Champions League.",
    "Champions League in Brazil is Champions League... didn't they speak Portuguese?",
);
assert_eq!(
    locale_us("login.not-found".try_into().unwrap(), Some(&f_args!["username" => "nobody"])),
    "\u{2068}Username\u{2069} \u{2068}nobody\u{2069} not found.",
    "attributes and arguments",
);
assert_eq!(
    locale_pt("login.not-found".try_into().unwrap(), Some(&f_args!["username" => "nobody"])),
    "\u{2068}Utilizador\u{2069} \u{2068}nobody\u{2069} não encontrado.",
    "attributes and arguments",
);
assert_eq!(
    locale_br("login.not-found".try_into().unwrap(), Some(&f_args!["username" => "nobody"])),
    "\u{2068}Usuário\u{2069} \u{2068}nobody\u{2069} não encontrado.",
    "attributes and arguments, usuário as Brazilian",
);
```
