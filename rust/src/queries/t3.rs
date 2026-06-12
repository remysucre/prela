// queries: 7a-c, 8a-d, 9a-d, 10a-c (queries.jl lines 591-753)
use crate::data::Data;
use crate::engine::*;
use super::helpers::*;
use super::sets::*;

pub const ENTRIES: &[super::Entry] = &[
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
    let q = d.movie.in_s(
        (&d.movie_production_year).ge(1980)
            .and((&d.movie_production_year).le(1995))
            .and((&d.movie_linked_by).o(
                (&d.movielink_type).o(&d.linktype_link).eq("features")))
    ).o(
        (&d.movie_cast).o(
            (&d.cast_person).in_s(
                (&d.person_aka).o(&d.akaname_name).rx(r"a")
                    .and((&d.person_name_pcode).ge("A"))
                    .and((&d.person_name_pcode).le("F"))
                    .and(
                        (&d.person_gender).eq("m")
                            .or(
                                (&d.person_gender).eq("f")
                                    .and((&d.person_name).rx(r"^B"))
                            )
                    )
                    .and((&d.person_info).in_s(
                        (&d.personinfo_type).o(&d.infotype_info).eq("mini biography")
                            .and((&d.personinfo_note).eq("Volker Boehm"))
                    ))
            ).o(&d.person_name)
        ).x(&d.movie_title)
    );
    min_row(q)
}

fn q7b(d: &Data) -> String {
    let q = d.movie.in_s(
        (&d.movie_production_year).ge(1980)
            .and((&d.movie_production_year).le(1984))
            .and((&d.movie_linked_by).o(
                (&d.movielink_type).o(&d.linktype_link).eq("features")))
    ).o(
        (&d.movie_cast).o(
            (&d.cast_person).in_s(
                (&d.person_aka).o(&d.akaname_name).rx(r"a")
                    .and((&d.person_name_pcode).rx(r"^D"))
                    .and((&d.person_gender).eq("m"))
                    .and((&d.person_info).in_s(
                        (&d.personinfo_type).o(&d.infotype_info).eq("mini biography")
                            .and((&d.personinfo_note).eq("Volker Boehm"))
                    ))
            ).o(&d.person_name)
        ).x(&d.movie_title)
    );
    min_row(q)
}

// Conjunct tree (∧ = Prod) — consumed via `member` only, so the value
// type stays opaque (`impl Rel<D = usize> + Probe`).
fn bio_filter_7c<'d>(d: &'d Data) -> impl Rel<D = usize> + Probe + 'd {
    (&d.personinfo_type).o(&d.infotype_info).eq("mini biography")
        .and(&d.personinfo_note)
}

fn q7c(d: &Data) -> String {
    let q = d.movie.in_s(
        (&d.movie_production_year).ge(1980)
            .and((&d.movie_production_year).le(2010))
            .and((&d.movie_linked_by).o(
                (&d.movielink_type).o(&d.linktype_link).in_v(
                    vec!["references","referenced in","features","featured in"])))
    ).o(
        (&d.movie_cast).o(
            (&d.cast_person).in_s(
                (&d.person_aka).o(&d.akaname_name).rx(r"a|^A")
                    .and((&d.person_name_pcode).ge("A"))
                    .and((&d.person_name_pcode).le("F"))
                    .and(
                        (&d.person_gender).eq("m")
                            .or(
                                (&d.person_gender).eq("f")
                                    .and((&d.person_name).rx(r"^A"))
                            )
                    )
                    .and((&d.person_info).in_s(bio_filter_7c(d)))
            ).o(
                (&d.person_name)
                    .x((&d.person_info).in_s(bio_filter_7c(d))
                        .o(&d.personinfo_info))
            )
        )
    );
    min_row(q)
}

fn q8a(d: &Data) -> String {
    let q = d.movie
        .in_s((&d.movie_company).in_s(
            (&d.company_country).eq("[jp]")
                .and((&d.company_note).rx(r"\(Japan\)"))
                .and((&d.company_note).nrx(r"\(USA\)"))
        ))
            .o(
                (&d.movie_cast).in_s(
                    (&d.cast_note).eq("(voice: English version)")
                        .and((&d.cast_role).o(&d.roletype_role).eq("actress"))
                        .and((&d.cast_person).in_s(
                            (&d.person_name).rx(r"Yo")
                                .and((&d.person_name).nrx(r"Yu"))
                        ))
                ).o((&d.cast_person).o((&d.person_aka).o(&d.akaname_name)))
                .x(&d.movie_title)
            );
    min_row(q)
}

fn q8b(d: &Data) -> String {
    let q = d.movie.in_s(
        (&d.movie_company).in_s(
            (&d.company_country).eq("[jp]")
                .and((&d.company_note).rx(r"\(Japan\)"))
                .and((&d.company_note).nrx(r"\(USA\)"))
                .and(
                    (&d.company_note).rx(r"\(2006\)")
                        .or((&d.company_note).rx(r"\(2007\)"))
                )
        )
            .and((&d.movie_production_year).ge(2006))
            .and((&d.movie_production_year).le(2007))
            .and(
                (&d.movie_title).rx(r"^One Piece")
                    .or((&d.movie_title).rx(r"^Dragon Ball Z"))
            )
    ).o(
        (&d.movie_cast).in_s(
            (&d.cast_note).eq("(voice: English version)")
                .and((&d.cast_role).o(&d.roletype_role).eq("actress"))
                .and((&d.cast_person).in_s(
                    (&d.person_name).rx(r"Yo")
                        .and((&d.person_name).nrx(r"Yu"))
                ))
        ).o((&d.cast_person).o((&d.person_aka).o(&d.akaname_name)))
        .x(&d.movie_title)
    );
    min_row(q)
}

// q8c/q8d differ only in the cast role.
fn q8cd(d: &Data, role: &'static str) -> String {
    let q = d.movie
        .in_s((&d.movie_company).o((&d.company_country).eq("[us]")))
            .o(
                (&d.movie_cast).in_s((&d.cast_role).o(&d.roletype_role).eq(role))
                    .o((&d.cast_person).o((&d.person_aka).o(&d.akaname_name)))
                    .x(&d.movie_title)
            );
    min_row(q)
}

fn q8c(d: &Data) -> String { q8cd(d, "writer") }
fn q8d(d: &Data) -> String { q8cd(d, "costume designer") }

fn q9a(d: &Data) -> String {
    let q = d.movie.in_s(
        (&d.movie_company).in_s(
            (&d.company_country).eq("[us]")
                .and(
                    (&d.company_note).rx(r"\(USA\)")
                        .or((&d.company_note).rx(r"\(worldwide\)"))
                )
        )
            .and((&d.movie_production_year).ge(2005))
            .and((&d.movie_production_year).le(2015))
    ).o(
        (&d.movie_cast).in_s(
            (&d.cast_note).in_v(voice4())
                .and((&d.cast_role).o(&d.roletype_role).eq("actress"))
                .and((&d.cast_person).in_s(
                    (&d.person_gender).eq("f")
                        .and((&d.person_name).rx(r"Ang"))
                ))
        ).o(
            (&d.cast_person).o((&d.person_aka).o(&d.akaname_name))
                .x((&d.cast_character).o(&d.character_name))
        )
        .x(&d.movie_title)
    );
    min_row(q)
}

fn q9b(d: &Data) -> String {
    let q = d.movie.in_s(
        (&d.movie_company).in_s(
            (&d.company_country).eq("[us]")
                .and((&d.company_note).rx(r"\(200.*\)"))
                .and(
                    (&d.company_note).rx(r"\(USA\)")
                        .or((&d.company_note).rx(r"\(worldwide\)"))
                )
        )
            .and((&d.movie_production_year).ge(2007))
            .and((&d.movie_production_year).le(2010))
    ).o(
        (&d.movie_cast).in_s(
            (&d.cast_note).eq("(voice)")
                .and((&d.cast_role).o(&d.roletype_role).eq("actress"))
                .and((&d.cast_person).in_s(
                    (&d.person_gender).eq("f")
                        .and((&d.person_name).rx(r"Angel"))
                ))
        ).o(
            (&d.cast_person).o((&d.person_aka).o(&d.akaname_name))
                .x((&d.cast_character).o(&d.character_name))
                .x((&d.cast_person).o(&d.person_name))
        )
        .x(&d.movie_title)
    );
    min_row(q)
}

fn q9c(d: &Data) -> String {
    let q = d.movie
        .in_s((&d.movie_company).o((&d.company_country).eq("[us]")))
            .o(
                (&d.movie_cast).in_s(
                    (&d.cast_note).in_v(voice4())
                        .and((&d.cast_role).o(&d.roletype_role).eq("actress"))
                        .and((&d.cast_person).in_s(
                            (&d.person_gender).eq("f")
                                .and((&d.person_name).rx(r"An"))
                        ))
                ).o(
                    (&d.cast_person).o((&d.person_aka).o(&d.akaname_name))
                        .x((&d.cast_character).o(&d.character_name))
                        .x((&d.cast_person).o(&d.person_name))
                )
                .x(&d.movie_title)
            );
    min_row(q)
}

fn q9d(d: &Data) -> String {
    let q = d.movie
        .in_s((&d.movie_company).o((&d.company_country).eq("[us]")))
            .o(
                (&d.movie_cast).in_s(
                    (&d.cast_note).in_v(voice4())
                        .and((&d.cast_role).o(&d.roletype_role).eq("actress"))
                        .and((&d.cast_person).o((&d.person_gender).eq("f")))
                ).o(
                    (&d.cast_person).o((&d.person_aka).o(&d.akaname_name))
                        .x((&d.cast_person).o(&d.person_name))
                        .x((&d.cast_character).o(&d.character_name))
                )
                .x(&d.movie_title)
            );
    min_row(q)
}

fn q10a(d: &Data) -> String {
    let q = d.movie.in_s(
        (&d.movie_company).o((&d.company_country).eq("[ru]"))
            .and((&d.movie_production_year).gt(2005))
    ).o(
        (&d.movie_cast).in_s(
            (&d.cast_note).rx(r"\(voice\)")
                .and((&d.cast_note).rx(r"\(uncredited\)"))
                .and((&d.cast_role).o(&d.roletype_role).eq("actor"))
        ).o((&d.cast_character).o(&d.character_name))
        .x(&d.movie_title)
    );
    min_row(q)
}

fn q10b(d: &Data) -> String {
    let q = d.movie.in_s(
        (&d.movie_company).o((&d.company_country).eq("[ru]"))
            .and((&d.movie_production_year).gt(2010))
    ).o(
        (&d.movie_cast).in_s(
            (&d.cast_note).rx(r"\(producer\)")
                .and((&d.cast_role).o(&d.roletype_role).eq("actor"))
        ).o((&d.cast_character).o(&d.character_name))
        .x(&d.movie_title)
    );
    min_row(q)
}

fn q10c(d: &Data) -> String {
    let q = d.movie.in_s(
        (&d.movie_company).o((&d.company_country).eq("[us]"))
            .and((&d.movie_production_year).gt(1990))
    ).o(
        (&d.movie_cast).in_s((&d.cast_note).rx(r"\(producer\)"))
            .o((&d.cast_character).o(&d.character_name))
            .x(&d.movie_title)
    );
    min_row(q)
}
