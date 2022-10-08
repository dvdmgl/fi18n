about = Sobre { brand-name }.
update-successful = { brand-name } foi atualizada.
-girl = rapariga
-you-have = Tu tens

i-am = Eu sou { $gender ->
        [masculine] um rapaz.
        [feminine] uma { -girl }.
       *[other] uma pessoa.
    }

movie-list = { -you-have } { $movies ->
       *[one] um filme
        [other] { $movies } filmes
    } para ver.
