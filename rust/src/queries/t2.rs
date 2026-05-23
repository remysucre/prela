// queries: queries.jl lines ~381-588 (22b, 22c, 22d, 5b, 5c, 15a-d, 11c-d, 13d, 6a-f)
use crate::data::Data;
use crate::engine::*;
use super::helpers::*;
use super::sets::*;

pub const ENTRIES: &[(&str, &str, fn(&Data) -> String)] = &[
    ("22d", "(#1.1) || 2.0 || 13 Productions", q22d),
    ("5b",  "(empty)", q5b),
    ("5c",  "11,830,420", q5c),
    ("15a", "USA:1 June 2007 || Battlestar Galactica: The Resistance", q15a),
    ("15b", "USA:27 April 2007 || RoboCop vs Terminator", q15b),
    ("15c", "USA:1 April 2003 || 24: Day Six - Debrief", q15c),
    ("15d", "(Not So) Instant Photo || 06/05", q15d),
    ("11c", "20th Century Fox Home Entertainment || (1997-2002) (worldwide) (all media) || 24", q11c),
    ("11d", "13th Street || (1954) (UK) (TV) || ...denn sie wissen nicht, was sie tun", q11d),
    ("13d", "\"O\" Films || 1.0 || #54 Meets #47", q13d),
    ("6a",  "marvel-cinematic-universe || Iron Man 3 || Downey Jr., Robert", q6a),
    ("6b",  "based-on-comic || The Avengers 2 || Downey Jr., Robert", q6b),
    ("6c",  "marvel-cinematic-universe || The Avengers 2 || Downey Jr., Robert", q6c),
    ("6d",  "based-on-comic || 2008 MTV Movie Awards || Downey Jr., Robert", q6d),
    ("6e",  "marvel-cinematic-universe || Iron Man 3 || Downey Jr., Robert", q6e),
    ("6f",  "based-on-comic || & Teller 2 || \"Steff\", Stefanie Oxmann Mcgaha", q6f),
];

fn q22d(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_info).in_s(
            (&d.info_type).o(&d.infotype_info).eq("countries").k()
                .and((&d.info_info).in_v(nordic10()).k())
        ).k()
            .and(
                (&d.movie_keyword).o(&d.keyword_keyword).in_v(murder4()).k()
                    .and(
                        (&d.movie_production_year).gt(2005).k()
                            .and((&d.movie_kind).o(&d.kind_kind).in_v(vec!["movie","episode"]).k())
                    )
            )
            .o(
                (&d.movie_title)
                    .x(
                        (&d.movie_data).o(
                            (&d.data_data).lt("8.5").k()
                                .and((&d.data_type).o(&d.infotype_info).eq("rating").k())
                                .o(&d.data_data)
                        )
                    )
                    .x(
                        (&d.movie_company).o(
                            (&d.company_country).ne("[us]").k()
                                .and((&d.company_type).o(&d.companytype_kind).eq("production companies").k())
                                .o(&d.company_name)
                        )
                    )
            )
    );
    let mut m: [Option<&'static str>; 3] = [None; 3];
    q.drive(|_, ((title, data), name)| {
        update(&mut m[0], title);
        update(&mut m[1], data);
        update(&mut m[2], name);
    });
    fmt3(m)
}

fn q5b(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_company).in_s(
            (&d.company_type).o(&d.companytype_kind).eq("production companies").k()
                .and(
                    (&d.company_note).rx(r"\(VHS\)").k()
                        .and(
                            (&d.company_note).rx(r"\(USA\)").k()
                                .and((&d.company_note).rx(r"\(1994\)").k())
                        )
                )
        ).k()
            .and(
                (&d.movie_info).o((&d.info_info).in_v(vec!["USA","America"])).k()
                    .and((&d.movie_production_year).gt(2010).k())
            )
            .o(&d.movie_title)
    );
    let mut m: Option<&'static str> = None;
    q.drive(|_, v| update(&mut m, v));
    fmt1(m)
}

fn q5c(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_company).in_s(
            (&d.company_type).o(&d.companytype_kind).eq("production companies").k()
                .and(
                    (&d.company_note).nrx(r"\(TV\)").k()
                        .and((&d.company_note).rx(r"\(USA\)").k())
                )
        ).k()
            .and(
                (&d.movie_info).o((&d.info_info).in_v(nordic10())).k()
                    .and((&d.movie_production_year).gt(1990).k())
            )
            .o(&d.movie_title)
    );
    let mut m: Option<&'static str> = None;
    q.drive(|_, v| update(&mut m, v));
    fmt1(m)
}

fn q15a(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_production_year).gt(2000).k()
            .and(
                (&d.movie_company).in_s(
                    (&d.company_country).eq("[us]").k()
                        .and(
                            (&d.company_note).rx(r"\(200.*\)").k()
                                .and((&d.company_note).rx(r"\(worldwide\)").k())
                        )
                ).k()
                    .and(
                        (&d.movie_keyword).k()
                            .and((&d.movie_aka).k())
                    )
            )
            .o(
                (&d.movie_info).o(
                    (&d.info_type).o(&d.infotype_info).eq("release dates").k()
                        .and(
                            (&d.info_info).rx(r"^USA:.* 200").k()
                                .and((&d.info_note).rx(r"internet").k())
                        )
                        .o(&d.info_info)
                )
                .x(&d.movie_title)
            )
    );
    let mut m: [Option<&'static str>; 2] = [None; 2];
    q.drive(|_, (info, title)| {
        update(&mut m[0], info);
        update(&mut m[1], title);
    });
    fmt2(m)
}

fn q15b(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_company).in_s(
            (&d.company_country).eq("[us]").k()
                .and(
                    (&d.company_name).eq("YouTube").k()
                        .and(
                            (&d.company_note).rx(r"\(200.*\)").k()
                                .and((&d.company_note).rx(r"\(worldwide\)").k())
                        )
                )
        ).k()
            .and(
                (&d.movie_keyword).k()
                    .and(
                        (&d.movie_aka).k()
                            .and(
                                (&d.movie_production_year).ge(2005).k()
                                    .and((&d.movie_production_year).le(2010).k())
                            )
                    )
            )
            .o(
                (&d.movie_info).o(
                    (&d.info_type).o(&d.infotype_info).eq("release dates").k()
                        .and(
                            (&d.info_info).rx(r"^USA:.* 200").k()
                                .and((&d.info_note).rx(r"internet").k())
                        )
                        .o(&d.info_info)
                )
                .x(&d.movie_title)
            )
    );
    let mut m: [Option<&'static str>; 2] = [None; 2];
    q.drive(|_, (info, title)| {
        update(&mut m[0], info);
        update(&mut m[1], title);
    });
    fmt2(m)
}

fn q15c(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_company).o((&d.company_country).eq("[us]")).k()
            .and(
                (&d.movie_keyword).k()
                    .and(
                        (&d.movie_aka).k()
                            .and((&d.movie_production_year).gt(1990).k())
                    )
            )
            .o(
                (&d.movie_info).o(
                    (&d.info_type).o(&d.infotype_info).eq("release dates").k()
                        .and(
                            (&d.info_info).rx(r"^USA:.* 199").k()
                                .or((&d.info_info).rx(r"^USA:.* 200").k())
                                .and((&d.info_note).rx(r"internet").k())
                        )
                        .o(&d.info_info)
                )
                .x(&d.movie_title)
            )
    );
    let mut m: [Option<&'static str>; 2] = [None; 2];
    q.drive(|_, (info, title)| {
        update(&mut m[0], info);
        update(&mut m[1], title);
    });
    fmt2(m)
}

fn q15d(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_company).o((&d.company_country).eq("[us]")).k()
            .and(
                (&d.movie_keyword).k()
                    .and(
                        (&d.movie_info).in_s(
                            (&d.info_type).o(&d.infotype_info).eq("release dates").k()
                                .and((&d.info_note).rx(r"internet").k())
                        ).k()
                            .and((&d.movie_production_year).gt(1990).k())
                    )
            )
            .o(
                (&d.movie_aka).o(&d.akatitle_title)
                    .x(&d.movie_title)
            )
    );
    let mut m: [Option<&'static str>; 2] = [None; 2];
    q.drive(|_, (aka, title)| {
        update(&mut m[0], aka);
        update(&mut m[1], title);
    });
    fmt2(m)
}

fn q11c(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_keyword).o(&d.keyword_keyword).in_v(vec!["sequel","revenge","based-on-novel"]).k()
            .and(
                (&d.movie_production_year).gt(1950).k()
                    .and((&d.movie_link).k())
            )
            .o(
                (&d.movie_company).o(
                    (&d.company_country).ne("[pl]").k()
                        .and(
                            (&d.company_name).rx(r"^20th Century Fox").k()
                                .or((&d.company_name).rx(r"^Twentieth Century Fox").k())
                                .and(
                                    (&d.company_type).o(&d.companytype_kind).ne("production companies").k()
                                        .and((&d.company_note).k())
                                )
                        )
                        .o((&d.company_name).x(&d.company_note))
                )
                .x(&d.movie_title)
            )
    );
    let mut m: [Option<&'static str>; 3] = [None; 3];
    q.drive(|_, ((name, note), title)| {
        update(&mut m[0], name);
        update(&mut m[1], note);
        update(&mut m[2], title);
    });
    fmt3(m)
}

fn q11d(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_keyword).o(&d.keyword_keyword).in_v(vec!["sequel","revenge","based-on-novel"]).k()
            .and(
                (&d.movie_production_year).gt(1950).k()
                    .and((&d.movie_link).k())
            )
            .o(
                (&d.movie_company).o(
                    (&d.company_country).ne("[pl]").k()
                        .and(
                            (&d.company_type).o(&d.companytype_kind).ne("production companies").k()
                                .and((&d.company_note).k())
                        )
                        .o((&d.company_name).x(&d.company_note))
                )
                .x(&d.movie_title)
            )
    );
    let mut m: [Option<&'static str>; 3] = [None; 3];
    q.drive(|_, ((name, note), title)| {
        update(&mut m[0], name);
        update(&mut m[1], note);
        update(&mut m[2], title);
    });
    fmt3(m)
}

fn q13d(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_kind).o(&d.kind_kind).eq("movie").k()
            .and(
                (&d.movie_info).o(
                    (&d.info_type).o(&d.infotype_info).eq("release dates")
                ).k()
            )
            .o(
                (&d.movie_company).o(
                    (&d.company_country).eq("[us]").k()
                        .and((&d.company_type).o(&d.companytype_kind).eq("production companies").k())
                        .o(&d.company_name)
                )
                .x(
                    (&d.movie_data).o(
                        (&d.data_type).o(&d.infotype_info).eq("rating").k()
                            .o(&d.data_data)
                    )
                )
                .x(&d.movie_title)
            )
    );
    let mut m: [Option<&'static str>; 3] = [None; 3];
    q.drive(|_, ((name, data), title)| {
        update(&mut m[0], name);
        update(&mut m[1], data);
        update(&mut m[2], title);
    });
    fmt3(m)
}

fn q6a(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_production_year).gt(2010).k()
            .and((&d.movie_keyword).o(&d.keyword_keyword).eq("marvel-cinematic-universe").k())
            .o(
                (&d.movie_keyword).o(&d.keyword_keyword).eq("marvel-cinematic-universe")
                    .x(&d.movie_title)
                    .x(
                        (&d.movie_cast).o(
                            (&d.cast_person).o((&d.person_name).rx(r"Downey.*Robert"))
                        )
                    )
            )
    );
    let mut m: [Option<&'static str>; 3] = [None; 3];
    q.drive(|_, ((kw, title), name)| {
        update(&mut m[0], kw);
        update(&mut m[1], title);
        update(&mut m[2], name);
    });
    fmt3(m)
}

fn q6b(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_production_year).gt(2014).k()
            .and((&d.movie_keyword).o(&d.keyword_keyword).in_v(kw8()).k())
            .o(
                (&d.movie_keyword).o(&d.keyword_keyword).in_v(kw8())
                    .x(&d.movie_title)
                    .x(
                        (&d.movie_cast).o(
                            (&d.cast_person).o((&d.person_name).rx(r"Downey.*Robert"))
                        )
                    )
            )
    );
    let mut m: [Option<&'static str>; 3] = [None; 3];
    q.drive(|_, ((kw, title), name)| {
        update(&mut m[0], kw);
        update(&mut m[1], title);
        update(&mut m[2], name);
    });
    fmt3(m)
}

fn q6c(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_production_year).gt(2014).k()
            .and((&d.movie_keyword).o(&d.keyword_keyword).eq("marvel-cinematic-universe").k())
            .o(
                (&d.movie_keyword).o(&d.keyword_keyword).eq("marvel-cinematic-universe")
                    .x(&d.movie_title)
                    .x(
                        (&d.movie_cast).o(
                            (&d.cast_person).o((&d.person_name).rx(r"Downey.*Robert"))
                        )
                    )
            )
    );
    let mut m: [Option<&'static str>; 3] = [None; 3];
    q.drive(|_, ((kw, title), name)| {
        update(&mut m[0], kw);
        update(&mut m[1], title);
        update(&mut m[2], name);
    });
    fmt3(m)
}

fn q6d(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_production_year).gt(2000).k()
            .and((&d.movie_keyword).o(&d.keyword_keyword).in_v(kw8()).k())
            .o(
                (&d.movie_keyword).o(&d.keyword_keyword).in_v(kw8())
                    .x(&d.movie_title)
                    .x(
                        (&d.movie_cast).o(
                            (&d.cast_person).o((&d.person_name).rx(r"Downey.*Robert"))
                        )
                    )
            )
    );
    let mut m: [Option<&'static str>; 3] = [None; 3];
    q.drive(|_, ((kw, title), name)| {
        update(&mut m[0], kw);
        update(&mut m[1], title);
        update(&mut m[2], name);
    });
    fmt3(m)
}

fn q6e(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_production_year).gt(2000).k()
            .and((&d.movie_keyword).o(&d.keyword_keyword).eq("marvel-cinematic-universe").k())
            .o(
                (&d.movie_keyword).o(&d.keyword_keyword).eq("marvel-cinematic-universe")
                    .x(&d.movie_title)
                    .x(
                        (&d.movie_cast).o(
                            (&d.cast_person).o((&d.person_name).rx(r"Downey.*Robert"))
                        )
                    )
            )
    );
    let mut m: [Option<&'static str>; 3] = [None; 3];
    q.drive(|_, ((kw, title), name)| {
        update(&mut m[0], kw);
        update(&mut m[1], title);
        update(&mut m[2], name);
    });
    fmt3(m)
}

fn q6f(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_production_year).gt(2000).k()
            .and((&d.movie_keyword).o(&d.keyword_keyword).in_v(kw8()).k())
            .o(
                (&d.movie_keyword).o(&d.keyword_keyword).in_v(kw8())
                    .x(&d.movie_title)
                    .x(
                        (&d.movie_cast).o((&d.cast_person).o(&d.person_name))
                    )
            )
    );
    let mut m: [Option<&'static str>; 3] = [None; 3];
    q.drive(|_, ((kw, title), name)| {
        update(&mut m[0], kw);
        update(&mut m[1], title);
        update(&mut m[2], name);
    });
    fmt3(m)
}
