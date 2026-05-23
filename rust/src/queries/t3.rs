// queries: 7a-c, 8a-d, 9a-d, 10a-c (queries.jl lines 591-753)
use crate::data::Data;
use crate::engine::*;
use super::helpers::*;
use super::sets::*;

pub const ENTRIES: &[(&str, &str, fn(&Data) -> String)] = &[
    ("7a",  "Antonioni, Michelangelo || Dressed to Kill",                                 q7a),
    ("7b",  "De Palma, Brian || Dressed to Kill",                                         q7b),
    ("7c",  "50 Cent || \"Boo\" Arnold was born Earl Arnold in Hattiesburg, Mississippi in 1966. His father gave him the nickname 'Boo' early in life and it stuck through grade school, high school, and college. He is still known as \"Boo\" to family and friends.  Raised in central Texas, Arnold played baseball at Texas Tech University where he graduated with a BA in Advertising and Marketing. While at Texas Tech he was also a member of the Texas Epsilon chapter of Phi Delta Theta fraternity. After college he worked with Young Life, an outreach to high school students, in San Antonio, Texas.  While with Young Life Arnold began taking extension courses through Fuller Theological Seminary and ultimately went full-time to Gordon-Conwell Theological Seminary in Boston, Massachusetts. At Gordon-Conwell he completed a Master's Degree in Divinity studying Theology, Philosophy, Church History, Biblical Languages (Hebrew & Greek), and Exegetical Methods. Following seminary he was involved with reconciliation efforts in the former Yugoslavia shortly after the war ended there in1995.  Arnold started acting in his early thirties in Texas. After an encouraging visit to Los Angeles where he spent time with childhood friend George Eads (of CSI Las Vegas) he decided to move to Los Angeles in 2001 to pursue acting full-time. While in Los Angeles he has studied acting with Judith Weston at Judith Weston Studio for Actors and Directors.  Arnold's acting career has been one of steady development, booking co-star and guest-star roles in nighttime television. He guest-starred opposite of Jane Seymour on the night time television drama Justice. He played the lead, Michael Hollister, in the film The Seer, written and directed by Patrick Masset (Friday Night Lights).  He was nominated Best Actor in the168 Film Festival for the role of Phil Stevens in the short-film Useless. In Useless he played a US Marshal who must choose between mercy and justice as he confronts the man who murdered his father. Arnold's performance in Useless confirmed his ability to carry lead roles, and he continues to work toward solidifying himself as a male lead in film and television.  Arnold married fellow Texan Stacy Rudd of San Antonio in 2003 and they are now raising their three children in the Los Angeles area.", q7c),
    ("8a",  "Chambers, Linda || .hack//Quantum",                                          q8a),
    ("8b",  "Chambers, Linda || Dragon Ball Z: Shin Budokai",                             q8b),
    ("8c",  "\"A.J.\" || #1 Cheerleader Camp",                                            q8c),
    ("8d",  "\"Jenny from the Block\" || #1 Cheerleader Camp",                            q8d),
    ("9a",  "AJ || Airport Announcer || Blue Harvest",                                    q9a),
    ("9b",  "AJ || Airport Announcer || Bassett, Angela || Blue Harvest",                 q9b),
    ("9c",  "'Annette' || 2nd Balladeer || Alborg, Ana Esther || (1975-01-20)",           q9c),
    ("9d",  "!!!, Toy || Aaron, Caroline || \"Cockamamie's\" Salesgirl || $15,000.00 Error", q9d),
    ("10a", "Actor || 12 Rounds",                                                         q10a),
    ("10b", "(empty)",                                                                    q10b),
    ("10c", "Himself || Evil Eyes: Behind the Scenes",                                    q10c),
];

fn q7a(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_production_year).ge(1980).k()
            .and((&d.movie_production_year).le(1995).k())
            .and((&d.movie_linked_by).o(
                (&d.movielink_type).o(&d.linktype_link).eq("features")).k())
            .o(
                (&d.movie_cast).o(
                    (&d.cast_person).in_s(
                        (&d.person_aka).o(&d.akaname_name).rx(r"a").k()
                            .and((&d.person_name_pcode).ge("A").k())
                            .and((&d.person_name_pcode).le("F").k())
                            .and(
                                (&d.person_gender).eq("m").k()
                                    .or(
                                        (&d.person_gender).eq("f").k()
                                            .and((&d.person_name).rx(r"^B").k())
                                    )
                            )
                            .and((&d.person_info).in_s(
                                (&d.personinfo_type).o(&d.infotype_info).eq("mini biography").k()
                                    .and((&d.personinfo_note).eq("Volker Boehm").k())
                            ).k())
                    ).k().o((&d.cast_person).o(&d.person_name))
                ).x(&d.movie_title)
            )
    );
    let mut m: [Option<&'static str>; 2] = [None; 2];
    q.drive(|_, (name, title)| {
        update(&mut m[0], name);
        update(&mut m[1], title);
    });
    fmt2(m)
}

fn q7b(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_production_year).ge(1980).k()
            .and((&d.movie_production_year).le(1984).k())
            .and((&d.movie_linked_by).o(
                (&d.movielink_type).o(&d.linktype_link).eq("features")).k())
            .o(
                (&d.movie_cast).o(
                    (&d.cast_person).in_s(
                        (&d.person_aka).o(&d.akaname_name).rx(r"a").k()
                            .and((&d.person_name_pcode).rx(r"^D").k())
                            .and((&d.person_gender).eq("m").k())
                            .and((&d.person_info).in_s(
                                (&d.personinfo_type).o(&d.infotype_info).eq("mini biography").k()
                                    .and((&d.personinfo_note).eq("Volker Boehm").k())
                            ).k())
                    ).k().o((&d.cast_person).o(&d.person_name))
                ).x(&d.movie_title)
            )
    );
    let mut m: [Option<&'static str>; 2] = [None; 2];
    q.drive(|_, (name, title)| {
        update(&mut m[0], name);
        update(&mut m[1], title);
    });
    fmt2(m)
}

fn bio_filter_7c<'d>(d: &'d Data) -> impl SetQ + 'd {
    (&d.personinfo_type).o(&d.infotype_info).eq("mini biography").k()
        .and((&d.personinfo_note).k())
}

fn q7c(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_production_year).ge(1980).k()
            .and((&d.movie_production_year).le(2010).k())
            .and((&d.movie_linked_by).o(
                (&d.movielink_type).o(&d.linktype_link).in_v(
                    vec!["references","referenced in","features","featured in"])).k())
            .o(
                (&d.movie_cast).o(
                    (&d.cast_person).in_s(
                        (&d.person_aka).o(&d.akaname_name).rx(r"a|^A").k()
                            .and((&d.person_name_pcode).ge("A").k())
                            .and((&d.person_name_pcode).le("F").k())
                            .and(
                                (&d.person_gender).eq("m").k()
                                    .or(
                                        (&d.person_gender).eq("f").k()
                                            .and((&d.person_name).rx(r"^A").k())
                                    )
                            )
                            .and((&d.person_info).in_s(bio_filter_7c(d)).k())
                    ).k().o(
                        (&d.cast_person).o(&d.person_name)
                            .x((&d.cast_person).o(
                                (&d.person_info).o(
                                    bio_filter_7c(d).o(&d.personinfo_info))))
                    )
                )
            )
    );
    let mut m: [Option<&'static str>; 2] = [None; 2];
    q.drive(|_, (name, bio)| {
        update(&mut m[0], name);
        update(&mut m[1], bio);
    });
    fmt2(m)
}

fn q8a(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_company).in_s(
            (&d.company_country).eq("[jp]").k()
                .and((&d.company_note).rx(r"\(Japan\)").k())
                .and((&d.company_note).nrx(r"\(USA\)").k())
        ).k()
            .o(
                (&d.movie_cast).o(
                    (&d.cast_note).eq("(voice: English version)").k()
                        .and((&d.cast_role).o(&d.roletype_role).eq("actress").k())
                        .and((&d.cast_person).in_s(
                            (&d.person_name).rx(r"Yo").k()
                                .and((&d.person_name).nrx(r"Yu").k())
                        ).k())
                        .o((&d.cast_person).o((&d.person_aka).o(&d.akaname_name)))
                ).x(&d.movie_title)
            )
    );
    let mut m: [Option<&'static str>; 2] = [None; 2];
    q.drive(|_, (name, title)| {
        update(&mut m[0], name);
        update(&mut m[1], title);
    });
    fmt2(m)
}

fn q8b(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_company).in_s(
            (&d.company_country).eq("[jp]").k()
                .and((&d.company_note).rx(r"\(Japan\)").k())
                .and((&d.company_note).nrx(r"\(USA\)").k())
                .and(
                    (&d.company_note).rx(r"\(2006\)").k()
                        .or((&d.company_note).rx(r"\(2007\)").k())
                )
        ).k()
            .and((&d.movie_production_year).ge(2006).k())
            .and((&d.movie_production_year).le(2007).k())
            .and(
                (&d.movie_title).rx(r"^One Piece").k()
                    .or((&d.movie_title).rx(r"^Dragon Ball Z").k())
            )
            .o(
                (&d.movie_cast).o(
                    (&d.cast_note).eq("(voice: English version)").k()
                        .and((&d.cast_role).o(&d.roletype_role).eq("actress").k())
                        .and((&d.cast_person).in_s(
                            (&d.person_name).rx(r"Yo").k()
                                .and((&d.person_name).nrx(r"Yu").k())
                        ).k())
                        .o((&d.cast_person).o((&d.person_aka).o(&d.akaname_name)))
                ).x(&d.movie_title)
            )
    );
    let mut m: [Option<&'static str>; 2] = [None; 2];
    q.drive(|_, (name, title)| {
        update(&mut m[0], name);
        update(&mut m[1], title);
    });
    fmt2(m)
}

fn q8c(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_company).o((&d.company_country).eq("[us]")).k()
            .o(
                (&d.movie_cast).o(
                    (&d.cast_role).o(&d.roletype_role).eq("writer").k()
                        .o((&d.cast_person).o((&d.person_aka).o(&d.akaname_name)))
                ).x(&d.movie_title)
            )
    );
    let mut m: [Option<&'static str>; 2] = [None; 2];
    q.drive(|_, (name, title)| {
        update(&mut m[0], name);
        update(&mut m[1], title);
    });
    fmt2(m)
}

fn q8d(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_company).o((&d.company_country).eq("[us]")).k()
            .o(
                (&d.movie_cast).o(
                    (&d.cast_role).o(&d.roletype_role).eq("costume designer").k()
                        .o((&d.cast_person).o((&d.person_aka).o(&d.akaname_name)))
                ).x(&d.movie_title)
            )
    );
    let mut m: [Option<&'static str>; 2] = [None; 2];
    q.drive(|_, (name, title)| {
        update(&mut m[0], name);
        update(&mut m[1], title);
    });
    fmt2(m)
}

fn q9a(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_company).in_s(
            (&d.company_country).eq("[us]").k()
                .and(
                    (&d.company_note).rx(r"\(USA\)").k()
                        .or((&d.company_note).rx(r"\(worldwide\)").k())
                )
        ).k()
            .and((&d.movie_production_year).ge(2005).k())
            .and((&d.movie_production_year).le(2015).k())
            .o(
                (&d.movie_cast).o(
                    (&d.cast_note).in_v(voice4()).k()
                        .and((&d.cast_role).o(&d.roletype_role).eq("actress").k())
                        .and((&d.cast_person).in_s(
                            (&d.person_gender).eq("f").k()
                                .and((&d.person_name).rx(r"Ang").k())
                        ).k())
                        .o(
                            (&d.cast_person).o((&d.person_aka).o(&d.akaname_name))
                                .x((&d.cast_character).o(&d.character_name))
                        )
                ).x(&d.movie_title)
            )
    );
    let mut m: [Option<&'static str>; 3] = [None; 3];
    q.drive(|_, ((aka, ch), title)| {
        update(&mut m[0], aka);
        update(&mut m[1], ch);
        update(&mut m[2], title);
    });
    fmt3(m)
}

fn q9b(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_company).in_s(
            (&d.company_country).eq("[us]").k()
                .and((&d.company_note).rx(r"\(200.*\)").k())
                .and(
                    (&d.company_note).rx(r"\(USA\)").k()
                        .or((&d.company_note).rx(r"\(worldwide\)").k())
                )
        ).k()
            .and((&d.movie_production_year).ge(2007).k())
            .and((&d.movie_production_year).le(2010).k())
            .o(
                (&d.movie_cast).o(
                    (&d.cast_note).eq("(voice)").k()
                        .and((&d.cast_role).o(&d.roletype_role).eq("actress").k())
                        .and((&d.cast_person).in_s(
                            (&d.person_gender).eq("f").k()
                                .and((&d.person_name).rx(r"Angel").k())
                        ).k())
                        .o(
                            (&d.cast_person).o((&d.person_aka).o(&d.akaname_name))
                                .x((&d.cast_character).o(&d.character_name))
                                .x((&d.cast_person).o(&d.person_name))
                        )
                ).x(&d.movie_title)
            )
    );
    let mut m: [Option<&'static str>; 4] = [None; 4];
    q.drive(|_, (((aka, ch), name), title)| {
        update(&mut m[0], aka);
        update(&mut m[1], ch);
        update(&mut m[2], name);
        update(&mut m[3], title);
    });
    fmt4(m)
}

fn q9c(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_company).o((&d.company_country).eq("[us]")).k()
            .o(
                (&d.movie_cast).o(
                    (&d.cast_note).in_v(voice4()).k()
                        .and((&d.cast_role).o(&d.roletype_role).eq("actress").k())
                        .and((&d.cast_person).in_s(
                            (&d.person_gender).eq("f").k()
                                .and((&d.person_name).rx(r"An").k())
                        ).k())
                        .o(
                            (&d.cast_person).o((&d.person_aka).o(&d.akaname_name))
                                .x((&d.cast_character).o(&d.character_name))
                                .x((&d.cast_person).o(&d.person_name))
                        )
                ).x(&d.movie_title)
            )
    );
    let mut m: [Option<&'static str>; 4] = [None; 4];
    q.drive(|_, (((aka, ch), name), title)| {
        update(&mut m[0], aka);
        update(&mut m[1], ch);
        update(&mut m[2], name);
        update(&mut m[3], title);
    });
    fmt4(m)
}

fn q9d(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_company).o((&d.company_country).eq("[us]")).k()
            .o(
                (&d.movie_cast).o(
                    (&d.cast_note).in_v(voice4()).k()
                        .and((&d.cast_role).o(&d.roletype_role).eq("actress").k())
                        .and((&d.cast_person).o((&d.person_gender).eq("f")).k())
                        .o(
                            (&d.cast_person).o((&d.person_aka).o(&d.akaname_name))
                                .x((&d.cast_person).o(&d.person_name))
                                .x((&d.cast_character).o(&d.character_name))
                        )
                ).x(&d.movie_title)
            )
    );
    let mut m: [Option<&'static str>; 4] = [None; 4];
    q.drive(|_, (((aka, name), ch), title)| {
        update(&mut m[0], aka);
        update(&mut m[1], name);
        update(&mut m[2], ch);
        update(&mut m[3], title);
    });
    fmt4(m)
}

fn q10a(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_company).o((&d.company_country).eq("[ru]")).k()
            .and((&d.movie_production_year).gt(2005).k())
            .o(
                (&d.movie_cast).o(
                    (&d.cast_note).rx(r"\(voice\)").k()
                        .and((&d.cast_note).rx(r"\(uncredited\)").k())
                        .and((&d.cast_role).o(&d.roletype_role).eq("actor").k())
                        .o((&d.cast_character).o(&d.character_name))
                ).x(&d.movie_title)
            )
    );
    let mut m: [Option<&'static str>; 2] = [None; 2];
    q.drive(|_, (name, title)| {
        update(&mut m[0], name);
        update(&mut m[1], title);
    });
    fmt2(m)
}

fn q10b(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_company).o((&d.company_country).eq("[ru]")).k()
            .and((&d.movie_production_year).gt(2010).k())
            .o(
                (&d.movie_cast).o(
                    (&d.cast_note).rx(r"\(producer\)").k()
                        .and((&d.cast_role).o(&d.roletype_role).eq("actor").k())
                        .o((&d.cast_character).o(&d.character_name))
                ).x(&d.movie_title)
            )
    );
    let mut m: [Option<&'static str>; 2] = [None; 2];
    q.drive(|_, (name, title)| {
        update(&mut m[0], name);
        update(&mut m[1], title);
    });
    fmt2(m)
}

fn q10c(d: &Data) -> String {
    let q = d.movie.o(
        (&d.movie_company).o((&d.company_country).eq("[us]")).k()
            .and((&d.movie_production_year).gt(1990).k())
            .o(
                (&d.movie_cast).o(
                    (&d.cast_note).rx(r"\(producer\)").k()
                        .o((&d.cast_character).o(&d.character_name))
                ).x(&d.movie_title)
            )
    );
    let mut m: [Option<&'static str>; 2] = [None; 2];
    q.drive(|_, (name, title)| {
        update(&mut m[0], name);
        update(&mut m[1], title);
    });
    fmt2(m)
}
