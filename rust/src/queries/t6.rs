// queries: 27a–33c (queries.jl lines 1114–1394)
use crate::data::Data;
use crate::engine::*;
use super::helpers::*;
use super::sets::*;

fn co_28<'d>(d: &'d Data) -> impl Rel<R = usize, D = usize> + Drive + Probe + 'd {
    (&d.movie_company).in_s(
        (&d.company_country).ne("[us]").k()
            .and((&d.company_note).nrx(r"\(USA\)").k())
            .and((&d.company_note).rx(r"\(200.*\)").k())
    )
}

fn dt_28ac<'d>(d: &'d Data) -> impl Rel<R = usize, D = usize> + Drive + Probe + 'd {
    (&d.movie_data).in_s(
        (&d.data_type).o(&d.infotype_info).eq("rating").k()
            .and((&d.data_data).lt("8.5").k())
    )
}

fn dt_28b<'d>(d: &'d Data) -> impl Rel<R = usize, D = usize> + Drive + Probe + 'd {
    (&d.movie_data).in_s(
        (&d.data_type).o(&d.infotype_info).eq("rating").k()
            .and((&d.data_data).gt("6.5").k())
    )
}

fn gf_horror<'d>(d: &'d Data) -> impl KeySet<D = usize> + DriveKeys + Member + 'd {
    (&d.info_type).o(&d.infotype_info).eq("genres").k()
        .and((&d.info_info).in_v(vec!["Horror", "Thriller"]).k())
}

fn gf_genre6<'d>(d: &'d Data) -> impl KeySet<D = usize> + DriveKeys + Member + 'd {
    (&d.info_type).o(&d.infotype_info).eq("genres").k()
        .and((&d.info_info).in_v(genre6()).k())
}

fn qlink_33a<'d>(d: &'d Data) -> impl Rel<R = usize, D = usize> + Drive + Probe + 'd {
    (&d.movie_link).in_s(
        (&d.movielink_type).o(&d.linktype_link).in_v(link3()).k()
            .and((&d.movielink_target).in_s(
                (&d.movie_kind).o(&d.kind_kind).eq("tv series").k()
                    .and((&d.movie_company).k())
                    .and((&d.movie_data).in_s(
                        (&d.data_type).o(&d.infotype_info).eq("rating").k()
                            .and((&d.data_data).lt("3.0").k())
                    ).k())
                    .and((&d.movie_production_year).ge(2005).k())
                    .and((&d.movie_production_year).le(2008).k())
            ).k())
    )
}

fn qlink_33b<'d>(d: &'d Data) -> impl Rel<R = usize, D = usize> + Drive + Probe + 'd {
    (&d.movie_link).in_s(
        (&d.movielink_type).o(&d.linktype_link).rx(r"follow").k()
            .and((&d.movielink_target).in_s(
                (&d.movie_kind).o(&d.kind_kind).eq("tv series").k()
                    .and((&d.movie_company).k())
                    .and((&d.movie_data).in_s(
                        (&d.data_type).o(&d.infotype_info).eq("rating").k()
                            .and((&d.data_data).lt("3.0").k())
                    ).k())
                    .and((&d.movie_production_year).eq(2007).k())
            ).k())
    )
}

fn qlink_33c<'d>(d: &'d Data) -> impl Rel<R = usize, D = usize> + Drive + Probe + 'd {
    (&d.movie_link).in_s(
        (&d.movielink_type).o(&d.linktype_link).in_v(link3()).k()
            .and((&d.movielink_target).in_s(
                (&d.movie_kind).o(&d.kind_kind).in_v(vec!["tv series", "episode"]).k()
                    .and((&d.movie_company).k())
                    .and((&d.movie_data).in_s(
                        (&d.data_type).o(&d.infotype_info).eq("rating").k()
                            .and((&d.data_data).lt("3.5").k())
                    ).k())
                    .and((&d.movie_production_year).ge(2000).k())
                    .and((&d.movie_production_year).le(2010).k())
            ).k())
    )
}

pub const ENTRIES: &[super::Entry] = &[
    ("27a", "Det Danske Filminstitut || followed by || Spår i mörker", q27a),
    ("27b", "Filmlance International AB || followed by || Vita nätter", q27b),
    ("27c", "Det Danske Filminstitut || followed by || Spår i mörker", q27c),
    ("28a", "01 Distribuzione || 2.9 || (#1.1)", q28a),
    ("28b", "20th Century Fox || 6.6 || (#1.1)", q28b),
    ("28c", "01 Distribuzione || 1.9 || (#1.1)", q28c),
    ("29a", "Queen || Andrews, Julie || Shrek 2", q29a),
    ("29b", "Queen || Andrews, Julie || Shrek 2", q29b),
    ("29c", "Lola || Andrews, Julie || Hoodwinked!", q29c),
    ("30a", "Horror || 100356 || 16 Blocks || Abrams, J.J.", q30a),
    ("30b", "Horror || 194782 || Freddy vs. Jason || Shannon, Damian", q30b),
    ("30c", "Action || 100356 || $ || Abernathy, Lewis", q30c),
    ("31a", "Horror || 1040 || 2001 Maniacs || Agnew, Jim", q31a),
    ("31b", "Horror || 129755 || Saw || Bousman, Darren Lynn", q31b),
    ("31c", "Action || 1008 || 11:14 || Abraham, Brad", q31c),
    ("32a", "(empty)", q32a),
    ("32b", "alternate language version of || 12 oz. Mouse || 'Angel': Season 2 Overview", q32b),
    ("33a", "495 Productions || 495 Productions || 3.3 || 2.7 || A Double Shot at Love || A Shot at Love with Tila Tequila", q33a),
    ("33b", "MTV Netherlands || 495 Productions || 3.3 || 2.7 || A Double Shot at Love || A Shot at Love with Tila Tequila", q33b),
    ("33c", "2BE || 495 Productions || 1.3 || 1.0 || A Double Shot at Love || A Double Shot at Love", q33c),
];

fn q27a(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_complete_cast).in_s(
            (&d.completecast_subject).o(&d.compcasttype_kind).in_v(vec!["cast", "crew"]).k()
                .and((&d.completecast_status).o(&d.compcasttype_kind).eq("complete").k())
        ).k()
            .and(film_or_warner_co(d).k())
            .and((&d.movie_keyword).o(&d.keyword_keyword).eq("sequel").k())
            .and(follow_link(d).k())
            .and((&d.movie_info).in_s((&d.info_info).in_v(vec!["Sweden", "Germany", "Swedish", "German"]).k()).k())
            .and((&d.movie_production_year).ge(1950).k())
            .and((&d.movie_production_year).le(2000).k())
            .o(
                film_or_warner_co(d).o(&d.company_name)
                    .x(follow_link(d).o((&d.movielink_type).o(&d.linktype_link)))
                    .x(&d.movie_title)
            )
    );
    min_row(q)
}

fn q27b(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_complete_cast).in_s(
            (&d.completecast_subject).o(&d.compcasttype_kind).in_v(vec!["cast", "crew"]).k()
                .and((&d.completecast_status).o(&d.compcasttype_kind).eq("complete").k())
        ).k()
            .and(film_or_warner_co(d).k())
            .and((&d.movie_keyword).o(&d.keyword_keyword).eq("sequel").k())
            .and(follow_link(d).k())
            .and((&d.movie_info).in_s((&d.info_info).in_v(vec!["Sweden", "Germany", "Swedish", "German"]).k()).k())
            .and((&d.movie_production_year).eq(1998).k())
            .o(
                film_or_warner_co(d).o(&d.company_name)
                    .x(follow_link(d).o((&d.movielink_type).o(&d.linktype_link)))
                    .x(&d.movie_title)
            )
    );
    min_row(q)
}

fn q27c(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_complete_cast).in_s(
            (&d.completecast_subject).o(&d.compcasttype_kind).eq("cast").k()
                .and((&d.completecast_status).o(&d.compcasttype_kind).rx(r"^complete").k())
        ).k()
            .and(film_or_warner_co(d).k())
            .and((&d.movie_keyword).o(&d.keyword_keyword).eq("sequel").k())
            .and(follow_link(d).k())
            .and((&d.movie_info).in_s((&d.info_info).in_v(nordic9()).k()).k())
            .and((&d.movie_production_year).ge(1950).k())
            .and((&d.movie_production_year).le(2010).k())
            .o(
                film_or_warner_co(d).o(&d.company_name)
                    .x(follow_link(d).o((&d.movielink_type).o(&d.linktype_link)))
                    .x(&d.movie_title)
            )
    );
    min_row(q)
}

fn q28a(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_complete_cast).in_s(
            (&d.completecast_subject).o(&d.compcasttype_kind).eq("crew").k()
                .and((&d.completecast_status).o(&d.compcasttype_kind).ne("complete+verified").k())
        ).k()
            .and(co_28(d).k())
            .and((&d.movie_info).in_s(
                (&d.info_type).o(&d.infotype_info).eq("countries").k()
                    .and((&d.info_info).in_v(nordic10()).k())
            ).k())
            .and(dt_28ac(d).k())
            .and((&d.movie_keyword).o(&d.keyword_keyword).in_v(murder4()).k())
            .and((&d.movie_kind).o(&d.kind_kind).in_v(vec!["movie", "episode"]).k())
            .and((&d.movie_production_year).gt(2000).k())
            .o(
                co_28(d).o(&d.company_name)
                    .x(dt_28ac(d).o(&d.data_data))
                    .x(&d.movie_title)
            )
    );
    min_row(q)
}

fn q28b(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_complete_cast).in_s(
            (&d.completecast_subject).o(&d.compcasttype_kind).eq("crew").k()
                .and((&d.completecast_status).o(&d.compcasttype_kind).ne("complete+verified").k())
        ).k()
            .and(co_28(d).k())
            .and((&d.movie_info).in_s(
                (&d.info_type).o(&d.infotype_info).eq("countries").k()
                    .and((&d.info_info).in_v(vec!["Sweden", "Germany", "Swedish", "German"]).k())
            ).k())
            .and(dt_28b(d).k())
            .and((&d.movie_keyword).o(&d.keyword_keyword).in_v(murder4()).k())
            .and((&d.movie_kind).o(&d.kind_kind).in_v(vec!["movie", "episode"]).k())
            .and((&d.movie_production_year).gt(2005).k())
            .o(
                co_28(d).o(&d.company_name)
                    .x(dt_28b(d).o(&d.data_data))
                    .x(&d.movie_title)
            )
    );
    min_row(q)
}

fn q28c(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_complete_cast).in_s(
            (&d.completecast_subject).o(&d.compcasttype_kind).eq("cast").k()
                .and((&d.completecast_status).o(&d.compcasttype_kind).eq("complete").k())
        ).k()
            .and(co_28(d).k())
            .and((&d.movie_info).in_s(
                (&d.info_type).o(&d.infotype_info).eq("countries").k()
                    .and((&d.info_info).in_v(nordic10()).k())
            ).k())
            .and(dt_28ac(d).k())
            .and((&d.movie_keyword).o(&d.keyword_keyword).in_v(murder4()).k())
            .and((&d.movie_kind).o(&d.kind_kind).in_v(vec!["movie", "episode"]).k())
            .and((&d.movie_production_year).gt(2005).k())
            .o(
                co_28(d).o(&d.company_name)
                    .x(dt_28ac(d).o(&d.data_data))
                    .x(&d.movie_title)
            )
    );
    min_row(q)
}

fn q29a(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_complete_cast).in_s(
            (&d.completecast_subject).o(&d.compcasttype_kind).eq("cast").k()
                .and((&d.completecast_status).o(&d.compcasttype_kind).eq("complete+verified").k())
        ).k()
            .and((&d.movie_company).o((&d.company_country).eq("[us]")).k())
            .and((&d.movie_info).in_s(
                (&d.info_type).o(&d.infotype_info).eq("release dates").k()
                    .and(
                        (&d.info_info).rx(r"^Japan:.*200").k()
                            .or((&d.info_info).rx(r"^USA:.*200").k())
                    )
            ).k())
            .and((&d.movie_keyword).o(&d.keyword_keyword).eq("computer-animation").k())
            .and((&d.movie_title).eq("Shrek 2").k())
            .and((&d.movie_production_year).ge(2000).k())
            .and((&d.movie_production_year).le(2010).k())
            .o(
                (&d.movie_cast).o(
                    (&d.cast_note).in_v(voice3()).k()
                        .and((&d.cast_role).o(&d.roletype_role).eq("actress").k())
                        .and((&d.cast_character).o((&d.character_name).eq("Queen")).k())
                        .and((&d.cast_person).in_s(
                            (&d.person_gender).eq("f").k()
                                .and((&d.person_name).rx(r"An").k())
                                .and((&d.person_aka).k())
                                .and((&d.person_info).in_s((&d.personinfo_type).o(&d.infotype_info).eq("trivia").k()).k())
                        ).k())
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

fn q29b(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_complete_cast).in_s(
            (&d.completecast_subject).o(&d.compcasttype_kind).eq("cast").k()
                .and((&d.completecast_status).o(&d.compcasttype_kind).eq("complete+verified").k())
        ).k()
            .and((&d.movie_company).o((&d.company_country).eq("[us]")).k())
            .and((&d.movie_info).in_s(
                (&d.info_type).o(&d.infotype_info).eq("release dates").k()
                    .and((&d.info_info).rx(r"^USA:.*200").k())
            ).k())
            .and((&d.movie_keyword).o(&d.keyword_keyword).eq("computer-animation").k())
            .and((&d.movie_title).eq("Shrek 2").k())
            .and((&d.movie_production_year).ge(2000).k())
            .and((&d.movie_production_year).le(2005).k())
            .o(
                (&d.movie_cast).o(
                    (&d.cast_note).in_v(voice3()).k()
                        .and((&d.cast_role).o(&d.roletype_role).eq("actress").k())
                        .and((&d.cast_character).o((&d.character_name).eq("Queen")).k())
                        .and((&d.cast_person).in_s(
                            (&d.person_gender).eq("f").k()
                                .and((&d.person_name).rx(r"An").k())
                                .and((&d.person_aka).k())
                                .and((&d.person_info).in_s((&d.personinfo_type).o(&d.infotype_info).eq("height").k()).k())
                        ).k())
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

fn q29c(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_complete_cast).in_s(
            (&d.completecast_subject).o(&d.compcasttype_kind).eq("cast").k()
                .and((&d.completecast_status).o(&d.compcasttype_kind).eq("complete+verified").k())
        ).k()
            .and((&d.movie_company).o((&d.company_country).eq("[us]")).k())
            .and((&d.movie_info).in_s(
                (&d.info_type).o(&d.infotype_info).eq("release dates").k()
                    .and(
                        (&d.info_info).rx(r"^Japan:.*200").k()
                            .or((&d.info_info).rx(r"^USA:.*200").k())
                    )
            ).k())
            .and((&d.movie_keyword).o(&d.keyword_keyword).eq("computer-animation").k())
            .and((&d.movie_production_year).ge(2000).k())
            .and((&d.movie_production_year).le(2010).k())
            .o(
                (&d.movie_cast).o(
                    (&d.cast_note).in_v(voice4()).k()
                        .and((&d.cast_role).o(&d.roletype_role).eq("actress").k())
                        .and((&d.cast_person).in_s(
                            (&d.person_gender).eq("f").k()
                                .and((&d.person_name).rx(r"An").k())
                                .and((&d.person_aka).k())
                                .and((&d.person_info).in_s((&d.personinfo_type).o(&d.infotype_info).eq("trivia").k()).k())
                        ).k())
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

fn q30a(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_complete_cast).in_s(
            (&d.completecast_subject).o(&d.compcasttype_kind).in_v(vec!["cast", "crew"]).k()
                .and((&d.completecast_status).o(&d.compcasttype_kind).eq("complete+verified").k())
        ).k()
            .and((&d.movie_info).in_s(gf_horror(d)).k())
            .and((&d.movie_keyword).o(&d.keyword_keyword).in_v(kw7()).k())
            .and((&d.movie_production_year).gt(2000).k())
            .o(
                (&d.movie_info).o(gf_horror(d).o(&d.info_info))
                    .x((&d.movie_data).o((&d.data_type).o(&d.infotype_info).eq("votes").k().o(&d.data_data)))
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

fn q30b(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_complete_cast).in_s(
            (&d.completecast_subject).o(&d.compcasttype_kind).in_v(vec!["cast", "crew"]).k()
                .and((&d.completecast_status).o(&d.compcasttype_kind).eq("complete+verified").k())
        ).k()
            .and((&d.movie_info).in_s(gf_horror(d)).k())
            .and((&d.movie_keyword).o(&d.keyword_keyword).in_v(kw7()).k())
            .and((&d.movie_production_year).gt(2000).k())
            .and(
                (&d.movie_title).rx(r"Freddy").k()
                    .or(
                        (&d.movie_title).rx(r"Jason").k()
                            .or((&d.movie_title).rx(r"^Saw").k())
                    )
            )
            .o(
                (&d.movie_info).o(gf_horror(d).o(&d.info_info))
                    .x((&d.movie_data).o((&d.data_type).o(&d.infotype_info).eq("votes").k().o(&d.data_data)))
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

fn q30c(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_complete_cast).in_s(
            (&d.completecast_subject).o(&d.compcasttype_kind).eq("cast").k()
                .and((&d.completecast_status).o(&d.compcasttype_kind).eq("complete+verified").k())
        ).k()
            .and((&d.movie_info).in_s(gf_genre6(d)).k())
            .and((&d.movie_keyword).o(&d.keyword_keyword).in_v(kw7()).k())
            .o(
                (&d.movie_info).o(gf_genre6(d).o(&d.info_info))
                    .x((&d.movie_data).o((&d.data_type).o(&d.infotype_info).eq("votes").k().o(&d.data_data)))
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

fn q31a(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_company).o((&d.company_name).rx(r"^Lionsgate")).k()
            .and((&d.movie_info).in_s(gf_horror(d)).k())
            .and((&d.movie_keyword).o(&d.keyword_keyword).in_v(kw7()).k())
            .o(
                (&d.movie_info).o(gf_horror(d).o(&d.info_info))
                    .x((&d.movie_data).o((&d.data_type).o(&d.infotype_info).eq("votes").k().o(&d.data_data)))
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

fn q31b(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_company).in_s(
            (&d.company_name).rx(r"^Lionsgate").k()
                .and((&d.company_note).rx(r"\(Blu-ray\)").k())
        ).k()
            .and((&d.movie_info).in_s(gf_horror(d)).k())
            .and((&d.movie_keyword).o(&d.keyword_keyword).in_v(kw7()).k())
            .and((&d.movie_production_year).gt(2000).k())
            .and(
                (&d.movie_title).rx(r"Freddy").k()
                    .or(
                        (&d.movie_title).rx(r"Jason").k()
                            .or((&d.movie_title).rx(r"^Saw").k())
                    )
            )
            .o(
                (&d.movie_info).o(gf_horror(d).o(&d.info_info))
                    .x((&d.movie_data).o((&d.data_type).o(&d.infotype_info).eq("votes").k().o(&d.data_data)))
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

fn q31c(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_company).o((&d.company_name).rx(r"^Lionsgate")).k()
            .and((&d.movie_info).in_s(gf_genre6(d)).k())
            .and((&d.movie_keyword).o(&d.keyword_keyword).in_v(kw7()).k())
            .o(
                (&d.movie_info).o(gf_genre6(d).o(&d.info_info))
                    .x((&d.movie_data).o((&d.data_type).o(&d.infotype_info).eq("votes").k().o(&d.data_data)))
                    .x(&d.movie_title)
                    .x((&d.movie_cast).o(
                        (&d.cast_note).in_v(writer5()).k()
                            .o((&d.cast_person).o(&d.person_name))
                    ))
            )
    );
    min_row(q)
}

// q32a/q32b differ only in the keyword constant.
fn q32(d: &Data, kw: &'static str) -> String {
    let q = d.movie.o(
        (&d.movie_keyword).o(&d.keyword_keyword).eq(kw).k()
            .and((&d.movie_link).k())
            .o(
                (&d.movie_link).o((&d.movielink_type).o(&d.linktype_link))
                    .x(&d.movie_title)
                    .x((&d.movie_link).o((&d.movielink_target).o(&d.movie_title)))
            )
    );
    min_row(q)
}

fn q32a(d: &Data) -> String { q32(d, "10,000-mile-club") }
fn q32b(d: &Data) -> String { q32(d, "character-name-in-title") }

fn q33a(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_kind).o(&d.kind_kind).eq("tv series").k()
            .and((&d.movie_company).o((&d.company_country).eq("[us]")).k())
            .and(qlink_33a(d).k())
            .o(
                (&d.movie_company).o((&d.company_country).eq("[us]").k().o(&d.company_name))
                    .x(qlink_33a(d).o((&d.movielink_target).o((&d.movie_company).o(&d.company_name))))
                    .x((&d.movie_data).o((&d.data_type).o(&d.infotype_info).eq("rating").k().o(&d.data_data)))
                    .x(qlink_33a(d).o((&d.movielink_target).o((&d.movie_data).o(
                        (&d.data_type).o(&d.infotype_info).eq("rating").k()
                            .and((&d.data_data).lt("3.0").k())
                            .o(&d.data_data)
                    ))))
                    .x(&d.movie_title)
                    .x(qlink_33a(d).o((&d.movielink_target).o(&d.movie_title)))
            )
    );
    min_row(q)
}

fn q33b(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_kind).o(&d.kind_kind).eq("tv series").k()
            .and((&d.movie_company).o((&d.company_country).eq("[nl]")).k())
            .and(qlink_33b(d).k())
            .o(
                (&d.movie_company).o((&d.company_country).eq("[nl]").k().o(&d.company_name))
                    .x(qlink_33b(d).o((&d.movielink_target).o((&d.movie_company).o(&d.company_name))))
                    .x((&d.movie_data).o((&d.data_type).o(&d.infotype_info).eq("rating").k().o(&d.data_data)))
                    .x(qlink_33b(d).o((&d.movielink_target).o((&d.movie_data).o(
                        (&d.data_type).o(&d.infotype_info).eq("rating").k()
                            .and((&d.data_data).lt("3.0").k())
                            .o(&d.data_data)
                    ))))
                    .x(&d.movie_title)
                    .x(qlink_33b(d).o((&d.movielink_target).o(&d.movie_title)))
            )
    );
    min_row(q)
}

fn q33c(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_kind).o(&d.kind_kind).in_v(vec!["tv series", "episode"]).k()
            .and((&d.movie_company).o((&d.company_country).ne("[us]")).k())
            .and(qlink_33c(d).k())
            .o(
                (&d.movie_company).o((&d.company_country).ne("[us]").k().o(&d.company_name))
                    .x(qlink_33c(d).o((&d.movielink_target).o((&d.movie_company).o(&d.company_name))))
                    .x((&d.movie_data).o((&d.data_type).o(&d.infotype_info).eq("rating").k().o(&d.data_data)))
                    .x(qlink_33c(d).o((&d.movielink_target).o((&d.movie_data).o(
                        (&d.data_type).o(&d.infotype_info).eq("rating").k()
                            .and((&d.data_data).lt("3.5").k())
                            .o(&d.data_data)
                    ))))
                    .x(&d.movie_title)
                    .x(qlink_33c(d).o((&d.movielink_target).o(&d.movie_title)))
            )
    );
    min_row(q)
}
