// queries: queries.jl lines 757-856 (templates 16-18)
use crate::data::Data;
use crate::engine::*;
use super::helpers::*;

pub const ENTRIES: &[(&str, &str, fn(&Data) -> String)] = &[
    ("16a", "Adams, Stan || Carol Burnett vs. Anthony Perkins", q16a),
    ("16b", "!!!, Toy || & Teller", q16b),
    ("16c", "\"Brooklyn\" Tony Danza || (#1.5)", q16c),
    ("16d", "\"Brooklyn\" Tony Danza || (#1.5)", q16d),
    ("17a", "B, Khaz", q17a),
    ("17b", "Z'Dar, Robert", q17b),
    ("17c", "X'Volaitis, John", q17c),
    ("17d", "Abrahamsson, Bertil", q17d),
    ("17e", "$hort, Too", q17e),
    ("17f", "'El Galgo PornoStar', Blanquito", q17f),
    ("18a", "$1,000 || 10 || 40 Days and 40 Nights", q18a),
    ("18b", "Horror || 8.1 || Agorable", q18b),
    ("18c", "Action || 10 || #PostModem", q18c),
];

fn q16a(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_company).o((&d.company_country).eq("[us]")).k()
            .and((&d.movie_keyword).o(&d.keyword_keyword).eq("character-name-in-title").k())
            .and((&d.movie_episode_nr).ge(50).k())
            .and((&d.movie_episode_nr).lt(100).k())
            .o(
                (&d.movie_cast).o((&d.cast_person).o((&d.person_aka).o(&d.akaname_name)))
                    .x(&d.movie_title)
            ),
    );
    let mut m: [Option<&'static str>; 2] = [None; 2];
    q.drive(|_, (name, title)| {
        update(&mut m[0], name);
        update(&mut m[1], title);
    });
    fmt2(m)
}

fn q16b(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_company).o((&d.company_country).eq("[us]")).k()
            .and((&d.movie_keyword).o(&d.keyword_keyword).eq("character-name-in-title").k())
            .o(
                (&d.movie_cast).o((&d.cast_person).o((&d.person_aka).o(&d.akaname_name)))
                    .x(&d.movie_title)
            ),
    );
    let mut m: [Option<&'static str>; 2] = [None; 2];
    q.drive(|_, (name, title)| {
        update(&mut m[0], name);
        update(&mut m[1], title);
    });
    fmt2(m)
}

fn q16c(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_company).o((&d.company_country).eq("[us]")).k()
            .and((&d.movie_keyword).o(&d.keyword_keyword).eq("character-name-in-title").k())
            .and((&d.movie_episode_nr).lt(100).k())
            .o(
                (&d.movie_cast).o((&d.cast_person).o((&d.person_aka).o(&d.akaname_name)))
                    .x(&d.movie_title)
            ),
    );
    let mut m: [Option<&'static str>; 2] = [None; 2];
    q.drive(|_, (name, title)| {
        update(&mut m[0], name);
        update(&mut m[1], title);
    });
    fmt2(m)
}

fn q16d(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_company).o((&d.company_country).eq("[us]")).k()
            .and((&d.movie_keyword).o(&d.keyword_keyword).eq("character-name-in-title").k())
            .and((&d.movie_episode_nr).ge(5).k())
            .and((&d.movie_episode_nr).lt(100).k())
            .o(
                (&d.movie_cast).o((&d.cast_person).o((&d.person_aka).o(&d.akaname_name)))
                    .x(&d.movie_title)
            ),
    );
    let mut m: [Option<&'static str>; 2] = [None; 2];
    q.drive(|_, (name, title)| {
        update(&mut m[0], name);
        update(&mut m[1], title);
    });
    fmt2(m)
}

fn q17a(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_company).o((&d.company_country).eq("[us]")).k()
            .and((&d.movie_keyword).o(&d.keyword_keyword).eq("character-name-in-title").k())
            .o(
                (&d.movie_cast).o((&d.cast_person).o((&d.person_name).rx(r"^B")))
            ),
    );
    let mut m: Option<&'static str> = None;
    q.drive(|_, name| update(&mut m, name));
    fmt1(m)
}

fn q17b(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_company).k()
            .and((&d.movie_keyword).o(&d.keyword_keyword).eq("character-name-in-title").k())
            .o(
                (&d.movie_cast).o((&d.cast_person).o((&d.person_name).rx(r"^Z")))
            ),
    );
    let mut m: Option<&'static str> = None;
    q.drive(|_, name| update(&mut m, name));
    fmt1(m)
}

fn q17c(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_company).k()
            .and((&d.movie_keyword).o(&d.keyword_keyword).eq("character-name-in-title").k())
            .o(
                (&d.movie_cast).o((&d.cast_person).o((&d.person_name).rx(r"^X")))
            ),
    );
    let mut m: Option<&'static str> = None;
    q.drive(|_, name| update(&mut m, name));
    fmt1(m)
}

fn q17d(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_company).k()
            .and((&d.movie_keyword).o(&d.keyword_keyword).eq("character-name-in-title").k())
            .o(
                (&d.movie_cast).o((&d.cast_person).o((&d.person_name).rx(r"Bert")))
            ),
    );
    let mut m: Option<&'static str> = None;
    q.drive(|_, name| update(&mut m, name));
    fmt1(m)
}

fn q17e(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_company).o((&d.company_country).eq("[us]")).k()
            .and((&d.movie_keyword).o(&d.keyword_keyword).eq("character-name-in-title").k())
            .o(
                (&d.movie_cast).o((&d.cast_person).o(&d.person_name))
            ),
    );
    let mut m: Option<&'static str> = None;
    q.drive(|_, name| update(&mut m, name));
    fmt1(m)
}

fn q17f(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_company).k()
            .and((&d.movie_keyword).o(&d.keyword_keyword).eq("character-name-in-title").k())
            .o(
                (&d.movie_cast).o((&d.cast_person).o((&d.person_name).rx(r"B")))
            ),
    );
    let mut m: Option<&'static str> = None;
    q.drive(|_, name| update(&mut m, name));
    fmt1(m)
}

fn ib_18a<'d>(d: &'d Data) -> impl Query<R = &'static str> + 'd {
    (&d.movie_info).o(
        (&d.info_type).o(&d.infotype_info).eq("budget").k()
            .o(&d.info_info)
    )
}

fn q18a(d: &Data) -> String {
    let q = d.movie.o(
        ib_18a(d).k()
            .and((&d.movie_cast).in_s(
                (&d.cast_note).in_v(vec!["(producer)", "(executive producer)"]).k()
                    .and((&d.cast_person).in_s(
                        (&d.person_gender).eq("m").k()
                            .and((&d.person_name).rx(r"Tim").k())
                    ).k())
            ).k())
            .o(
                ib_18a(d)
                    .x((&d.movie_data).o(
                        (&d.data_type).o(&d.infotype_info).eq("votes").k()
                            .o(&d.data_data)
                    ))
                    .x(&d.movie_title)
            ),
    );
    let mut m: [Option<&'static str>; 3] = [None; 3];
    q.drive(|_, ((ib, dv), title)| {
        update(&mut m[0], ib);
        update(&mut m[1], dv);
        update(&mut m[2], title);
    });
    fmt3(m)
}

fn gf_18b<'d>(d: &'d Data) -> impl SetQ + 'd {
    (&d.info_type).o(&d.infotype_info).eq("genres").k()
        .and((&d.info_info).in_v(vec!["Horror", "Thriller"]).k())
        .minus((&d.info_note).k())
}

fn q18b(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_info).in_s(gf_18b(d)).k()
            .and((&d.movie_production_year).ge(2008).k())
            .and((&d.movie_production_year).le(2014).k())
            .and((&d.movie_cast).in_s(
                (&d.cast_note).in_v(super::sets::writer5()).k()
                    .and((&d.cast_person).in_s((&d.person_gender).eq("f").k()).k())
            ).k())
            .o(
                (&d.movie_info).o(gf_18b(d).o(&d.info_info))
                    .x((&d.movie_data).o(
                        (&d.data_type).o(&d.infotype_info).eq("rating").k()
                            .and((&d.data_data).gt("8.0").k())
                            .o(&d.data_data)
                    ))
                    .x(&d.movie_title)
            ),
    );
    let mut m: [Option<&'static str>; 3] = [None; 3];
    q.drive(|_, ((ig, dr), title)| {
        update(&mut m[0], ig);
        update(&mut m[1], dr);
        update(&mut m[2], title);
    });
    fmt3(m)
}

fn gf_18c<'d>(d: &'d Data) -> impl SetQ + 'd {
    (&d.info_type).o(&d.infotype_info).eq("genres").k()
        .and((&d.info_info).in_v(super::sets::genre6()).k())
}

fn q18c(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_info).in_s(gf_18c(d)).k()
            .and((&d.movie_cast).in_s(
                (&d.cast_note).in_v(super::sets::writer5()).k()
                    .and((&d.cast_person).in_s((&d.person_gender).eq("m").k()).k())
            ).k())
            .o(
                (&d.movie_info).o(gf_18c(d).o(&d.info_info))
                    .x((&d.movie_data).o(
                        (&d.data_type).o(&d.infotype_info).eq("votes").k()
                            .o(&d.data_data)
                    ))
                    .x(&d.movie_title)
            ),
    );
    let mut m: [Option<&'static str>; 3] = [None; 3];
    q.drive(|_, ((ig, dv), title)| {
        update(&mut m[0], ig);
        update(&mut m[1], dv);
        update(&mut m[2], title);
    });
    fmt3(m)
}
