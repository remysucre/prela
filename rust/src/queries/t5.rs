// queries: 19a-26c (queries.jl lines 859-1111)
use crate::data::Data;
use crate::engine::*;
use super::helpers::*;
use super::sets::*;

fn k_23ab<'d>(d: &'d Data) -> impl Rel<R = &'static str, D = i64> + Drive + Probe + 'd {
    (&d.movie_kind).o(&d.kind_kind).eq("movie")
}

fn k_23c<'d>(d: &'d Data) -> impl Rel<R = &'static str, D = i64> + Drive + Probe + 'd {
    (&d.movie_kind).o(&d.kind_kind)
        .in_v(vec!["movie", "tv movie", "video movie", "video game"])
}

fn gf_25ab<'d>(d: &'d Data) -> impl KeySet<D = i64> + DriveKeys + Member + 'd {
    (&d.info_type).o(&d.infotype_info).eq("genres").k()
        .and((&d.info_info).eq("Horror").k())
}

fn gf_25c<'d>(d: &'d Data) -> impl KeySet<D = i64> + DriveKeys + Member + 'd {
    (&d.info_type).o(&d.infotype_info).eq("genres").k()
        .and((&d.info_info).in_v(genre6()).k())
}

pub const ENTRIES: &[super::Entry] = &[
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
    min_row(q)
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
    min_row(q)
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
    min_row(q)
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
    min_row(q)
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
    min_row(q)
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
    min_row(q)
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
    min_row(q)
}

// q21a/b/c differ only in the country list and year range.
fn q21(d: &Data, countries: Vec<&'static str>, ylo: i64, yhi: i64) -> String {
    let q = d.movie.o(
        film_or_warner_co(d).k()
            .and(
                (&d.movie_keyword).o(&d.keyword_keyword).eq("sequel").k()
                    .and(
                        follow_link(d).k()
                            .and(
                                (&d.movie_info).o((&d.info_info).in_v(countries)).k()
                                    .and(
                                        (&d.movie_production_year).ge(ylo).k()
                                            .and((&d.movie_production_year).le(yhi).k())
                                    )
                            )
                    )
            )
            .o(
                film_or_warner_co(d).o(&d.company_name)
                    .x(follow_link(d).o((&d.movielink_type).o(&d.linktype_link)))
                    .x(&d.movie_title)
            )
    );
    min_row(q)
}

fn q21a(d: &Data) -> String { q21(d, nordic8(), 1950, 2000) }
fn q21b(d: &Data) -> String { q21(d, vec!["Germany", "German"], 2000, 2010) }
fn q21c(d: &Data) -> String { q21(d, nordic9(), 1950, 2010) }

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
    min_row(q)
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
    min_row(q)
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
    min_row(q)
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
    min_row(q)
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
    min_row(q)
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
    min_row(q)
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
    min_row(q)
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
    min_row(q)
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
    min_row(q)
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
    min_row(q)
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
    min_row(q)
}
