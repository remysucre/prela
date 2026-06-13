// queries: 7a-c, 8a-d, 9a-d, 10a-c (queries.jl lines 591-753)

use crate::engine::*;
use crate::job_schema::*;
use crate::queries::helpers::min_row;
use crate::queries::sets::voice4;

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

fn q7a() -> String {
    min_row(movie()
        .when(production_year().ge(1980)
              .and(production_year().le(1995))
              .and(linked_by().ty().text().eq("features")))
        .get(cast().get(person()
                        .when(alias().text().rx(r"a")
                              .and(name_pcode_cf().ge("A"))
                              .and(name_pcode_cf().le("F"))
                              .and(gender().eq("m")
                                   .or(gender().eq("f").and(Person::name().rx(r"^B"))))
                              .and(bio().get(PersonInfo::ty().text().eq("mini biography")
                                             .and(PersonInfo::note().eq("Volker Boehm")))))
                        .name())
             .and(title())))
}

fn q7b() -> String {
    min_row(movie()
        .when(production_year().ge(1980)
              .and(production_year().le(1984))
              .and(linked_by().ty().text().eq("features")))
        .get(cast().get(person()
                        .when(alias().text().rx(r"a")
                              .and(name_pcode_cf().rx(r"^D"))
                              .and(gender().eq("m"))
                              .and(bio().get(PersonInfo::ty().text().eq("mini biography")
                                             .and(PersonInfo::note().eq("Volker Boehm")))))
                        .name())
             .and(title())))
}

// Conjunct tree (∧ = Prod) — consumed via `member` only, so the value
// type stays opaque (`impl Query<D = Id<PersonInfo>> + Probe`).
fn bio_filter_7c() -> impl Query<D = Id<PersonInfo>> + Probe {
    PersonInfo::ty().text().eq("mini biography")
        .and(PersonInfo::note())
}

fn q7c() -> String {
    min_row(movie()
        .when(production_year().ge(1980)
              .and(production_year().le(2010))
              .and(linked_by().ty().text().is_in(
                  ["references", "referenced in", "features", "featured in"])))
        .get(cast().get(person()
                        .when(alias().text().rx(r"a|^A")
                              .and(name_pcode_cf().ge("A"))
                              .and(name_pcode_cf().le("F"))
                              .and(gender().eq("m")
                                   .or(gender().eq("f").and(Person::name().rx(r"^A"))))
                              .and(bio().when(bio_filter_7c())))
                        .get(Person::name()
                             .and(bio().when(bio_filter_7c()).info())))))
}

fn q8a() -> String {
    min_row(movie()
        .when(company().get(country().eq("[jp]")
                            .and(Company::note().rx(r"\(Japan\)"))
                            .and(Company::note().nrx(r"\(USA\)"))))
        .get(cast()
             .when(Cast::note().eq("(voice: English version)")
                   .and(role().text().eq("actress"))
                   .and(person().when(Person::name().rx(r"Yo")
                                      .and(Person::name().nrx(r"Yu")))))
             .person().alias().text()
             .and(title())))
}

fn q8b() -> String {
    min_row(movie()
        .when(company().get(country().eq("[jp]")
                            .and(Company::note().rx(r"\(Japan\)"))
                            .and(Company::note().nrx(r"\(USA\)"))
                            .and(Company::note().rx(r"\(2006\)")
                                 .or(Company::note().rx(r"\(2007\)"))))
              .and(production_year().ge(2006))
              .and(production_year().le(2007))
              .and(title().rx(r"^One Piece").or(title().rx(r"^Dragon Ball Z"))))
        .get(cast()
             .when(Cast::note().eq("(voice: English version)")
                   .and(role().text().eq("actress"))
                   .and(person().when(Person::name().rx(r"Yo")
                                      .and(Person::name().nrx(r"Yu")))))
             .person().alias().text()
             .and(title())))
}

// q8c/q8d differ only in the cast role.
fn q8cd(role_: &'static str) -> String {
    min_row(movie()
        .when(company().country().eq("[us]"))
        .get(cast().when(role().text().eq(role_))
             .person().alias().text()
             .and(title())))
}

fn q8c() -> String { q8cd("writer") }
fn q8d() -> String { q8cd("costume designer") }

fn q9a() -> String {
    min_row(movie()
        .when(company().get(country().eq("[us]")
                            .and(Company::note().rx(r"\(USA\)")
                                 .or(Company::note().rx(r"\(worldwide\)"))))
              .and(production_year().ge(2005))
              .and(production_year().le(2015)))
        .get(cast()
             .when(Cast::note().is_in(voice4())
                   .and(role().text().eq("actress"))
                   .and(person().when(gender().eq("f")
                                      .and(Person::name().rx(r"Ang")))))
             .get(person().alias().text()
                  .and(character().text()))
             .and(title())))
}

fn q9b() -> String {
    min_row(movie()
        .when(company().get(country().eq("[us]")
                            .and(Company::note().rx(r"\(200.*\)"))
                            .and(Company::note().rx(r"\(USA\)")
                                 .or(Company::note().rx(r"\(worldwide\)"))))
              .and(production_year().ge(2007))
              .and(production_year().le(2010)))
        .get(cast()
             .when(Cast::note().eq("(voice)")
                   .and(role().text().eq("actress"))
                   .and(person().when(gender().eq("f")
                                      .and(Person::name().rx(r"Angel")))))
             .get(person().alias().text()
                  .and(character().text())
                  .and(person().name()))
             .and(title())))
}

fn q9c() -> String {
    min_row(movie()
        .when(company().country().eq("[us]"))
        .get(cast()
             .when(Cast::note().is_in(voice4())
                   .and(role().text().eq("actress"))
                   .and(person().when(gender().eq("f")
                                      .and(Person::name().rx(r"An")))))
             .get(person().alias().text()
                  .and(character().text())
                  .and(person().name()))
             .and(title())))
}

fn q9d() -> String {
    min_row(movie()
        .when(company().country().eq("[us]"))
        .get(cast()
             .when(Cast::note().is_in(voice4())
                   .and(role().text().eq("actress"))
                   .and(person().when(gender().eq("f"))))
             .get(person().alias().text()
                  .and(person().name())
                  .and(character().text()))
             .and(title())))
}

fn q10a() -> String {
    min_row(movie()
        .when(company().country().eq("[ru]")
              .and(production_year().gt(2005)))
        .get(cast()
             .when(Cast::note().rx(r"\(voice\)")
                   .and(Cast::note().rx(r"\(uncredited\)"))
                   .and(role().text().eq("actor")))
             .character().text()
             .and(title())))
}

fn q10b() -> String {
    min_row(movie()
        .when(company().country().eq("[ru]")
              .and(production_year().gt(2010)))
        .get(cast()
             .when(Cast::note().rx(r"\(producer\)")
                   .and(role().text().eq("actor")))
             .character().text()
             .and(title())))
}

fn q10c() -> String {
    min_row(movie()
        .when(company().country().eq("[us]")
              .and(production_year().gt(1990)))
        .get(cast().when(Cast::note().rx(r"\(producer\)"))
             .character().text()
             .and(title())))
}
