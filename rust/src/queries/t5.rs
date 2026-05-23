// queries: 19a-26c (queries.jl lines 859-1111)
use crate::data::Data;
use crate::engine::*;
use super::helpers::*;
use super::sets::*;

fn co_21<'d>(d: &'d Data) -> impl Query<R = i64> + 'd {
    (&d.movie_company).in_s(
        (&d.company_country).ne("[pl]").k()
            .and(
                (&d.company_name).rx(r"Film").k()
                    .or((&d.company_name).rx(r"Warner").k())
            )
            .and(
                (&d.company_type).o(&d.companytype_kind).eq("production companies").k()
                    .minus((&d.company_note).k())
            )
    )
}

fn lk_21<'d>(d: &'d Data) -> impl Query<R = i64> + 'd {
    (&d.movie_link).in_s(
        (&d.movielink_type).o(&d.linktype_link).rx(r"follow").k()
    )
}

fn k_23ab<'d>(d: &'d Data) -> impl Query<R = &'static str> + 'd {
    (&d.movie_kind).o(&d.kind_kind).eq("movie")
}

fn k_23c<'d>(d: &'d Data) -> impl Query<R = &'static str> + 'd {
    (&d.movie_kind).o(&d.kind_kind)
        .in_v(vec!["movie", "tv movie", "video movie", "video game"])
}

fn gf_25ab<'d>(d: &'d Data) -> impl SetQ + 'd {
    (&d.info_type).o(&d.infotype_info).eq("genres").k()
        .and((&d.info_info).eq("Horror").k())
}

fn gf_25c<'d>(d: &'d Data) -> impl SetQ + 'd {
    (&d.info_type).o(&d.infotype_info).eq("genres").k()
        .and((&d.info_info).in_v(genre6()).k())
}

pub const ENTRIES: &[(&str, &str, fn(&Data) -> String)] = &[
    ("19a", "Angeline, Moriah || Blue Harvest", q19a),
    ("19b", "Jolie, Angelina || Kung Fu Panda", q19b),
    ("19c", "Alborg, Ana Esther || .hack//Akusei heni vol. 2", q19c),
    ("19d", "Aaron, Caroline || $9.99", q19d),
    ("20a", "Disaster Movie", q20a),
    ("20b", "Iron Man", q20b),
    ("20c", "Abell, Alistair || ...And Then I...", q20c),
    ("21a", "Det Danske Filminstitut || followed by || Der Serienkiller - Klinge des Todes", q21a),
    ("21b", "Filmlance International AB || followed by || Hämndens pris", q21b),
    ("21c", "Churchill Films || followed by || Batman Beyond", q21c),
    ("23a", "movie || The Analysts", q23a),
    ("23b", "movie || The Big Mope", q23b),
    ("23c", "movie || Dirt Merchant", q23c),
    ("24a", "Additional Voices || Baker, Andrea || Baiohazâdo 6", q24a),
    ("24b", "Tigress || Jolie, Angelina || Kung Fu Panda 2", q24b),
    ("25a", "Horror || 10 || -- And Now the Screaming Starts! || Abdallah, Damon", q25a),
    ("25b", "Horror || 138 || Vampire Boys || Campbell, Jeremiah", q25b),
    ("25c", "Action || 10 || $ || Aakeson, Kim Fupz", q25c),
    ("26a", "'Agua' Man || Acereda, Hermie || 7.1 || 3:10 to Yuma", q26a),
    ("26b", "Bank Manager || 8.2 || Inception", q26b),
    ("26c", "'Agua' Man || 1.9 || 12 Rounds", q26c),
];

fn q19a(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_company).in_s(
            (&d.company_country).eq("[us]").k()
                .and(
                    (&d.company_note).rx(r"\(USA\)").k()
                        .or((&d.company_note).rx(r"\(worldwide\)").k())
                )
        ).k()
            .and(
                (&d.movie_info).in_s(
                    (&d.info_type).o(&d.infotype_info).eq("release dates").k()
                        .and(
                            (&d.info_info).rx(r"^Japan:.*200").k()
                                .or((&d.info_info).rx(r"^USA:.*200").k())
                        )
                ).k()
                    .and(
                        (&d.movie_production_year).ge(2005).k()
                            .and((&d.movie_production_year).le(2009).k())
                    )
            )
            .o(
                (&d.movie_cast).o(
                    (&d.cast_note).in_v(voice4()).k()
                        .and(
                            (&d.cast_role).o(&d.roletype_role).eq("actress").k()
                                .and(
                                    (&d.cast_character).k()
                                        .and((&d.cast_person).in_s(
                                            (&d.person_gender).eq("f").k()
                                                .and(
                                                    (&d.person_name).rx(r"Ang").k()
                                                        .and((&d.person_aka).k())
                                                )
                                        ).k())
                                )
                        )
                        .o((&d.cast_person).o(&d.person_name))
                )
                    .x(&d.movie_title)
            )
    );
    let mut m: [Option<&'static str>; 2] = [None; 2];
    q.drive(|_, (name, title)| {
        update(&mut m[0], name);
        update(&mut m[1], title);
    });
    fmt2(m)
}

fn q19b(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_company).in_s(
            (&d.company_country).eq("[us]").k()
                .and(
                    (&d.company_note).rx(r"\(200.*\)").k()
                        .and(
                            (&d.company_note).rx(r"\(USA\)").k()
                                .or((&d.company_note).rx(r"\(worldwide\)").k())
                        )
                )
        ).k()
            .and(
                (&d.movie_info).in_s(
                    (&d.info_type).o(&d.infotype_info).eq("release dates").k()
                        .and(
                            (&d.info_info).rx(r"^Japan:.*2007").k()
                                .or((&d.info_info).rx(r"^USA:.*2008").k())
                        )
                ).k()
                    .and(
                        (&d.movie_production_year).ge(2007).k()
                            .and(
                                (&d.movie_production_year).le(2008).k()
                                    .and((&d.movie_title).rx(r"Kung.*Fu.*Panda").k())
                            )
                    )
            )
            .o(
                (&d.movie_cast).o(
                    (&d.cast_note).eq("(voice)").k()
                        .and(
                            (&d.cast_role).o(&d.roletype_role).eq("actress").k()
                                .and(
                                    (&d.cast_character).k()
                                        .and((&d.cast_person).in_s(
                                            (&d.person_gender).eq("f").k()
                                                .and(
                                                    (&d.person_name).rx(r"Angel").k()
                                                        .and((&d.person_aka).k())
                                                )
                                        ).k())
                                )
                        )
                        .o((&d.cast_person).o(&d.person_name))
                )
                    .x(&d.movie_title)
            )
    );
    let mut m: [Option<&'static str>; 2] = [None; 2];
    q.drive(|_, (name, title)| {
        update(&mut m[0], name);
        update(&mut m[1], title);
    });
    fmt2(m)
}

fn q19c(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_company).o((&d.company_country).eq("[us]")).k()
            .and(
                (&d.movie_info).in_s(
                    (&d.info_type).o(&d.infotype_info).eq("release dates").k()
                        .and(
                            (&d.info_info).rx(r"^Japan:.*200").k()
                                .or((&d.info_info).rx(r"^USA:.*200").k())
                        )
                ).k()
                    .and((&d.movie_production_year).gt(2000).k())
            )
            .o(
                (&d.movie_cast).o(
                    (&d.cast_note).in_v(voice4()).k()
                        .and(
                            (&d.cast_role).o(&d.roletype_role).eq("actress").k()
                                .and(
                                    (&d.cast_character).k()
                                        .and((&d.cast_person).in_s(
                                            (&d.person_gender).eq("f").k()
                                                .and(
                                                    (&d.person_name).rx(r"An").k()
                                                        .and((&d.person_aka).k())
                                                )
                                        ).k())
                                )
                        )
                        .o((&d.cast_person).o(&d.person_name))
                )
                    .x(&d.movie_title)
            )
    );
    let mut m: [Option<&'static str>; 2] = [None; 2];
    q.drive(|_, (name, title)| {
        update(&mut m[0], name);
        update(&mut m[1], title);
    });
    fmt2(m)
}

fn q19d(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_company).o((&d.company_country).eq("[us]")).k()
            .and(
                (&d.movie_info).o(
                    (&d.info_type).o(&d.infotype_info).eq("release dates")
                ).k()
                    .and((&d.movie_production_year).gt(2000).k())
            )
            .o(
                (&d.movie_cast).o(
                    (&d.cast_note).in_v(voice4()).k()
                        .and(
                            (&d.cast_role).o(&d.roletype_role).eq("actress").k()
                                .and(
                                    (&d.cast_character).k()
                                        .and((&d.cast_person).in_s(
                                            (&d.person_gender).eq("f").k()
                                                .and((&d.person_aka).k())
                                        ).k())
                                )
                        )
                        .o((&d.cast_person).o(&d.person_name))
                )
                    .x(&d.movie_title)
            )
    );
    let mut m: [Option<&'static str>; 2] = [None; 2];
    q.drive(|_, (name, title)| {
        update(&mut m[0], name);
        update(&mut m[1], title);
    });
    fmt2(m)
}

fn q20a(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_complete_cast).in_s(
            (&d.completecast_subject).o(&d.compcasttype_kind).eq("cast").k()
                .and((&d.completecast_status).o(&d.compcasttype_kind).rx(r"complete").k())
        ).k()
            .and(
                (&d.movie_keyword).o(&d.keyword_keyword).in_v(kw8()).k()
                    .and(
                        (&d.movie_kind).o(&d.kind_kind).eq("movie").k()
                            .and(
                                (&d.movie_production_year).gt(1950).k()
                                    .and((&d.movie_cast).o(
                                        (&d.cast_character).in_s(
                                            (&d.character_name).nrx(r"Sherlock").k()
                                                .and(
                                                    (&d.character_name).rx(r"Tony.*Stark").k()
                                                        .or((&d.character_name).rx(r"Iron.*Man").k())
                                                )
                                        )
                                    ).k())
                            )
                    )
            )
            .o(&d.movie_title)
    );
    let mut m: Option<&'static str> = None;
    q.drive(|_, t| update(&mut m, t));
    fmt1(m)
}

fn q20b(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_complete_cast).in_s(
            (&d.completecast_subject).o(&d.compcasttype_kind).eq("cast").k()
                .and((&d.completecast_status).o(&d.compcasttype_kind).rx(r"complete").k())
        ).k()
            .and(
                (&d.movie_keyword).o(&d.keyword_keyword).in_v(kw8()).k()
                    .and(
                        (&d.movie_kind).o(&d.kind_kind).eq("movie").k()
                            .and(
                                (&d.movie_production_year).gt(2000).k()
                                    .and((&d.movie_cast).in_s(
                                        (&d.cast_character).in_s(
                                            (&d.character_name).nrx(r"Sherlock").k()
                                                .and(
                                                    (&d.character_name).rx(r"Tony.*Stark").k()
                                                        .or((&d.character_name).rx(r"Iron.*Man").k())
                                                )
                                        ).k()
                                            .and((&d.cast_person).o(
                                                (&d.person_name).rx(r"Downey.*Robert")
                                            ).k())
                                    ).k())
                            )
                    )
            )
            .o(&d.movie_title)
    );
    let mut m: Option<&'static str> = None;
    q.drive(|_, t| update(&mut m, t));
    fmt1(m)
}

fn q20c(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_complete_cast).in_s(
            (&d.completecast_subject).o(&d.compcasttype_kind).eq("cast").k()
                .and((&d.completecast_status).o(&d.compcasttype_kind).rx(r"complete").k())
        ).k()
            .and(
                (&d.movie_keyword).o(&d.keyword_keyword).in_v(kw10()).k()
                    .and(
                        (&d.movie_kind).o(&d.kind_kind).eq("movie").k()
                            .and((&d.movie_production_year).gt(2000).k())
                    )
            )
            .o(
                (&d.movie_cast).o(
                    (&d.cast_character).o((&d.character_name).rx(r"[Mm]an")).k()
                        .o((&d.cast_person).o(&d.person_name))
                )
                    .x(&d.movie_title)
            )
    );
    let mut m: [Option<&'static str>; 2] = [None; 2];
    q.drive(|_, (name, title)| {
        update(&mut m[0], name);
        update(&mut m[1], title);
    });
    fmt2(m)
}

fn q21a(d: &Data) -> String {
    let q = d.movie.o(
        co_21(d).k()
            .and(
                (&d.movie_keyword).o(&d.keyword_keyword).eq("sequel").k()
                    .and(
                        lk_21(d).k()
                            .and(
                                (&d.movie_info).o((&d.info_info).in_v(nordic8())).k()
                                    .and(
                                        (&d.movie_production_year).ge(1950).k()
                                            .and((&d.movie_production_year).le(2000).k())
                                    )
                            )
                    )
            )
            .o(
                co_21(d).o(&d.company_name)
                    .x(lk_21(d).o((&d.movielink_type).o(&d.linktype_link)))
                    .x(&d.movie_title)
            )
    );
    let mut m: [Option<&'static str>; 3] = [None; 3];
    q.drive(|_, ((co, lk), title)| {
        update(&mut m[0], co);
        update(&mut m[1], lk);
        update(&mut m[2], title);
    });
    fmt3(m)
}

fn q21b(d: &Data) -> String {
    let q = d.movie.o(
        co_21(d).k()
            .and(
                (&d.movie_keyword).o(&d.keyword_keyword).eq("sequel").k()
                    .and(
                        lk_21(d).k()
                            .and(
                                (&d.movie_info).o(
                                    (&d.info_info).in_v(vec!["Germany", "German"])
                                ).k()
                                    .and(
                                        (&d.movie_production_year).ge(2000).k()
                                            .and((&d.movie_production_year).le(2010).k())
                                    )
                            )
                    )
            )
            .o(
                co_21(d).o(&d.company_name)
                    .x(lk_21(d).o((&d.movielink_type).o(&d.linktype_link)))
                    .x(&d.movie_title)
            )
    );
    let mut m: [Option<&'static str>; 3] = [None; 3];
    q.drive(|_, ((co, lk), title)| {
        update(&mut m[0], co);
        update(&mut m[1], lk);
        update(&mut m[2], title);
    });
    fmt3(m)
}

fn q21c(d: &Data) -> String {
    let q = d.movie.o(
        co_21(d).k()
            .and(
                (&d.movie_keyword).o(&d.keyword_keyword).eq("sequel").k()
                    .and(
                        lk_21(d).k()
                            .and(
                                (&d.movie_info).o((&d.info_info).in_v(nordic9())).k()
                                    .and(
                                        (&d.movie_production_year).ge(1950).k()
                                            .and((&d.movie_production_year).le(2010).k())
                                    )
                            )
                    )
            )
            .o(
                co_21(d).o(&d.company_name)
                    .x(lk_21(d).o((&d.movielink_type).o(&d.linktype_link)))
                    .x(&d.movie_title)
            )
    );
    let mut m: [Option<&'static str>; 3] = [None; 3];
    q.drive(|_, ((co, lk), title)| {
        update(&mut m[0], co);
        update(&mut m[1], lk);
        update(&mut m[2], title);
    });
    fmt3(m)
}

fn q23a(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_complete_cast).o(
            (&d.completecast_status).o(&d.compcasttype_kind).eq("complete+verified")
        ).k()
            .and(
                (&d.movie_company).o((&d.company_country).eq("[us]")).k()
                    .and(
                        (&d.movie_info).in_s(
                            (&d.info_type).o(&d.infotype_info).eq("release dates").k()
                                .and(
                                    (&d.info_note).rx(r"internet").k()
                                        .and(
                                            (&d.info_info).rx(r"^USA:.* 199").k()
                                                .or((&d.info_info).rx(r"^USA:.* 200").k())
                                        )
                                )
                        ).k()
                            .and(
                                k_23ab(d).k()
                                    .and(
                                        (&d.movie_keyword).k()
                                            .and((&d.movie_production_year).gt(2000).k())
                                    )
                            )
                    )
            )
            .o(k_23ab(d).x(&d.movie_title))
    );
    let mut m: [Option<&'static str>; 2] = [None; 2];
    q.drive(|_, (k, t)| {
        update(&mut m[0], k);
        update(&mut m[1], t);
    });
    fmt2(m)
}

fn q23b(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_complete_cast).o(
            (&d.completecast_status).o(&d.compcasttype_kind).eq("complete+verified")
        ).k()
            .and(
                (&d.movie_company).o((&d.company_country).eq("[us]")).k()
                    .and(
                        (&d.movie_info).in_s(
                            (&d.info_type).o(&d.infotype_info).eq("release dates").k()
                                .and(
                                    (&d.info_note).rx(r"internet").k()
                                        .and((&d.info_info).rx(r"^USA:.* 200").k())
                                )
                        ).k()
                            .and(
                                k_23ab(d).k()
                                    .and(
                                        (&d.movie_keyword).o(&d.keyword_keyword)
                                            .in_v(vec!["nerd", "loner", "alienation", "dignity"]).k()
                                            .and((&d.movie_production_year).gt(2000).k())
                                    )
                            )
                    )
            )
            .o(k_23ab(d).x(&d.movie_title))
    );
    let mut m: [Option<&'static str>; 2] = [None; 2];
    q.drive(|_, (k, t)| {
        update(&mut m[0], k);
        update(&mut m[1], t);
    });
    fmt2(m)
}

fn q23c(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_complete_cast).o(
            (&d.completecast_status).o(&d.compcasttype_kind).eq("complete+verified")
        ).k()
            .and(
                (&d.movie_company).o((&d.company_country).eq("[us]")).k()
                    .and(
                        (&d.movie_info).in_s(
                            (&d.info_type).o(&d.infotype_info).eq("release dates").k()
                                .and(
                                    (&d.info_note).rx(r"internet").k()
                                        .and(
                                            (&d.info_info).rx(r"^USA:.* 199").k()
                                                .or((&d.info_info).rx(r"^USA:.* 200").k())
                                        )
                                )
                        ).k()
                            .and(
                                k_23c(d).k()
                                    .and(
                                        (&d.movie_keyword).k()
                                            .and((&d.movie_production_year).gt(1990).k())
                                    )
                            )
                    )
            )
            .o(k_23c(d).x(&d.movie_title))
    );
    let mut m: [Option<&'static str>; 2] = [None; 2];
    q.drive(|_, (k, t)| {
        update(&mut m[0], k);
        update(&mut m[1], t);
    });
    fmt2(m)
}

fn q24a(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_company).o((&d.company_country).eq("[us]")).k()
            .and(
                (&d.movie_info).in_s(
                    (&d.info_type).o(&d.infotype_info).eq("release dates").k()
                        .and(
                            (&d.info_info).rx(r"^Japan:.*201").k()
                                .or((&d.info_info).rx(r"^USA:.*201").k())
                        )
                ).k()
                    .and(
                        (&d.movie_keyword).o(&d.keyword_keyword)
                            .in_v(vec!["hero", "martial-arts", "hand-to-hand-combat"]).k()
                            .and((&d.movie_production_year).gt(2010).k())
                    )
            )
            .o(
                (&d.movie_cast).o(
                    (&d.cast_note).in_v(voice4()).k()
                        .and(
                            (&d.cast_role).o(&d.roletype_role).eq("actress").k()
                                .and((&d.cast_person).in_s(
                                    (&d.person_gender).eq("f").k()
                                        .and(
                                            (&d.person_name).rx(r"An").k()
                                                .and((&d.person_aka).k())
                                        )
                                ).k())
                        )
                        .o(
                            (&d.cast_character).o(&d.character_name)
                                .x((&d.cast_person).o(&d.person_name))
                        )
                )
                    .x(&d.movie_title)
            )
    );
    let mut m: [Option<&'static str>; 3] = [None; 3];
    q.drive(|_, ((ch, pn), t)| {
        update(&mut m[0], ch);
        update(&mut m[1], pn);
        update(&mut m[2], t);
    });
    fmt3(m)
}

fn q24b(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_company).in_s(
            (&d.company_country).eq("[us]").k()
                .and((&d.company_name).eq("DreamWorks Animation").k())
        ).k()
            .and(
                (&d.movie_info).in_s(
                    (&d.info_type).o(&d.infotype_info).eq("release dates").k()
                        .and(
                            (&d.info_info).rx(r"^Japan:.*201").k()
                                .or((&d.info_info).rx(r"^USA:.*201").k())
                        )
                ).k()
                    .and(
                        (&d.movie_keyword).o(&d.keyword_keyword)
                            .in_v(vec!["hero", "martial-arts", "hand-to-hand-combat", "computer-animated-movie"]).k()
                            .and(
                                (&d.movie_production_year).gt(2010).k()
                                    .and((&d.movie_title).rx(r"^Kung Fu Panda").k())
                            )
                    )
            )
            .o(
                (&d.movie_cast).o(
                    (&d.cast_note).in_v(voice4()).k()
                        .and(
                            (&d.cast_role).o(&d.roletype_role).eq("actress").k()
                                .and((&d.cast_person).in_s(
                                    (&d.person_gender).eq("f").k()
                                        .and(
                                            (&d.person_name).rx(r"An").k()
                                                .and((&d.person_aka).k())
                                        )
                                ).k())
                        )
                        .o(
                            (&d.cast_character).o(&d.character_name)
                                .x((&d.cast_person).o(&d.person_name))
                        )
                )
                    .x(&d.movie_title)
            )
    );
    let mut m: [Option<&'static str>; 3] = [None; 3];
    q.drive(|_, ((ch, pn), t)| {
        update(&mut m[0], ch);
        update(&mut m[1], pn);
        update(&mut m[2], t);
    });
    fmt3(m)
}

fn q25a(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_info).in_s(gf_25ab(d)).k()
            .and(
                (&d.movie_keyword).o(&d.keyword_keyword)
                    .in_v(vec!["murder", "blood", "gore", "death", "female-nudity"]).k()
            )
            .o(
                (&d.movie_info).o(gf_25ab(d).o(&d.info_info))
                    .x((&d.movie_data).o(
                        (&d.data_type).o(&d.infotype_info).eq("votes").k()
                            .o(&d.data_data)
                    ))
                    .x(&d.movie_title)
                    .x((&d.movie_cast).o(
                        (&d.cast_note).in_v(writer5()).k()
                            .and((&d.cast_person).o((&d.person_gender).eq("m")).k())
                            .o((&d.cast_person).o(&d.person_name))
                    ))
            )
    );
    let mut m: [Option<&'static str>; 4] = [None; 4];
    q.drive(|_, (((info, votes), title), name)| {
        update(&mut m[0], info);
        update(&mut m[1], votes);
        update(&mut m[2], title);
        update(&mut m[3], name);
    });
    fmt4(m)
}

fn q25b(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_info).in_s(gf_25ab(d)).k()
            .and(
                (&d.movie_keyword).o(&d.keyword_keyword)
                    .in_v(vec!["murder", "blood", "gore", "death", "female-nudity"]).k()
                    .and(
                        (&d.movie_production_year).gt(2010).k()
                            .and((&d.movie_title).rx(r"^Vampire").k())
                    )
            )
            .o(
                (&d.movie_info).o(gf_25ab(d).o(&d.info_info))
                    .x((&d.movie_data).o(
                        (&d.data_type).o(&d.infotype_info).eq("votes").k()
                            .o(&d.data_data)
                    ))
                    .x(&d.movie_title)
                    .x((&d.movie_cast).o(
                        (&d.cast_note).in_v(writer5()).k()
                            .and((&d.cast_person).o((&d.person_gender).eq("m")).k())
                            .o((&d.cast_person).o(&d.person_name))
                    ))
            )
    );
    let mut m: [Option<&'static str>; 4] = [None; 4];
    q.drive(|_, (((info, votes), title), name)| {
        update(&mut m[0], info);
        update(&mut m[1], votes);
        update(&mut m[2], title);
        update(&mut m[3], name);
    });
    fmt4(m)
}

fn q25c(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_info).in_s(gf_25c(d)).k()
            .and((&d.movie_keyword).o(&d.keyword_keyword).in_v(kw7()).k())
            .o(
                (&d.movie_info).o(gf_25c(d).o(&d.info_info))
                    .x((&d.movie_data).o(
                        (&d.data_type).o(&d.infotype_info).eq("votes").k()
                            .o(&d.data_data)
                    ))
                    .x(&d.movie_title)
                    .x((&d.movie_cast).o(
                        (&d.cast_note).in_v(writer5()).k()
                            .and((&d.cast_person).o((&d.person_gender).eq("m")).k())
                            .o((&d.cast_person).o(&d.person_name))
                    ))
            )
    );
    let mut m: [Option<&'static str>; 4] = [None; 4];
    q.drive(|_, (((info, votes), title), name)| {
        update(&mut m[0], info);
        update(&mut m[1], votes);
        update(&mut m[2], title);
        update(&mut m[3], name);
    });
    fmt4(m)
}

fn q26a(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_complete_cast).in_s(
            (&d.completecast_subject).o(&d.compcasttype_kind).eq("cast").k()
                .and((&d.completecast_status).o(&d.compcasttype_kind).rx(r"complete").k())
        ).k()
            .and(
                (&d.movie_keyword).o(&d.keyword_keyword).in_v(kw10()).k()
                    .and(
                        (&d.movie_kind).o(&d.kind_kind).eq("movie").k()
                            .and((&d.movie_production_year).gt(2000).k())
                    )
            )
            .o(
                (&d.movie_cast).o(
                    (&d.cast_character).o((&d.character_name).rx(r"[Mm]an")).k()
                        .o(
                            (&d.cast_character).o(&d.character_name)
                                .x((&d.cast_person).o(&d.person_name))
                        )
                )
                    .x((&d.movie_data).o(
                        (&d.data_type).o(&d.infotype_info).eq("rating").k()
                            .and((&d.data_data).gt("7.0").k())
                            .o(&d.data_data)
                    ))
                    .x(&d.movie_title)
            )
    );
    let mut m: [Option<&'static str>; 4] = [None; 4];
    q.drive(|_, (((ch, pn), rd), t)| {
        update(&mut m[0], ch);
        update(&mut m[1], pn);
        update(&mut m[2], rd);
        update(&mut m[3], t);
    });
    fmt4(m)
}

fn q26b(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_complete_cast).in_s(
            (&d.completecast_subject).o(&d.compcasttype_kind).eq("cast").k()
                .and((&d.completecast_status).o(&d.compcasttype_kind).rx(r"complete").k())
        ).k()
            .and(
                (&d.movie_keyword).o(&d.keyword_keyword)
                    .in_v(vec!["superhero", "marvel-comics", "based-on-comic", "fight"]).k()
                    .and(
                        (&d.movie_kind).o(&d.kind_kind).eq("movie").k()
                            .and((&d.movie_production_year).gt(2005).k())
                    )
            )
            .o(
                (&d.movie_cast).o(
                    (&d.cast_character).o((&d.character_name).rx(r"[Mm]an")).k()
                        .o((&d.cast_character).o(&d.character_name))
                )
                    .x((&d.movie_data).o(
                        (&d.data_type).o(&d.infotype_info).eq("rating").k()
                            .and((&d.data_data).gt("8.0").k())
                            .o(&d.data_data)
                    ))
                    .x(&d.movie_title)
            )
    );
    let mut m: [Option<&'static str>; 3] = [None; 3];
    q.drive(|_, ((ch, rd), t)| {
        update(&mut m[0], ch);
        update(&mut m[1], rd);
        update(&mut m[2], t);
    });
    fmt3(m)
}

fn q26c(d: &Data) -> String {
    let rd = (&d.movie_data).o(
        (&d.data_type).o(&d.infotype_info).eq("rating").k().o(&d.data_data)
    );
    let q = d.movie.o(
        (&d.movie_complete_cast).in_s(
            (&d.completecast_subject).o(&d.compcasttype_kind).eq("cast").k()
                .and((&d.completecast_status).o(&d.compcasttype_kind).rx(r"complete").k())
        ).k()
            .and(
                (&d.movie_keyword).o(&d.keyword_keyword).in_v(kw10()).k()
                    .and(
                        (&d.movie_kind).o(&d.kind_kind).eq("movie").k()
                            .and((&d.movie_production_year).gt(2000).k())
                    )
            )
            .o(
                (&d.movie_cast).o(
                    (&d.cast_character).o((&d.character_name).rx(r"[Mm]an")).k()
                        .o((&d.cast_character).o(&d.character_name))
                )
                    .x(rd)
                    .x(&d.movie_title)
            )
    );
    let mut m: [Option<&'static str>; 3] = [None; 3];
    q.drive(|_, ((ch, rd), t)| {
        update(&mut m[0], ch);
        update(&mut m[1], rd);
        update(&mut m[2], t);
    });
    fmt3(m)
}
