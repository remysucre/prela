// All 113 JOB queries, plus the method-chain demo. The section comments
// below mark which templates each block covers.
//
// Each query takes the loaded database and opens by destructuring the
// entities it touches (`let Movie { title, keyword, .. } = &db.movie;`).
// The bindings are the relations themselves, so the combinators hang off
// them directly. `db` is `&'static` (main leaks it once), which is why the
// plans these functions return carry no lifetime and `ENTRIES` can stay a
// `const` array of fn pointers.

use crate::engine::*;
use crate::job_queries::helpers::{Row, film_or_warner_co, follow_link, min_row};
#[cfg(all(test, feature = "test"))]
use crate::job_queries::helpers::min_result;
use crate::job_queries::sets::{
    genre6, kw7, kw8, kw10, link3, murder4, nordic8, nordic9, nordic10, voice3, voice4, writer5,
};
use crate::job_schema::*;

pub const ENTRIES: &[super::Entry] = &[
    // --- templates 1-5, 11-15, 22 ---
    ("2a", "'Doc'", |db| min_row(q2a(db))),
    ("2d", "& Teller", |db| min_row(q2d(db))),
    ("3b", "300: Rise of an Empire", |db| min_row(q3b(db))),
    ("4a", "5.1 || & Teller 2", |db| min_row(q4a(db))),
    ("13a", "Afghanistan:24 June 2012 || 1.0 || &Me", |db| {
        min_row(q13a(db))
    }),
    (
        "11a",
        "Churchill Films || followed by || Batman Beyond",
        |db| min_row(q11a(db)),
    ),
    ("22a", "(empty)", |db| min_row(q22a(db))),
    (
        "1a",
        "(A Warner Bros.-First National Picture) (presents) || A Clockwork Orange || 1934",
        |db| min_row(q1a(db)),
    ),
    ("5a", "(empty)", |db| min_row(q5a(db))),
    ("12a", "10th Grade Reunion Films || 8.1 || 3:20", |db| {
        min_row(q12a(db))
    }),
    ("14a", "1.0 || $lowdown", |db| min_row(q14a(db))),
    (
        "1b",
        "(Set Decoration Rentals) (uncredited) || Disaster Movie || 2008",
        |db| min_row(q1b(db)),
    ),
    ("2b", "'Doc'", |db| min_row(q2b(db))),
    ("2c", "(empty)", |db| min_row(q2c(db))),
    ("3a", "2 Days in New York", |db| min_row(q3a(db))),
    ("3c", "& Teller 2", |db| min_row(q3c(db))),
    ("4b", "9.1 || Batman: Arkham City", |db| min_row(q4b(db))),
    (
        "11b",
        "Filmlance International AB || follows || The Money Man",
        |db| min_row(q11b(db)),
    ),
    ("13b", "501audio || 1.8 || 5 Time Champion", |db| {
        min_row(q13b(db))
    }),
    ("1c", "(co-production) || Intouchables || 2011", |db| {
        min_row(q1c(db))
    }),
    (
        "1d",
        "(Set Decoration Rentals) (uncredited) || Disaster Movie || 2004",
        |db| min_row(q1d(db)),
    ),
    ("4c", "2.1 || & Teller 2", |db| min_row(q4c(db))),
    ("12b", "$10,000 || Birdemic: Shock and Terror", |db| {
        min_row(q12b(db))
    }),
    ("12c", "\"Oh That Gus!\" || 7.1 || $1.11", |db| {
        min_row(q12c(db))
    }),
    ("13c", "DL Sites || 1.8 || Champion", |db| min_row(q13c(db))),
    ("14b", "6.4 || Of Dolls and Murder", |db| min_row(q14b(db))),
    ("14c", "1.0 || $lowdown", |db| min_row(q14c(db))),
    ("22b", "(empty)", |db| min_row(q22b(db))),
    ("22c", "(empty)", |db| min_row(q22c(db))),
    // --- 22d, 5b, 5c, 15a-d, 11c-d, 13d, 6a-f ---
    ("22d", "(#1.1) || 2.0 || 13 Productions", |db| {
        min_row(q22d(db))
    }),
    ("5b", "(empty)", |db| min_row(q5b(db))),
    ("5c", "11,830,420", |db| min_row(q5c(db))),
    (
        "15a",
        "USA:1 June 2007 || Battlestar Galactica: The Resistance",
        |db| min_row(q15a(db)),
    ),
    ("15b", "USA:27 April 2007 || RoboCop vs Terminator", |db| {
        min_row(q15b(db))
    }),
    ("15c", "USA:1 April 2003 || 24: Day Six - Debrief", |db| {
        min_row(q15c(db))
    }),
    ("15d", "(Not So) Instant Photo || 06/05", |db| {
        min_row(q15d(db))
    }),
    (
        "11c",
        "20th Century Fox Home Entertainment || (1997-2002) (worldwide) (all media) || 24",
        |db| min_row(q11c(db)),
    ),
    (
        "11d",
        "13th Street || (1954) (UK) (TV) || ...denn sie wissen nicht, was sie tun",
        |db| min_row(q11d(db)),
    ),
    ("13d", "\"O\" Films || 1.0 || #54 Meets #47", |db| {
        min_row(q13d(db))
    }),
    (
        "6a",
        "marvel-cinematic-universe || Downey Jr., Robert || Iron Man 3",
        |db| min_row(q6a(db)),
    ),
    (
        "6b",
        "based-on-comic || Downey Jr., Robert || The Avengers 2",
        |db| min_row(q6b(db)),
    ),
    (
        "6c",
        "marvel-cinematic-universe || Downey Jr., Robert || The Avengers 2",
        |db| min_row(q6c(db)),
    ),
    (
        "6d",
        "based-on-comic || Downey Jr., Robert || 2008 MTV Movie Awards",
        |db| min_row(q6d(db)),
    ),
    (
        "6e",
        "marvel-cinematic-universe || Downey Jr., Robert || Iron Man 3",
        |db| min_row(q6e(db)),
    ),
    (
        "6f",
        "based-on-comic || \"Steff\", Stefanie Oxmann Mcgaha || & Teller 2",
        |db| min_row(q6f(db)),
    ),
    // --- 7a-c, 8a-d, 9a-d, 10a-c ---
    ("7a", "Antonioni, Michelangelo || Dressed to Kill", |db| {
        min_row(q7a(db))
    }),
    ("7b", "De Palma, Brian || Dressed to Kill", |db| {
        min_row(q7b(db))
    }),
    (
        "7c",
        "50 Cent || \"Boo\" Arnold was born Earl Arnold in Hattiesburg, Mississippi in 1966. His father gave him the nickname 'Boo' early in life and it stuck through grade school, high school, and college. He is still known as \"Boo\" to family and friends.  Raised in central Texas, Arnold played baseball at Texas Tech University where he graduated with a BA in Advertising and Marketing. While at Texas Tech he was also a member of the Texas Epsilon chapter of Phi Delta Theta fraternity. After college he worked with Young Life, an outreach to high school students, in San Antonio, Texas.  While with Young Life Arnold began taking extension courses through Fuller Theological Seminary and ultimately went full-time to Gordon-Conwell Theological Seminary in Boston, Massachusetts. At Gordon-Conwell he completed a Master's Degree in Divinity studying Theology, Philosophy, Church History, Biblical Languages (Hebrew & Greek), and Exegetical Methods. Following seminary he was involved with reconciliation efforts in the former Yugoslavia shortly after the war ended there in1995.  Arnold started acting in his early thirties in Texas. After an encouraging visit to Los Angeles where he spent time with childhood friend George Eads (of CSI Las Vegas) he decided to move to Los Angeles in 2001 to pursue acting full-time. While in Los Angeles he has studied acting with Judith Weston at Judith Weston Studio for Actors and Directors.  Arnold's acting career has been one of steady development, booking co-star and guest-star roles in nighttime television. He guest-starred opposite of Jane Seymour on the night time television drama Justice. He played the lead, Michael Hollister, in the film The Seer, written and directed by Patrick Masset (Friday Night Lights).  He was nominated Best Actor in the168 Film Festival for the role of Phil Stevens in the short-film Useless. In Useless he played a US Marshal who must choose between mercy and justice as he confronts the man who murdered his father. Arnold's performance in Useless confirmed his ability to carry lead roles, and he continues to work toward solidifying himself as a male lead in film and television.  Arnold married fellow Texan Stacy Rudd of San Antonio in 2003 and they are now raising their three children in the Los Angeles area.",
        |db| min_row(q7c(db)),
    ),
    ("8a", "Chambers, Linda || .hack//Quantum", |db| {
        min_row(q8a(db))
    }),
    (
        "8b",
        "Chambers, Linda || Dragon Ball Z: Shin Budokai",
        |db| min_row(q8b(db)),
    ),
    ("8c", "\"A.J.\" || #1 Cheerleader Camp", |db| {
        min_row(q8c(db))
    }),
    (
        "8d",
        "\"Jenny from the Block\" || #1 Cheerleader Camp",
        |db| min_row(q8d(db)),
    ),
    ("9a", "AJ || Airport Announcer || Blue Harvest", |db| {
        min_row(q9a(db))
    }),
    (
        "9b",
        "AJ || Airport Announcer || Bassett, Angela || Blue Harvest",
        |db| min_row(q9b(db)),
    ),
    (
        "9c",
        "'Annette' || 2nd Balladeer || Alborg, Ana Esther || (1975-01-20)",
        |db| min_row(q9c(db)),
    ),
    (
        "9d",
        "!!!, Toy || Aaron, Caroline || \"Cockamamie's\" Salesgirl || $15,000.00 Error",
        |db| min_row(q9d(db)),
    ),
    ("10a", "Actor || 12 Rounds", |db| min_row(q10a(db))),
    ("10b", "(empty)", |db| min_row(q10b(db))),
    ("10c", "Himself || Evil Eyes: Behind the Scenes", |db| {
        min_row(q10c(db))
    }),
    // --- templates 16-18 ---
    (
        "16a",
        "Adams, Stan || Carol Burnett vs. Anthony Perkins",
        |db| min_row(q16a(db)),
    ),
    ("16b", "!!!, Toy || & Teller", |db| min_row(q16b(db))),
    ("16c", "\"Brooklyn\" Tony Danza || (#1.5)", |db| {
        min_row(q16c(db))
    }),
    ("16d", "\"Brooklyn\" Tony Danza || (#1.5)", |db| {
        min_row(q16d(db))
    }),
    ("17a", "B, Khaz", |db| min_row(q17a(db))),
    ("17b", "Z'Dar, Robert", |db| min_row(q17b(db))),
    ("17c", "X'Volaitis, John", |db| min_row(q17c(db))),
    ("17d", "Abrahamsson, Bertil", |db| min_row(q17d(db))),
    ("17e", "$hort, Too", |db| min_row(q17e(db))),
    ("17f", "'El Galgo PornoStar', Blanquito", |db| {
        min_row(q17f(db))
    }),
    ("18a", "$1,000 || 10 || 40 Days and 40 Nights", |db| {
        min_row(q18a(db))
    }),
    ("18b", "Horror || 8.1 || Agorable", |db| min_row(q18b(db))),
    ("18c", "Action || 10 || #PostModem", |db| min_row(q18c(db))),
    // --- 19a-26c ---
    ("19a", "Angeline, Moriah || Blue Harvest", |db| {
        min_row(q19a(db))
    }),
    ("19b", "Jolie, Angelina || Kung Fu Panda", |db| {
        min_row(q19b(db))
    }),
    (
        "19c",
        "Alborg, Ana Esther || .hack//Akusei heni vol. 2",
        |db| min_row(q19c(db)),
    ),
    ("19d", "Aaron, Caroline || $9.99", |db| min_row(q19d(db))),
    ("20a", "Disaster Movie", |db| min_row(q20a(db))),
    ("20b", "Iron Man", |db| min_row(q20b(db))),
    ("20c", "Abell, Alistair || ...And Then I...", |db| {
        min_row(q20c(db))
    }),
    (
        "21a",
        "Det Danske Filminstitut || followed by || Der Serienkiller - Klinge des Todes",
        |db| min_row(q21a(db)),
    ),
    (
        "21b",
        "Filmlance International AB || followed by || Hämndens pris",
        |db| min_row(q21b(db)),
    ),
    (
        "21c",
        "Churchill Films || followed by || Batman Beyond",
        |db| min_row(q21c(db)),
    ),
    ("23a", "movie || The Analysts", |db| min_row(q23a(db))),
    ("23b", "movie || The Big Mope", |db| min_row(q23b(db))),
    ("23c", "movie || Dirt Merchant", |db| min_row(q23c(db))),
    (
        "24a",
        "Additional Voices || Baker, Andrea || Baiohazâdo 6",
        |db| min_row(q24a(db)),
    ),
    (
        "24b",
        "Tigress || Jolie, Angelina || Kung Fu Panda 2",
        |db| min_row(q24b(db)),
    ),
    (
        "25a",
        "Horror || 10 || -- And Now the Screaming Starts! || Abdallah, Damon",
        |db| min_row(q25a(db)),
    ),
    (
        "25b",
        "Horror || 138 || Vampire Boys || Campbell, Jeremiah",
        |db| min_row(q25b(db)),
    ),
    ("25c", "Action || 10 || $ || Aakeson, Kim Fupz", |db| {
        min_row(q25c(db))
    }),
    (
        "26a",
        "'Agua' Man || Acereda, Hermie || 7.1 || 3:10 to Yuma",
        |db| min_row(q26a(db)),
    ),
    ("26b", "Bank Manager || 8.2 || Inception", |db| {
        min_row(q26b(db))
    }),
    ("26c", "'Agua' Man || 1.9 || 12 Rounds", |db| {
        min_row(q26c(db))
    }),
    // --- 27a-33c ---
    (
        "27a",
        "Det Danske Filminstitut || followed by || Spår i mörker",
        |db| min_row(q27a(db)),
    ),
    (
        "27b",
        "Filmlance International AB || followed by || Vita nätter",
        |db| min_row(q27b(db)),
    ),
    (
        "27c",
        "Det Danske Filminstitut || followed by || Spår i mörker",
        |db| min_row(q27c(db)),
    ),
    ("28a", "01 Distribuzione || 2.9 || (#1.1)", |db| {
        min_row(q28a(db))
    }),
    ("28b", "20th Century Fox || 6.6 || (#1.1)", |db| {
        min_row(q28b(db))
    }),
    ("28c", "01 Distribuzione || 1.9 || (#1.1)", |db| {
        min_row(q28c(db))
    }),
    ("29a", "Queen || Andrews, Julie || Shrek 2", |db| {
        min_row(q29a(db))
    }),
    ("29b", "Queen || Andrews, Julie || Shrek 2", |db| {
        min_row(q29b(db))
    }),
    ("29c", "Lola || Andrews, Julie || Hoodwinked!", |db| {
        min_row(q29c(db))
    }),
    (
        "30a",
        "Horror || 100356 || 16 Blocks || Abrams, J.J.",
        |db| min_row(q30a(db)),
    ),
    (
        "30b",
        "Horror || 194782 || Freddy vs. Jason || Shannon, Damian",
        |db| min_row(q30b(db)),
    ),
    ("30c", "Action || 100356 || $ || Abernathy, Lewis", |db| {
        min_row(q30c(db))
    }),
    (
        "31a",
        "Horror || 1040 || 2001 Maniacs || Agnew, Jim",
        |db| min_row(q31a(db)),
    ),
    (
        "31b",
        "Horror || 129755 || Saw || Bousman, Darren Lynn",
        |db| min_row(q31b(db)),
    ),
    ("31c", "Action || 1008 || 11:14 || Abraham, Brad", |db| {
        min_row(q31c(db))
    }),
    ("32a", "(empty)", |db| min_row(q32a(db))),
    (
        "32b",
        "alternate language version of || 12 oz. Mouse || 'Angel': Season 2 Overview",
        |db| min_row(q32b(db)),
    ),
    (
        "33a",
        "495 Productions || 495 Productions || 3.3 || 2.7 || A Double Shot at Love || A Shot at Love with Tila Tequila",
        |db| min_row(q33a(db)),
    ),
    (
        "33b",
        "MTV Netherlands || 495 Productions || 3.3 || 2.7 || A Double Shot at Love || A Shot at Love with Tila Tequila",
        |db| min_row(q33b(db)),
    ),
    (
        "33c",
        "2BE || 495 Productions || 1.3 || 1.0 || A Double Shot at Love || A Double Shot at Love",
        |db| min_row(q33c(db)),
    ),
    // --- method-chain demo ---
    (
        "6a/method",
        "marvel-cinematic-universe || Downey Jr., Robert || Iron Man 3",
        |db| min_row(q6a_methods(db)),
    ),
];

#[cfg(all(test, feature = "test"))]
pub(crate) fn differential(
    name: &str,
    db: &'static Job,
) -> Result<crate::test::result::ResultSet, String> {
    let result = match name {
        "1a" => min_result(q1a(db), 3),
        "1b" => min_result(q1b(db), 3),
        "1c" => min_result(q1c(db), 3),
        "1d" => min_result(q1d(db), 3),
        "2a" => min_result(q2a(db), 1),
        "2b" => min_result(q2b(db), 1),
        "2c" => min_result(q2c(db), 1),
        "2d" => min_result(q2d(db), 1),
        "3a" => min_result(q3a(db), 1),
        "3b" => min_result(q3b(db), 1),
        "3c" => min_result(q3c(db), 1),
        "4a" => min_result(q4a(db), 2),
        "4b" => min_result(q4b(db), 2),
        "4c" => min_result(q4c(db), 2),
        "5a" => min_result(q5a(db), 1),
        "5b" => min_result(q5b(db), 1),
        "5c" => min_result(q5c(db), 1),
        "6a" => min_result(q6a(db), 3),
        "6b" => min_result(q6b(db), 3),
        "6c" => min_result(q6c(db), 3),
        "6d" => min_result(q6d(db), 3),
        "6e" => min_result(q6e(db), 3),
        "6f" => min_result(q6f(db), 3),
        "7a" => min_result(q7a(db), 2),
        "7b" => min_result(q7b(db), 2),
        "7c" => min_result(q7c(db), 2),
        "8a" => min_result(q8a(db), 2),
        "8b" => min_result(q8b(db), 2),
        "8c" => min_result(q8c(db), 2),
        "8d" => min_result(q8d(db), 2),
        "9a" => min_result(q9a(db), 3),
        "9b" => min_result(q9b(db), 4),
        "9c" => min_result(q9c(db), 4),
        "9d" => min_result(q9d(db), 4),
        "10a" => min_result(q10a(db), 2),
        "10b" => min_result(q10b(db), 2),
        "10c" => min_result(q10c(db), 2),
        "11a" => min_result(q11a(db), 3),
        "11b" => min_result(q11b(db), 3),
        "11c" => min_result(q11c(db), 3),
        "11d" => min_result(q11d(db), 3),
        "12a" => min_result(q12a(db), 3),
        "12b" => min_result(q12b(db), 2),
        "12c" => min_result(q12c(db), 3),
        "13a" => min_result(q13a(db), 3),
        "13b" => min_result(q13b(db), 3),
        "13c" => min_result(q13c(db), 3),
        "13d" => min_result(q13d(db), 3),
        "14a" => min_result(q14a(db), 2),
        "14b" => min_result(q14b(db), 2),
        "14c" => min_result(q14c(db), 2),
        "15a" => min_result(q15a(db), 2),
        "15b" => min_result(q15b(db), 2),
        "15c" => min_result(q15c(db), 2),
        "15d" => min_result(q15d(db), 2),
        "16a" => min_result(q16a(db), 2),
        "16b" => min_result(q16b(db), 2),
        "16c" => min_result(q16c(db), 2),
        "16d" => min_result(q16d(db), 2),
        // The SQL projects the same MIN(name) twice under distinct aliases.
        "17a" => min_result(q17a(db).map(|name| (name, name)), 2),
        "17b" => min_result(q17b(db).map(|name| (name, name)), 2),
        "17c" => min_result(q17c(db).map(|name| (name, name)), 2),
        "17d" => min_result(q17d(db), 1),
        "17e" => min_result(q17e(db), 1),
        "17f" => min_result(q17f(db), 1),
        "18a" => min_result(q18a(db), 3),
        "18b" => min_result(q18b(db), 3),
        "18c" => min_result(q18c(db), 3),
        "19a" => min_result(q19a(db), 2),
        "19b" => min_result(q19b(db), 2),
        "19c" => min_result(q19c(db), 2),
        "19d" => min_result(q19d(db), 2),
        "20a" => min_result(q20a(db), 1),
        "20b" => min_result(q20b(db), 1),
        "20c" => min_result(q20c(db), 2),
        "21a" => min_result(q21a(db), 3),
        "21b" => min_result(q21b(db), 3),
        "21c" => min_result(q21c(db), 3),
        "22a" => min_result(q22a(db), 3),
        "22b" => min_result(q22b(db), 3),
        "22c" => min_result(q22c(db), 3),
        "22d" => min_result(q22d(db), 3),
        "23a" => min_result(q23a(db), 2),
        "23b" => min_result(q23b(db), 2),
        "23c" => min_result(q23c(db), 2),
        "24a" => min_result(q24a(db), 3),
        "24b" => min_result(q24b(db), 3),
        "25a" => min_result(q25a(db), 4),
        "25b" => min_result(q25b(db), 4),
        "25c" => min_result(q25c(db), 4),
        "26a" => min_result(q26a(db), 4),
        "26b" => min_result(q26b(db), 3),
        "26c" => min_result(q26c(db), 3),
        "27a" => min_result(q27a(db), 3),
        "27b" => min_result(q27b(db), 3),
        "27c" => min_result(q27c(db), 3),
        "28a" => min_result(q28a(db), 3),
        "28b" => min_result(q28b(db), 3),
        "28c" => min_result(q28c(db), 3),
        "29a" => min_result(q29a(db), 3),
        "29b" => min_result(q29b(db), 3),
        "29c" => min_result(q29c(db), 3),
        "30a" => min_result(q30a(db), 4),
        "30b" => min_result(q30b(db), 4),
        "30c" => min_result(q30c(db), 4),
        "31a" => min_result(q31a(db), 4),
        "31b" => min_result(q31b(db), 4),
        "31c" => min_result(q31c(db), 4),
        "32a" => min_result(q32a(db), 3),
        "32b" => min_result(q32b(db), 3),
        "33a" => min_result(q33a(db), 6),
        "33b" => min_result(q33b(db), 6),
        "33c" => min_result(q33c(db), 6),
        _ => return Err(format!("unknown JOB query {name}")),
    };
    Ok(result)
}

// ===== queries: templates 1-5, 11-15, 22 — movie-only =====

// q2a–q2d differ only in the company country code.
fn q2(db: &'static Job, cc: &'static str) -> impl Drive<R: Row> {
    let Movie {
        title,
        keyword,
        company,
        ..
    } = &db.movie;
    let Company { country, .. } = &db.company;
    db.movie
        .with(
            keyword
                .eq("character-name-in-title")
                .and(company.select(country).eq(cc)),
        )
        .select(title)
}

fn q2a(db: &'static Job) -> impl Drive<R: Row> {
    q2(db, "[de]")
}
fn q2b(db: &'static Job) -> impl Drive<R: Row> {
    q2(db, "[nl]")
}
fn q2c(db: &'static Job) -> impl Drive<R: Row> {
    q2(db, "[sm]")
}
fn q2d(db: &'static Job) -> impl Drive<R: Row> {
    q2(db, "[us]")
}

fn q3b(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        keyword,
        info,
        ..
    } = &db.movie;
    let Info {
        info: info_info, ..
    } = &db.info;
    db.movie
        .with(
            keyword
                .rx(r"sequel")
                .and(info.select(info_info).eq("Bulgaria"))
                .and(production_year.gt(2010)),
        )
        .select(title)
}

// q4a–q4c differ only in the year cutoff and rating threshold.
fn q4(db: &'static Job, year: i64, rating: &'static str) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        keyword,
        data,
        ..
    } = &db.movie;
    let Data {
        text: data_text,
        ty: data_ty,
        ..
    } = &db.data;
    db.movie
        .with(keyword.rx_dict(r"sequel").and(production_year.gt(year)))
        .select(
            data.with(data_ty.eq("rating").and(data_text.gt(rating)))
                .select(data_text)
                .and(title),
        )
}

fn q4a(db: &'static Job) -> impl Drive<R: Row> {
    q4(db, 2005, "5.0")
}
fn q4b(db: &'static Job) -> impl Drive<R: Row> {
    q4(db, 2010, "9.0")
}
fn q4c(db: &'static Job) -> impl Drive<R: Row> {
    q4(db, 1990, "2.0")
}

fn q13a(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        kind,
        company,
        info,
        data,
        ..
    } = &db.movie;
    let Company {
        country,
        ty: company_ty,
        ..
    } = &db.company;
    let Data {
        text: data_text,
        ty: data_ty,
        ..
    } = &db.data;
    let Info {
        info: info_info,
        ty: info_ty,
        ..
    } = &db.info;
    db.movie
        .with(
            company
                .select(
                    country
                        .eq("[de]")
                        .and(company_ty.eq("production companies")),
                )
                .and(kind.eq("movie")),
        )
        .select(
            info.with(info_ty.eq("release dates"))
                .select(info_info)
                .and(data.with(data_ty.eq("rating")).select(data_text))
                .and(title),
        )
}

fn q11a(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        keyword,
        company,
        link,
        ..
    } = &db.movie;
    let Company {
        name: company_name,
        country,
        note: company_note,
        ty: company_ty,
        ..
    } = &db.company;
    let MovieLink {
        ty: movielink_ty, ..
    } = &db.movie_link;
    db.movie
        .with(
            link.and(keyword.eq("sequel"))
                .and(production_year.between(1950, 2000)),
        )
        .select(
            company
                .with(
                    country
                        .ne("[pl]")
                        .and(company_name.rx(r"Film|Warner"))
                        .and(company_ty.eq("production companies"))
                        .minus(company_note),
                )
                .select(company_name)
                .and(link.select(movielink_ty).rx(r"follow"))
                .and(title),
        )
}

fn q22a(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        kind,
        production_year,
        keyword,
        company,
        info,
        data,
        ..
    } = &db.movie;
    let Company {
        name: company_name,
        country,
        note: company_note,
        ty: company_ty,
        ..
    } = &db.company;
    let Data {
        text: data_text,
        ty: data_ty,
        ..
    } = &db.data;
    let Info {
        info: info_info,
        ty: info_ty,
        ..
    } = &db.info;
    db.movie
        .with(
            info.select(
                info_ty
                    .eq("countries")
                    .and(info_info.is_in(["Germany", "German", "USA", "American"])),
            )
            .and(keyword.is_in(murder4()))
            .and(production_year.gt(2008))
            .and(kind.is_in(["movie", "episode"])),
        )
        .select(
            title
                .and(
                    data.with(data_text.lt("7.0").and(data_ty.eq("rating")))
                        .select(data_text),
                )
                .and(
                    company
                        .with(
                            company_note
                                .nrx(r"\(USA\)")
                                .and(company_note.rx(r"\(200.*\)"))
                                .and(country.ne("[us]"))
                                .and(company_ty.eq("production companies")),
                        )
                        .select(company_name),
                ),
        )
}

fn q1a(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        company,
        data,
        ..
    } = &db.movie;
    let Company {
        note: company_note,
        ty: company_ty,
        ..
    } = &db.company;
    let Data { ty: data_ty, .. } = &db.data;
    db.movie
        .with(data.select(data_ty).eq("top 250 rank"))
        .select(
            company
                .with(
                    company_ty
                        .eq("production companies")
                        .and(company_note.nrx(r"\(as Metro-Goldwyn-Mayer Pictures\)"))
                        .and(company_note.rx(r"\(co-production\)|\(presents\)")),
                )
                .select(company_note)
                .and(title)
                .and(production_year),
        )
}

fn q5a(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        company,
        info,
        ..
    } = &db.movie;
    let Company {
        note: company_note,
        ty: company_ty,
        ..
    } = &db.company;
    let Info {
        info: info_info, ..
    } = &db.info;
    db.movie
        .with(
            company
                .select(
                    company_ty
                        .eq("production companies")
                        .and(company_note.rx(r"\(theatrical\)"))
                        .and(company_note.rx(r"\(France\)")),
                )
                .and(info.select(info_info).is_in(nordic8()))
                .and(production_year.gt(2005)),
        )
        .select(title)
}

fn q12a(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        company,
        info,
        data,
        ..
    } = &db.movie;
    let Company {
        name: company_name,
        country,
        ty: company_ty,
        ..
    } = &db.company;
    let Data {
        text: data_text,
        ty: data_ty,
        ..
    } = &db.data;
    let Info {
        info: info_info,
        ty: info_ty,
        ..
    } = &db.info;
    db.movie
        .with(
            info.select(
                info_ty
                    .eq("genres")
                    .and(info_info.is_in(["Drama", "Horror"])),
            )
            .and(production_year.ge(2005))
            .and(production_year.le(2008)),
        )
        .select(
            company
                .with(
                    country
                        .eq("[us]")
                        .and(company_ty.eq("production companies")),
                )
                .select(company_name)
                .and(
                    data.with(data_ty.eq("rating").and(data_text.gt("8.0")))
                        .select(data_text),
                )
                .and(title),
        )
}

fn q14a(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        kind,
        production_year,
        keyword,
        info,
        data,
        ..
    } = &db.movie;
    let Data {
        text: data_text,
        ty: data_ty,
        ..
    } = &db.data;
    let Info {
        info: info_info,
        ty: info_ty,
        ..
    } = &db.info;
    db.movie
        .with(
            keyword
                .is_in(murder4())
                .and(kind.eq("movie"))
                .and(info.select(info_ty.eq("countries").and(info_info.is_in([
                    "Sweden",
                    "Norway",
                    "Germany",
                    "Denmark",
                    "Swedish",
                    "Denish",
                    "Norwegian",
                    "German",
                    "USA",
                    "American",
                ]))))
                .and(production_year.gt(2010)),
        )
        .select(
            data.with(data_ty.eq("rating").and(data_text.lt("8.5")))
                .select(data_text)
                .and(title),
        )
}

fn q1b(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        company,
        data,
        ..
    } = &db.movie;
    let Company {
        note: company_note,
        ty: company_ty,
        ..
    } = &db.company;
    let Data { ty: data_ty, .. } = &db.data;
    db.movie
        .with(
            data.select(data_ty)
                .eq("bottom 10 rank")
                .and(production_year.ge(2005))
                .and(production_year.le(2010)),
        )
        .select(
            company
                .with(
                    company_ty
                        .eq("production companies")
                        .and(company_note.nrx(r"\(as Metro-Goldwyn-Mayer Pictures\)")),
                )
                .select(company_note)
                .and(title)
                .and(production_year),
        )
}

// q3a/q3c differ only in the country list and the year cutoff.
fn q3ac(db: &'static Job, countries: Vec<&'static str>, year: i64) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        keyword,
        info,
        ..
    } = &db.movie;
    let Info {
        info: info_info, ..
    } = &db.info;
    db.movie
        .with(
            keyword
                .rx(r"sequel")
                .and(info.select(info_info).is_in(countries))
                .and(production_year.gt(year)),
        )
        .select(title)
}

fn q3a(db: &'static Job) -> impl Drive<R: Row> {
    q3ac(db, nordic8(), 2005)
}
fn q3c(db: &'static Job) -> impl Drive<R: Row> {
    q3ac(
        db,
        vec![
            "Sweden",
            "Norway",
            "Germany",
            "Denmark",
            "Swedish",
            "Denish",
            "Norwegian",
            "German",
            "USA",
            "American",
        ],
        1990,
    )
}

fn q11b(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        keyword,
        company,
        link,
        ..
    } = &db.movie;
    let Company {
        name: company_name,
        country,
        note: company_note,
        ty: company_ty,
        ..
    } = &db.company;
    let MovieLink {
        ty: movielink_ty, ..
    } = &db.movie_link;
    db.movie
        .with(
            link.and(keyword.eq("sequel"))
                .and(production_year.eq(1998))
                .and(title.rx(r"Money")),
        )
        .select(
            company
                .with(
                    country
                        .ne("[pl]")
                        .and(company_name.rx(r"Film|Warner"))
                        .and(company_ty.eq("production companies"))
                        .minus(company_note),
                )
                .select(company_name)
                .and(link.select(movielink_ty).rx(r"follows"))
                .and(title),
        )
}

fn q13b(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        kind,
        company,
        info,
        data,
        ..
    } = &db.movie;
    let Company {
        name: company_name,
        country,
        ty: company_ty,
        ..
    } = &db.company;
    let Data {
        text: data_text,
        ty: data_ty,
        ..
    } = &db.data;
    let Info { ty: info_ty, .. } = &db.info;
    db.movie
        .with(
            kind.eq("movie")
                .and(info.select(info_ty).eq("release dates"))
                .and(title.ne(""))
                .and(title.rx(r"Champion|Loser")),
        )
        .select(
            company
                .with(
                    country
                        .eq("[us]")
                        .and(company_ty.eq("production companies")),
                )
                .select(company_name)
                .and(data.with(data_ty.eq("rating")).select(data_text))
                .and(title),
        )
}

fn q1c(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        company,
        data,
        ..
    } = &db.movie;
    let Company {
        note: company_note,
        ty: company_ty,
        ..
    } = &db.company;
    let Data { ty: data_ty, .. } = &db.data;
    db.movie
        .with(
            data.select(data_ty)
                .eq("top 250 rank")
                .and(production_year.gt(2010)),
        )
        .select(
            company
                .with(
                    company_ty
                        .eq("production companies")
                        .and(company_note.nrx(r"\(as Metro-Goldwyn-Mayer Pictures\)"))
                        .and(company_note.rx(r"\(co-production\)")),
                )
                .select(company_note)
                .and(title)
                .and(production_year),
        )
}

fn q1d(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        company,
        data,
        ..
    } = &db.movie;
    let Company {
        note: company_note,
        ty: company_ty,
        ..
    } = &db.company;
    let Data { ty: data_ty, .. } = &db.data;
    db.movie
        .with(
            data.select(data_ty)
                .eq("bottom 10 rank")
                .and(production_year.gt(2000)),
        )
        .select(
            company
                .with(
                    company_ty
                        .eq("production companies")
                        .and(company_note.nrx(r"\(as Metro-Goldwyn-Mayer Pictures\)")),
                )
                .select(company_note)
                .and(title)
                .and(production_year),
        )
}

fn q12b(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        company,
        info,
        data,
        ..
    } = &db.movie;
    let Company {
        country,
        ty: company_ty,
        ..
    } = &db.company;
    let Data { ty: data_ty, .. } = &db.data;
    let Info {
        info: info_info,
        ty: info_ty,
        ..
    } = &db.info;
    db.movie
        .with(
            company
                .select(
                    country
                        .eq("[us]")
                        .and(company_ty.is_in(["production companies", "distributors"])),
                )
                .and(data.select(data_ty).eq("bottom 10 rank"))
                .and(production_year.gt(2000))
                .and(title.rx(r"^Birdemic|Movie")),
        )
        .select(info.with(info_ty.eq("budget")).select(info_info).and(title))
}

fn q12c(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        company,
        info,
        data,
        ..
    } = &db.movie;
    let Company {
        name: company_name,
        country,
        ty: company_ty,
        ..
    } = &db.company;
    let Data {
        text: data_text,
        ty: data_ty,
        ..
    } = &db.data;
    let Info {
        info: info_info,
        ty: info_ty,
        ..
    } = &db.info;
    db.movie
        .with(
            info.select(
                info_ty
                    .eq("genres")
                    .and(info_info.is_in(["Drama", "Horror", "Western", "Family"])),
            )
            .and(production_year.ge(2000))
            .and(production_year.le(2010)),
        )
        .select(
            company
                .with(
                    country
                        .eq("[us]")
                        .and(company_ty.eq("production companies")),
                )
                .select(company_name)
                .and(
                    data.with(data_ty.eq("rating").and(data_text.gt("7.0")))
                        .select(data_text),
                )
                .and(title),
        )
}

fn q13c(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        kind,
        company,
        info,
        data,
        ..
    } = &db.movie;
    let Company {
        name: company_name,
        country,
        ty: company_ty,
        ..
    } = &db.company;
    let Data {
        text: data_text,
        ty: data_ty,
        ..
    } = &db.data;
    let Info { ty: info_ty, .. } = &db.info;
    db.movie
        .with(
            kind.eq("movie")
                .and(info.select(info_ty).eq("release dates"))
                .and(title.ne(""))
                .and(title.rx(r"^Champion|^Loser")),
        )
        .select(
            company
                .with(
                    country
                        .eq("[us]")
                        .and(company_ty.eq("production companies")),
                )
                .select(company_name)
                .and(data.with(data_ty.eq("rating")).select(data_text))
                .and(title),
        )
}

fn q14b(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        kind,
        production_year,
        keyword,
        info,
        data,
        ..
    } = &db.movie;
    let Data {
        text: data_text,
        ty: data_ty,
        ..
    } = &db.data;
    let Info {
        info: info_info,
        ty: info_ty,
        ..
    } = &db.info;
    db.movie
        .with(
            keyword
                .is_in(["murder", "murder-in-title"])
                .and(kind.eq("movie"))
                .and(info.select(info_ty.eq("countries").and(info_info.is_in([
                    "Sweden",
                    "Norway",
                    "Germany",
                    "Denmark",
                    "Swedish",
                    "Denish",
                    "Norwegian",
                    "German",
                    "USA",
                    "American",
                ]))))
                .and(production_year.gt(2010))
                .and(title.rx(r"murder|Murder|Mord")),
        )
        .select(
            data.with(data_ty.eq("rating").and(data_text.gt("6.0")))
                .select(data_text)
                .and(title),
        )
}

fn q14c(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        kind,
        production_year,
        keyword,
        info,
        data,
        ..
    } = &db.movie;
    let Data {
        text: data_text,
        ty: data_ty,
        ..
    } = &db.data;
    let Info {
        info: info_info,
        ty: info_ty,
        ..
    } = &db.info;
    db.movie
        .with(
            keyword
                .is_in(murder4())
                .and(kind.is_in(["movie", "episode"]))
                .and(info.select(info_ty.eq("countries").and(info_info.is_in(nordic10()))))
                .and(production_year.gt(2005)),
        )
        .select(
            data.with(data_ty.eq("rating").and(data_text.lt("8.5")))
                .select(data_text)
                .and(title),
        )
}

fn q22b(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        kind,
        production_year,
        keyword,
        company,
        info,
        data,
        ..
    } = &db.movie;
    let Company {
        name: company_name,
        country,
        note: company_note,
        ty: company_ty,
        ..
    } = &db.company;
    let Data {
        text: data_text,
        ty: data_ty,
        ..
    } = &db.data;
    let Info {
        info: info_info,
        ty: info_ty,
        ..
    } = &db.info;
    db.movie
        .with(
            info.select(
                info_ty
                    .eq("countries")
                    .and(info_info.is_in(["Germany", "German", "USA", "American"])),
            )
            .and(keyword.is_in(murder4()))
            .and(production_year.gt(2009))
            .and(kind.is_in(["movie", "episode"])),
        )
        .select(
            title
                .and(
                    data.with(data_text.lt("7.0").and(data_ty.eq("rating")))
                        .select(data_text),
                )
                .and(
                    company
                        .with(
                            company_note
                                .nrx(r"\(USA\)")
                                .and(company_note.rx(r"\(200.*\)"))
                                .and(country.ne("[us]"))
                                .and(company_ty.eq("production companies")),
                        )
                        .select(company_name),
                ),
        )
}

fn q22c(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        kind,
        production_year,
        keyword,
        company,
        info,
        data,
        ..
    } = &db.movie;
    let Company {
        name: company_name,
        country,
        note: company_note,
        ty: company_ty,
        ..
    } = &db.company;
    let Data {
        text: data_text,
        ty: data_ty,
        ..
    } = &db.data;
    let Info {
        info: info_info,
        ty: info_ty,
        ..
    } = &db.info;
    db.movie
        .with(
            info.select(info_ty.eq("countries").and(info_info.is_in(nordic10())))
                .and(keyword.is_in(murder4()))
                .and(production_year.gt(2005))
                .and(kind.is_in(["movie", "episode"])),
        )
        .select(
            title
                .and(
                    data.with(data_text.lt("8.5").and(data_ty.eq("rating")))
                        .select(data_text),
                )
                .and(
                    company
                        .with(
                            company_note
                                .nrx(r"\(USA\)")
                                .and(company_note.rx(r"\(200.*\)"))
                                .and(country.ne("[us]"))
                                .and(company_ty.eq("production companies")),
                        )
                        .select(company_name),
                ),
        )
}

// ===== queries: 22d, 5b, 5c, 15a-d, 11c-d, 13d, 6a-f =====

fn q22d(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        kind,
        production_year,
        keyword,
        company,
        info,
        data,
        ..
    } = &db.movie;
    let Company {
        name: company_name,
        country,
        ty: company_ty,
        ..
    } = &db.company;
    let Data {
        text: data_text,
        ty: data_ty,
        ..
    } = &db.data;
    let Info {
        info: info_info,
        ty: info_ty,
        ..
    } = &db.info;
    db.movie
        .with(
            info.select(info_ty.eq("countries").and(info_info.is_in(nordic10())))
                .and(keyword.is_in(murder4()))
                .and(production_year.gt(2005))
                .and(kind.is_in(["movie", "episode"])),
        )
        .select(
            title
                .and(
                    data.with(data_text.lt("8.5").and(data_ty.eq("rating")))
                        .select(data_text),
                )
                .and(
                    company
                        .with(
                            country
                                .ne("[us]")
                                .and(company_ty.eq("production companies")),
                        )
                        .select(company_name),
                ),
        )
}

fn q5b(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        company,
        info,
        ..
    } = &db.movie;
    let Company {
        note: company_note,
        ty: company_ty,
        ..
    } = &db.company;
    let Info {
        info: info_info, ..
    } = &db.info;
    db.movie
        .with(
            company
                .select(
                    company_ty
                        .eq("production companies")
                        .and(company_note.rx(r"\(VHS\)"))
                        .and(company_note.rx(r"\(USA\)"))
                        .and(company_note.rx(r"\(1994\)")),
                )
                .and(info.select(info_info).is_in(["USA", "America"]))
                .and(production_year.gt(2010)),
        )
        .select(title)
}

fn q5c(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        company,
        info,
        ..
    } = &db.movie;
    let Company {
        note: company_note,
        ty: company_ty,
        ..
    } = &db.company;
    let Info {
        info: info_info, ..
    } = &db.info;
    db.movie
        .with(
            company
                .select(
                    company_ty
                        .eq("production companies")
                        .and(company_note.nrx(r"\(TV\)"))
                        .and(company_note.rx(r"\(USA\)")),
                )
                .and(info.select(info_info).is_in(nordic10()))
                .and(production_year.gt(1990)),
        )
        .select(title)
}

fn q15a(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        keyword,
        company,
        info,
        aka,
        ..
    } = &db.movie;
    let Company {
        country,
        note: company_note,
        ..
    } = &db.company;
    let Info {
        info: info_info,
        ty: info_ty,
        note: info_note,
        ..
    } = &db.info;
    db.movie
        .with(
            production_year
                .gt(2000)
                .and(
                    company.select(
                        country
                            .eq("[us]")
                            .and(company_note.rx(r"\(200.*\)"))
                            .and(company_note.rx(r"\(worldwide\)")),
                    ),
                )
                .and(keyword)
                .and(aka),
        )
        .select(
            info.with(
                info_ty
                    .eq("release dates")
                    .and(info_info.rx(r"^USA:.* 200"))
                    .and(info_note.rx(r"internet")),
            )
            .select(info_info)
            .and(title),
        )
}

fn q15b(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        keyword,
        company,
        info,
        aka,
        ..
    } = &db.movie;
    let Company {
        name: company_name,
        country,
        note: company_note,
        ..
    } = &db.company;
    let Info {
        info: info_info,
        ty: info_ty,
        note: info_note,
        ..
    } = &db.info;
    db.movie
        .with(
            company
                .select(
                    country
                        .eq("[us]")
                        .and(company_name.eq("YouTube"))
                        .and(company_note.rx(r"\(200.*\)"))
                        .and(company_note.rx(r"\(worldwide\)")),
                )
                .and(keyword)
                .and(aka)
                .and(production_year.ge(2005))
                .and(production_year.le(2010)),
        )
        .select(
            info.with(
                info_ty
                    .eq("release dates")
                    .and(info_info.rx(r"^USA:.* 200"))
                    .and(info_note.rx(r"internet")),
            )
            .select(info_info)
            .and(title),
        )
}

fn q15c(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        keyword,
        company,
        info,
        aka,
        ..
    } = &db.movie;
    let Company { country, .. } = &db.company;
    let Info {
        info: info_info,
        ty: info_ty,
        note: info_note,
        ..
    } = &db.info;
    db.movie
        .with(
            company
                .select(country)
                .eq("[us]")
                .and(keyword)
                .and(aka)
                .and(production_year.gt(1990)),
        )
        .select(
            info.with(
                info_ty
                    .eq("release dates")
                    .and(info_info.rx(r"^USA:.* 199|^USA:.* 200"))
                    .and(info_note.rx(r"internet")),
            )
            .select(info_info)
            .and(title),
        )
}

fn q15d(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        keyword,
        company,
        info,
        aka,
        ..
    } = &db.movie;
    let Company { country, .. } = &db.company;
    let Info {
        ty: info_ty,
        note: info_note,
        ..
    } = &db.info;
    db.movie
        .with(
            company
                .select(country)
                .eq("[us]")
                .and(keyword)
                .and(info.select(info_ty.eq("release dates").and(info_note.rx(r"internet"))))
                .and(production_year.gt(1990)),
        )
        .select(aka.and(title))
}

fn q11c(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        keyword,
        company,
        link,
        ..
    } = &db.movie;
    let Company {
        name: company_name,
        country,
        note: company_note,
        ty: company_ty,
        ..
    } = &db.company;
    db.movie
        .with(
            keyword
                .is_in(["sequel", "revenge", "based-on-novel"])
                .and(production_year.gt(1950))
                .and(link),
        )
        .select(
            company
                .with(
                    country
                        .ne("[pl]")
                        .and(company_name.rx(r"^20th Century Fox|^Twentieth Century Fox"))
                        .and(company_ty.ne("production companies"))
                        .and(company_note),
                )
                .select(company_name.and(company_note))
                .and(title),
        )
}

fn q11d(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        keyword,
        company,
        link,
        ..
    } = &db.movie;
    let Company {
        name: company_name,
        country,
        note: company_note,
        ty: company_ty,
        ..
    } = &db.company;
    db.movie
        .with(
            keyword
                .is_in(["sequel", "revenge", "based-on-novel"])
                .and(production_year.gt(1950))
                .and(link),
        )
        .select(
            company
                .with(
                    country
                        .ne("[pl]")
                        .and(company_ty.ne("production companies"))
                        .and(company_note),
                )
                .select(company_name.and(company_note))
                .and(title),
        )
}

fn q13d(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        kind,
        company,
        info,
        data,
        ..
    } = &db.movie;
    let Company {
        name: company_name,
        country,
        ty: company_ty,
        ..
    } = &db.company;
    let Data {
        text: data_text,
        ty: data_ty,
        ..
    } = &db.data;
    let Info { ty: info_ty, .. } = &db.info;
    db.movie
        .with(
            kind.eq("movie")
                .and(info.select(info_ty).eq("release dates")),
        )
        .select(
            company
                .with(
                    country
                        .eq("[us]")
                        .and(company_ty.eq("production companies")),
                )
                .select(company_name)
                .and(data.with(data_ty.eq("rating")).select(data_text))
                .and(title),
        )
}

// q6a/c/e share the marvel-cinematic-universe keyword and q6b/d the kw8
// list; within each pair only the year cutoff varies.
fn q6_marvel(db: &'static Job, year: i64) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        keyword,
        cast,
        ..
    } = &db.movie;
    let Cast { person, .. } = &db.cast;
    let Person {
        name: person_name, ..
    } = &db.person;
    let kw = || keyword.eq("marvel-cinematic-universe");
    let downey = cast
        .select(person)
        .select(person_name)
        .rx(r"Downey.*Robert");
    db.movie
        .with(production_year.gt(year).and(kw()))
        .select(kw().and(downey).and(title))
}

fn q6_comic(db: &'static Job, year: i64) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        keyword,
        cast,
        ..
    } = &db.movie;
    let Cast { person, .. } = &db.cast;
    let Person {
        name: person_name, ..
    } = &db.person;
    let kw = || keyword.is_in(kw8());
    let downey = cast
        .select(person)
        .select(person_name)
        .rx(r"Downey.*Robert");
    db.movie
        .with(production_year.gt(year).and(kw()))
        .select(kw().and(downey).and(title))
}

fn q6a(db: &'static Job) -> impl Drive<R: Row> {
    q6_marvel(db, 2010)
}
fn q6b(db: &'static Job) -> impl Drive<R: Row> {
    q6_comic(db, 2014)
}
fn q6c(db: &'static Job) -> impl Drive<R: Row> {
    q6_marvel(db, 2014)
}
fn q6d(db: &'static Job) -> impl Drive<R: Row> {
    q6_comic(db, 2000)
}
fn q6e(db: &'static Job) -> impl Drive<R: Row> {
    q6_marvel(db, 2000)
}

fn q6f(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        keyword,
        cast,
        ..
    } = &db.movie;
    let Cast { person, .. } = &db.cast;
    let Person {
        name: person_name, ..
    } = &db.person;
    let kw = || keyword.is_in(kw8());
    let cast_name = cast.select(person).select(person_name);
    db.movie
        .with(production_year.gt(2000).and(kw()))
        .select(kw().and(cast_name).and(title))
}

// ===== queries: 7a-c, 8a-d, 9a-d, 10a-c =====

fn q7a(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        cast,
        linked_by,
        ..
    } = &db.movie;
    let Cast { person, .. } = &db.cast;
    let MovieLink {
        ty: movielink_ty, ..
    } = &db.movie_link;
    let Person {
        name: person_name,
        gender,
        alias,
        bio,
        name_pcode_cf,
        ..
    } = &db.person;
    let PersonInfo {
        ty: personinfo_ty,
        note: personinfo_note,
        ..
    } = &db.person_info;
    db.movie
        .with(
            production_year
                .ge(1980)
                .and(production_year.le(1995))
                .and(linked_by.select(movielink_ty).eq("features")),
        )
        .select(
            cast.select(
                person
                    .with(
                        alias
                            .rx(r"a")
                            .and(name_pcode_cf.ge("A"))
                            .and(name_pcode_cf.le("F"))
                            // m ∨ (f ∧ name~^B), spelled {m,f} ∖ (f ∖ ^B):
                            // ∨ is member-only and can't sit inside a probed ∧-tree.
                            .and(
                                gender
                                    .is_in(["m", "f"])
                                    .minus(gender.eq("f").minus(person_name.rx(r"^B"))),
                            )
                            .and(
                                bio.select(
                                    personinfo_ty
                                        .eq("mini biography")
                                        .and(personinfo_note.eq("Volker Boehm")),
                                ),
                            ),
                    )
                    .select(person_name),
            )
            .and(title),
        )
}

fn q7b(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        cast,
        linked_by,
        ..
    } = &db.movie;
    let Cast { person, .. } = &db.cast;
    let MovieLink {
        ty: movielink_ty, ..
    } = &db.movie_link;
    let Person {
        name: person_name,
        gender,
        alias,
        bio,
        name_pcode_cf,
        ..
    } = &db.person;
    let PersonInfo {
        ty: personinfo_ty,
        note: personinfo_note,
        ..
    } = &db.person_info;
    db.movie
        .with(
            production_year
                .ge(1980)
                .and(production_year.le(1984))
                .and(linked_by.select(movielink_ty).eq("features")),
        )
        .select(
            cast.select(
                person
                    .with(
                        alias
                            .rx(r"a")
                            .and(name_pcode_cf.rx(r"^D"))
                            .and(gender.eq("m"))
                            .and(
                                bio.select(
                                    personinfo_ty
                                        .eq("mini biography")
                                        .and(personinfo_note.eq("Volker Boehm")),
                                ),
                            ),
                    )
                    .select(person_name),
            )
            .and(title),
        )
}

// Conjunct tree (∧ = Prod) — consumed via `member` only, so the value
// type stays opaque (`impl Query<D = Id<PersonInfo>> + Probe`).
fn bio_filter_7c(db: &'static Job) -> impl Query<D = Id<PersonInfo>> + Probe {
    let PersonInfo {
        ty: personinfo_ty,
        note: personinfo_note,
        ..
    } = &db.person_info;
    personinfo_ty.eq("mini biography").and(personinfo_note)
}

fn q7c(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        production_year,
        cast,
        linked_by,
        ..
    } = &db.movie;
    let Cast { person, .. } = &db.cast;
    let MovieLink {
        ty: movielink_ty, ..
    } = &db.movie_link;
    let Person {
        name: person_name,
        gender,
        alias,
        bio,
        name_pcode_cf,
        ..
    } = &db.person;
    let PersonInfo {
        info: personinfo_info,
        ..
    } = &db.person_info;
    db.movie
        .with(production_year.ge(1980).and(production_year.le(2010)).and(
            linked_by.select(movielink_ty).is_in([
                "references",
                "referenced in",
                "features",
                "featured in",
            ]),
        ))
        .select(
            cast.select(
                person
                    .with(
                        alias
                            .rx(r"a|^A")
                            .and(name_pcode_cf.ge("A"))
                            .and(name_pcode_cf.le("F"))
                            // m ∨ (f ∧ name~^A), spelled {m,f} ∖ (f ∖ ^A):
                            // ∨ is member-only and can't sit inside a probed ∧-tree.
                            .and(
                                gender
                                    .is_in(["m", "f"])
                                    .minus(gender.eq("f").minus(person_name.rx(r"^A"))),
                            )
                            .and(bio.with(bio_filter_7c(db))),
                    )
                    .select(person_name.and(bio.with(bio_filter_7c(db)).select(personinfo_info))),
            ),
        )
}

fn q8a(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        company,
        cast,
        ..
    } = &db.movie;
    let Cast {
        person,
        role,
        note: cast_note,
        ..
    } = &db.cast;
    let Company {
        country,
        note: company_note,
        ..
    } = &db.company;
    let Person {
        name: person_name,
        alias,
        ..
    } = &db.person;
    db.movie
        .with(
            company.select(
                country
                    .eq("[jp]")
                    .and(company_note.rx(r"\(Japan\)"))
                    .and(company_note.nrx(r"\(USA\)")),
            ),
        )
        .select(
            cast.with(
                cast_note
                    .eq("(voice: English version)")
                    .and(role.eq("actress"))
                    .and(person.with(person_name.rx(r"Yo").and(person_name.nrx(r"Yu")))),
            )
            .select(person)
            .select(alias)
            .and(title),
        )
}

fn q8b(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        company,
        cast,
        ..
    } = &db.movie;
    let Cast {
        person,
        role,
        note: cast_note,
        ..
    } = &db.cast;
    let Company {
        country,
        note: company_note,
        ..
    } = &db.company;
    let Person {
        name: person_name,
        alias,
        ..
    } = &db.person;
    db.movie
        .with(
            company
                .select(
                    country
                        .eq("[jp]")
                        .and(company_note.rx(r"\(Japan\)"))
                        .and(company_note.nrx(r"\(USA\)"))
                        .and(company_note.rx(r"\(2006\)|\(2007\)")),
                )
                .and(production_year.ge(2006))
                .and(production_year.le(2007))
                .and(title.rx(r"^One Piece|^Dragon Ball Z")),
        )
        .select(
            cast.with(
                cast_note
                    .eq("(voice: English version)")
                    .and(role.eq("actress"))
                    .and(person.with(person_name.rx(r"Yo").and(person_name.nrx(r"Yu")))),
            )
            .select(person)
            .select(alias)
            .and(title),
        )
}

// q8c/q8d differ only in the cast role.
fn q8cd(db: &'static Job, role_: &'static str) -> impl Drive<R: Row> {
    let Movie {
        title,
        company,
        cast,
        ..
    } = &db.movie;
    let Cast { person, role, .. } = &db.cast;
    let Company { country, .. } = &db.company;
    let Person { alias, .. } = &db.person;
    db.movie.with(company.select(country).eq("[us]")).select(
        cast.with(role.eq(role_))
            .select(person)
            .select(alias)
            .and(title),
    )
}

fn q8c(db: &'static Job) -> impl Drive<R: Row> {
    q8cd(db, "writer")
}
fn q8d(db: &'static Job) -> impl Drive<R: Row> {
    q8cd(db, "costume designer")
}

fn q9a(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        company,
        cast,
        ..
    } = &db.movie;
    let Cast {
        person,
        role,
        note: cast_note,
        character,
        ..
    } = &db.cast;
    let Company {
        country,
        note: company_note,
        ..
    } = &db.company;
    let Person {
        name: person_name,
        gender,
        alias,
        ..
    } = &db.person;
    db.movie
        .with(
            company
                .select(
                    country
                        .eq("[us]")
                        .and(company_note.rx(r"\(USA\)|\(worldwide\)")),
                )
                .and(production_year.ge(2005))
                .and(production_year.le(2015)),
        )
        .select(
            cast.with(
                cast_note
                    .is_in(voice4())
                    .and(role.eq("actress"))
                    .and(person.with(gender.eq("f").and(person_name.rx(r"Ang")))),
            )
            .select(person.select(alias).and(character))
            .and(title),
        )
}

fn q9b(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        company,
        cast,
        ..
    } = &db.movie;
    let Cast {
        person,
        role,
        note: cast_note,
        character,
        ..
    } = &db.cast;
    let Company {
        country,
        note: company_note,
        ..
    } = &db.company;
    let Person {
        name: person_name,
        gender,
        alias,
        ..
    } = &db.person;
    db.movie
        .with(
            company
                .select(
                    country
                        .eq("[us]")
                        .and(company_note.rx(r"\(200.*\)"))
                        .and(company_note.rx(r"\(USA\)|\(worldwide\)")),
                )
                .and(production_year.ge(2007))
                .and(production_year.le(2010)),
        )
        .select(
            cast.with(
                cast_note
                    .eq("(voice)")
                    .and(role.eq("actress"))
                    .and(person.with(gender.eq("f").and(person_name.rx(r"Angel")))),
            )
            .select(
                person
                    .select(alias)
                    .and(character)
                    .and(person.select(person_name)),
            )
            .and(title),
        )
}

fn q9c(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        company,
        cast,
        ..
    } = &db.movie;
    let Cast {
        person,
        role,
        note: cast_note,
        character,
        ..
    } = &db.cast;
    let Company { country, .. } = &db.company;
    let Person {
        name: person_name,
        gender,
        alias,
        ..
    } = &db.person;
    db.movie.with(company.select(country).eq("[us]")).select(
        cast.with(
            cast_note
                .is_in(voice4())
                .and(role.eq("actress"))
                .and(person.with(gender.eq("f").and(person_name.rx(r"An")))),
        )
        .select(
            person
                .select(alias)
                .and(character)
                .and(person.select(person_name)),
        )
        .and(title),
    )
}

fn q9d(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        company,
        cast,
        ..
    } = &db.movie;
    let Cast {
        person,
        role,
        note: cast_note,
        character,
        ..
    } = &db.cast;
    let Company { country, .. } = &db.company;
    let Person {
        name: person_name,
        gender,
        alias,
        ..
    } = &db.person;
    db.movie.with(company.select(country).eq("[us]")).select(
        cast.with(
            cast_note
                .is_in(voice4())
                .and(role.eq("actress"))
                .and(person.with(gender.eq("f"))),
        )
        .select(
            person
                .select(alias)
                .and(person.select(person_name))
                .and(character),
        )
        .and(title),
    )
}

fn q10a(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        company,
        cast,
        ..
    } = &db.movie;
    let Cast {
        role,
        note: cast_note,
        character,
        ..
    } = &db.cast;
    let Company { country, .. } = &db.company;
    db.movie
        .with(
            company
                .select(country)
                .eq("[ru]")
                .and(production_year.gt(2005)),
        )
        .select(
            cast.with(
                cast_note
                    .rx(r"\(voice\)")
                    .and(cast_note.rx(r"\(uncredited\)"))
                    .and(role.eq("actor")),
            )
            .select(character)
            .and(title),
        )
}

fn q10b(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        company,
        cast,
        ..
    } = &db.movie;
    let Cast {
        role,
        note: cast_note,
        character,
        ..
    } = &db.cast;
    let Company { country, .. } = &db.company;
    db.movie
        .with(
            company
                .select(country)
                .eq("[ru]")
                .and(production_year.gt(2010)),
        )
        .select(
            cast.with(cast_note.rx(r"\(producer\)").and(role.eq("actor")))
                .select(character)
                .and(title),
        )
}

fn q10c(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        company,
        cast,
        ..
    } = &db.movie;
    let Cast {
        note: cast_note,
        character,
        ..
    } = &db.cast;
    let Company { country, .. } = &db.company;
    db.movie
        .with(
            company
                .select(country)
                .eq("[us]")
                .and(production_year.gt(1990)),
        )
        .select(
            cast.with(cast_note.rx(r"\(producer\)"))
                .select(character)
                .and(title),
        )
}

// ===== queries: templates 16-18 =====

// q16a/q16d differ only in the episode_nr lower bound.
fn q16ad(db: &'static Job, lo: i64) -> impl Drive<R: Row> {
    let Movie {
        title,
        episode_nr,
        keyword,
        company,
        cast,
        ..
    } = &db.movie;
    let Cast { person, .. } = &db.cast;
    let Company { country, .. } = &db.company;
    let Person { alias, .. } = &db.person;
    db.movie
        .with(
            company
                .select(country)
                .eq("[us]")
                .and(keyword.eq("character-name-in-title"))
                .and(episode_nr.ge(lo))
                .and(episode_nr.lt(100)),
        )
        .select(cast.select(person).select(alias).and(title))
}

fn q16a(db: &'static Job) -> impl Drive<R: Row> {
    q16ad(db, 50)
}
fn q16d(db: &'static Job) -> impl Drive<R: Row> {
    q16ad(db, 5)
}

fn q16b(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        keyword,
        company,
        cast,
        ..
    } = &db.movie;
    let Cast { person, .. } = &db.cast;
    let Company { country, .. } = &db.company;
    let Person { alias, .. } = &db.person;
    db.movie
        .with(
            company
                .select(country)
                .eq("[us]")
                .and(keyword.eq("character-name-in-title")),
        )
        .select(cast.select(person).select(alias).and(title))
}

fn q16c(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        episode_nr,
        keyword,
        company,
        cast,
        ..
    } = &db.movie;
    let Cast { person, .. } = &db.cast;
    let Company { country, .. } = &db.company;
    let Person { alias, .. } = &db.person;
    db.movie
        .with(
            company
                .select(country)
                .eq("[us]")
                .and(keyword.eq("character-name-in-title"))
                .and(episode_nr.lt(100)),
        )
        .select(cast.select(person).select(alias).and(title))
}

fn q17a(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        keyword,
        company,
        cast,
        ..
    } = &db.movie;
    let Cast { person, .. } = &db.cast;
    let Company { country, .. } = &db.company;
    let Person {
        name: person_name, ..
    } = &db.person;
    db.movie
        .with(
            company
                .select(country)
                .eq("[us]")
                .and(keyword.eq("character-name-in-title")),
        )
        .select(cast.select(person).select(person_name).rx(r"^B"))
}

// q17b/c/d/f differ only in the person-name regex.
fn q17_any_co(db: &'static Job, re: &str) -> impl Drive<R: Row> {
    let Movie {
        keyword,
        company,
        cast,
        ..
    } = &db.movie;
    let Cast { person, .. } = &db.cast;
    let Person {
        name: person_name, ..
    } = &db.person;
    db.movie
        .with(company.and(keyword.eq("character-name-in-title")))
        .select(cast.select(person).select(person_name).rx(re))
}

fn q17b(db: &'static Job) -> impl Drive<R: Row> {
    q17_any_co(db, r"^Z")
}
fn q17c(db: &'static Job) -> impl Drive<R: Row> {
    q17_any_co(db, r"^X")
}
fn q17d(db: &'static Job) -> impl Drive<R: Row> {
    q17_any_co(db, r"Bert")
}
fn q17f(db: &'static Job) -> impl Drive<R: Row> {
    q17_any_co(db, r"B")
}

fn q17e(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        keyword,
        company,
        cast,
        ..
    } = &db.movie;
    let Cast { person, .. } = &db.cast;
    let Company { country, .. } = &db.company;
    let Person {
        name: person_name, ..
    } = &db.person;
    db.movie
        .with(
            company
                .select(country)
                .eq("[us]")
                .and(keyword.eq("character-name-in-title")),
        )
        .select(cast.select(person).select(person_name))
}

fn ib_18a(db: &'static Job) -> impl Query<R = &'static str, D = Id<Movie>> + Drive + Probe {
    let Movie { info, .. } = &db.movie;
    let Info {
        info: info_info,
        ty: info_ty,
        ..
    } = &db.info;
    info.with(info_ty.eq("budget")).select(info_info)
}

fn q18a(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title, cast, data, ..
    } = &db.movie;
    let Cast {
        person,
        note: cast_note,
        ..
    } = &db.cast;
    let Data {
        text: data_text,
        ty: data_ty,
        ..
    } = &db.data;
    let Person {
        name: person_name,
        gender,
        ..
    } = &db.person;
    db.movie
        .with(
            ib_18a(db).and(
                cast.select(
                    cast_note
                        .is_in(["(producer)", "(executive producer)"])
                        .and(person.select(gender.eq("m").and(person_name.rx(r"Tim")))),
                ),
            ),
        )
        .select(
            ib_18a(db)
                .and(data.with(data_ty.eq("votes")).select(data_text))
                .and(title),
        )
}

// Conjunct/diff tree (∧ = Prod, - = Diff) — consumed via `member` only, so
// the value type stays opaque (`impl Query<D = Id<Info>> + Probe`).
fn gf_18b(db: &'static Job) -> impl Query<D = Id<Info>> + Probe {
    let Info {
        info: info_info,
        ty: info_ty,
        note: info_note,
        ..
    } = &db.info;
    info_ty
        .eq("genres")
        .and(info_info.is_in(["Horror", "Thriller"]))
        .minus(info_note)
}

fn q18b(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        cast,
        info,
        data,
        ..
    } = &db.movie;
    let Cast {
        person,
        note: cast_note,
        ..
    } = &db.cast;
    let Data {
        text: data_text,
        ty: data_ty,
        ..
    } = &db.data;
    let Info {
        info: info_info, ..
    } = &db.info;
    let Person { gender, .. } = &db.person;
    db.movie
        .with(
            info.with(gf_18b(db))
                .and(production_year.ge(2008))
                .and(production_year.le(2014))
                .and(
                    cast.select(
                        cast_note
                            .is_in(writer5())
                            .and(person.select(gender.eq("f"))),
                    ),
                ),
        )
        .select(
            info.with(gf_18b(db))
                .select(info_info)
                .and(
                    data.with(data_ty.eq("rating").and(data_text.gt("8.0")))
                        .select(data_text),
                )
                .and(title),
        )
}

fn gf_18c(db: &'static Job) -> impl Query<D = Id<Info>> + Probe {
    let Info {
        info: info_info,
        ty: info_ty,
        ..
    } = &db.info;
    info_ty.eq("genres").and(info_info.is_in(genre6()))
}

fn q18c(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        cast,
        info,
        data,
        ..
    } = &db.movie;
    let Cast {
        person,
        note: cast_note,
        ..
    } = &db.cast;
    let Data {
        text: data_text,
        ty: data_ty,
        ..
    } = &db.data;
    let Info {
        info: info_info, ..
    } = &db.info;
    let Person { gender, .. } = &db.person;
    db.movie
        .with(
            info.with(gf_18c(db)).and(
                cast.select(
                    cast_note
                        .is_in(writer5())
                        .and(person.select(gender.eq("m"))),
                ),
            ),
        )
        .select(
            info.with(gf_18c(db))
                .select(info_info)
                .and(data.with(data_ty.eq("votes")).select(data_text))
                .and(title),
        )
}

// ===== queries: 19a-26c =====

fn k_23ab(db: &'static Job) -> impl Query<R = &'static str, D = Id<Movie>> + Drive + Probe {
    let Movie { kind, .. } = &db.movie;
    kind.eq("movie")
}

fn k_23c(db: &'static Job) -> impl Query<R = &'static str, D = Id<Movie>> + Drive + Probe {
    let Movie { kind, .. } = &db.movie;
    kind.is_in(["movie", "tv movie", "video movie", "video game"])
}

// Conjunct trees (∧ = Prod) — consumed via `member` only, so the value
// type stays opaque (`impl Query<D = Id<Info>> + Probe`).
fn gf_25ab(db: &'static Job) -> impl Query<D = Id<Info>> + Probe {
    let Info {
        info: info_info,
        ty: info_ty,
        ..
    } = &db.info;
    info_ty.eq("genres").and(info_info.eq("Horror"))
}

fn gf_25c(db: &'static Job) -> impl Query<D = Id<Info>> + Probe {
    let Info {
        info: info_info,
        ty: info_ty,
        ..
    } = &db.info;
    info_ty.eq("genres").and(info_info.is_in(genre6()))
}

fn q19a(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        company,
        cast,
        info,
        ..
    } = &db.movie;
    let Cast {
        person,
        role,
        note: cast_note,
        character,
        ..
    } = &db.cast;
    let Company {
        country,
        note: company_note,
        ..
    } = &db.company;
    let Info {
        info: info_info,
        ty: info_ty,
        ..
    } = &db.info;
    let Person {
        name: person_name,
        gender,
        alias,
        ..
    } = &db.person;
    db.movie
        .with(
            company
                .select(
                    country
                        .eq("[us]")
                        .and(company_note.rx(r"\(USA\)|\(worldwide\)")),
                )
                .and(
                    info.select(
                        info_ty
                            .eq("release dates")
                            .and(info_info.rx(r"^Japan:.*200|^USA:.*200")),
                    ),
                )
                .and(production_year.ge(2005))
                .and(production_year.le(2009)),
        )
        .select(
            cast.with(
                cast_note
                    .is_in(voice4())
                    .and(role.eq("actress"))
                    .and(character)
                    .and(person.with(gender.eq("f").and(person_name.rx(r"Ang")).and(alias))),
            )
            .select(person)
            .select(person_name)
            .and(title),
        )
}

fn q19b(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        company,
        cast,
        info,
        ..
    } = &db.movie;
    let Cast {
        person,
        role,
        note: cast_note,
        character,
        ..
    } = &db.cast;
    let Company {
        country,
        note: company_note,
        ..
    } = &db.company;
    let Info {
        info: info_info,
        ty: info_ty,
        ..
    } = &db.info;
    let Person {
        name: person_name,
        gender,
        alias,
        ..
    } = &db.person;
    db.movie
        .with(
            company
                .select(
                    country
                        .eq("[us]")
                        .and(company_note.rx(r"\(200.*\)"))
                        .and(company_note.rx(r"\(USA\)|\(worldwide\)")),
                )
                .and(
                    info.select(
                        info_ty
                            .eq("release dates")
                            .and(info_info.rx(r"^Japan:.*2007|^USA:.*2008")),
                    ),
                )
                .and(production_year.ge(2007))
                .and(production_year.le(2008))
                .and(title.rx(r"Kung.*Fu.*Panda")),
        )
        .select(
            cast.with(
                cast_note
                    .eq("(voice)")
                    .and(role.eq("actress"))
                    .and(character)
                    .and(person.with(gender.eq("f").and(person_name.rx(r"Angel")).and(alias))),
            )
            .select(person)
            .select(person_name)
            .and(title),
        )
}

fn q19c(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        company,
        cast,
        info,
        ..
    } = &db.movie;
    let Cast {
        person,
        role,
        note: cast_note,
        character,
        ..
    } = &db.cast;
    let Company { country, .. } = &db.company;
    let Info {
        info: info_info,
        ty: info_ty,
        ..
    } = &db.info;
    let Person {
        name: person_name,
        gender,
        alias,
        ..
    } = &db.person;
    db.movie
        .with(
            company
                .select(country)
                .eq("[us]")
                .and(
                    info.select(
                        info_ty
                            .eq("release dates")
                            .and(info_info.rx(r"^Japan:.*200|^USA:.*200")),
                    ),
                )
                .and(production_year.gt(2000)),
        )
        .select(
            cast.with(
                cast_note
                    .is_in(voice4())
                    .and(role.eq("actress"))
                    .and(character)
                    .and(person.with(gender.eq("f").and(person_name.rx(r"An")).and(alias))),
            )
            .select(person)
            .select(person_name)
            .and(title),
        )
}

fn q19d(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        company,
        cast,
        info,
        ..
    } = &db.movie;
    let Cast {
        person,
        role,
        note: cast_note,
        character,
        ..
    } = &db.cast;
    let Company { country, .. } = &db.company;
    let Info { ty: info_ty, .. } = &db.info;
    let Person {
        name: person_name,
        gender,
        alias,
        ..
    } = &db.person;
    db.movie
        .with(
            company
                .select(country)
                .eq("[us]")
                .and(info.select(info_ty).eq("release dates"))
                .and(production_year.gt(2000)),
        )
        .select(
            cast.with(
                cast_note
                    .is_in(voice4())
                    .and(role.eq("actress"))
                    .and(character)
                    .and(person.with(gender.eq("f").and(alias))),
            )
            .select(person)
            .select(person_name)
            .and(title),
        )
}

fn q20a(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        kind,
        production_year,
        keyword,
        cast,
        complete_cast,
        ..
    } = &db.movie;
    let Cast { character, .. } = &db.cast;
    let CompleteCast {
        status, subject, ..
    } = &db.complete_cast;
    db.movie
        .with(
            complete_cast
                .select(subject.eq("cast").and(status.rx(r"complete")))
                .and(keyword.is_in(kw8()))
                .and(kind.eq("movie"))
                .and(production_year.gt(1950))
                .and(cast.select(character.nrx(r"Sherlock").rx(r"Tony.*Stark|Iron.*Man"))),
        )
        .select(title)
}

fn q20b(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        kind,
        production_year,
        keyword,
        cast,
        complete_cast,
        ..
    } = &db.movie;
    let Cast {
        person, character, ..
    } = &db.cast;
    let CompleteCast {
        status, subject, ..
    } = &db.complete_cast;
    let Person {
        name: person_name, ..
    } = &db.person;
    db.movie
        .with(
            complete_cast
                .select(subject.eq("cast").and(status.rx(r"complete")))
                .and(keyword.is_in(kw8()))
                .and(kind.eq("movie"))
                .and(production_year.gt(2000))
                .and(
                    cast.select(
                        character
                            .nrx(r"Sherlock")
                            .rx(r"Tony.*Stark|Iron.*Man")
                            .and(person.select(person_name).rx(r"Downey.*Robert")),
                    ),
                ),
        )
        .select(title)
}

fn q20c(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        kind,
        production_year,
        keyword,
        cast,
        complete_cast,
        ..
    } = &db.movie;
    let Cast {
        person, character, ..
    } = &db.cast;
    let CompleteCast {
        status, subject, ..
    } = &db.complete_cast;
    let Person {
        name: person_name, ..
    } = &db.person;
    db.movie
        .with(
            complete_cast
                .select(subject.eq("cast").and(status.rx(r"complete")))
                .and(keyword.is_in(kw10()))
                .and(kind.eq("movie"))
                .and(production_year.gt(2000)),
        )
        .select(
            cast.with(character.rx(r"[Mm]an"))
                .select(person)
                .select(person_name)
                .and(title),
        )
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
fn q21(db: &'static Job, countries: Vec<&'static str>, ylo: i64, yhi: i64) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        keyword,
        info,
        link,
        ..
    } = &db.movie;
    let Company {
        name: company_name, ..
    } = &db.company;
    let Info {
        info: info_info, ..
    } = &db.info;
    db.movie
        .with(
            link.and(keyword.eq("sequel"))
                .and(production_year.between(ylo, yhi))
                .and(info.select(info_info).is_in(countries))
                .and(follow_link(db)),
        )
        .select(
            film_or_warner_co(db)
                .select(company_name)
                .and(follow_link(db))
                .and(title),
        )
}

fn q21a(db: &'static Job) -> impl Drive<R: Row> {
    q21(db, nordic8(), 1950, 2000)
}
fn q21b(db: &'static Job) -> impl Drive<R: Row> {
    q21(db, vec!["Germany", "German"], 2000, 2010)
}
fn q21c(db: &'static Job) -> impl Drive<R: Row> {
    q21(db, nordic9(), 1950, 2010)
}

fn q23a(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        keyword,
        company,
        info,
        complete_cast,
        ..
    } = &db.movie;
    let Company { country, .. } = &db.company;
    let CompleteCast { status, .. } = &db.complete_cast;
    let Info {
        info: info_info,
        ty: info_ty,
        note: info_note,
        ..
    } = &db.info;
    db.movie
        .with(
            complete_cast
                .select(status)
                .eq("complete+verified")
                .and(company.select(country).eq("[us]"))
                .and(
                    info.select(
                        info_ty
                            .eq("release dates")
                            .and(info_note.rx(r"internet"))
                            .and(info_info.rx(r"^USA:.* 199|^USA:.* 200")),
                    ),
                )
                .and(k_23ab(db))
                .and(keyword)
                .and(production_year.gt(2000)),
        )
        .select(k_23ab(db).and(title))
}

fn q23b(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        keyword,
        company,
        info,
        complete_cast,
        ..
    } = &db.movie;
    let Company { country, .. } = &db.company;
    let CompleteCast { status, .. } = &db.complete_cast;
    let Info {
        info: info_info,
        ty: info_ty,
        note: info_note,
        ..
    } = &db.info;
    db.movie
        .with(
            complete_cast
                .select(status)
                .eq("complete+verified")
                .and(company.select(country).eq("[us]"))
                .and(
                    info.select(
                        info_ty
                            .eq("release dates")
                            .and(info_note.rx(r"internet"))
                            .and(info_info.rx(r"^USA:.* 200")),
                    ),
                )
                .and(k_23ab(db))
                .and(keyword.is_in(["nerd", "loner", "alienation", "dignity"]))
                .and(production_year.gt(2000)),
        )
        .select(k_23ab(db).and(title))
}

fn q23c(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        keyword,
        company,
        info,
        complete_cast,
        ..
    } = &db.movie;
    let Company { country, .. } = &db.company;
    let CompleteCast { status, .. } = &db.complete_cast;
    let Info {
        info: info_info,
        ty: info_ty,
        note: info_note,
        ..
    } = &db.info;
    db.movie
        .with(
            complete_cast
                .select(status)
                .eq("complete+verified")
                .and(company.select(country).eq("[us]"))
                .and(
                    info.select(
                        info_ty
                            .eq("release dates")
                            .and(info_note.rx(r"internet"))
                            .and(info_info.rx(r"^USA:.* 199|^USA:.* 200")),
                    ),
                )
                .and(k_23c(db))
                .and(keyword)
                .and(production_year.gt(1990)),
        )
        .select(k_23c(db).and(title))
}

fn q24a(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        keyword,
        company,
        cast,
        info,
        ..
    } = &db.movie;
    let Cast {
        person,
        role,
        note: cast_note,
        character,
        ..
    } = &db.cast;
    let Company { country, .. } = &db.company;
    let Info {
        info: info_info,
        ty: info_ty,
        ..
    } = &db.info;
    let Person {
        name: person_name,
        gender,
        alias,
        ..
    } = &db.person;
    db.movie
        .with(
            company
                .select(country)
                .eq("[us]")
                .and(
                    info.select(
                        info_ty
                            .eq("release dates")
                            .and(info_info.rx(r"^Japan:.*201|^USA:.*201")),
                    ),
                )
                .and(keyword.is_in(["hero", "martial-arts", "hand-to-hand-combat"]))
                .and(production_year.gt(2010)),
        )
        .select(
            cast.with(
                cast_note
                    .is_in(voice4())
                    .and(role.eq("actress"))
                    .and(person.with(gender.eq("f").and(person_name.rx(r"An")).and(alias))),
            )
            .select(character.and(person.select(person_name)))
            .and(title),
        )
}

fn q24b(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        keyword,
        company,
        cast,
        info,
        ..
    } = &db.movie;
    let Cast {
        person,
        role,
        note: cast_note,
        character,
        ..
    } = &db.cast;
    let Company {
        name: company_name,
        country,
        ..
    } = &db.company;
    let Info {
        info: info_info,
        ty: info_ty,
        ..
    } = &db.info;
    let Person {
        name: person_name,
        gender,
        alias,
        ..
    } = &db.person;
    db.movie
        .with(
            company
                .select(
                    country
                        .eq("[us]")
                        .and(company_name.eq("DreamWorks Animation")),
                )
                .and(
                    info.select(
                        info_ty
                            .eq("release dates")
                            .and(info_info.rx(r"^Japan:.*201|^USA:.*201")),
                    ),
                )
                .and(keyword.is_in([
                    "hero",
                    "martial-arts",
                    "hand-to-hand-combat",
                    "computer-animated-movie",
                ]))
                .and(production_year.gt(2010))
                .and(title.rx(r"^Kung Fu Panda")),
        )
        .select(
            cast.with(
                cast_note
                    .is_in(voice4())
                    .and(role.eq("actress"))
                    .and(person.with(gender.eq("f").and(person_name.rx(r"An")).and(alias))),
            )
            .select(character.and(person.select(person_name)))
            .and(title),
        )
}

fn q25a(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        keyword,
        cast,
        info,
        data,
        ..
    } = &db.movie;
    let Cast {
        person,
        note: cast_note,
        ..
    } = &db.cast;
    let Data {
        text: data_text,
        ty: data_ty,
        ..
    } = &db.data;
    let Info {
        info: info_info, ..
    } = &db.info;
    let Person {
        name: person_name,
        gender,
        ..
    } = &db.person;
    db.movie
        .with(info.with(gf_25ab(db)).and(keyword.is_in([
            "murder",
            "blood",
            "gore",
            "death",
            "female-nudity",
        ])))
        .select(
            info.with(gf_25ab(db))
                .select(info_info)
                .and(data.with(data_ty.eq("votes")).select(data_text))
                .and(title)
                .and(
                    cast.with(cast_note.is_in(writer5()).and(person.with(gender.eq("m"))))
                        .select(person)
                        .select(person_name),
                ),
        )
}

fn q25b(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        keyword,
        cast,
        info,
        data,
        ..
    } = &db.movie;
    let Cast {
        person,
        note: cast_note,
        ..
    } = &db.cast;
    let Data {
        text: data_text,
        ty: data_ty,
        ..
    } = &db.data;
    let Info {
        info: info_info, ..
    } = &db.info;
    let Person {
        name: person_name,
        gender,
        ..
    } = &db.person;
    db.movie
        .with(
            info.with(gf_25ab(db))
                .and(keyword.is_in(["murder", "blood", "gore", "death", "female-nudity"]))
                .and(production_year.gt(2010))
                .and(title.rx(r"^Vampire")),
        )
        .select(
            info.with(gf_25ab(db))
                .select(info_info)
                .and(data.with(data_ty.eq("votes")).select(data_text))
                .and(title)
                .and(
                    cast.with(cast_note.is_in(writer5()).and(person.with(gender.eq("m"))))
                        .select(person)
                        .select(person_name),
                ),
        )
}

fn q25c(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        keyword,
        cast,
        info,
        data,
        ..
    } = &db.movie;
    let Cast {
        person,
        note: cast_note,
        ..
    } = &db.cast;
    let Data {
        text: data_text,
        ty: data_ty,
        ..
    } = &db.data;
    let Info {
        info: info_info, ..
    } = &db.info;
    let Person {
        name: person_name,
        gender,
        ..
    } = &db.person;
    db.movie
        .with(info.with(gf_25c(db)).and(keyword.is_in(kw7())))
        .select(
            info.with(gf_25c(db))
                .select(info_info)
                .and(data.with(data_ty.eq("votes")).select(data_text))
                .and(title)
                .and(
                    cast.with(cast_note.is_in(writer5()).and(person.with(gender.eq("m"))))
                        .select(person)
                        .select(person_name),
                ),
        )
}

fn q26a(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        kind,
        production_year,
        keyword,
        cast,
        data,
        complete_cast,
        ..
    } = &db.movie;
    let Cast {
        person, character, ..
    } = &db.cast;
    let CompleteCast {
        status, subject, ..
    } = &db.complete_cast;
    let Data {
        text: data_text,
        ty: data_ty,
        ..
    } = &db.data;
    let Person {
        name: person_name, ..
    } = &db.person;
    db.movie
        .with(
            complete_cast
                .select(subject.eq("cast").and(status.rx(r"complete")))
                .and(keyword.is_in(kw10()))
                .and(kind.eq("movie"))
                .and(production_year.gt(2000)),
        )
        .select(
            cast.with(character.rx(r"[Mm]an"))
                .select(character.and(person.select(person_name)))
                .and(
                    data.with(data_ty.eq("rating").and(data_text.gt("7.0")))
                        .select(data_text),
                )
                .and(title),
        )
}

fn q26b(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        kind,
        production_year,
        keyword,
        cast,
        data,
        complete_cast,
        ..
    } = &db.movie;
    let Cast { character, .. } = &db.cast;
    let CompleteCast {
        status, subject, ..
    } = &db.complete_cast;
    let Data {
        text: data_text,
        ty: data_ty,
        ..
    } = &db.data;
    db.movie
        .with(
            complete_cast
                .select(subject.eq("cast").and(status.rx(r"complete")))
                .and(keyword.is_in(["superhero", "marvel-comics", "based-on-comic", "fight"]))
                .and(kind.eq("movie"))
                .and(production_year.gt(2005)),
        )
        .select(
            cast.with(character.rx(r"[Mm]an"))
                .select(character)
                .and(
                    data.with(data_ty.eq("rating").and(data_text.gt("8.0")))
                        .select(data_text),
                )
                .and(title),
        )
}

fn q26c(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        kind,
        production_year,
        keyword,
        cast,
        data,
        complete_cast,
        ..
    } = &db.movie;
    let Cast { character, .. } = &db.cast;
    let CompleteCast {
        status, subject, ..
    } = &db.complete_cast;
    let Data {
        text: data_text,
        ty: data_ty,
        ..
    } = &db.data;
    let rd = data.with(data_ty.eq("rating")).select(data_text);
    db.movie
        .with(
            complete_cast
                .select(subject.eq("cast").and(status.rx(r"complete")))
                .and(keyword.is_in(kw10()))
                .and(kind.eq("movie"))
                .and(production_year.gt(2000)),
        )
        .select(
            cast.with(character.rx(r"[Mm]an"))
                .select(character)
                .and(rd)
                .and(title),
        )
}

// ===== queries: 27a–33c =====

fn co_28(db: &'static Job) -> impl Query<R = Id<Company>, D = Id<Movie>> + Drive + Probe {
    let Movie { company, .. } = &db.movie;
    let Company {
        country,
        note: company_note,
        ..
    } = &db.company;
    company.with(
        country
            .ne("[us]")
            .and(company_note.nrx(r"\(USA\)"))
            .and(company_note.rx(r"\(200.*\)")),
    )
}

fn dt_28ac(db: &'static Job) -> impl Query<R = Id<Data>, D = Id<Movie>> + Drive + Probe {
    let Movie { data, .. } = &db.movie;
    let Data {
        text: data_text,
        ty: data_ty,
        ..
    } = &db.data;
    data.with(data_ty.eq("rating").and(data_text.lt("8.5")))
}

fn dt_28b(db: &'static Job) -> impl Query<R = Id<Data>, D = Id<Movie>> + Drive + Probe {
    let Movie { data, .. } = &db.movie;
    let Data {
        text: data_text,
        ty: data_ty,
        ..
    } = &db.data;
    data.with(data_ty.eq("rating").and(data_text.gt("6.5")))
}

// Conjunct trees (∧ = Prod) — consumed via `member` only, so the value
// type stays opaque (`impl Query<D = Id<Info>> + Probe`).
fn gf_horror(db: &'static Job) -> impl Query<D = Id<Info>> + Probe {
    let Info {
        info: info_info,
        ty: info_ty,
        ..
    } = &db.info;
    info_ty
        .eq("genres")
        .and(info_info.is_in(["Horror", "Thriller"]))
}

fn gf_genre6(db: &'static Job) -> impl Query<D = Id<Info>> + Probe {
    let Info {
        info: info_info,
        ty: info_ty,
        ..
    } = &db.info;
    info_ty.eq("genres").and(info_info.is_in(genre6()))
}

fn qlink_33a(db: &'static Job) -> impl Query<R = Id<MovieLink>, D = Id<Movie>> + Drive + Probe {
    let Movie {
        kind,
        production_year,
        company,
        data,
        link,
        ..
    } = &db.movie;
    let Data {
        text: data_text,
        ty: data_ty,
        ..
    } = &db.data;
    let MovieLink {
        target,
        ty: movielink_ty,
        ..
    } = &db.movie_link;
    link.with(
        movielink_ty.is_in(link3()).and(
            target.with(
                kind.eq("tv series")
                    .and(company)
                    .and(data.with(data_ty.eq("rating").and(data_text.lt("3.0"))))
                    .and(production_year.ge(2005))
                    .and(production_year.le(2008)),
            ),
        ),
    )
}

fn qlink_33b(db: &'static Job) -> impl Query<R = Id<MovieLink>, D = Id<Movie>> + Drive + Probe {
    let Movie {
        kind,
        production_year,
        company,
        data,
        link,
        ..
    } = &db.movie;
    let Data {
        text: data_text,
        ty: data_ty,
        ..
    } = &db.data;
    let MovieLink {
        target,
        ty: movielink_ty,
        ..
    } = &db.movie_link;
    link.with(
        movielink_ty.rx(r"follow").and(
            target.with(
                kind.eq("tv series")
                    .and(company)
                    .and(data.with(data_ty.eq("rating").and(data_text.lt("3.0"))))
                    .and(production_year.eq(2007)),
            ),
        ),
    )
}

fn qlink_33c(db: &'static Job) -> impl Query<R = Id<MovieLink>, D = Id<Movie>> + Drive + Probe {
    let Movie {
        kind,
        production_year,
        company,
        data,
        link,
        ..
    } = &db.movie;
    let Data {
        text: data_text,
        ty: data_ty,
        ..
    } = &db.data;
    let MovieLink {
        target,
        ty: movielink_ty,
        ..
    } = &db.movie_link;
    link.with(
        movielink_ty.is_in(link3()).and(
            target.with(
                kind.is_in(["tv series", "episode"])
                    .and(company)
                    .and(data.with(data_ty.eq("rating").and(data_text.lt("3.5"))))
                    .and(production_year.ge(2000))
                    .and(production_year.le(2010)),
            ),
        ),
    )
}

fn q27a(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        keyword,
        info,
        complete_cast,
        link,
        ..
    } = &db.movie;
    let Company {
        name: company_name, ..
    } = &db.company;
    let CompleteCast {
        status, subject, ..
    } = &db.complete_cast;
    let Info {
        info: info_info, ..
    } = &db.info;
    db.movie
        .with(
            link.and(keyword.eq("sequel"))
                .and(production_year.between(1950, 2000))
                .and(info.select(info_info.is_in(["Sweden", "Germany", "Swedish", "German"])))
                .and(
                    complete_cast
                        .select(subject.is_in(["cast", "crew"]).and(status.eq("complete"))),
                )
                .and(follow_link(db)),
        )
        .select(
            film_or_warner_co(db)
                .select(company_name)
                .and(follow_link(db))
                .and(title),
        )
}

fn q27b(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        keyword,
        info,
        complete_cast,
        link,
        ..
    } = &db.movie;
    let Company {
        name: company_name, ..
    } = &db.company;
    let CompleteCast {
        status, subject, ..
    } = &db.complete_cast;
    let Info {
        info: info_info, ..
    } = &db.info;
    db.movie
        .with(
            link.and(keyword.eq("sequel"))
                .and(production_year.eq(1998))
                .and(info.select(info_info.is_in(["Sweden", "Germany", "Swedish", "German"])))
                .and(
                    complete_cast
                        .select(subject.is_in(["cast", "crew"]).and(status.eq("complete"))),
                )
                .and(follow_link(db)),
        )
        .select(
            film_or_warner_co(db)
                .select(company_name)
                .and(follow_link(db))
                .and(title),
        )
}

fn q27c(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        keyword,
        info,
        complete_cast,
        link,
        ..
    } = &db.movie;
    let Company {
        name: company_name, ..
    } = &db.company;
    let CompleteCast {
        status, subject, ..
    } = &db.complete_cast;
    let Info {
        info: info_info, ..
    } = &db.info;
    db.movie
        .with(
            link.and(keyword.eq("sequel"))
                .and(production_year.between(1950, 2010))
                .and(info.select(info_info.is_in(nordic9())))
                .and(complete_cast.select(subject.eq("cast").and(status.rx(r"^complete"))))
                .and(follow_link(db)),
        )
        .select(
            film_or_warner_co(db)
                .select(company_name)
                .and(follow_link(db))
                .and(title),
        )
}

fn q28a(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        kind,
        production_year,
        keyword,
        info,
        complete_cast,
        ..
    } = &db.movie;
    let Company {
        name: company_name, ..
    } = &db.company;
    let CompleteCast {
        status, subject, ..
    } = &db.complete_cast;
    let Data {
        text: data_text, ..
    } = &db.data;
    let Info {
        info: info_info,
        ty: info_ty,
        ..
    } = &db.info;
    db.movie
        .with(
            complete_cast
                .select(subject.eq("crew").and(status.ne("complete+verified")))
                .and(co_28(db))
                .and(info.select(info_ty.eq("countries").and(info_info.is_in(nordic10()))))
                .and(dt_28ac(db))
                .and(keyword.is_in(murder4()))
                .and(kind.is_in(["movie", "episode"]))
                .and(production_year.gt(2000)),
        )
        .select(
            co_28(db)
                .select(company_name)
                .and(dt_28ac(db).select(data_text))
                .and(title),
        )
}

fn q28b(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        kind,
        production_year,
        keyword,
        info,
        complete_cast,
        ..
    } = &db.movie;
    let Company {
        name: company_name, ..
    } = &db.company;
    let CompleteCast {
        status, subject, ..
    } = &db.complete_cast;
    let Data {
        text: data_text, ..
    } = &db.data;
    let Info {
        info: info_info,
        ty: info_ty,
        ..
    } = &db.info;
    db.movie
        .with(
            complete_cast
                .select(subject.eq("crew").and(status.ne("complete+verified")))
                .and(co_28(db))
                .and(
                    info.select(
                        info_ty
                            .eq("countries")
                            .and(info_info.is_in(["Sweden", "Germany", "Swedish", "German"])),
                    ),
                )
                .and(dt_28b(db))
                .and(keyword.is_in(murder4()))
                .and(kind.is_in(["movie", "episode"]))
                .and(production_year.gt(2005)),
        )
        .select(
            co_28(db)
                .select(company_name)
                .and(dt_28b(db).select(data_text))
                .and(title),
        )
}

fn q28c(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        kind,
        production_year,
        keyword,
        info,
        complete_cast,
        ..
    } = &db.movie;
    let Company {
        name: company_name, ..
    } = &db.company;
    let CompleteCast {
        status, subject, ..
    } = &db.complete_cast;
    let Data {
        text: data_text, ..
    } = &db.data;
    let Info {
        info: info_info,
        ty: info_ty,
        ..
    } = &db.info;
    db.movie
        .with(
            complete_cast
                .select(subject.eq("cast").and(status.eq("complete")))
                .and(co_28(db))
                .and(info.select(info_ty.eq("countries").and(info_info.is_in(nordic10()))))
                .and(dt_28ac(db))
                .and(keyword.is_in(murder4()))
                .and(kind.is_in(["movie", "episode"]))
                .and(production_year.gt(2005)),
        )
        .select(
            co_28(db)
                .select(company_name)
                .and(dt_28ac(db).select(data_text))
                .and(title),
        )
}

fn q29a(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        keyword,
        company,
        cast,
        info,
        complete_cast,
        ..
    } = &db.movie;
    let Cast {
        person,
        role,
        note: cast_note,
        character,
        ..
    } = &db.cast;
    let Company { country, .. } = &db.company;
    let CompleteCast {
        status, subject, ..
    } = &db.complete_cast;
    let Info {
        info: info_info,
        ty: info_ty,
        ..
    } = &db.info;
    let Person {
        name: person_name,
        gender,
        alias,
        bio,
        ..
    } = &db.person;
    let PersonInfo {
        ty: personinfo_ty, ..
    } = &db.person_info;
    db.movie
        .with(
            complete_cast
                .select(subject.eq("cast").and(status.eq("complete+verified")))
                .and(company.select(country).eq("[us]"))
                .and(
                    info.select(
                        info_ty
                            .eq("release dates")
                            .and(info_info.rx(r"^Japan:.*200|^USA:.*200")),
                    ),
                )
                .and(keyword.eq("computer-animation"))
                .and(title.eq("Shrek 2"))
                .and(production_year.ge(2000))
                .and(production_year.le(2010)),
        )
        .select(
            cast.with(
                cast_note
                    .is_in(voice3())
                    .and(role.eq("actress"))
                    .and(character.eq("Queen"))
                    .and(
                        person.with(
                            gender
                                .eq("f")
                                .and(person_name.rx(r"An"))
                                .and(alias)
                                .and(bio.select(personinfo_ty.eq("trivia"))),
                        ),
                    ),
            )
            .select(character.and(person.select(person_name)))
            .and(title),
        )
}

fn q29b(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        keyword,
        company,
        cast,
        info,
        complete_cast,
        ..
    } = &db.movie;
    let Cast {
        person,
        role,
        note: cast_note,
        character,
        ..
    } = &db.cast;
    let Company { country, .. } = &db.company;
    let CompleteCast {
        status, subject, ..
    } = &db.complete_cast;
    let Info {
        info: info_info,
        ty: info_ty,
        ..
    } = &db.info;
    let Person {
        name: person_name,
        gender,
        alias,
        bio,
        ..
    } = &db.person;
    let PersonInfo {
        ty: personinfo_ty, ..
    } = &db.person_info;
    db.movie
        .with(
            complete_cast
                .select(subject.eq("cast").and(status.eq("complete+verified")))
                .and(company.select(country).eq("[us]"))
                .and(info.select(info_ty.eq("release dates").and(info_info.rx(r"^USA:.*200"))))
                .and(keyword.eq("computer-animation"))
                .and(title.eq("Shrek 2"))
                .and(production_year.ge(2000))
                .and(production_year.le(2005)),
        )
        .select(
            cast.with(
                cast_note
                    .is_in(voice3())
                    .and(role.eq("actress"))
                    .and(character.eq("Queen"))
                    .and(
                        person.with(
                            gender
                                .eq("f")
                                .and(person_name.rx(r"An"))
                                .and(alias)
                                .and(bio.select(personinfo_ty.eq("height"))),
                        ),
                    ),
            )
            .select(character.and(person.select(person_name)))
            .and(title),
        )
}

fn q29c(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        keyword,
        company,
        cast,
        info,
        complete_cast,
        ..
    } = &db.movie;
    let Cast {
        person,
        role,
        note: cast_note,
        character,
        ..
    } = &db.cast;
    let Company { country, .. } = &db.company;
    let CompleteCast {
        status, subject, ..
    } = &db.complete_cast;
    let Info {
        info: info_info,
        ty: info_ty,
        ..
    } = &db.info;
    let Person {
        name: person_name,
        gender,
        alias,
        bio,
        ..
    } = &db.person;
    let PersonInfo {
        ty: personinfo_ty, ..
    } = &db.person_info;
    db.movie
        .with(
            complete_cast
                .select(subject.eq("cast").and(status.eq("complete+verified")))
                .and(company.select(country).eq("[us]"))
                .and(
                    info.select(
                        info_ty
                            .eq("release dates")
                            .and(info_info.rx(r"^Japan:.*200|^USA:.*200")),
                    ),
                )
                .and(keyword.eq("computer-animation"))
                .and(production_year.ge(2000))
                .and(production_year.le(2010)),
        )
        .select(
            cast.with(
                cast_note.is_in(voice4()).and(role.eq("actress")).and(
                    person.with(
                        gender
                            .eq("f")
                            .and(person_name.rx(r"An"))
                            .and(alias)
                            .and(bio.select(personinfo_ty.eq("trivia"))),
                    ),
                ),
            )
            .select(character.and(person.select(person_name)))
            .and(title),
        )
}

fn q30a(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        keyword,
        cast,
        info,
        data,
        complete_cast,
        ..
    } = &db.movie;
    let Cast {
        person,
        note: cast_note,
        ..
    } = &db.cast;
    let CompleteCast {
        status, subject, ..
    } = &db.complete_cast;
    let Data {
        text: data_text,
        ty: data_ty,
        ..
    } = &db.data;
    let Info {
        info: info_info, ..
    } = &db.info;
    let Person {
        name: person_name,
        gender,
        ..
    } = &db.person;
    db.movie
        .with(
            complete_cast
                .select(
                    subject
                        .is_in(["cast", "crew"])
                        .and(status.eq("complete+verified")),
                )
                .and(info.with(gf_horror(db)))
                .and(keyword.is_in(kw7()))
                .and(production_year.gt(2000)),
        )
        .select(
            info.with(gf_horror(db))
                .select(info_info)
                .and(data.with(data_ty.eq("votes")).select(data_text))
                .and(title)
                .and(
                    cast.with(cast_note.is_in(writer5()).and(person.with(gender.eq("m"))))
                        .select(person)
                        .select(person_name),
                ),
        )
}

fn q30b(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        keyword,
        cast,
        info,
        data,
        complete_cast,
        ..
    } = &db.movie;
    let Cast {
        person,
        note: cast_note,
        ..
    } = &db.cast;
    let CompleteCast {
        status, subject, ..
    } = &db.complete_cast;
    let Data {
        text: data_text,
        ty: data_ty,
        ..
    } = &db.data;
    let Info {
        info: info_info, ..
    } = &db.info;
    let Person {
        name: person_name,
        gender,
        ..
    } = &db.person;
    db.movie
        .with(
            complete_cast
                .select(
                    subject
                        .is_in(["cast", "crew"])
                        .and(status.eq("complete+verified")),
                )
                .and(info.with(gf_horror(db)))
                .and(keyword.is_in(kw7()))
                .and(production_year.gt(2000))
                .and(title.rx(r"Freddy|Jason|^Saw")),
        )
        .select(
            info.with(gf_horror(db))
                .select(info_info)
                .and(data.with(data_ty.eq("votes")).select(data_text))
                .and(title)
                .and(
                    cast.with(cast_note.is_in(writer5()).and(person.with(gender.eq("m"))))
                        .select(person)
                        .select(person_name),
                ),
        )
}

fn q30c(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        keyword,
        cast,
        info,
        data,
        complete_cast,
        ..
    } = &db.movie;
    let Cast {
        person,
        note: cast_note,
        ..
    } = &db.cast;
    let CompleteCast {
        status, subject, ..
    } = &db.complete_cast;
    let Data {
        text: data_text,
        ty: data_ty,
        ..
    } = &db.data;
    let Info {
        info: info_info, ..
    } = &db.info;
    let Person {
        name: person_name,
        gender,
        ..
    } = &db.person;
    db.movie
        .with(
            complete_cast
                .select(subject.eq("cast").and(status.eq("complete+verified")))
                .and(info.with(gf_genre6(db)))
                .and(keyword.is_in(kw7())),
        )
        .select(
            info.with(gf_genre6(db))
                .select(info_info)
                .and(data.with(data_ty.eq("votes")).select(data_text))
                .and(title)
                .and(
                    cast.with(cast_note.is_in(writer5()).and(person.with(gender.eq("m"))))
                        .select(person)
                        .select(person_name),
                ),
        )
}

fn q31a(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        keyword,
        company,
        cast,
        info,
        data,
        ..
    } = &db.movie;
    let Cast {
        person,
        note: cast_note,
        ..
    } = &db.cast;
    let Company {
        name: company_name, ..
    } = &db.company;
    let Data {
        text: data_text,
        ty: data_ty,
        ..
    } = &db.data;
    let Info {
        info: info_info, ..
    } = &db.info;
    let Person {
        name: person_name,
        gender,
        ..
    } = &db.person;
    db.movie
        .with(
            company
                .select(company_name)
                .rx(r"^Lionsgate")
                .and(info.with(gf_horror(db)))
                .and(keyword.is_in(kw7())),
        )
        .select(
            info.with(gf_horror(db))
                .select(info_info)
                .and(data.with(data_ty.eq("votes")).select(data_text))
                .and(title)
                .and(
                    cast.with(cast_note.is_in(writer5()).and(person.with(gender.eq("m"))))
                        .select(person)
                        .select(person_name),
                ),
        )
}

fn q31b(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        keyword,
        company,
        cast,
        info,
        data,
        ..
    } = &db.movie;
    let Cast {
        person,
        note: cast_note,
        ..
    } = &db.cast;
    let Company {
        name: company_name,
        note: company_note,
        ..
    } = &db.company;
    let Data {
        text: data_text,
        ty: data_ty,
        ..
    } = &db.data;
    let Info {
        info: info_info, ..
    } = &db.info;
    let Person {
        name: person_name,
        gender,
        ..
    } = &db.person;
    db.movie
        .with(
            company
                .select(
                    company_name
                        .rx(r"^Lionsgate")
                        .and(company_note.rx(r"\(Blu-ray\)")),
                )
                .and(info.with(gf_horror(db)))
                .and(keyword.is_in(kw7()))
                .and(production_year.gt(2000))
                .and(title.rx(r"Freddy|Jason|^Saw")),
        )
        .select(
            info.with(gf_horror(db))
                .select(info_info)
                .and(data.with(data_ty.eq("votes")).select(data_text))
                .and(title)
                .and(
                    cast.with(cast_note.is_in(writer5()).and(person.with(gender.eq("m"))))
                        .select(person)
                        .select(person_name),
                ),
        )
}

fn q31c(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        keyword,
        company,
        cast,
        info,
        data,
        ..
    } = &db.movie;
    let Cast {
        person,
        note: cast_note,
        ..
    } = &db.cast;
    let Company {
        name: company_name, ..
    } = &db.company;
    let Data {
        text: data_text,
        ty: data_ty,
        ..
    } = &db.data;
    let Info {
        info: info_info, ..
    } = &db.info;
    let Person {
        name: person_name, ..
    } = &db.person;
    db.movie
        .with(
            company
                .select(company_name)
                .rx(r"^Lionsgate")
                .and(info.with(gf_genre6(db)))
                .and(keyword.is_in(kw7())),
        )
        .select(
            info.with(gf_genre6(db))
                .select(info_info)
                .and(data.with(data_ty.eq("votes")).select(data_text))
                .and(title)
                .and(
                    cast.with(cast_note.is_in(writer5()))
                        .select(person)
                        .select(person_name),
                ),
        )
}

// q32a/q32b differ only in the keyword constant.
fn q32(db: &'static Job, kw: &'static str) -> impl Drive<R: Row> {
    let Movie {
        title,
        keyword,
        link,
        ..
    } = &db.movie;
    let MovieLink {
        target,
        ty: movielink_ty,
        ..
    } = &db.movie_link;
    db.movie.with(link.and(keyword.eq(kw))).select(
        link.select(movielink_ty)
            .and(title)
            .and(link.select(target).select(title)),
    )
}

fn q32a(db: &'static Job) -> impl Drive<R: Row> {
    q32(db, "10,000-mile-club")
}
fn q32b(db: &'static Job) -> impl Drive<R: Row> {
    q32(db, "character-name-in-title")
}

fn q33a(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        kind,
        company,
        data,
        ..
    } = &db.movie;
    let Company {
        name: company_name,
        country,
        ..
    } = &db.company;
    let Data {
        text: data_text,
        ty: data_ty,
        ..
    } = &db.data;
    let MovieLink { target, .. } = &db.movie_link;
    db.movie
        .with(
            kind.eq("tv series")
                .and(company.select(country).eq("[us]"))
                .and(qlink_33a(db)),
        )
        .select(
            company
                .with(country.eq("[us]"))
                .select(company_name)
                .and(
                    qlink_33a(db)
                        .select(target)
                        .select(company)
                        .select(company_name),
                )
                .and(data.with(data_ty.eq("rating")).select(data_text))
                .and(
                    qlink_33a(db).select(target).select(
                        data.with(data_ty.eq("rating").and(data_text.lt("3.0")))
                            .select(data_text),
                    ),
                )
                .and(title)
                .and(qlink_33a(db).select(target).select(title)),
        )
}

fn q33b(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        kind,
        company,
        data,
        ..
    } = &db.movie;
    let Company {
        name: company_name,
        country,
        ..
    } = &db.company;
    let Data {
        text: data_text,
        ty: data_ty,
        ..
    } = &db.data;
    let MovieLink { target, .. } = &db.movie_link;
    db.movie
        .with(
            kind.eq("tv series")
                .and(company.select(country).eq("[nl]"))
                .and(qlink_33b(db)),
        )
        .select(
            company
                .with(country.eq("[nl]"))
                .select(company_name)
                .and(
                    qlink_33b(db)
                        .select(target)
                        .select(company)
                        .select(company_name),
                )
                .and(data.with(data_ty.eq("rating")).select(data_text))
                .and(
                    qlink_33b(db).select(target).select(
                        data.with(data_ty.eq("rating").and(data_text.lt("3.0")))
                            .select(data_text),
                    ),
                )
                .and(title)
                .and(qlink_33b(db).select(target).select(title)),
        )
}

fn q33c(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        kind,
        company,
        data,
        ..
    } = &db.movie;
    let Company {
        name: company_name,
        country,
        ..
    } = &db.company;
    let Data {
        text: data_text,
        ty: data_ty,
        ..
    } = &db.data;
    let MovieLink { target, .. } = &db.movie_link;
    db.movie
        .with(
            kind.is_in(["tv series", "episode"])
                .and(company.select(country).ne("[us]"))
                .and(qlink_33c(db)),
        )
        .select(
            company
                .with(country.ne("[us]"))
                .select(company_name)
                .and(
                    qlink_33c(db)
                        .select(target)
                        .select(company)
                        .select(company_name),
                )
                .and(data.with(data_ty.eq("rating")).select(data_text))
                .and(
                    qlink_33c(db).select(target).select(
                        data.with(data_ty.eq("rating").and(data_text.lt("3.5")))
                            .select(data_text),
                    ),
                )
                .and(title)
                .and(qlink_33c(db).select(target).select(title)),
        )
}

// ===== Reference example of the method-chain form. Kept as a registered
// query so `cargo asm` always has a known symbol to inspect. =====

// q6a — movie : (year > 2010) ∧ (keyword == "marvel-...")
//             → (keyword == "marvel-...") × title
//             × (cast → person → name ~ "Downey…")
//
// Operator legend (engine.rs::QueryExt; everything roots on IntoQuery, so
// the destructured columns — `keyword`, `title`, `cast` — mix freely with
// plan nodes):
//   .select(b)    composition (a set is an identity relation, so set∘Query is
//            the same Compose — no keyset projection), and also how you
//            navigate: `cast.select(person).select(person_name)` walks
//            Movie → Cast → Person → name, one column per hop
//            Lookup tables are not hops: `kind`, `keyword`, `role`, `ty`
//            are dictionary-encoded string columns (`Dict`/`DictSet`),
//            so `kind.eq("movie")` compares the string directly
//   .and(b)    product (×)
//   .and     ∧ — alias for the product; conjunct trees are consumed via
//            the flat short-circuit `member` (restriction = `.with`)
//   .or      ∨ — probe-only membership union (drive with `.union`)
//   .minus   value-bearing difference (key-based member test)
//   .with    restriction — keep rows whose value is a member
//   .eq / .ne / .gt / .lt / .ge / .le / .is_in / .rx / .nrx  predicates
pub fn q6a_methods(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        keyword,
        cast,
        ..
    } = &db.movie;
    let Cast { person, .. } = &db.cast;
    let Person {
        name: person_name, ..
    } = &db.person;
    let kw_marvel = || keyword.eq("marvel-cinematic-universe");
    let q = db
        .movie
        .with(production_year.gt(2010).and(kw_marvel()))
        .select(
            kw_marvel().and(title).and(
                cast.select(person)
                    .select(person_name)
                    .rx(r"Downey.*Robert"),
            ),
        );
    q
}
