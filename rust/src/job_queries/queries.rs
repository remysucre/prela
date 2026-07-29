// All 113 JOB queries (queries.jl lines 107..1394).

use crate::engine::*;
use crate::job_schema::*;
use crate::job_queries::helpers::{kw_rx, min_row, film_or_warner_co, follow_link, Row};
use crate::job_queries::sets::{
    genre6, kw7, kw8, kw10, link3, murder4, nordic8, nordic9, nordic10, voice3, voice4, writer5,
};

fn k_23ab() -> impl Query<R = &'static str, D = Id<Movie>> + Drive + Probe {
    kind.eq("movie")
}

fn k_23c() -> impl Query<R = &'static str, D = Id<Movie>> + Drive + Probe {
    kind.text()
        .is_in(["movie", "tv movie", "video movie", "video game"])
}

// Conjunct trees (∧ = Prod) — consumed via `member` only, so the value
// type stays opaque (`impl Query<D = Id<Info>> + Probe`).
fn gf_25ab() -> impl Query<D = Id<Info>> + Probe {
    Info::ty.eq("genres")
        .and(Info::info.eq("Horror"))
}

fn gf_25c() -> impl Query<D = Id<Info>> + Probe {
    Info::ty.eq("genres")
        .and(Info::info.is_in(genre6()))
}

fn co_28() -> impl Query<R = Id<Company>, D = Id<Movie>> + Drive + Probe {
    company.with(country.ne("[us]")
            .and(Company::note.nrx(r"\(USA\)"))
            .and(Company::note.rx(r"\(200.*\)")))
}

fn dt_28ac() -> impl Query<R = Id<Data>, D = Id<Movie>> + Drive + Probe {
    data.with(Data::ty.eq("rating")
         .and(Data::text.lt("8.5")))
}

fn dt_28b() -> impl Query<R = Id<Data>, D = Id<Movie>> + Drive + Probe {
    data.with(Data::ty.eq("rating")
         .and(Data::text.gt("6.5")))
}

// Conjunct trees (∧ = Prod) — consumed via `member` only, so the value
// type stays opaque (`impl Query<D = Id<Info>> + Probe`).
fn gf_horror() -> impl Query<D = Id<Info>> + Probe {
    Info::ty.eq("genres")
        .and(Info::info.is_in(["Horror", "Thriller"]))
}

fn gf_genre6() -> impl Query<D = Id<Info>> + Probe {
    Info::ty.eq("genres")
        .and(Info::info.is_in(genre6()))
}

fn qlink_33a() -> impl Query<R = Id<MovieLink>, D = Id<Movie>> + Drive + Probe {
    link.with(MovieLink::ty.is_in(link3())
         .and(target.with(kind.eq("tv series")
                     .and(company)
                     .and(data.with(Data::ty.eq("rating")
                               .and(Data::text.lt("3.0"))))
                     .and(production_year.ge(2005))
                     .and(production_year.le(2008)))))
}

fn qlink_33b() -> impl Query<R = Id<MovieLink>, D = Id<Movie>> + Drive + Probe {
    link.with(MovieLink::ty.rx(r"follow")
         .and(target.with(kind.eq("tv series")
                     .and(company)
                     .and(data.with(Data::ty.eq("rating")
                               .and(Data::text.lt("3.0"))))
                     .and(production_year.eq(2007)))))
}

fn qlink_33c() -> impl Query<R = Id<MovieLink>, D = Id<Movie>> + Drive + Probe {
    link.with(MovieLink::ty.is_in(link3())
         .and(target.with(kind.is_in(["tv series", "episode"])
                     .and(company)
                     .and(data.with(Data::ty.eq("rating")
                               .and(Data::text.lt("3.5"))))
                     .and(production_year.ge(2000))
                     .and(production_year.le(2010)))))
}

pub const ENTRIES: &[super::Entry] = &[
    // templates 1-5, 11-15, 22 — movie-only
    ("2a",  "'Doc'",                                                                    || min_row(q2a())),
    ("2d",  "& Teller",                                                                 || min_row(q2d())),
    ("3b",  "300: Rise of an Empire",                                                   || min_row(q3b())),
    ("4a",  "5.1 || & Teller 2",                                                        || min_row(q4a())),
    ("13a", "Afghanistan:24 June 2012 || 1.0 || &Me",                                   || min_row(q13a())),
    ("11a", "Churchill Films || followed by || Batman Beyond",                          || min_row(q11a())),
    ("22a", "(empty)",                                                                  || min_row(q22a())),
    ("1a",  "(A Warner Bros.-First National Picture) (presents) || A Clockwork Orange || 1934", || min_row(q1a())),
    ("5a",  "(empty)",                                                                  || min_row(q5a())),
    ("12a", "10th Grade Reunion Films || 8.1 || 3:20",                                  || min_row(q12a())),
    ("14a", "1.0 || $lowdown",                                                          || min_row(q14a())),
    ("1b",  "(Set Decoration Rentals) (uncredited) || Disaster Movie || 2008",          || min_row(q1b())),
    ("2b",  "'Doc'",                                                                    || min_row(q2b())),
    ("2c",  "(empty)",                                                                  || min_row(q2c())),
    ("3a",  "2 Days in New York",                                                       || min_row(q3a())),
    ("3c",  "& Teller 2",                                                               || min_row(q3c())),
    ("4b",  "9.1 || Batman: Arkham City",                                               || min_row(q4b())),
    ("11b", "Filmlance International AB || follows || The Money Man",                   || min_row(q11b())),
    ("13b", "501audio || 1.8 || 5 Time Champion",                                       || min_row(q13b())),
    ("1c",  "(co-production) || Intouchables || 2011",                                  || min_row(q1c())),
    ("1d",  "(Set Decoration Rentals) (uncredited) || Disaster Movie || 2004",          || min_row(q1d())),
    ("4c",  "2.1 || & Teller 2",                                                        || min_row(q4c())),
    ("12b", "$10,000 || Birdemic: Shock and Terror",                                    || min_row(q12b())),
    ("12c", "\"Oh That Gus!\" || 7.1 || $1.11",                                         || min_row(q12c())),
    ("13c", "DL Sites || 1.8 || Champion",                                              || min_row(q13c())),
    ("14b", "6.4 || Of Dolls and Murder",                                               || min_row(q14b())),
    ("14c", "1.0 || $lowdown",                                                          || min_row(q14c())),
    ("22b", "(empty)",                                                                  || min_row(q22b())),
    ("22c", "(empty)",                                                                  || min_row(q22c())),

    // 22d, 5b, 5c, 15a-d, 11c-d, 13d, 6a-f
    ("22d", "(#1.1) || 2.0 || 13 Productions", || min_row(q22d())),
    ("5b",  "(empty)", || min_row(q5b())),
    ("5c",  "11,830,420", || min_row(q5c())),
    ("15a", "USA:1 June 2007 || Battlestar Galactica: The Resistance", || min_row(q15a())),
    ("15b", "USA:27 April 2007 || RoboCop vs Terminator", || min_row(q15b())),
    ("15c", "USA:1 April 2003 || 24: Day Six - Debrief", || min_row(q15c())),
    ("15d", "(Not So) Instant Photo || 06/05", || min_row(q15d())),
    ("11c", "20th Century Fox Home Entertainment || (1997-2002) (worldwide) (all media) || 24", || min_row(q11c())),
    ("11d", "13th Street || (1954) (UK) (TV) || ...denn sie wissen nicht, was sie tun", || min_row(q11d())),
    ("13d", "\"O\" Films || 1.0 || #54 Meets #47", || min_row(q13d())),
    ("6a",  "marvel-cinematic-universe || Iron Man 3 || Downey Jr., Robert", || min_row(q6a())),
    ("6b",  "based-on-comic || The Avengers 2 || Downey Jr., Robert", || min_row(q6b())),
    ("6c",  "marvel-cinematic-universe || The Avengers 2 || Downey Jr., Robert", || min_row(q6c())),
    ("6d",  "based-on-comic || 2008 MTV Movie Awards || Downey Jr., Robert", || min_row(q6d())),
    ("6e",  "marvel-cinematic-universe || Iron Man 3 || Downey Jr., Robert", || min_row(q6e())),
    ("6f",  "based-on-comic || & Teller 2 || \"Steff\", Stefanie Oxmann Mcgaha", || min_row(q6f())),

    // 7a-c, 8a-d, 9a-d, 10a-c
    ("7a",  "Antonioni, Michelangelo || Dressed to Kill",                                 || min_row(q7a())),
    ("7b",  "De Palma, Brian || Dressed to Kill",                                         || min_row(q7b())),
    ("7c",  "50 Cent || \"Boo\" Arnold was born Earl Arnold in Hattiesburg, Mississippi in 1966. His father gave him the nickname 'Boo' early in life and it stuck through grade school, high school, and college. He is still known as \"Boo\" to family and friends.  Raised in central Texas, Arnold played baseball at Texas Tech University where he graduated with a BA in Advertising and Marketing. While at Texas Tech he was also a member of the Texas Epsilon chapter of Phi Delta Theta fraternity. After college he worked with Young Life, an outreach to high school students, in San Antonio, Texas.  While with Young Life Arnold began taking extension courses through Fuller Theological Seminary and ultimately went full-time to Gordon-Conwell Theological Seminary in Boston, Massachusetts. At Gordon-Conwell he completed a Master's Degree in Divinity studying Theology, Philosophy, Church History, Biblical Languages (Hebrew & Greek), and Exegetical Methods. Following seminary he was involved with reconciliation efforts in the former Yugoslavia shortly after the war ended there in1995.  Arnold started acting in his early thirties in Texas. After an encouraging visit to Los Angeles where he spent time with childhood friend George Eads (of CSI Las Vegas) he decided to move to Los Angeles in 2001 to pursue acting full-time. While in Los Angeles he has studied acting with Judith Weston at Judith Weston Studio for Actors and Directors.  Arnold's acting career has been one of steady development, booking co-star and guest-star roles in nighttime television. He guest-starred opposite of Jane Seymour on the night time television drama Justice. He played the lead, Michael Hollister, in the film The Seer, written and directed by Patrick Masset (Friday Night Lights).  He was nominated Best Actor in the168 Film Festival for the role of Phil Stevens in the short-film Useless. In Useless he played a US Marshal who must choose between mercy and justice as he confronts the man who murdered his father. Arnold's performance in Useless confirmed his ability to carry lead roles, and he continues to work toward solidifying himself as a male lead in film and television.  Arnold married fellow Texan Stacy Rudd of San Antonio in 2003 and they are now raising their three children in the Los Angeles area.", || min_row(q7c())),
    ("8a",  "Chambers, Linda || .hack//Quantum",                                          || min_row(q8a())),
    ("8b",  "Chambers, Linda || Dragon Ball Z: Shin Budokai",                             || min_row(q8b())),
    ("8c",  "\"A.J.\" || #1 Cheerleader Camp",                                            || min_row(q8c())),
    ("8d",  "\"Jenny from the Block\" || #1 Cheerleader Camp",                            || min_row(q8d())),
    ("9a",  "AJ || Airport Announcer || Blue Harvest",                                    || min_row(q9a())),
    ("9b",  "AJ || Airport Announcer || Bassett, Angela || Blue Harvest",                 || min_row(q9b())),
    ("9c",  "'Annette' || 2nd Balladeer || Alborg, Ana Esther || (1975-01-20)",           || min_row(q9c())),
    ("9d",  "!!!, Toy || Aaron, Caroline || \"Cockamamie's\" Salesgirl || $15,000.00 Error", || min_row(q9d())),
    ("10a", "Actor || 12 Rounds",                                                         || min_row(q10a())),
    ("10b", "(empty)",                                                                    || min_row(q10b())),
    ("10c", "Himself || Evil Eyes: Behind the Scenes",                                    || min_row(q10c())),

    // templates 16-18
    ("16a", "Adams, Stan || Carol Burnett vs. Anthony Perkins", || min_row(q16a())),
    ("16b", "!!!, Toy || & Teller", || min_row(q16b())),
    ("16c", "\"Brooklyn\" Tony Danza || (#1.5)", || min_row(q16c())),
    ("16d", "\"Brooklyn\" Tony Danza || (#1.5)", || min_row(q16d())),
    ("17a", "B, Khaz", || min_row(q17a())),
    ("17b", "Z'Dar, Robert", || min_row(q17b())),
    ("17c", "X'Volaitis, John", || min_row(q17c())),
    ("17d", "Abrahamsson, Bertil", || min_row(q17d())),
    ("17e", "$hort, Too", || min_row(q17e())),
    ("17f", "'El Galgo PornoStar', Blanquito", || min_row(q17f())),
    ("18a", "$1,000 || 10 || 40 Days and 40 Nights", || min_row(q18a())),
    ("18b", "Horror || 8.1 || Agorable", || min_row(q18b())),
    ("18c", "Action || 10 || #PostModem", || min_row(q18c())),

    // 19a-26c
    ("19a", "Angeline, Moriah || Blue Harvest", || min_row(q19a())),
    ("19b", "Jolie, Angelina || Kung Fu Panda", || min_row(q19b())),
    ("19c", "Alborg, Ana Esther || .hack//Akusei heni vol. 2", || min_row(q19c())),
    ("19d", "Aaron, Caroline || $9.99", || min_row(q19d())),
    ("20a", "Disaster Movie", || min_row(q20a())),
    ("20b", "Iron Man", || min_row(q20b())),
    ("20c", "Abell, Alistair || ...And Then I...", || min_row(q20c())),
    ("21a", "Det Danske Filminstitut || followed by || Der Serienkiller - Klinge des Todes", || min_row(q21a())),
    ("21b", "Filmlance International AB || followed by || Hämndens pris", || min_row(q21b())),
    ("21c", "Churchill Films || followed by || Batman Beyond", || min_row(q21c())),
    ("23a", "movie || The Analysts", || min_row(q23a())),
    ("23b", "movie || The Big Mope", || min_row(q23b())),
    ("23c", "movie || Dirt Merchant", || min_row(q23c())),
    ("24a", "Additional Voices || Baker, Andrea || Baiohazâdo 6", || min_row(q24a())),
    ("24b", "Tigress || Jolie, Angelina || Kung Fu Panda 2", || min_row(q24b())),
    ("25a", "Horror || 10 || -- And Now the Screaming Starts! || Abdallah, Damon", || min_row(q25a())),
    ("25b", "Horror || 138 || Vampire Boys || Campbell, Jeremiah", || min_row(q25b())),
    ("25c", "Action || 10 || $ || Aakeson, Kim Fupz", || min_row(q25c())),
    ("26a", "'Agua' Man || Acereda, Hermie || 7.1 || 3:10 to Yuma", || min_row(q26a())),
    ("26b", "Bank Manager || 8.2 || Inception", || min_row(q26b())),
    ("26c", "'Agua' Man || 1.9 || 12 Rounds", || min_row(q26c())),

    // 27a-33c
    ("27a", "Det Danske Filminstitut || followed by || Spår i mörker", || min_row(q27a())),
    ("27b", "Filmlance International AB || followed by || Vita nätter", || min_row(q27b())),
    ("27c", "Det Danske Filminstitut || followed by || Spår i mörker", || min_row(q27c())),
    ("28a", "01 Distribuzione || 2.9 || (#1.1)", || min_row(q28a())),
    ("28b", "20th Century Fox || 6.6 || (#1.1)", || min_row(q28b())),
    ("28c", "01 Distribuzione || 1.9 || (#1.1)", || min_row(q28c())),
    ("29a", "Queen || Andrews, Julie || Shrek 2", || min_row(q29a())),
    ("29b", "Queen || Andrews, Julie || Shrek 2", || min_row(q29b())),
    ("29c", "Lola || Andrews, Julie || Hoodwinked!", || min_row(q29c())),
    ("30a", "Horror || 100356 || 16 Blocks || Abrams, J.J.", || min_row(q30a())),
    ("30b", "Horror || 194782 || Freddy vs. Jason || Shannon, Damian", || min_row(q30b())),
    ("30c", "Action || 100356 || $ || Abernathy, Lewis", || min_row(q30c())),
    ("31a", "Horror || 1040 || 2001 Maniacs || Agnew, Jim", || min_row(q31a())),
    ("31b", "Horror || 129755 || Saw || Bousman, Darren Lynn", || min_row(q31b())),
    ("31c", "Action || 1008 || 11:14 || Abraham, Brad", || min_row(q31c())),
    ("32a", "(empty)", || min_row(q32a())),
    ("32b", "alternate language version of || 12 oz. Mouse || 'Angel': Season 2 Overview", || min_row(q32b())),
    ("33a", "495 Productions || 495 Productions || 3.3 || 2.7 || A Double Shot at Love || A Shot at Love with Tila Tequila", || min_row(q33a())),
    ("33b", "MTV Netherlands || 495 Productions || 3.3 || 2.7 || A Double Shot at Love || A Shot at Love with Tila Tequila", || min_row(q33b())),
    ("33c", "2BE || 495 Productions || 1.3 || 1.0 || A Double Shot at Love || A Double Shot at Love", || min_row(q33c())),
];

// q2a–q2d differ only in the company country code.
fn q2(cc: &'static str) -> impl Drive<R: Row> {
    movie.with(keyword.eq("character-name-in-title")
          .and(company.country().eq(cc)))
        .title()
}

fn q2a() -> impl Drive<R: Row> { q2("[de]") }
fn q2b() -> impl Drive<R: Row> { q2("[nl]") }
fn q2c() -> impl Drive<R: Row> { q2("[sm]") }
fn q2d() -> impl Drive<R: Row> { q2("[us]") }

fn q3b() -> impl Drive<R: Row> {
    movie.with(keyword.rx(r"sequel")
          .and(info.eq("Bulgaria"))
          .and(production_year.gt(2010)))
        .title()
}

// q4a–q4c differ only in the year cutoff and rating threshold.
fn q4(year: i64, rating: &'static str) -> impl Drive<R: Row> {
    movie.with(keyword.with(kw_rx(r"sequel"))
          .and(production_year.gt(year)))
       .select(data.with(Data::ty.eq("rating")
                    .and(Data::text.gt(rating))).text()
          .and(title))
}

fn q4a() -> impl Drive<R: Row> { q4(2005, "5.0") }
fn q4b() -> impl Drive<R: Row> { q4(2010, "9.0") }
fn q4c() -> impl Drive<R: Row> { q4(1990, "2.0") }

fn q13a() -> impl Drive<R: Row> {
    movie.with(company.select(country.eq("[de]")
                         .and(Company::ty.eq("production companies")))
          .and(kind.eq("movie")))
       .select(info.with(Info::ty.eq("release dates")).info()
          .and(data.with(Data::ty.eq("rating")).text())
          .and(title))
}

fn q11a() -> impl Drive<R: Row> {
    movie.with(link
          .and(keyword.eq("sequel"))
          .and(production_year.between(1950, 2000)))
       .select(company.with(country.ne("[pl]")
                       .and(Company::name.rx(r"Film|Warner"))
                       .and(Company::ty.eq("production companies"))
                     .minus(Company::note)).name()
          .and(link.ty().rx(r"follow"))
          .and(title))
}

fn q22a() -> impl Drive<R: Row> {
    movie.with(info.select(Info::ty.eq("countries")
                      .and(Info::info.is_in(["Germany", "German", "USA", "American"])))
          .and(keyword.is_in(murder4()))
          .and(production_year.gt(2008))
          .and(kind.is_in(["movie", "episode"])))
       .select(title
          .and(data.with(Data::text.lt("7.0")
                    .and(Data::ty.eq("rating"))).text())
          .and(company.with(Company::note.nrx(r"\(USA\)")
                       .and(Company::note.rx(r"\(200.*\)"))
                       .and(country.ne("[us]"))
                       .and(Company::ty.eq("production companies"))).name()))
}

fn q1a() -> impl Drive<R: Row> {
    movie.with(data.ty().eq("top 250 rank"))
       .select(company.with(Company::ty.eq("production companies")
                       .and(Company::note.nrx(r"\(as Metro-Goldwyn-Mayer Pictures\)"))
                       .and(Company::note.rx(r"\(co-production\)|\(presents\)"))).note()
          .and(title)
          .and(production_year))
}

fn q5a() -> impl Drive<R: Row> {
    movie.with(company.select(Company::ty.eq("production companies")
                         .and(Company::note.rx(r"\(theatrical\)"))
                         .and(Company::note.rx(r"\(France\)")))
          .and(info.is_in(nordic8()))
          .and(production_year.gt(2005)))
        .title()
}

fn q12a() -> impl Drive<R: Row> {
    movie.with(info.select(Info::ty.eq("genres")
                      .and(Info::info.is_in(["Drama", "Horror"])))
          .and(production_year.ge(2005))
          .and(production_year.le(2008)))
       .select(company.with(country.eq("[us]")
                       .and(Company::ty.eq("production companies"))).name()
          .and(data.with(Data::ty.eq("rating")
                    .and(Data::text.gt("8.0"))).text())
          .and(title))
}

fn q14a() -> impl Drive<R: Row> {
    movie.with(keyword.is_in(murder4())
          .and(kind.eq("movie"))
          .and(info.select(Info::ty.eq("countries")
                      .and(Info::info.is_in(["Sweden","Norway","Germany","Denmark","Swedish","Denish","Norwegian","German","USA","American"]))))
          .and(production_year.gt(2010)))
       .select(data.with(Data::ty.eq("rating")
                    .and(Data::text.lt("8.5"))).text()
          .and(title))
}

fn q1b() -> impl Drive<R: Row> {
    movie.with(data.ty().eq("bottom 10 rank")
          .and(production_year.ge(2005))
          .and(production_year.le(2010)))
       .select(company.with(Company::ty.eq("production companies")
                       .and(Company::note.nrx(r"\(as Metro-Goldwyn-Mayer Pictures\)"))).note()
          .and(title)
          .and(production_year))
}

// q3a/q3c differ only in the country list and the year cutoff.
fn q3ac(countries: Vec<&'static str>, year: i64) -> impl Drive<R: Row> {
    movie.with(keyword.rx(r"sequel")
          .and(info.is_in(countries))
          .and(production_year.gt(year)))
        .title()
}

fn q3a() -> impl Drive<R: Row> { q3ac(nordic8(), 2005) }
fn q3c() -> impl Drive<R: Row> {
    q3ac(vec!["Sweden","Norway","Germany","Denmark","Swedish","Denish","Norwegian","German","USA","American"], 1990)
}

fn q11b() -> impl Drive<R: Row> {
    movie.with(link
          .and(keyword.eq("sequel"))
          .and(production_year.eq(1998))
          .and(title.rx(r"Money")))
       .select(company.with(country.ne("[pl]")
                       .and(Company::name.rx(r"Film|Warner"))
                       .and(Company::ty.eq("production companies"))
                     .minus(Company::note)).name()
          .and(link.ty().rx(r"follows"))
          .and(title))
}

fn q13b() -> impl Drive<R: Row> {
    movie.with(kind.eq("movie")
          .and(info.ty().eq("release dates"))
          .and(title.ne(""))
          .and(title.rx(r"Champion|Loser")))
       .select(company.with(country.eq("[us]")
                       .and(Company::ty.eq("production companies"))).name()
          .and(data.with(Data::ty.eq("rating")).text())
          .and(title))
}

fn q1c() -> impl Drive<R: Row> {
    movie.with(data.ty().eq("top 250 rank")
          .and(production_year.gt(2010)))
       .select(company.with(Company::ty.eq("production companies")
                       .and(Company::note.nrx(r"\(as Metro-Goldwyn-Mayer Pictures\)"))
                       .and(Company::note.rx(r"\(co-production\)"))).note()
          .and(title)
          .and(production_year))
}

fn q1d() -> impl Drive<R: Row> {
    movie.with(data.ty().eq("bottom 10 rank")
          .and(production_year.gt(2000)))
       .select(company.with(Company::ty.eq("production companies")
                       .and(Company::note.nrx(r"\(as Metro-Goldwyn-Mayer Pictures\)"))).note()
          .and(title)
          .and(production_year))
}

fn q12b() -> impl Drive<R: Row> {
    movie.with(company.select(country.eq("[us]")
                         .and(Company::ty.is_in(["production companies", "distributors"])))
          .and(data.ty().eq("bottom 10 rank"))
          .and(production_year.gt(2000))
          .and(title.rx(r"^Birdemic|Movie")))
       .select(info.with(Info::ty.eq("budget")).info()
          .and(title))
}

fn q12c() -> impl Drive<R: Row> {
    movie.with(info.select(Info::ty.eq("genres")
                      .and(Info::info.is_in(["Drama", "Horror", "Western", "Family"])))
          .and(production_year.ge(2000))
          .and(production_year.le(2010)))
       .select(company.with(country.eq("[us]")
                       .and(Company::ty.eq("production companies"))).name()
          .and(data.with(Data::ty.eq("rating")
                    .and(Data::text.gt("7.0"))).text())
          .and(title))
}

fn q13c() -> impl Drive<R: Row> {
    movie.with(kind.eq("movie")
          .and(info.ty().eq("release dates"))
          .and(title.ne(""))
          .and(title.rx(r"^Champion|^Loser")))
       .select(company.with(country.eq("[us]")
                       .and(Company::ty.eq("production companies"))).name()
          .and(data.with(Data::ty.eq("rating")).text())
          .and(title))
}

fn q14b() -> impl Drive<R: Row> {
    movie.with(keyword.is_in(["murder", "murder-in-title"])
          .and(kind.eq("movie"))
          .and(info.select(Info::ty.eq("countries")
                      .and(Info::info.is_in(["Sweden","Norway","Germany","Denmark","Swedish","Denish","Norwegian","German","USA","American"]))))
          .and(production_year.gt(2010))
          .and(title.rx(r"murder|Murder|Mord")))
       .select(data.with(Data::ty.eq("rating")
                    .and(Data::text.gt("6.0"))).text()
          .and(title))
}

fn q14c() -> impl Drive<R: Row> {
    movie.with(keyword.is_in(murder4())
          .and(kind.is_in(["movie", "episode"]))
          .and(info.select(Info::ty.eq("countries")
                      .and(Info::info.is_in(nordic10()))))
          .and(production_year.gt(2005)))
       .select(data.with(Data::ty.eq("rating")
                    .and(Data::text.lt("8.5"))).text()
          .and(title))
}

fn q22b() -> impl Drive<R: Row> {
    movie.with(info.select(Info::ty.eq("countries")
                      .and(Info::info.is_in(["Germany", "German", "USA", "American"])))
          .and(keyword.is_in(murder4()))
          .and(production_year.gt(2009))
          .and(kind.is_in(["movie", "episode"])))
       .select(title
          .and(data.with(Data::text.lt("7.0")
                    .and(Data::ty.eq("rating"))).text())
          .and(company.with(Company::note.nrx(r"\(USA\)")
                       .and(Company::note.rx(r"\(200.*\)"))
                       .and(country.ne("[us]"))
                       .and(Company::ty.eq("production companies"))).name()))
}

fn q22c() -> impl Drive<R: Row> {
    movie.with(info.select(Info::ty.eq("countries")
                      .and(Info::info.is_in(nordic10())))
          .and(keyword.is_in(murder4()))
          .and(production_year.gt(2005))
          .and(kind.is_in(["movie", "episode"])))
       .select(title
          .and(data.with(Data::text.lt("8.5")
                    .and(Data::ty.eq("rating"))).text())
          .and(company.with(Company::note.nrx(r"\(USA\)")
                       .and(Company::note.rx(r"\(200.*\)"))
                       .and(country.ne("[us]"))
                       .and(Company::ty.eq("production companies"))).name()))
}

fn q22d() -> impl Drive<R: Row> {
    movie.with(info.select(Info::ty.eq("countries")
                      .and(Info::info.is_in(nordic10())))
          .and(keyword.is_in(murder4()))
          .and(production_year.gt(2005))
          .and(kind.is_in(["movie", "episode"])))
       .select(title
          .and(data.with(Data::text.lt("8.5")
                    .and(Data::ty.eq("rating"))).text())
          .and(company.with(country.ne("[us]")
                       .and(Company::ty.eq("production companies"))).name()))
}

fn q5b() -> impl Drive<R: Row> {
    movie.with(company.select(Company::ty.eq("production companies")
                         .and(Company::note.rx(r"\(VHS\)"))
                         .and(Company::note.rx(r"\(USA\)"))
                         .and(Company::note.rx(r"\(1994\)")))
          .and(info.is_in(["USA", "America"]))
          .and(production_year.gt(2010)))
        .title()
}

fn q5c() -> impl Drive<R: Row> {
    movie.with(company.select(Company::ty.eq("production companies")
                         .and(Company::note.nrx(r"\(TV\)"))
                         .and(Company::note.rx(r"\(USA\)")))
          .and(info.is_in(nordic10()))
          .and(production_year.gt(1990)))
        .title()
}

fn q15a() -> impl Drive<R: Row> {
    movie.with(production_year.gt(2000)
          .and(company.select(country.eq("[us]")
                         .and(Company::note.rx(r"\(200.*\)"))
                         .and(Company::note.rx(r"\(worldwide\)"))))
          .and(keyword)
          .and(aka))
       .select(info.with(Info::ty.eq("release dates")
                    .and(Info::info.rx(r"^USA:.* 200"))
                    .and(Info::note.rx(r"internet"))).info()
          .and(title))
}

fn q15b() -> impl Drive<R: Row> {
    movie.with(company.select(country.eq("[us]")
                         .and(Company::name.eq("YouTube"))
                         .and(Company::note.rx(r"\(200.*\)"))
                         .and(Company::note.rx(r"\(worldwide\)")))
          .and(keyword)
          .and(aka)
          .and(production_year.ge(2005))
          .and(production_year.le(2010)))
       .select(info.with(Info::ty.eq("release dates")
                    .and(Info::info.rx(r"^USA:.* 200"))
                    .and(Info::note.rx(r"internet"))).info()
          .and(title))
}

fn q15c() -> impl Drive<R: Row> {
    movie.with(company.country().eq("[us]")
          .and(keyword)
          .and(aka)
          .and(production_year.gt(1990)))
       .select(info.with(Info::ty.eq("release dates")
                    .and(Info::info.rx(r"^USA:.* 199|^USA:.* 200"))
                    .and(Info::note.rx(r"internet"))).info()
          .and(title))
}

fn q15d() -> impl Drive<R: Row> {
    movie.with(company.country().eq("[us]")
          .and(keyword)
          .and(info.select(Info::ty.eq("release dates")
                      .and(Info::note.rx(r"internet"))))
          .and(production_year.gt(1990)))
       .select(aka.text()
          .and(title))
}

fn q11c() -> impl Drive<R: Row> {
    movie.with(keyword.is_in(["sequel", "revenge", "based-on-novel"])
          .and(production_year.gt(1950))
          .and(link))
       .select(company.with(country.ne("[pl]")
                       .and(Company::name.rx(r"^20th Century Fox|^Twentieth Century Fox"))
                       .and(Company::ty.ne("production companies"))
                       .and(Company::note)).select(Company::name.and(Company::note))
          .and(title))
}

fn q11d() -> impl Drive<R: Row> {
    movie.with(keyword.is_in(["sequel", "revenge", "based-on-novel"])
          .and(production_year.gt(1950))
          .and(link))
       .select(company.with(country.ne("[pl]")
                       .and(Company::ty.ne("production companies"))
                       .and(Company::note)).select(Company::name.and(Company::note))
          .and(title))
}

fn q13d() -> impl Drive<R: Row> {
    movie.with(kind.eq("movie")
          .and(info.ty().eq("release dates")))
       .select(company.with(country.eq("[us]")
                       .and(Company::ty.eq("production companies"))).name()
          .and(data.with(Data::ty.eq("rating")).text())
          .and(title))
}

// q6a/c/e share the marvel-cinematic-universe keyword and q6b/d the kw8
// list; within each pair only the year cutoff varies.
fn q6_marvel(year: i64) -> impl Drive<R: Row> {
    let kw = || keyword.eq("marvel-cinematic-universe");
    let downey = cast.person().rx(r"Downey.*Robert");
    movie.with(production_year.gt(year).and(kw()))
       .select(kw().and(title).and(downey))
}

fn q6_comic(year: i64) -> impl Drive<R: Row> {
    let kw = || keyword.is_in(kw8());
    let downey = cast.person().rx(r"Downey.*Robert");
    movie.with(production_year.gt(year).and(kw()))
       .select(kw().and(title).and(downey))
}

fn q6a() -> impl Drive<R: Row> { q6_marvel(2010) }
fn q6b() -> impl Drive<R: Row> { q6_comic(2014) }
fn q6c() -> impl Drive<R: Row> { q6_marvel(2014) }
fn q6d() -> impl Drive<R: Row> { q6_comic(2000) }
fn q6e() -> impl Drive<R: Row> { q6_marvel(2000) }

fn q6f() -> impl Drive<R: Row> {
    let kw = || keyword.is_in(kw8());
    let cast_name = cast.person().name();
    movie.with(production_year.gt(2000).and(kw()))
       .select(kw().and(title).and(cast_name))
}

fn q7a() -> impl Drive<R: Row> {
    movie.with(production_year.ge(1980)
          .and(production_year.le(1995))
          .and(linked_by.ty().eq("features")))
       .select(cast.select(person
                        .with(alias.rx(r"a")
                         .and(name_pcode_cf.ge("A"))
                         .and(name_pcode_cf.le("F"))
                         // m ∨ (f ∧ name~^B), spelled {m,f} ∖ (f ∖ ^B):
                         // ∨ is member-only and can't sit inside a probed ∧-tree.
                         .and(gender.is_in(["m", "f"])
                          .minus(gender.eq("f").minus(Person::name.rx(r"^B"))))
                         .and(bio.select(PersonInfo::ty.eq("mini biography")
                                    .and(PersonInfo::note.eq("Volker Boehm")))))
                        .name())
          .and(title))
}

fn q7b() -> impl Drive<R: Row> {
    movie.with(production_year.ge(1980)
          .and(production_year.le(1984))
          .and(linked_by.ty().eq("features")))
       .select(cast.select(person
                        .with(alias.rx(r"a")
                         .and(name_pcode_cf.rx(r"^D"))
                         .and(gender.eq("m"))
                         .and(bio.select(PersonInfo::ty.eq("mini biography")
                                    .and(PersonInfo::note.eq("Volker Boehm")))))
                        .name())
          .and(title))
}

// Conjunct tree (∧ = Prod) — consumed via `member` only, so the value
// type stays opaque (`impl Query<D = Id<PersonInfo>> + Probe`).
fn bio_filter_7c() -> impl Query<D = Id<PersonInfo>> + Probe {
    PersonInfo::ty.eq("mini biography")
        .and(PersonInfo::note)
}

fn q7c() -> impl Drive<R: Row> {
    movie.with(production_year.ge(1980)
          .and(production_year.le(2010))
          .and(linked_by.ty().is_in(
                  ["references", "referenced in", "features", "featured in"])))
       .select(cast.select(person
                        .with(alias.rx(r"a|^A")
                         .and(name_pcode_cf.ge("A"))
                         .and(name_pcode_cf.le("F"))
                         // m ∨ (f ∧ name~^A), spelled {m,f} ∖ (f ∖ ^A):
                         // ∨ is member-only and can't sit inside a probed ∧-tree.
                         .and(gender.is_in(["m", "f"])
                          .minus(gender.eq("f").minus(Person::name.rx(r"^A"))))
                         .and(bio.with(bio_filter_7c())))
                        .select(Person::name
                           .and(bio.with(bio_filter_7c()).info()))))
}

fn q8a() -> impl Drive<R: Row> {
    movie.with(company.select(country.eq("[jp]")
                         .and(Company::note.rx(r"\(Japan\)"))
                         .and(Company::note.nrx(r"\(USA\)"))))
       .select(cast
             .with(Cast::note.eq("(voice: English version)")
              .and(role.eq("actress"))
              .and(person.with(Person::name.rx(r"Yo")
                          .and(Person::name.nrx(r"Yu")))))
             .person().alias().text()
          .and(title))
}

fn q8b() -> impl Drive<R: Row> {
    movie.with(company.select(country.eq("[jp]")
                         .and(Company::note.rx(r"\(Japan\)"))
                         .and(Company::note.nrx(r"\(USA\)"))
                         .and(Company::note.rx(r"\(2006\)|\(2007\)")))
          .and(production_year.ge(2006))
          .and(production_year.le(2007))
          .and(title.rx(r"^One Piece|^Dragon Ball Z")))
       .select(cast
             .with(Cast::note.eq("(voice: English version)")
              .and(role.eq("actress"))
              .and(person.with(Person::name.rx(r"Yo")
                          .and(Person::name.nrx(r"Yu")))))
             .person().alias().text()
          .and(title))
}

// q8c/q8d differ only in the cast role.
fn q8cd(role_: &'static str) -> impl Drive<R: Row> {
    movie.with(company.country().eq("[us]"))
       .select(cast.with(role.eq(role_))
             .person().alias().text()
          .and(title))
}

fn q8c() -> impl Drive<R: Row> { q8cd("writer") }
fn q8d() -> impl Drive<R: Row> { q8cd("costume designer") }

fn q9a() -> impl Drive<R: Row> {
    movie.with(company.select(country.eq("[us]")
                         .and(Company::note.rx(r"\(USA\)|\(worldwide\)")))
          .and(production_year.ge(2005))
          .and(production_year.le(2015)))
       .select(cast
             .with(Cast::note.is_in(voice4())
              .and(role.eq("actress"))
              .and(person.with(gender.eq("f")
                          .and(Person::name.rx(r"Ang")))))
             .select(person.alias().text()
                .and(character.text()))
          .and(title))
}

fn q9b() -> impl Drive<R: Row> {
    movie.with(company.select(country.eq("[us]")
                         .and(Company::note.rx(r"\(200.*\)"))
                         .and(Company::note.rx(r"\(USA\)|\(worldwide\)")))
          .and(production_year.ge(2007))
          .and(production_year.le(2010)))
       .select(cast
             .with(Cast::note.eq("(voice)")
              .and(role.eq("actress"))
              .and(person.with(gender.eq("f")
                          .and(Person::name.rx(r"Angel")))))
             .select(person.alias().text()
                .and(character.text())
                .and(person.name()))
          .and(title))
}

fn q9c() -> impl Drive<R: Row> {
    movie.with(company.country().eq("[us]"))
       .select(cast
             .with(Cast::note.is_in(voice4())
              .and(role.eq("actress"))
              .and(person.with(gender.eq("f")
                          .and(Person::name.rx(r"An")))))
             .select(person.alias().text()
                .and(character.text())
                .and(person.name()))
          .and(title))
}

fn q9d() -> impl Drive<R: Row> {
    movie.with(company.country().eq("[us]"))
       .select(cast
             .with(Cast::note.is_in(voice4())
              .and(role.eq("actress"))
              .and(person.with(gender.eq("f"))))
             .select(person.alias().text()
                .and(person.name())
                .and(character.text()))
          .and(title))
}

fn q10a() -> impl Drive<R: Row> {
    movie.with(company.country().eq("[ru]")
          .and(production_year.gt(2005)))
       .select(cast
             .with(Cast::note.rx(r"\(voice\)")
              .and(Cast::note.rx(r"\(uncredited\)"))
              .and(role.eq("actor")))
             .character().text()
          .and(title))
}

fn q10b() -> impl Drive<R: Row> {
    movie.with(company.country().eq("[ru]")
          .and(production_year.gt(2010)))
       .select(cast
             .with(Cast::note.rx(r"\(producer\)")
              .and(role.eq("actor")))
             .character().text()
          .and(title))
}

fn q10c() -> impl Drive<R: Row> {
    movie.with(company.country().eq("[us]")
          .and(production_year.gt(1990)))
       .select(cast.with(Cast::note.rx(r"\(producer\)"))
             .character().text()
          .and(title))
}

// q16a/q16d differ only in the episode_nr lower bound.
fn q16ad(lo: i64) -> impl Drive<R: Row> {
    movie.with(company.country().eq("[us]")
          .and(keyword.eq("character-name-in-title"))
          .and(episode_nr.ge(lo))
          .and(episode_nr.lt(100)))
       .select(cast.person().alias().text()
          .and(title))
}

fn q16a() -> impl Drive<R: Row> { q16ad(50) }
fn q16d() -> impl Drive<R: Row> { q16ad(5) }

fn q16b() -> impl Drive<R: Row> {
    movie.with(company.country().eq("[us]")
          .and(keyword.eq("character-name-in-title")))
       .select(cast.person().alias().text()
          .and(title))
}

fn q16c() -> impl Drive<R: Row> {
    movie.with(company.country().eq("[us]")
          .and(keyword.eq("character-name-in-title"))
          .and(episode_nr.lt(100)))
       .select(cast.person().alias().text()
          .and(title))
}

fn q17a() -> impl Drive<R: Row> {
    movie.with(company.country().eq("[us]")
          .and(keyword.eq("character-name-in-title")))
       .select(cast.person().rx(r"^B"))
}

// q17b/c/d/f differ only in the person-name regex.
fn q17_any_co(re: &str) -> impl Drive<R: Row> {
    movie.with(company
          .and(keyword.eq("character-name-in-title")))
       .select(cast.person().rx(re))
}

fn q17b() -> impl Drive<R: Row> { q17_any_co(r"^Z") }
fn q17c() -> impl Drive<R: Row> { q17_any_co(r"^X") }
fn q17d() -> impl Drive<R: Row> { q17_any_co(r"Bert") }
fn q17f() -> impl Drive<R: Row> { q17_any_co(r"B") }

fn q17e() -> impl Drive<R: Row> {
    movie.with(company.country().eq("[us]")
          .and(keyword.eq("character-name-in-title")))
       .select(cast.person().name())
}

fn ib_18a() -> impl Query<R = &'static str, D = Id<Movie>> + Drive + Probe {
    info.with(Info::ty.eq("budget")).info()
}

fn q18a() -> impl Drive<R: Row> {
    movie.with(ib_18a()
          .and(cast.select(Cast::note.is_in(["(producer)", "(executive producer)"])
                      .and(person.select(gender.eq("m")
                                    .and(Person::name.rx(r"Tim")))))))
       .select(ib_18a()
          .and(data.with(Data::ty.eq("votes")).text())
          .and(title))
}

// Conjunct/diff tree (∧ = Prod, - = Diff) — consumed via `member` only, so
// the value type stays opaque (`impl Query<D = Id<Info>> + Probe`).
fn gf_18b() -> impl Query<D = Id<Info>> + Probe {
    Info::ty.eq("genres")
        .and(Info::info.is_in(["Horror", "Thriller"]))
        .minus(Info::note)
}

fn q18b() -> impl Drive<R: Row> {
    movie.with(info.with(gf_18b())
          .and(production_year.ge(2008))
          .and(production_year.le(2014))
          .and(cast.select(Cast::note.is_in(writer5())
                      .and(person.select(gender.eq("f"))))))
       .select(info.with(gf_18b()).info()
          .and(data.with(Data::ty.eq("rating")
                    .and(Data::text.gt("8.0"))).text())
          .and(title))
}

fn gf_18c() -> impl Query<D = Id<Info>> + Probe {
    Info::ty.eq("genres")
        .and(Info::info.is_in(genre6()))
}

fn q18c() -> impl Drive<R: Row> {
    movie.with(info.with(gf_18c())
          .and(cast.select(Cast::note.is_in(writer5())
                      .and(person.select(gender.eq("m"))))))
       .select(info.with(gf_18c()).info()
          .and(data.with(Data::ty.eq("votes")).text())
          .and(title))
}

fn q19a() -> impl Drive<R: Row> {
    movie.with(company.select(country.eq("[us]")
                         .and(Company::note.rx(r"\(USA\)|\(worldwide\)")))
          .and(info.select(Info::ty.eq("release dates")
                      .and(Info::info.rx(r"^Japan:.*200|^USA:.*200"))))
          .and(production_year.ge(2005))
          .and(production_year.le(2009)))
       .select(cast
             .with(Cast::note.is_in(voice4())
              .and(role.eq("actress"))
              .and(character)
              .and(person.with(gender.eq("f")
                          .and(Person::name.rx(r"Ang"))
                          .and(alias))))
             .person().name()
          .and(title))
}

fn q19b() -> impl Drive<R: Row> {
    movie.with(company.select(country.eq("[us]")
                         .and(Company::note.rx(r"\(200.*\)"))
                         .and(Company::note.rx(r"\(USA\)|\(worldwide\)")))
          .and(info.select(Info::ty.eq("release dates")
                      .and(Info::info.rx(r"^Japan:.*2007|^USA:.*2008"))))
          .and(production_year.ge(2007))
          .and(production_year.le(2008))
          .and(title.rx(r"Kung.*Fu.*Panda")))
       .select(cast
             .with(Cast::note.eq("(voice)")
              .and(role.eq("actress"))
              .and(character)
              .and(person.with(gender.eq("f")
                          .and(Person::name.rx(r"Angel"))
                          .and(alias))))
             .person().name()
          .and(title))
}

fn q19c() -> impl Drive<R: Row> {
    movie.with(company.country().eq("[us]")
          .and(info.select(Info::ty.eq("release dates")
                      .and(Info::info.rx(r"^Japan:.*200|^USA:.*200"))))
          .and(production_year.gt(2000)))
       .select(cast
             .with(Cast::note.is_in(voice4())
              .and(role.eq("actress"))
              .and(character)
              .and(person.with(gender.eq("f")
                          .and(Person::name.rx(r"An"))
                          .and(alias))))
             .person().name()
          .and(title))
}

fn q19d() -> impl Drive<R: Row> {
    movie.with(company.country().eq("[us]")
          .and(info.ty().eq("release dates"))
          .and(production_year.gt(2000)))
       .select(cast
             .with(Cast::note.is_in(voice4())
              .and(role.eq("actress"))
              .and(character)
              .and(person.with(gender.eq("f")
                          .and(alias))))
             .person().name()
          .and(title))
}

fn q20a() -> impl Drive<R: Row> {
    movie.with(complete_cast.select(subject.eq("cast")
                               .and(status.rx(r"complete")))
          .and(keyword.is_in(kw8()))
          .and(kind.eq("movie"))
          .and(production_year.gt(1950))
          .and(cast.select(character.select(Character::text.nrx(r"Sherlock")
                                       .and(Character::text.rx(r"Tony.*Stark|Iron.*Man"))))))
        .title()
}

fn q20b() -> impl Drive<R: Row> {
    movie.with(complete_cast.select(subject.eq("cast")
                               .and(status.rx(r"complete")))
          .and(keyword.is_in(kw8()))
          .and(kind.eq("movie"))
          .and(production_year.gt(2000))
          .and(cast.select(character.select(Character::text.nrx(r"Sherlock")
                                       .and(Character::text.rx(r"Tony.*Stark|Iron.*Man")))
                      .and(person.rx(r"Downey.*Robert")))))
        .title()
}

fn q20c() -> impl Drive<R: Row> {
    movie.with(complete_cast.select(subject.eq("cast")
                               .and(status.rx(r"complete")))
          .and(keyword.is_in(kw10()))
          .and(kind.eq("movie"))
          .and(production_year.gt(2000)))
       .select(cast.with(character.rx(r"[Mm]an"))
             .person().name()
          .and(title))
}

// q21a/b/c differ only in the country list and year range.
//
// `co` appears ONLY in the select, like q11a's: the select is a join, so a
// movie with no Film/Warner company simply probes to no row — repeating it as
// a `with` conjunct filters nothing extra and costs a company walk plus the
// `Film|Warner` regex on all 2.5M movies. What is left in `with` is ordered
// cheapest-and-most-selective first: the keyword test alone cuts the drive to
// a few thousand movies, so the year, country and link tests run on almost
// nothing.
//
// Bare `link` leads: only 6.4k of the 2.5M movies have one, against ~10k for
// the keyword, and its test is a CSR-emptiness check rather than a walk of
// the movie's keyword ids. `lk` in the select makes it redundant, so it costs
// nothing but the ordering.
fn q21(countries: Vec<&'static str>, ylo: i64, yhi: i64) -> impl Drive<R: Row> {
    movie.with(link
          .and(keyword.eq("sequel"))
          .and(production_year.between(ylo, yhi))
          .and(info.is_in(countries))
          .and(follow_link()))
       .select(film_or_warner_co().name()
          .and(follow_link())
          .and(title))
}

fn q21a() -> impl Drive<R: Row> { q21(nordic8(), 1950, 2000) }
fn q21b() -> impl Drive<R: Row> { q21(vec!["Germany", "German"], 2000, 2010) }
fn q21c() -> impl Drive<R: Row> { q21(nordic9(), 1950, 2010) }

fn q23a() -> impl Drive<R: Row> {
    movie.with(complete_cast.status().eq("complete+verified")
          .and(company.country().eq("[us]"))
          .and(info.select(Info::ty.eq("release dates")
                      .and(Info::note.rx(r"internet"))
                      .and(Info::info.rx(r"^USA:.* 199|^USA:.* 200"))))
          .and(k_23ab())
          .and(keyword)
          .and(production_year.gt(2000)))
       .select(k_23ab().and(title))
}

fn q23b() -> impl Drive<R: Row> {
    movie.with(complete_cast.status().eq("complete+verified")
          .and(company.country().eq("[us]"))
          .and(info.select(Info::ty.eq("release dates")
                      .and(Info::note.rx(r"internet"))
                      .and(Info::info.rx(r"^USA:.* 200"))))
          .and(k_23ab())
          .and(keyword.is_in(["nerd", "loner", "alienation", "dignity"]))
          .and(production_year.gt(2000)))
       .select(k_23ab().and(title))
}

fn q23c() -> impl Drive<R: Row> {
    movie.with(complete_cast.status().eq("complete+verified")
          .and(company.country().eq("[us]"))
          .and(info.select(Info::ty.eq("release dates")
                      .and(Info::note.rx(r"internet"))
                      .and(Info::info.rx(r"^USA:.* 199|^USA:.* 200"))))
          .and(k_23c())
          .and(keyword)
          .and(production_year.gt(1990)))
       .select(k_23c().and(title))
}

fn q24a() -> impl Drive<R: Row> {
    movie.with(company.country().eq("[us]")
          .and(info.select(Info::ty.eq("release dates")
                      .and(Info::info.rx(r"^Japan:.*201|^USA:.*201"))))
          .and(keyword.is_in(["hero", "martial-arts", "hand-to-hand-combat"]))
          .and(production_year.gt(2010)))
       .select(cast
             .with(Cast::note.is_in(voice4())
              .and(role.eq("actress"))
              .and(person.with(gender.eq("f")
                          .and(Person::name.rx(r"An"))
                          .and(alias))))
             .select(character.text()
                .and(person.name()))
          .and(title))
}

fn q24b() -> impl Drive<R: Row> {
    movie.with(company.select(country.eq("[us]")
                         .and(Company::name.eq("DreamWorks Animation")))
          .and(info.select(Info::ty.eq("release dates")
                      .and(Info::info.rx(r"^Japan:.*201|^USA:.*201"))))
          .and(keyword.is_in(["hero", "martial-arts", "hand-to-hand-combat", "computer-animated-movie"]))
          .and(production_year.gt(2010))
          .and(title.rx(r"^Kung Fu Panda")))
       .select(cast
             .with(Cast::note.is_in(voice4())
              .and(role.eq("actress"))
              .and(person.with(gender.eq("f")
                          .and(Person::name.rx(r"An"))
                          .and(alias))))
             .select(character.text()
                .and(person.name()))
          .and(title))
}

fn q25a() -> impl Drive<R: Row> {
    movie.with(info.with(gf_25ab())
          .and(keyword.is_in(["murder", "blood", "gore", "death", "female-nudity"])))
       .select(info.with(gf_25ab()).info()
          .and(data.with(Data::ty.eq("votes")).text())
          .and(title)
          .and(cast.with(Cast::note.is_in(writer5())
                    .and(person.with(gender.eq("m")))).person().name()))
}

fn q25b() -> impl Drive<R: Row> {
    movie.with(info.with(gf_25ab())
          .and(keyword.is_in(["murder", "blood", "gore", "death", "female-nudity"]))
          .and(production_year.gt(2010))
          .and(title.rx(r"^Vampire")))
       .select(info.with(gf_25ab()).info()
          .and(data.with(Data::ty.eq("votes")).text())
          .and(title)
          .and(cast.with(Cast::note.is_in(writer5())
                    .and(person.with(gender.eq("m")))).person().name()))
}

fn q25c() -> impl Drive<R: Row> {
    movie.with(info.with(gf_25c())
          .and(keyword.is_in(kw7())))
       .select(info.with(gf_25c()).info()
          .and(data.with(Data::ty.eq("votes")).text())
          .and(title)
          .and(cast.with(Cast::note.is_in(writer5())
                    .and(person.with(gender.eq("m")))).person().name()))
}

fn q26a() -> impl Drive<R: Row> {
    movie.with(complete_cast.select(subject.eq("cast")
                               .and(status.rx(r"complete")))
          .and(keyword.is_in(kw10()))
          .and(kind.eq("movie"))
          .and(production_year.gt(2000)))
       .select(cast.with(character.rx(r"[Mm]an"))
             .select(character.text()
                .and(person.name()))
          .and(data.with(Data::ty.eq("rating")
                    .and(Data::text.gt("7.0"))).text())
          .and(title))
}

fn q26b() -> impl Drive<R: Row> {
    movie.with(complete_cast.select(subject.eq("cast")
                               .and(status.rx(r"complete")))
          .and(keyword.is_in(["superhero", "marvel-comics", "based-on-comic", "fight"]))
          .and(kind.eq("movie"))
          .and(production_year.gt(2005)))
       .select(cast.with(character.rx(r"[Mm]an"))
             .character().text()
          .and(data.with(Data::ty.eq("rating")
                    .and(Data::text.gt("8.0"))).text())
          .and(title))
}

fn q26c() -> impl Drive<R: Row> {
    let rd = data.with(Data::ty.eq("rating")).text();
    movie.with(complete_cast.select(subject.eq("cast")
                               .and(status.rx(r"complete")))
          .and(keyword.is_in(kw10()))
          .and(kind.eq("movie"))
          .and(production_year.gt(2000)))
       .select(cast.with(character.rx(r"[Mm]an"))
             .character().text()
          .and(rd)
          .and(title))
}

fn q27a() -> impl Drive<R: Row> {
    movie.with(link
          .and(keyword.eq("sequel"))
          .and(production_year.between(1950, 2000))
          .and(info.select(Info::info.is_in(["Sweden", "Germany", "Swedish", "German"])))
          .and(complete_cast.select(subject.is_in(["cast", "crew"])
                               .and(status.eq("complete"))))
          .and(follow_link()))
       .select(film_or_warner_co().name()
          .and(follow_link())
          .and(title))
}

fn q27b() -> impl Drive<R: Row> {
    movie.with(link
          .and(keyword.eq("sequel"))
          .and(production_year.eq(1998))
          .and(info.select(Info::info.is_in(["Sweden", "Germany", "Swedish", "German"])))
          .and(complete_cast.select(subject.is_in(["cast", "crew"])
                               .and(status.eq("complete"))))
          .and(follow_link()))
       .select(film_or_warner_co().name()
          .and(follow_link())
          .and(title))
}

fn q27c() -> impl Drive<R: Row> {
    movie.with(link
          .and(keyword.eq("sequel"))
          .and(production_year.between(1950, 2010))
          .and(info.select(Info::info.is_in(nordic9())))
          .and(complete_cast.select(subject.eq("cast")
                               .and(status.rx(r"^complete"))))
          .and(follow_link()))
       .select(film_or_warner_co().name()
          .and(follow_link())
          .and(title))
}

fn q28a() -> impl Drive<R: Row> {
    movie.with(complete_cast.select(subject.eq("crew")
                               .and(status.ne("complete+verified")))
          .and(co_28())
          .and(info.select(Info::ty.eq("countries")
                      .and(Info::info.is_in(nordic10()))))
          .and(dt_28ac())
          .and(keyword.is_in(murder4()))
          .and(kind.is_in(["movie", "episode"]))
          .and(production_year.gt(2000)))
       .select(co_28().name()
          .and(dt_28ac().text())
          .and(title))
}

fn q28b() -> impl Drive<R: Row> {
    movie.with(complete_cast.select(subject.eq("crew")
                               .and(status.ne("complete+verified")))
          .and(co_28())
          .and(info.select(Info::ty.eq("countries")
                      .and(Info::info.is_in(["Sweden", "Germany", "Swedish", "German"]))))
          .and(dt_28b())
          .and(keyword.is_in(murder4()))
          .and(kind.is_in(["movie", "episode"]))
          .and(production_year.gt(2005)))
       .select(co_28().name()
          .and(dt_28b().text())
          .and(title))
}

fn q28c() -> impl Drive<R: Row> {
    movie.with(complete_cast.select(subject.eq("cast")
                               .and(status.eq("complete")))
          .and(co_28())
          .and(info.select(Info::ty.eq("countries")
                      .and(Info::info.is_in(nordic10()))))
          .and(dt_28ac())
          .and(keyword.is_in(murder4()))
          .and(kind.is_in(["movie", "episode"]))
          .and(production_year.gt(2005)))
       .select(co_28().name()
          .and(dt_28ac().text())
          .and(title))
}

fn q29a() -> impl Drive<R: Row> {
    movie.with(complete_cast.select(subject.eq("cast")
                               .and(status.eq("complete+verified")))
          .and(company.country().eq("[us]"))
          .and(info.select(Info::ty.eq("release dates")
                      .and(Info::info.rx(r"^Japan:.*200|^USA:.*200"))))
          .and(keyword.eq("computer-animation"))
          .and(title.eq("Shrek 2"))
          .and(production_year.ge(2000))
          .and(production_year.le(2010)))
       .select(cast
             .with(Cast::note.is_in(voice3())
              .and(role.eq("actress"))
              .and(character.eq("Queen"))
              .and(person.with(gender.eq("f")
                          .and(Person::name.rx(r"An"))
                          .and(alias)
                          .and(bio.select(PersonInfo::ty.eq("trivia"))))))
             .select(character.text()
                .and(person.name()))
          .and(title))
}

fn q29b() -> impl Drive<R: Row> {
    movie.with(complete_cast.select(subject.eq("cast")
                               .and(status.eq("complete+verified")))
          .and(company.country().eq("[us]"))
          .and(info.select(Info::ty.eq("release dates")
                      .and(Info::info.rx(r"^USA:.*200"))))
          .and(keyword.eq("computer-animation"))
          .and(title.eq("Shrek 2"))
          .and(production_year.ge(2000))
          .and(production_year.le(2005)))
       .select(cast
             .with(Cast::note.is_in(voice3())
              .and(role.eq("actress"))
              .and(character.eq("Queen"))
              .and(person.with(gender.eq("f")
                          .and(Person::name.rx(r"An"))
                          .and(alias)
                          .and(bio.select(PersonInfo::ty.eq("height"))))))
             .select(character.text()
                .and(person.name()))
          .and(title))
}

fn q29c() -> impl Drive<R: Row> {
    movie.with(complete_cast.select(subject.eq("cast")
                               .and(status.eq("complete+verified")))
          .and(company.country().eq("[us]"))
          .and(info.select(Info::ty.eq("release dates")
                      .and(Info::info.rx(r"^Japan:.*200|^USA:.*200"))))
          .and(keyword.eq("computer-animation"))
          .and(production_year.ge(2000))
          .and(production_year.le(2010)))
       .select(cast
             .with(Cast::note.is_in(voice4())
              .and(role.eq("actress"))
              .and(person.with(gender.eq("f")
                          .and(Person::name.rx(r"An"))
                          .and(alias)
                          .and(bio.select(PersonInfo::ty.eq("trivia"))))))
             .select(character.text()
                .and(person.name()))
          .and(title))
}

fn q30a() -> impl Drive<R: Row> {
    movie.with(complete_cast.select(subject.is_in(["cast", "crew"])
                               .and(status.eq("complete+verified")))
          .and(info.with(gf_horror()))
          .and(keyword.is_in(kw7()))
          .and(production_year.gt(2000)))
       .select(info.with(gf_horror()).info()
          .and(data.with(Data::ty.eq("votes")).text())
          .and(title)
          .and(cast.with(Cast::note.is_in(writer5())
                    .and(person.with(gender.eq("m")))).person().name()))
}

fn q30b() -> impl Drive<R: Row> {
    movie.with(complete_cast.select(subject.is_in(["cast", "crew"])
                               .and(status.eq("complete+verified")))
          .and(info.with(gf_horror()))
          .and(keyword.is_in(kw7()))
          .and(production_year.gt(2000))
          .and(title.rx(r"Freddy|Jason|^Saw")))
       .select(info.with(gf_horror()).info()
          .and(data.with(Data::ty.eq("votes")).text())
          .and(title)
          .and(cast.with(Cast::note.is_in(writer5())
                    .and(person.with(gender.eq("m")))).person().name()))
}

fn q30c() -> impl Drive<R: Row> {
    movie.with(complete_cast.select(subject.eq("cast")
                               .and(status.eq("complete+verified")))
          .and(info.with(gf_genre6()))
          .and(keyword.is_in(kw7())))
       .select(info.with(gf_genre6()).info()
          .and(data.with(Data::ty.eq("votes")).text())
          .and(title)
          .and(cast.with(Cast::note.is_in(writer5())
                    .and(person.with(gender.eq("m")))).person().name()))
}

fn q31a() -> impl Drive<R: Row> {
    movie.with(company.rx(r"^Lionsgate")
          .and(info.with(gf_horror()))
          .and(keyword.is_in(kw7())))
       .select(info.with(gf_horror()).info()
          .and(data.with(Data::ty.eq("votes")).text())
          .and(title)
          .and(cast.with(Cast::note.is_in(writer5())
                    .and(person.with(gender.eq("m")))).person().name()))
}

fn q31b() -> impl Drive<R: Row> {
    movie.with(company.select(Company::name.rx(r"^Lionsgate")
                         .and(Company::note.rx(r"\(Blu-ray\)")))
          .and(info.with(gf_horror()))
          .and(keyword.is_in(kw7()))
          .and(production_year.gt(2000))
          .and(title.rx(r"Freddy|Jason|^Saw")))
       .select(info.with(gf_horror()).info()
          .and(data.with(Data::ty.eq("votes")).text())
          .and(title)
          .and(cast.with(Cast::note.is_in(writer5())
                    .and(person.with(gender.eq("m")))).person().name()))
}

fn q31c() -> impl Drive<R: Row> {
    movie.with(company.rx(r"^Lionsgate")
          .and(info.with(gf_genre6()))
          .and(keyword.is_in(kw7())))
       .select(info.with(gf_genre6()).info()
          .and(data.with(Data::ty.eq("votes")).text())
          .and(title)
          .and(cast.with(Cast::note.is_in(writer5())).person().name()))
}

// q32a/q32b differ only in the keyword constant.
fn q32(kw: &'static str) -> impl Drive<R: Row> {
    movie.with(link
          .and(keyword.eq(kw)))
       .select(link.ty().text()
          .and(title)
          .and(link.target().title()))
}

fn q32a() -> impl Drive<R: Row> { q32("10,000-mile-club") }
fn q32b() -> impl Drive<R: Row> { q32("character-name-in-title") }

fn q33a() -> impl Drive<R: Row> {
    movie.with(kind.eq("tv series")
          .and(company.country().eq("[us]"))
          .and(qlink_33a()))
       .select(company.with(country.eq("[us]")).name()
          .and(qlink_33a().target().company().name())
          .and(data.with(Data::ty.eq("rating")).text())
          .and(qlink_33a().target().select(data.with(Data::ty.eq("rating")
                                                .and(Data::text.lt("3.0"))).text()))
          .and(title)
          .and(qlink_33a().target().title()))
}

fn q33b() -> impl Drive<R: Row> {
    movie.with(kind.eq("tv series")
          .and(company.country().eq("[nl]"))
          .and(qlink_33b()))
       .select(company.with(country.eq("[nl]")).name()
          .and(qlink_33b().target().company().name())
          .and(data.with(Data::ty.eq("rating")).text())
          .and(qlink_33b().target().select(data.with(Data::ty.eq("rating")
                                                .and(Data::text.lt("3.0"))).text()))
          .and(title)
          .and(qlink_33b().target().title()))
}

fn q33c() -> impl Drive<R: Row> {
    movie.with(kind.is_in(["tv series", "episode"])
          .and(company.country().ne("[us]"))
          .and(qlink_33c()))
       .select(company.with(country.ne("[us]")).name()
          .and(qlink_33c().target().company().name())
          .and(data.with(Data::ty.eq("rating")).text())
          .and(qlink_33c().target().select(data.with(Data::ty.eq("rating")
                                                .and(Data::text.lt("3.5"))).text()))
          .and(title)
          .and(qlink_33c().target().title()))
}
