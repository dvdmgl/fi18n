-movie = movie

movie-list = { $username }, you have { $movies ->
       *[one] one { -movie }
        [other] { $movies } { -movie }s
    } to watch in { brand-name }.
    .title = { TITLE(-movie) }s list
