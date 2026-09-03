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
#[cfg(feature = "test")]
use crate::job_queries::helpers::min_result;
use crate::job_queries::helpers::{Row, film_or_warner_co, follow_link, min_row};
use crate::job_queries::sets::{
    genre6, kw7, kw8, kw10, link3, murder4, nordic8, nordic9, nordic10, voice3, voice4, writer5,
};
use crate::job_schema::*;

macro_rules! job_queries {
    ($($name:literal => $query:ident $(.$adapt:ident($($arg:tt)*))*, $oracle:literal;)*) => {
        /// Every query, paired with the output the full IMDb dataset produces.
        pub const ENTRIES: &[super::Entry] = &[
            $(($name, $oracle, |db| min_row($query(db))),)*
        ];

        /// The same queries as typed cells, for comparison against DuckDB.
        #[cfg(feature = "test")]
        pub fn differential(
            name: &str,
            db: &'static Job,
        ) -> Result<Vec<crate::job_queries::helpers::Result>, String> {
            match name {
                $($name => Ok(min_result($query(db) $(.$adapt($($arg)*))*)),)*
                _ => Err(format!("unknown JOB query {name}")),
            }
        }
    };
}

// Each line names a query once: its JOB name, the function implementing it,
// and the oracle string the full IMDb dataset produces. A trailing method
// chain adapts the plan for the differential comparison only, where the JOB
// SQL projects a column that the benchmark reports once (queries 17a-c).
job_queries! {
    // --- templates 1-5, 11-15, 22 ---
    "2a" => q2a, "'Doc'";
    "2d" => q2d, "& Teller";
    "3b" => q3b, "300: Rise of an Empire";
    "4a" => q4a, "5.1 || & Teller 2";
    "13a" => q13a, "Afghanistan:24 June 2012 || 1.0 || &Me";
    "11a" => q11a, "Churchill Films || followed by || Batman Beyond";
    "22a" => q22a, "(empty)";
    "1a" => q1a, "(A Warner Bros.-First National Picture) (presents) || A Clockwork Orange || 1934";
    "5a" => q5a, "(empty)";
    "12a" => q12a, "10th Grade Reunion Films || 8.1 || 3:20";
    "14a" => q14a, "1.0 || $lowdown";
    "1b" => q1b, "(Set Decoration Rentals) (uncredited) || Disaster Movie || 2008";
    "2b" => q2b, "'Doc'";
    "2c" => q2c, "(empty)";
    "3a" => q3a, "2 Days in New York";
    "3c" => q3c, "& Teller 2";
    "4b" => q4b, "9.1 || Batman: Arkham City";
    "11b" => q11b, "Filmlance International AB || follows || The Money Man";
    "13b" => q13b, "501audio || 1.8 || 5 Time Champion";
    "1c" => q1c, "(co-production) || Intouchables || 2011";
    "1d" => q1d, "(Set Decoration Rentals) (uncredited) || Disaster Movie || 2004";
    "4c" => q4c, "2.1 || & Teller 2";
    "12b" => q12b, "$10,000 || Birdemic: Shock and Terror";
    "12c" => q12c, "\"Oh That Gus!\" || 7.1 || $1.11";
    "13c" => q13c, "DL Sites || 1.8 || Champion";
    "14b" => q14b, "6.4 || Of Dolls and Murder";
    "14c" => q14c, "1.0 || $lowdown";
    "22b" => q22b, "(empty)";
    "22c" => q22c, "(empty)";
    // --- 22d, 5b, 5c, 15a-d, 11c-d, 13d, 6a-f ---
    "22d" => q22d, "(#1.1) || 2.0 || 13 Productions";
    "5b" => q5b, "(empty)";
    "5c" => q5c, "11,830,420";
    "15a" => q15a, "USA:1 June 2007 || Battlestar Galactica: The Resistance";
    "15b" => q15b, "USA:27 April 2007 || RoboCop vs Terminator";
    "15c" => q15c, "USA:1 April 2003 || 24: Day Six - Debrief";
    "15d" => q15d, "(Not So) Instant Photo || 06/05";
    "11c" => q11c, "20th Century Fox Home Entertainment || (1997-2002) (worldwide) (all media) || 24";
    "11d" => q11d, "13th Street || (1954) (UK) (TV) || ...denn sie wissen nicht, was sie tun";
    "13d" => q13d, "\"O\" Films || 1.0 || #54 Meets #47";
    "6a" => q6a, "marvel-cinematic-universe || Downey Jr., Robert || Iron Man 3";
    "6b" => q6b, "based-on-comic || Downey Jr., Robert || The Avengers 2";
    "6c" => q6c, "marvel-cinematic-universe || Downey Jr., Robert || The Avengers 2";
    "6d" => q6d, "based-on-comic || Downey Jr., Robert || 2008 MTV Movie Awards";
    "6e" => q6e, "marvel-cinematic-universe || Downey Jr., Robert || Iron Man 3";
    "6f" => q6f, "based-on-comic || \"Steff\", Stefanie Oxmann Mcgaha || & Teller 2";
    // --- 7a-c, 8a-d, 9a-d, 10a-c ---
    "7a" => q7a, "Antonioni, Michelangelo || Dressed to Kill";
    "7b" => q7b, "De Palma, Brian || Dressed to Kill";
    "7c" => q7c, "50 Cent || \"Boo\" Arnold was born Earl Arnold in Hattiesburg, Mississippi in 1966. His father gave him the nickname 'Boo' early in life and it stuck through grade school, high school, and college. He is still known as \"Boo\" to family and friends.  Raised in central Texas, Arnold played baseball at Texas Tech University where he graduated with a BA in Advertising and Marketing. While at Texas Tech he was also a member of the Texas Epsilon chapter of Phi Delta Theta fraternity. After college he worked with Young Life, an outreach to high school students, in San Antonio, Texas.  While with Young Life Arnold began taking extension courses through Fuller Theological Seminary and ultimately went full-time to Gordon-Conwell Theological Seminary in Boston, Massachusetts. At Gordon-Conwell he completed a Master's Degree in Divinity studying Theology, Philosophy, Church History, Biblical Languages (Hebrew & Greek), and Exegetical Methods. Following seminary he was involved with reconciliation efforts in the former Yugoslavia shortly after the war ended there in1995.  Arnold started acting in his early thirties in Texas. After an encouraging visit to Los Angeles where he spent time with childhood friend George Eads (of CSI Las Vegas) he decided to move to Los Angeles in 2001 to pursue acting full-time. While in Los Angeles he has studied acting with Judith Weston at Judith Weston Studio for Actors and Directors.  Arnold's acting career has been one of steady development, booking co-star and guest-star roles in nighttime television. He guest-starred opposite of Jane Seymour on the night time television drama Justice. He played the lead, Michael Hollister, in the film The Seer, written and directed by Patrick Masset (Friday Night Lights).  He was nominated Best Actor in the168 Film Festival for the role of Phil Stevens in the short-film Useless. In Useless he played a US Marshal who must choose between mercy and justice as he confronts the man who murdered his father. Arnold's performance in Useless confirmed his ability to carry lead roles, and he continues to work toward solidifying himself as a male lead in film and television.  Arnold married fellow Texan Stacy Rudd of San Antonio in 2003 and they are now raising their three children in the Los Angeles area.";
    "8a" => q8a, "Chambers, Linda || .hack//Quantum";
    "8b" => q8b, "Chambers, Linda || Dragon Ball Z: Shin Budokai";
    "8c" => q8c, "\"A.J.\" || #1 Cheerleader Camp";
    "8d" => q8d, "\"Jenny from the Block\" || #1 Cheerleader Camp";
    "9a" => q9a, "AJ || Airport Announcer || Blue Harvest";
    "9b" => q9b, "AJ || Airport Announcer || Bassett, Angela || Blue Harvest";
    "9c" => q9c, "'Annette' || 2nd Balladeer || Alborg, Ana Esther || (1975-01-20)";
    "9d" => q9d, "!!!, Toy || Aaron, Caroline || \"Cockamamie's\" Salesgirl || $15,000.00 Error";
    "10a" => q10a, "Actor || 12 Rounds";
    "10b" => q10b, "(empty)";
    "10c" => q10c, "Himself || Evil Eyes: Behind the Scenes";
    // --- templates 16-18 ---
    "16a" => q16a, "Adams, Stan || Carol Burnett vs. Anthony Perkins";
    "16b" => q16b, "!!!, Toy || & Teller";
    "16c" => q16c, "\"Brooklyn\" Tony Danza || (#1.5)";
    "16d" => q16d, "\"Brooklyn\" Tony Danza || (#1.5)";
    "17a" => q17a.map(|name| (name, name)), "B, Khaz";
    "17b" => q17b.map(|name| (name, name)), "Z'Dar, Robert";
    "17c" => q17c.map(|name| (name, name)), "X'Volaitis, John";
    "17d" => q17d, "Abrahamsson, Bertil";
    "17e" => q17e, "$hort, Too";
    "17f" => q17f, "'El Galgo PornoStar', Blanquito";
    "18a" => q18a, "$1,000 || 10 || 40 Days and 40 Nights";
    "18b" => q18b, "Horror || 8.1 || Agorable";
    "18c" => q18c, "Action || 10 || #PostModem";
    // --- 19a-26c ---
    "19a" => q19a, "Angeline, Moriah || Blue Harvest";
    "19b" => q19b, "Jolie, Angelina || Kung Fu Panda";
    "19c" => q19c, "Alborg, Ana Esther || .hack//Akusei heni vol. 2";
    "19d" => q19d, "Aaron, Caroline || $9.99";
    "20a" => q20a, "Disaster Movie";
    "20b" => q20b, "Iron Man";
    "20c" => q20c, "Abell, Alistair || ...And Then I...";
    "21a" => q21a, "Det Danske Filminstitut || followed by || Der Serienkiller - Klinge des Todes";
    "21b" => q21b, "Filmlance International AB || followed by || Hämndens pris";
    "21c" => q21c, "Churchill Films || followed by || Batman Beyond";
    "23a" => q23a, "movie || The Analysts";
    "23b" => q23b, "movie || The Big Mope";
    "23c" => q23c, "movie || Dirt Merchant";
    "24a" => q24a, "Additional Voices || Baker, Andrea || Baiohazâdo 6";
    "24b" => q24b, "Tigress || Jolie, Angelina || Kung Fu Panda 2";
    "25a" => q25a, "Horror || 10 || -- And Now the Screaming Starts! || Abdallah, Damon";
    "25b" => q25b, "Horror || 138 || Vampire Boys || Campbell, Jeremiah";
    "25c" => q25c, "Action || 10 || $ || Aakeson, Kim Fupz";
    "26a" => q26a, "'Agua' Man || Acereda, Hermie || 7.1 || 3:10 to Yuma";
    "26b" => q26b, "Bank Manager || 8.2 || Inception";
    "26c" => q26c, "'Agua' Man || 1.9 || 12 Rounds";
    // --- 27a-33c ---
    "27a" => q27a, "Det Danske Filminstitut || followed by || Spår i mörker";
    "27b" => q27b, "Filmlance International AB || followed by || Vita nätter";
    "27c" => q27c, "Det Danske Filminstitut || followed by || Spår i mörker";
    "28a" => q28a, "01 Distribuzione || 2.9 || (#1.1)";
    "28b" => q28b, "20th Century Fox || 6.6 || (#1.1)";
    "28c" => q28c, "01 Distribuzione || 1.9 || (#1.1)";
    "29a" => q29a, "Queen || Andrews, Julie || Shrek 2";
    "29b" => q29b, "Queen || Andrews, Julie || Shrek 2";
    "29c" => q29c, "Lola || Andrews, Julie || Hoodwinked!";
    "30a" => q30a, "Horror || 100356 || 16 Blocks || Abrams, J.J.";
    "30b" => q30b, "Horror || 194782 || Freddy vs. Jason || Shannon, Damian";
    "30c" => q30c, "Action || 100356 || $ || Abernathy, Lewis";
    "31a" => q31a, "Horror || 1040 || 2001 Maniacs || Agnew, Jim";
    "31b" => q31b, "Horror || 129755 || Saw || Bousman, Darren Lynn";
    "31c" => q31c, "Action || 1008 || 11:14 || Abraham, Brad";
    "32a" => q32a, "(empty)";
    "32b" => q32b, "alternate language version of || 12 oz. Mouse || 'Angel': Season 2 Overview";
    "33a" => q33a, "495 Productions || 495 Productions || 3.3 || 2.7 || A Double Shot at Love || A Shot at Love with Tila Tequila";
    "33b" => q33b, "MTV Netherlands || 495 Productions || 3.3 || 2.7 || A Double Shot at Love || A Shot at Love with Tila Tequila";
    "33c" => q33c, "2BE || 495 Productions || 1.3 || 1.0 || A Double Shot at Love || A Double Shot at Love";
    // --- method-chain demo ---
    "6a/method" => q6a_methods, "marvel-cinematic-universe || Downey Jr., Robert || Iron Man 3";
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
