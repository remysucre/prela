// queries: 7a-c, 8a-d, 9a-d, 10a-c (queries.jl lines 591-753)

use crate::engine::*;
use crate::job_queries::helpers::{Row, min_row};
use crate::job_queries::sets::voice4;
use crate::job_schema::*;

pub const ENTRIES: &[super::Entry] = &[
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
];

fn q7a(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        title,
        production_year,
        cast,
        linked_by,
        ..
    } = &db.movie;
    let AkaName {
        text: akaname_text, ..
    } = &db.aka_name;
    let Cast { person, .. } = &db.cast;
    let InfoType {
        text: infotype_text,
        ..
    } = &db.info_type;
    let LinkType {
        text: linktype_text,
        ..
    } = &db.link_type;
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
    let movie = db.movie.all();
    movie
        .with(
            production_year.ge(1980).and(production_year.le(1995)).and(
                linked_by
                    .select(movielink_ty)
                    .select(linktype_text)
                    .eq("features"),
            ),
        )
        .select(
            cast.select(
                person
                    .with(
                        alias
                            .select(akaname_text)
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
                                        .select(infotype_text)
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
    let AkaName {
        text: akaname_text, ..
    } = &db.aka_name;
    let Cast { person, .. } = &db.cast;
    let InfoType {
        text: infotype_text,
        ..
    } = &db.info_type;
    let LinkType {
        text: linktype_text,
        ..
    } = &db.link_type;
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
    let movie = db.movie.all();
    movie
        .with(
            production_year.ge(1980).and(production_year.le(1984)).and(
                linked_by
                    .select(movielink_ty)
                    .select(linktype_text)
                    .eq("features"),
            ),
        )
        .select(
            cast.select(
                person
                    .with(
                        alias
                            .select(akaname_text)
                            .rx(r"a")
                            .and(name_pcode_cf.rx(r"^D"))
                            .and(gender.eq("m"))
                            .and(
                                bio.select(
                                    personinfo_ty
                                        .select(infotype_text)
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
    let InfoType {
        text: infotype_text,
        ..
    } = &db.info_type;
    let PersonInfo {
        ty: personinfo_ty,
        note: personinfo_note,
        ..
    } = &db.person_info;
    personinfo_ty
        .select(infotype_text)
        .eq("mini biography")
        .and(personinfo_note)
}

fn q7c(db: &'static Job) -> impl Drive<R: Row> {
    let Movie {
        production_year,
        cast,
        linked_by,
        ..
    } = &db.movie;
    let AkaName {
        text: akaname_text, ..
    } = &db.aka_name;
    let Cast { person, .. } = &db.cast;
    let LinkType {
        text: linktype_text,
        ..
    } = &db.link_type;
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
    let movie = db.movie.all();
    movie
        .with(production_year.ge(1980).and(production_year.le(2010)).and(
            linked_by.select(movielink_ty).select(linktype_text).is_in([
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
                            .select(akaname_text)
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
    let AkaName {
        text: akaname_text, ..
    } = &db.aka_name;
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
    let RoleType {
        text: roletype_text,
        ..
    } = &db.role_type;
    let movie = db.movie.all();
    movie
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
                    .and(role.select(roletype_text).eq("actress"))
                    .and(person.with(person_name.rx(r"Yo").and(person_name.nrx(r"Yu")))),
            )
            .select(person)
            .select(alias)
            .select(akaname_text)
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
    let AkaName {
        text: akaname_text, ..
    } = &db.aka_name;
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
    let RoleType {
        text: roletype_text,
        ..
    } = &db.role_type;
    let movie = db.movie.all();
    movie
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
                    .and(role.select(roletype_text).eq("actress"))
                    .and(person.with(person_name.rx(r"Yo").and(person_name.nrx(r"Yu")))),
            )
            .select(person)
            .select(alias)
            .select(akaname_text)
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
    let AkaName {
        text: akaname_text, ..
    } = &db.aka_name;
    let Cast { person, role, .. } = &db.cast;
    let Company { country, .. } = &db.company;
    let Person { alias, .. } = &db.person;
    let RoleType {
        text: roletype_text,
        ..
    } = &db.role_type;
    let movie = db.movie.all();
    movie.with(company.select(country).eq("[us]")).select(
        cast.with(role.select(roletype_text).eq(role_))
            .select(person)
            .select(alias)
            .select(akaname_text)
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
    let AkaName {
        text: akaname_text, ..
    } = &db.aka_name;
    let Cast {
        person,
        role,
        note: cast_note,
        character,
        ..
    } = &db.cast;
    let Character {
        text: character_text,
        ..
    } = &db.character;
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
    let RoleType {
        text: roletype_text,
        ..
    } = &db.role_type;
    let movie = db.movie.all();
    movie
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
                    .and(role.select(roletype_text).eq("actress"))
                    .and(person.with(gender.eq("f").and(person_name.rx(r"Ang")))),
            )
            .select(
                person
                    .select(alias)
                    .select(akaname_text)
                    .and(character.select(character_text)),
            )
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
    let AkaName {
        text: akaname_text, ..
    } = &db.aka_name;
    let Cast {
        person,
        role,
        note: cast_note,
        character,
        ..
    } = &db.cast;
    let Character {
        text: character_text,
        ..
    } = &db.character;
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
    let RoleType {
        text: roletype_text,
        ..
    } = &db.role_type;
    let movie = db.movie.all();
    movie
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
                    .and(role.select(roletype_text).eq("actress"))
                    .and(person.with(gender.eq("f").and(person_name.rx(r"Angel")))),
            )
            .select(
                person
                    .select(alias)
                    .select(akaname_text)
                    .and(character.select(character_text))
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
    let AkaName {
        text: akaname_text, ..
    } = &db.aka_name;
    let Cast {
        person,
        role,
        note: cast_note,
        character,
        ..
    } = &db.cast;
    let Character {
        text: character_text,
        ..
    } = &db.character;
    let Company { country, .. } = &db.company;
    let Person {
        name: person_name,
        gender,
        alias,
        ..
    } = &db.person;
    let RoleType {
        text: roletype_text,
        ..
    } = &db.role_type;
    let movie = db.movie.all();
    movie.with(company.select(country).eq("[us]")).select(
        cast.with(
            cast_note
                .is_in(voice4())
                .and(role.select(roletype_text).eq("actress"))
                .and(person.with(gender.eq("f").and(person_name.rx(r"An")))),
        )
        .select(
            person
                .select(alias)
                .select(akaname_text)
                .and(character.select(character_text))
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
    let AkaName {
        text: akaname_text, ..
    } = &db.aka_name;
    let Cast {
        person,
        role,
        note: cast_note,
        character,
        ..
    } = &db.cast;
    let Character {
        text: character_text,
        ..
    } = &db.character;
    let Company { country, .. } = &db.company;
    let Person {
        name: person_name,
        gender,
        alias,
        ..
    } = &db.person;
    let RoleType {
        text: roletype_text,
        ..
    } = &db.role_type;
    let movie = db.movie.all();
    movie.with(company.select(country).eq("[us]")).select(
        cast.with(
            cast_note
                .is_in(voice4())
                .and(role.select(roletype_text).eq("actress"))
                .and(person.with(gender.eq("f"))),
        )
        .select(
            person
                .select(alias)
                .select(akaname_text)
                .and(person.select(person_name))
                .and(character.select(character_text)),
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
    let Character {
        text: character_text,
        ..
    } = &db.character;
    let Company { country, .. } = &db.company;
    let RoleType {
        text: roletype_text,
        ..
    } = &db.role_type;
    let movie = db.movie.all();
    movie
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
                    .and(role.select(roletype_text).eq("actor")),
            )
            .select(character)
            .select(character_text)
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
    let Character {
        text: character_text,
        ..
    } = &db.character;
    let Company { country, .. } = &db.company;
    let RoleType {
        text: roletype_text,
        ..
    } = &db.role_type;
    let movie = db.movie.all();
    movie
        .with(
            company
                .select(country)
                .eq("[ru]")
                .and(production_year.gt(2010)),
        )
        .select(
            cast.with(
                cast_note
                    .rx(r"\(producer\)")
                    .and(role.select(roletype_text).eq("actor")),
            )
            .select(character)
            .select(character_text)
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
    let Character {
        text: character_text,
        ..
    } = &db.character;
    let Company { country, .. } = &db.company;
    let movie = db.movie.all();
    movie
        .with(
            company
                .select(country)
                .eq("[us]")
                .and(production_year.gt(1990)),
        )
        .select(
            cast.with(cast_note.rx(r"\(producer\)"))
                .select(character)
                .select(character_text)
                .and(title),
        )
}
