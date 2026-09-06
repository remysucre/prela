#![cfg_attr(rustfmt, rustfmt_skip)]
// All 113 JOB queries, plus the method-chain demo, built by `entries`. The
// columns are destructured once at the top of `entries` under one unique
// name each (`title`, `company_name`, `info_ty`, ...), and every query is a
// closure over them, in name order, written out in full so that each reads
// on its own; a sub-query that a query needs in both its with and its select
// is a local closure. `db` is `&'static` (main leaks it once), which is why
// the closures can be boxed as `'static` runners and the plans they return
// carry no lifetime.

use crate::engine::*;
use crate::job_queries::helpers::{Row, film_or_warner_co, follow_link, min_row};
use crate::job_queries::Entry;
use crate::job_queries::sets::{
    genre6, horror2, kw7, kw8, kw10, link3, murder4, nordic8, nordic9, nordic10, voice3, voice4, writer5,
};
use crate::job_schema::*;

fn entry(name: &'static str, oracle: &'static str, q: impl Fn() -> String + 'static) -> Entry {
    (name, oracle, Box::new(move |_| q()))
}

pub fn entries(db: &'static Job) -> Vec<Entry> {
    let Movie { title, kind, production_year, episode_nr, keyword, aka,
                company, cast, info, data, complete_cast, link, linked_by, .. } = &db.movie;
    let Cast { person, role, note: cast_note, character, .. } = &db.cast;
    let Person { name: person_name, gender, alias, bio, name_pcode_cf, .. } = &db.person;
    let Company { name: company_name, country, note: company_note, ty: company_ty, .. } = &db.company;
    let Info { info: info_info, ty: info_ty, note: info_note, .. } = &db.info;
    let Data { text: data_text, ty: data_ty, .. } = &db.data;
    let PersonInfo { info: personinfo_info, ty: personinfo_ty, note: personinfo_note, .. } = &db.person_info;
    let MovieLink { target, ty: link_ty, .. } = &db.movie_link;
    let CompleteCast { status, subject, .. } = &db.complete_cast;

    let q1a = move || {
        db.movie.with(data.select(data_ty).eq("top 250 rank"))
                .select(company.with(company_ty.eq("production companies"))
                               .select(company_note.nrx(r"\(as Metro-Goldwyn-Mayer Pictures\)")
                                                   .rx(r"\(co-production\)|\(presents\)"))
                        .and(title)
                        .and(production_year))
    };

    let q1b = move || {
        db.movie.with(data.select(data_ty).eq("bottom 10 rank")
                      .and(production_year.between(2005, 2010)))
                .select(company.with(company_ty.eq("production companies"))
                               .select(company_note.nrx(r"\(as Metro-Goldwyn-Mayer Pictures\)"))
                        .and(title)
                        .and(production_year))
    };

    let q1c = move || {
        db.movie.with(data.select(data_ty).eq("top 250 rank")
                      .and(production_year.gt(2010)))
                .select(company.with(company_ty.eq("production companies"))
                               .select(company_note.nrx(r"\(as Metro-Goldwyn-Mayer Pictures\)")
                                                   .rx(r"\(co-production\)"))
                        .and(title)
                        .and(production_year))
    };

    let q1d = move || {
        db.movie.with(data.select(data_ty).eq("bottom 10 rank")
                      .and(production_year.gt(2000)))
                .select(company.with(company_ty.eq("production companies"))
                               .select(company_note.nrx(r"\(as Metro-Goldwyn-Mayer Pictures\)"))
                        .and(title)
                        .and(production_year))
    };

    let q2a = move || {
        db.movie.with(keyword.eq("character-name-in-title")
                      .and(company.select(country).eq("[de]")))
                .select(title)
    };

    let q2b = move || {
        db.movie.with(keyword.eq("character-name-in-title")
                      .and(company.select(country).eq("[nl]")))
                .select(title)
    };

    let q2c = move || {
        db.movie.with(keyword.eq("character-name-in-title")
                      .and(company.select(country).eq("[sm]")))
                .select(title)
    };

    let q2d = move || {
        db.movie.with(keyword.eq("character-name-in-title")
                      .and(company.select(country).eq("[us]")))
                .select(title)
    };

    let q3a = move || {
        db.movie.with(keyword.rx(r"sequel")
                      .and(info.select(info_info).is_in(nordic8()))
                      .and(production_year.gt(2005)))
                .select(title)
    };

    let q3b = move || {
        db.movie.with(keyword.rx(r"sequel")
                      .and(info.select(info_info).eq("Bulgaria"))
                      .and(production_year.gt(2010)))
                .select(title)
    };

    let q3c = move || {
        db.movie.with(keyword.rx(r"sequel")
                      .and(info.select(info_info).is_in(["Sweden", "Norway", "Germany", "Denmark", "Swedish",
                                                   "Denish", "Norwegian", "German", "USA", "American"]))
                      .and(production_year.gt(1990)))
                .select(title)
    };

    let q4a = move || {
        db.movie.with(keyword.rx_dict(r"sequel")
                      .and(production_year.gt(2005)))
                .select(data.with(data_ty.eq("rating")).select(data_text.gt("5.0"))
                        .and(title))
    };

    let q4b = move || {
        db.movie.with(keyword.rx_dict(r"sequel")
                      .and(production_year.gt(2010)))
                .select(data.with(data_ty.eq("rating")).select(data_text.gt("9.0"))
                        .and(title))
    };

    let q4c = move || {
        db.movie.with(keyword.rx_dict(r"sequel")
                      .and(production_year.gt(1990)))
                .select(data.with(data_ty.eq("rating")).select(data_text.gt("2.0"))
                        .and(title))
    };

    let q5a = move || {
        db.movie.with(company.with(company_ty.eq("production companies")
                                   .and(company_note.rx(r"\(theatrical\)").rx(r"\(France\)")))
                      .and(info.select(info_info).is_in(nordic8()))
                      .and(production_year.gt(2005)))
                .select(title)
    };

    let q5b = move || {
        db.movie.with(company.with(company_ty.eq("production companies")
                                   .and(company_note.rx(r"\(VHS\)").rx(r"\(USA\)").rx(r"\(1994\)")))
                      .and(info.select(info_info).is_in(["USA", "America"]))
                      .and(production_year.gt(2010)))
                .select(title)
    };

    let q5c = move || {
        db.movie.with(company.with(company_ty.eq("production companies")
                                   .and(company_note.nrx(r"\(TV\)").rx(r"\(USA\)")))
                      .and(info.select(info_info).is_in(nordic10()))
                      .and(production_year.gt(1990)))
                .select(title)
    };

    let q6a = move || {
        let kw = || keyword.eq("marvel-cinematic-universe");
        db.movie.with(production_year.gt(2010)
                      .and(kw()))
                .select(kw()
                        .and(title)
                        .and(cast.select(person).select(person_name).rx(r"Downey.*Robert")))
    };

    let q6b = move || {
        let kw = || keyword.is_in(kw8());
        db.movie.with(production_year.gt(2014)
                      .and(kw()))
                .select(kw()
                        .and(title)
                        .and(cast.select(person).select(person_name).rx(r"Downey.*Robert")))
    };

    let q6c = move || {
        let kw = || keyword.eq("marvel-cinematic-universe");
        db.movie.with(production_year.gt(2014)
                      .and(kw()))
                .select(kw()
                        .and(title)
                        .and(cast.select(person).select(person_name).rx(r"Downey.*Robert")))
    };

    let q6d = move || {
        let kw = || keyword.is_in(kw8());
        db.movie.with(production_year.gt(2000)
                      .and(kw()))
                .select(kw()
                        .and(title)
                        .and(cast.select(person).select(person_name).rx(r"Downey.*Robert")))
    };

    let q6e = move || {
        let kw = || keyword.eq("marvel-cinematic-universe");
        db.movie.with(production_year.gt(2000)
                      .and(kw()))
                .select(kw()
                        .and(title)
                        .and(cast.select(person).select(person_name).rx(r"Downey.*Robert")))
    };

    let q6f = move || {
        let kw = || keyword.is_in(kw8());
        let cast_name = cast.select(person).select(person_name);
        db.movie.with(production_year.gt(2000)
                      .and(kw()))
                .select(kw()
                        .and(title)
                        .and(cast_name))
    };

    let q7a = move || {
        db.movie.with(production_year.between(1980, 1995)
                      .and(linked_by.select(link_ty).eq("features")))
                .select(cast.select(person.with(alias.rx(r"a")
                                                .and(name_pcode_cf.between("A", "F"))
                                                // m ∨ (f ∧ person_name~^B), spelled {m,f} ∖ (f ∖ ^B):
                                                // ∨ is member-only and can't sit inside a probed ∧-tree.
                                                .and(gender.is_in(["m", "f"])
                                                           .minus(gender.eq("f").minus(person_name.rx(r"^B"))))
                                                .and(bio.with(personinfo_ty.eq("mini biography")
                                                              .and(personinfo_note.eq("Volker Boehm")))))
                                          .select(person_name))
                        .and(title))
    };

    let q7b = move || {
        db.movie.with(production_year.between(1980, 1984)
                      .and(linked_by.select(link_ty).eq("features")))
                .select(cast.select(person.with(alias.rx(r"a")
                                                .and(name_pcode_cf.rx(r"^D"))
                                                .and(gender.eq("m"))
                                                .and(bio.with(personinfo_ty.eq("mini biography")
                                                              .and(personinfo_note.eq("Volker Boehm")))))
                                          .select(person_name))
                        .and(title))
    };

    let q7c = move || {
        // Conjunct tree (∧ = Prod) — consumed via `member` only.
        let bio_filter = || personinfo_ty.eq("mini biography").and(personinfo_note);
        db.movie.with(production_year.between(1980, 2010)
                      .and(linked_by.select(link_ty).is_in(["references", "referenced in", "features", "featured in"])))
                .select(cast.select(person.with(alias.rx(r"a|^A")
                                                .and(name_pcode_cf.between("A", "F"))
                                                // m ∨ (f ∧ name~^A), spelled {m,f} ∖ (f ∖ ^A):
                                                // ∨ is member-only and can't sit inside a probed ∧-tree.
                                                .and(gender.is_in(["m", "f"])
                                                           .minus(gender.eq("f").minus(person_name.rx(r"^A"))))
                                                .and(bio.with(bio_filter())))
                                          .select(person_name
                                                  .and(bio.with(bio_filter()).select(personinfo_info)))))
    };

    let q8a = move || {
        db.movie.with(company.with(country.eq("[jp]")
                                   .and(company_note.rx(r"\(Japan\)").nrx(r"\(USA\)"))))
                .select(cast.with(cast_note.eq("(voice: English version)")
                                  .and(role.eq("actress"))
                                  .and(person.with(person_name.rx(r"Yo").nrx(r"Yu"))))
                            .select(person).select(alias)
                        .and(title))
    };

    let q8b = move || {
        db.movie.with(company.with(country.eq("[jp]")
                                   .and(company_note.rx(r"\(Japan\)").nrx(r"\(USA\)").rx(r"\(2006\)|\(2007\)")))
                      .and(production_year.between(2006, 2007))
                      .and(title.rx(r"^One Piece|^Dragon Ball Z")))
                .select(cast.with(cast_note.eq("(voice: English version)")
                                  .and(role.eq("actress"))
                                  .and(person.with(person_name.rx(r"Yo").nrx(r"Yu"))))
                            .select(person).select(alias)
                        .and(title))
    };

    let q8c = move || {
        db.movie.with(company.select(country).eq("[us]"))
                .select(cast.with(role.eq("writer")).select(person).select(alias)
                        .and(title))
    };

    let q8d = move || {
        db.movie.with(company.select(country).eq("[us]"))
                .select(cast.with(role.eq("costume designer")).select(person).select(alias)
                        .and(title))
    };

    let q9a = move || {
        db.movie.with(company.with(country.eq("[us]")
                                   .and(company_note.rx(r"\(USA\)|\(worldwide\)")))
                      .and(production_year.between(2005, 2015)))
                .select(cast.with(cast_note.is_in(voice4())
                                  .and(role.eq("actress"))
                                  .and(person.with(gender.eq("f").and(person_name.rx(r"Ang")))))
                            .select(person.select(alias).and(character))
                        .and(title))
    };

    let q9b = move || {
        db.movie.with(company.with(country.eq("[us]")
                                   .and(company_note.rx(r"\(200.*\)").rx(r"\(USA\)|\(worldwide\)")))
                      .and(production_year.between(2007, 2010)))
                .select(cast.with(cast_note.eq("(voice)")
                                  .and(role.eq("actress"))
                                  .and(person.with(gender.eq("f").and(person_name.rx(r"Angel")))))
                            .select(person.select(alias)
                                    .and(character)
                                    .and(person.select(person_name)))
                        .and(title))
    };

    let q9c = move || {
        db.movie.with(company.select(country).eq("[us]"))
                .select(cast.with(cast_note.is_in(voice4())
                                  .and(role.eq("actress"))
                                  .and(person.with(gender.eq("f").and(person_name.rx(r"An")))))
                            .select(person.select(alias)
                                    .and(character)
                                    .and(person.select(person_name)))
                        .and(title))
    };

    let q9d = move || {
        db.movie.with(company.select(country).eq("[us]"))
                .select(cast.with(cast_note.is_in(voice4())
                                  .and(role.eq("actress"))
                                  .and(person.with(gender.eq("f"))))
                            .select(person.select(alias)
                                    .and(person.select(person_name))
                                    .and(character))
                        .and(title))
    };

    let q10a = move || {
        db.movie.with(company.select(country).eq("[ru]")
                      .and(production_year.gt(2005)))
                .select(cast.with(cast_note.rx(r"\(voice\)").rx(r"\(uncredited\)")
                                  .and(role.eq("actor")))
                            .select(character)
                        .and(title))
    };

    let q10b = move || {
        db.movie.with(company.select(country).eq("[ru]")
                      .and(production_year.gt(2010)))
                .select(cast.with(cast_note.rx(r"\(producer\)")
                                  .and(role.eq("actor")))
                            .select(character)
                        .and(title))
    };

    let q10c = move || {
        db.movie.with(company.select(country).eq("[us]")
                      .and(production_year.gt(1990)))
                .select(cast.with(cast_note.rx(r"\(producer\)")).select(character)
                        .and(title))
    };

    let q11a = move || {
        db.movie.with(keyword.eq("sequel")
                      .and(production_year.between(1950, 2000)))
                .select(company.with(country.ne("[pl]")
                                     .and(company_name.rx(r"Film|Warner"))
                                     .and(company_ty.eq("production companies"))
                                     .minus(company_note))
                               .select(company_name)
                        .and(link.select(link_ty).rx(r"follow"))
                        .and(title))
    };

    let q11b = move || {
        db.movie.with(keyword.eq("sequel")
                      .and(production_year.eq(1998))
                      .and(title.rx(r"Money")))
                .select(company.with(country.ne("[pl]")
                                     .and(company_name.rx(r"Film|Warner"))
                                     .and(company_ty.eq("production companies"))
                                     .minus(company_note))
                               .select(company_name)
                        .and(link.select(link_ty).rx(r"follows"))
                        .and(title))
    };

    let q11c = move || {
        db.movie.with(keyword.is_in(["sequel", "revenge", "based-on-novel"])
                      .and(production_year.gt(1950))
                      .and(link))
                .select(company.with(country.ne("[pl]")
                                     .and(company_name.rx(r"^20th Century Fox|^Twentieth Century Fox"))
                                     .and(company_ty.ne("production companies"))
                                     .and(company_note))
                               .select(company_name.and(company_note))
                        .and(title))
    };

    let q11d = move || {
        db.movie.with(keyword.is_in(["sequel", "revenge", "based-on-novel"])
                      .and(production_year.gt(1950))
                      .and(link))
                .select(company.with(country.ne("[pl]")
                                     .and(company_ty.ne("production companies"))
                                     .and(company_note))
                               .select(company_name.and(company_note))
                        .and(title))
    };

    let q12a = move || {
        db.movie.with(info.with(info_ty.eq("genres")
                                .and(info_info.is_in(["Drama", "Horror"])))
                      .and(production_year.between(2005, 2008)))
                .select(company.with(country.eq("[us]")
                                     .and(company_ty.eq("production companies")))
                               .select(company_name)
                        .and(data.with(data_ty.eq("rating")).select(data_text.gt("8.0")))
                        .and(title))
    };

    let q12b = move || {
        db.movie.with(company.with(country.eq("[us]")
                                   .and(company_ty.is_in(["production companies", "distributors"])))
                      .and(data.select(data_ty).eq("bottom 10 rank"))
                      .and(production_year.gt(2000))
                      .and(title.rx(r"^Birdemic|Movie")))
                .select(info.with(info_ty.eq("budget")).select(info_info)
                        .and(title))
    };

    let q12c = move || {
        db.movie.with(info.with(info_ty.eq("genres")
                                .and(info_info.is_in(["Drama", "Horror", "Western", "Family"])))
                      .and(production_year.between(2000, 2010)))
                .select(company.with(country.eq("[us]")
                                     .and(company_ty.eq("production companies")))
                               .select(company_name)
                        .and(data.with(data_ty.eq("rating")).select(data_text.gt("7.0")))
                        .and(title))
    };

    let q13a = move || {
        db.movie.with(company.with(country.eq("[de]")
                                   .and(company_ty.eq("production companies")))
                      .and(kind.eq("movie")))
                .select(info.with(info_ty.eq("release dates")).select(info_info)
                        .and(data.with(data_ty.eq("rating")).select(data_text))
                        .and(title))
    };

    let q13b = move || {
        db.movie.with(kind.eq("movie")
                      .and(info.select(info_ty).eq("release dates"))
                      .and(title.ne("").rx(r"Champion|Loser")))
                .select(company.with(country.eq("[us]")
                                     .and(company_ty.eq("production companies")))
                               .select(company_name)
                        .and(data.with(data_ty.eq("rating")).select(data_text))
                        .and(title))
    };

    let q13c = move || {
        db.movie.with(kind.eq("movie")
                      .and(info.select(info_ty).eq("release dates"))
                      .and(title.ne("").rx(r"^Champion|^Loser")))
                .select(company.with(country.eq("[us]")
                                     .and(company_ty.eq("production companies")))
                               .select(company_name)
                        .and(data.with(data_ty.eq("rating")).select(data_text))
                        .and(title))
    };

    let q13d = move || {
        db.movie.with(kind.eq("movie")
                      .and(info.select(info_ty).eq("release dates")))
                .select(company.with(country.eq("[us]")
                                     .and(company_ty.eq("production companies")))
                               .select(company_name)
                        .and(data.with(data_ty.eq("rating")).select(data_text))
                        .and(title))
    };

    let q14a = move || {
        db.movie.with(keyword.is_in(murder4())
                      .and(kind.eq("movie"))
                      .and(info.with(info_ty.eq("countries")
                                     .and(info_info.is_in(["Sweden", "Norway", "Germany", "Denmark", "Swedish",
                                                           "Denish", "Norwegian", "German", "USA", "American"]))))
                      .and(production_year.gt(2010)))
                .select(data.with(data_ty.eq("rating")).select(data_text.lt("8.5"))
                        .and(title))
    };

    let q14b = move || {
        db.movie.with(keyword.is_in(["murder", "murder-in-title"])
                      .and(kind.eq("movie"))
                      .and(info.with(info_ty.eq("countries")
                                     .and(info_info.is_in(["Sweden", "Norway", "Germany", "Denmark", "Swedish",
                                                           "Denish", "Norwegian", "German", "USA", "American"]))))
                      .and(production_year.gt(2010))
                      .and(title.rx(r"murder|Murder|Mord")))
                .select(data.with(data_ty.eq("rating")).select(data_text.gt("6.0"))
                        .and(title))
    };

    let q14c = move || {
        db.movie.with(keyword.is_in(murder4())
                      .and(kind.is_in(["movie", "episode"]))
                      .and(info.with(info_ty.eq("countries").and(info_info.is_in(nordic10()))))
                      .and(production_year.gt(2005)))
                .select(data.with(data_ty.eq("rating")).select(data_text.lt("8.5"))
                        .and(title))
    };

    let q15a = move || {
        db.movie.with(production_year.gt(2000)
                      .and(company.with(country.eq("[us]")
                                        .and(company_note.rx(r"\(200.*\)").rx(r"\(worldwide\)"))))
                      .and(keyword)
                      .and(aka))
                .select(info.with(info_ty.eq("release dates")
                                  .and(info_note.rx(r"internet")))
                            .select(info_info.rx(r"^USA:.* 200"))
                        .and(title))
    };

    let q15b = move || {
        db.movie.with(company.with(country.eq("[us]")
                                   .and(company_name.eq("YouTube"))
                                   .and(company_note.rx(r"\(200.*\)").rx(r"\(worldwide\)")))
                      .and(keyword)
                      .and(aka)
                      .and(production_year.between(2005, 2010)))
                .select(info.with(info_ty.eq("release dates")
                                  .and(info_note.rx(r"internet")))
                            .select(info_info.rx(r"^USA:.* 200"))
                        .and(title))
    };

    let q15c = move || {
        db.movie.with(company.select(country).eq("[us]")
                      .and(keyword)
                      .and(aka)
                      .and(production_year.gt(1990)))
                .select(info.with(info_ty.eq("release dates")
                                  .and(info_note.rx(r"internet")))
                            .select(info_info.rx(r"^USA:.* 199|^USA:.* 200"))
                        .and(title))
    };

    let q15d = move || {
        db.movie.with(company.select(country).eq("[us]")
                      .and(keyword)
                      .and(info.with(info_ty.eq("release dates").and(info_note.rx(r"internet"))))
                      .and(production_year.gt(1990)))
                .select(aka.and(title))
    };

    let q16a = move || {
        db.movie.with(company.select(country).eq("[us]")
                      .and(keyword.eq("character-name-in-title"))
                      .and(episode_nr.ge(50).lt(100)))
                .select(cast.select(person).select(alias)
                        .and(title))
    };

    let q16b = move || {
        db.movie.with(company.select(country).eq("[us]")
                      .and(keyword.eq("character-name-in-title")))
                .select(cast.select(person).select(alias)
                        .and(title))
    };

    let q16c = move || {
        db.movie.with(company.select(country).eq("[us]")
                      .and(keyword.eq("character-name-in-title"))
                      .and(episode_nr.lt(100)))
                .select(cast.select(person).select(alias)
                        .and(title))
    };

    let q16d = move || {
        db.movie.with(company.select(country).eq("[us]")
                      .and(keyword.eq("character-name-in-title"))
                      .and(episode_nr.ge(5).lt(100)))
                .select(cast.select(person).select(alias)
                        .and(title))
    };

    let q17a = move || {
        db.movie.with(company.select(country).eq("[us]")
                      .and(keyword.eq("character-name-in-title")))
                .select(cast.select(person).select(person_name).rx(r"^B"))
    };

    let q17b = move || {
        db.movie.with(company
                      .and(keyword.eq("character-name-in-title")))
                .select(cast.select(person).select(person_name).rx(r"^Z"))
    };

    let q17c = move || {
        db.movie.with(company
                      .and(keyword.eq("character-name-in-title")))
                .select(cast.select(person).select(person_name).rx(r"^X"))
    };

    let q17d = move || {
        db.movie.with(company
                      .and(keyword.eq("character-name-in-title")))
                .select(cast.select(person).select(person_name).rx(r"Bert"))
    };

    let q17e = move || {
        db.movie.with(company.select(country).eq("[us]")
                      .and(keyword.eq("character-name-in-title")))
                .select(cast.select(person).select(person_name))
    };

    let q17f = move || {
        db.movie.with(company
                      .and(keyword.eq("character-name-in-title")))
                .select(cast.select(person).select(person_name).rx(r"B"))
    };

    let q18a = move || {
        let budget = || info.with(info_ty.eq("budget")).select(info_info);
        db.movie.with(budget()
                      .and(cast.with(cast_note.is_in(["(producer)", "(executive producer)"])
                                     .and(person.with(gender.eq("m").and(person_name.rx(r"Tim")))))))
                .select(budget()
                        .and(data.with(data_ty.eq("votes")).select(data_text))
                        .and(title))
    };

    let q18b = move || {
        // Conjunct/diff tree (∧ = Prod, - = Diff) — consumed via `member` only.
        let gf = || info_ty.eq("genres").and(info_info.is_in(horror2())).minus(info_note);
        db.movie.with(info.with(gf())
                      .and(production_year.between(2008, 2014))
                      .and(cast.with(cast_note.is_in(writer5())
                                     .and(person.select(gender).eq("f")))))
                .select(info.with(gf()).select(info_info)
                        .and(data.with(data_ty.eq("rating")).select(data_text.gt("8.0")))
                        .and(title))
    };

    let q18c = move || {
        let gf = || info_ty.eq("genres").and(info_info.is_in(genre6()));
        db.movie.with(info.with(gf())
                      .and(cast.with(cast_note.is_in(writer5())
                                     .and(person.select(gender).eq("m")))))
                .select(info.with(gf()).select(info_info)
                        .and(data.with(data_ty.eq("votes")).select(data_text))
                        .and(title))
    };

    let q19a = move || {
        db.movie.with(company.with(country.eq("[us]")
                                   .and(company_note.rx(r"\(USA\)|\(worldwide\)")))
                      .and(info.with(info_ty.eq("release dates")
                                     .and(info_info.rx(r"^Japan:.*200|^USA:.*200"))))
                      .and(production_year.between(2005, 2009)))
                .select(cast.with(cast_note.is_in(voice4())
                                  .and(role.eq("actress"))
                                  .and(character)
                                  .and(person.with(gender.eq("f")
                                                   .and(person_name.rx(r"Ang"))
                                                   .and(alias))))
                            .select(person).select(person_name)
                        .and(title))
    };

    let q19b = move || {
        db.movie.with(company.with(country.eq("[us]")
                                   .and(company_note.rx(r"\(200.*\)").rx(r"\(USA\)|\(worldwide\)")))
                      .and(info.with(info_ty.eq("release dates")
                                     .and(info_info.rx(r"^Japan:.*2007|^USA:.*2008"))))
                      .and(production_year.between(2007, 2008))
                      .and(title.rx(r"Kung.*Fu.*Panda")))
                .select(cast.with(cast_note.eq("(voice)")
                                  .and(role.eq("actress"))
                                  .and(character)
                                  .and(person.with(gender.eq("f")
                                                   .and(person_name.rx(r"Angel"))
                                                   .and(alias))))
                            .select(person).select(person_name)
                        .and(title))
    };

    let q19c = move || {
        db.movie.with(company.select(country).eq("[us]")
                      .and(info.with(info_ty.eq("release dates")
                                     .and(info_info.rx(r"^Japan:.*200|^USA:.*200"))))
                      .and(production_year.gt(2000)))
                .select(cast.with(cast_note.is_in(voice4())
                                  .and(role.eq("actress"))
                                  .and(character)
                                  .and(person.with(gender.eq("f")
                                                   .and(person_name.rx(r"An"))
                                                   .and(alias))))
                            .select(person).select(person_name)
                        .and(title))
    };

    let q19d = move || {
        db.movie.with(company.select(country).eq("[us]")
                      .and(info.select(info_ty).eq("release dates"))
                      .and(production_year.gt(2000)))
                .select(cast.with(cast_note.is_in(voice4())
                                  .and(role.eq("actress"))
                                  .and(character)
                                  .and(person.with(gender.eq("f").and(alias))))
                            .select(person).select(person_name)
                        .and(title))
    };

    let q20a = move || {
        db.movie.with(complete_cast.with(subject.eq("cast").and(status.rx(r"complete")))
                      .and(keyword.is_in(kw8()))
                      .and(kind.eq("movie"))
                      .and(production_year.gt(1950))
                      .and(cast.select(character).nrx(r"Sherlock").rx(r"Tony.*Stark|Iron.*Man")))
                .select(title)
    };

    let q20b = move || {
        db.movie.with(complete_cast.with(subject.eq("cast").and(status.rx(r"complete")))
                      .and(keyword.is_in(kw8()))
                      .and(kind.eq("movie"))
                      .and(production_year.gt(2000))
                      .and(cast.with(character.nrx(r"Sherlock").rx(r"Tony.*Stark|Iron.*Man")
                                     .and(person.select(person_name).rx(r"Downey.*Robert")))))
                .select(title)
    };

    let q20c = move || {
        db.movie.with(complete_cast.with(subject.eq("cast").and(status.rx(r"complete")))
                      .and(keyword.is_in(kw10()))
                      .and(kind.eq("movie"))
                      .and(production_year.gt(2000)))
                .select(cast.with(character.rx(r"[Mm]an")).select(person).select(person_name)
                        .and(title))
    };

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
    let q21a = move || {
        db.movie.with(link.and(keyword.eq("sequel"))
                      .and(production_year.between(1950, 2000))
                      .and(info.select(info_info).is_in(nordic8()))
                      .and(follow_link(db)))
                .select(film_or_warner_co(db).select(company_name)
                        .and(follow_link(db))
                        .and(title))
    };

    let q21b = move || {
        db.movie.with(link.and(keyword.eq("sequel"))
                      .and(production_year.between(2000, 2010))
                      .and(info.select(info_info).is_in(["Germany", "German"]))
                      .and(follow_link(db)))
                .select(film_or_warner_co(db).select(company_name)
                        .and(follow_link(db))
                        .and(title))
    };

    let q21c = move || {
        db.movie.with(link.and(keyword.eq("sequel"))
                      .and(production_year.between(1950, 2010))
                      .and(info.select(info_info).is_in(nordic9()))
                      .and(follow_link(db)))
                .select(film_or_warner_co(db).select(company_name)
                        .and(follow_link(db))
                        .and(title))
    };

    let q22a = move || {
        db.movie.with(info.with(info_ty.eq("countries")
                                .and(info_info.is_in(["Germany", "German", "USA", "American"])))
                      .and(keyword.is_in(murder4()))
                      .and(production_year.gt(2008))
                      .and(kind.is_in(["movie", "episode"])))
                .select(title
                        .and(data.with(data_ty.eq("rating")).select(data_text.lt("7.0")))
                        .and(company.with(company_note.nrx(r"\(USA\)").rx(r"\(200.*\)")
                                          .and(country.ne("[us]"))
                                          .and(company_ty.eq("production companies")))
                                    .select(company_name)))
    };

    let q22b = move || {
        db.movie.with(info.with(info_ty.eq("countries")
                                .and(info_info.is_in(["Germany", "German", "USA", "American"])))
                      .and(keyword.is_in(murder4()))
                      .and(production_year.gt(2009))
                      .and(kind.is_in(["movie", "episode"])))
                .select(title
                        .and(data.with(data_ty.eq("rating")).select(data_text.lt("7.0")))
                        .and(company.with(company_note.nrx(r"\(USA\)").rx(r"\(200.*\)")
                                          .and(country.ne("[us]"))
                                          .and(company_ty.eq("production companies")))
                                    .select(company_name)))
    };

    let q22c = move || {
        db.movie.with(info.with(info_ty.eq("countries")
                                .and(info_info.is_in(nordic10())))
                      .and(keyword.is_in(murder4()))
                      .and(production_year.gt(2005))
                      .and(kind.is_in(["movie", "episode"])))
                .select(title
                        .and(data.with(data_ty.eq("rating")).select(data_text.lt("8.5")))
                        .and(company.with(company_note.nrx(r"\(USA\)").rx(r"\(200.*\)")
                                          .and(country.ne("[us]"))
                                          .and(company_ty.eq("production companies")))
                                    .select(company_name)))
    };

    let q22d = move || {
        db.movie.with(info.with(info_ty.eq("countries").and(info_info.is_in(nordic10())))
                      .and(keyword.is_in(murder4()))
                      .and(production_year.gt(2005))
                      .and(kind.is_in(["movie", "episode"])))
                .select(title
                        .and(data.with(data_ty.eq("rating")).select(data_text.lt("8.5")))
                        .and(company.with(country.ne("[us]")
                                          .and(company_ty.eq("production companies")))
                                    .select(company_name)))
    };

    let q23a = move || {
        let k = || kind.eq("movie");
        db.movie.with(complete_cast.select(status).eq("complete+verified")
                      .and(company.select(country).eq("[us]"))
                      .and(info.with(info_ty.eq("release dates")
                                     .and(info_note.rx(r"internet"))
                                     .and(info_info.rx(r"^USA:.* 199|^USA:.* 200"))))
                      .and(k())
                      .and(keyword)
                      .and(production_year.gt(2000)))
                .select(k().and(title))
    };

    let q23b = move || {
        let k = || kind.eq("movie");
        db.movie.with(complete_cast.select(status).eq("complete+verified")
                      .and(company.select(country).eq("[us]"))
                      .and(info.with(info_ty.eq("release dates")
                                     .and(info_note.rx(r"internet"))
                                     .and(info_info.rx(r"^USA:.* 200"))))
                      .and(k())
                      .and(keyword.is_in(["nerd", "loner", "alienation", "dignity"]))
                      .and(production_year.gt(2000)))
                .select(k().and(title))
    };

    let q23c = move || {
        let k = || kind.is_in(["movie", "tv movie", "video movie", "video game"]);
        db.movie.with(complete_cast.select(status).eq("complete+verified")
                      .and(company.select(country).eq("[us]"))
                      .and(info.with(info_ty.eq("release dates")
                                     .and(info_note.rx(r"internet"))
                                     .and(info_info.rx(r"^USA:.* 199|^USA:.* 200"))))
                      .and(k())
                      .and(keyword)
                      .and(production_year.gt(1990)))
                .select(k().and(title))
    };

    let q24a = move || {
        db.movie.with(company.select(country).eq("[us]")
                      .and(info.with(info_ty.eq("release dates")
                                     .and(info_info.rx(r"^Japan:.*201|^USA:.*201"))))
                      .and(keyword.is_in(["hero", "martial-arts", "hand-to-hand-combat"]))
                      .and(production_year.gt(2010)))
                .select(cast.with(cast_note.is_in(voice4())
                                  .and(role.eq("actress"))
                                  .and(person.with(gender.eq("f")
                                                   .and(person_name.rx(r"An"))
                                                   .and(alias))))
                            .select(character.and(person.select(person_name)))
                        .and(title))
    };

    let q24b = move || {
        db.movie.with(company.with(country.eq("[us]")
                                   .and(company_name.eq("DreamWorks Animation")))
                      .and(info.with(info_ty.eq("release dates")
                                     .and(info_info.rx(r"^Japan:.*201|^USA:.*201"))))
                      .and(keyword.is_in(["hero", "martial-arts", "hand-to-hand-combat", "computer-animated-movie"]))
                      .and(production_year.gt(2010))
                      .and(title.rx(r"^Kung Fu Panda")))
                .select(cast.with(cast_note.is_in(voice4())
                                  .and(role.eq("actress"))
                                  .and(person.with(gender.eq("f")
                                                   .and(person_name.rx(r"An"))
                                                   .and(alias))))
                            .select(character.and(person.select(person_name)))
                        .and(title))
    };

    let q25a = move || {
        let gf = || info_ty.eq("genres").and(info_info.eq("Horror"));
        db.movie.with(info.with(gf())
                      .and(keyword.is_in(["murder", "blood", "gore", "death", "female-nudity"])))
                .select(info.with(gf()).select(info_info)
                        .and(data.with(data_ty.eq("votes")).select(data_text))
                        .and(title)
                        .and(cast.with(cast_note.is_in(writer5())
                                       .and(person.with(gender.eq("m"))))
                                 .select(person).select(person_name)))
    };

    let q25b = move || {
        let gf = || info_ty.eq("genres").and(info_info.eq("Horror"));
        db.movie.with(info.with(gf())
                      .and(keyword.is_in(["murder", "blood", "gore", "death", "female-nudity"]))
                      .and(production_year.gt(2010))
                      .and(title.rx(r"^Vampire")))
                .select(info.with(gf()).select(info_info)
                        .and(data.with(data_ty.eq("votes")).select(data_text))
                        .and(title)
                        .and(cast.with(cast_note.is_in(writer5())
                                       .and(person.with(gender.eq("m"))))
                                 .select(person).select(person_name)))
    };

    let q25c = move || {
        let gf = || info_ty.eq("genres").and(info_info.is_in(genre6()));
        db.movie.with(info.with(gf())
                      .and(keyword.is_in(kw7())))
                .select(info.with(gf()).select(info_info)
                        .and(data.with(data_ty.eq("votes")).select(data_text))
                        .and(title)
                        .and(cast.with(cast_note.is_in(writer5())
                                       .and(person.with(gender.eq("m"))))
                                 .select(person).select(person_name)))
    };

    let q26a = move || {
        db.movie.with(complete_cast.with(subject.eq("cast").and(status.rx(r"complete")))
                      .and(keyword.is_in(kw10()))
                      .and(kind.eq("movie"))
                      .and(production_year.gt(2000)))
                .select(cast.with(character.rx(r"[Mm]an")).select(character.and(person.select(person_name)))
                        .and(data.with(data_ty.eq("rating")).select(data_text.gt("7.0")))
                        .and(title))
    };

    let q26b = move || {
        db.movie.with(complete_cast.with(subject.eq("cast").and(status.rx(r"complete")))
                      .and(keyword.is_in(["superhero", "marvel-comics", "based-on-comic", "fight"]))
                      .and(kind.eq("movie"))
                      .and(production_year.gt(2005)))
                .select(cast.with(character.rx(r"[Mm]an")).select(character)
                        .and(data.with(data_ty.eq("rating")).select(data_text.gt("8.0")))
                        .and(title))
    };

    let q26c = move || {
        let rd = data.with(data_ty.eq("rating")).select(data_text);
        db.movie.with(complete_cast.with(subject.eq("cast").and(status.rx(r"complete")))
                      .and(keyword.is_in(kw10()))
                      .and(kind.eq("movie"))
                      .and(production_year.gt(2000)))
                .select(cast.with(character.rx(r"[Mm]an")).select(character)
                        .and(rd)
                        .and(title))
    };

    let q27a = move || {
        db.movie.with(link.and(keyword.eq("sequel"))
                      .and(production_year.between(1950, 2000))
                      .and(info.select(info_info).is_in(["Sweden", "Germany", "Swedish", "German"]))
                      .and(complete_cast.with(subject.is_in(["cast", "crew"]).and(status.eq("complete"))))
                      .and(follow_link(db)))
                .select(film_or_warner_co(db).select(company_name)
                        .and(follow_link(db))
                        .and(title))
    };

    let q27b = move || {
        db.movie.with(link.and(keyword.eq("sequel"))
                      .and(production_year.eq(1998))
                      .and(info.select(info_info).is_in(["Sweden", "Germany", "Swedish", "German"]))
                      .and(complete_cast.with(subject.is_in(["cast", "crew"]).and(status.eq("complete"))))
                      .and(follow_link(db)))
                .select(film_or_warner_co(db).select(company_name)
                        .and(follow_link(db))
                        .and(title))
    };

    let q27c = move || {
        db.movie.with(link.and(keyword.eq("sequel"))
                      .and(production_year.between(1950, 2010))
                      .and(info.select(info_info).is_in(nordic9()))
                      .and(complete_cast.with(subject.eq("cast").and(status.rx(r"^complete"))))
                      .and(follow_link(db)))
                .select(film_or_warner_co(db).select(company_name)
                        .and(follow_link(db))
                        .and(title))
    };

    let q28a = move || {
        let co = || company.with(country.ne("[us]").and(company_note.nrx(r"\(USA\)").rx(r"\(200.*\)")));
        let dt = || data.with(data_ty.eq("rating").and(data_text.lt("8.5")));
        db.movie.with(complete_cast.with(subject.eq("crew").and(status.ne("complete+verified")))
                      .and(co())
                      .and(info.with(info_ty.eq("countries").and(info_info.is_in(nordic10()))))
                      .and(dt())
                      .and(keyword.is_in(murder4()))
                      .and(kind.is_in(["movie", "episode"]))
                      .and(production_year.gt(2000)))
                .select(co().select(company_name)
                        .and(dt().select(data_text))
                        .and(title))
    };

    let q28b = move || {
        let co = || company.with(country.ne("[us]").and(company_note.nrx(r"\(USA\)").rx(r"\(200.*\)")));
        let dt = || data.with(data_ty.eq("rating").and(data_text.gt("6.5")));
        db.movie.with(complete_cast.with(subject.eq("crew").and(status.ne("complete+verified")))
                      .and(co())
                      .and(info.with(info_ty.eq("countries").and(info_info.is_in(["Sweden", "Germany", "Swedish", "German"]))))
                      .and(dt())
                      .and(keyword.is_in(murder4()))
                      .and(kind.is_in(["movie", "episode"]))
                      .and(production_year.gt(2005)))
                .select(co().select(company_name)
                        .and(dt().select(data_text))
                        .and(title))
    };

    let q28c = move || {
        let co = || company.with(country.ne("[us]").and(company_note.nrx(r"\(USA\)").rx(r"\(200.*\)")));
        let dt = || data.with(data_ty.eq("rating").and(data_text.lt("8.5")));
        db.movie.with(complete_cast.with(subject.eq("cast").and(status.eq("complete")))
                      .and(co())
                      .and(info.with(info_ty.eq("countries").and(info_info.is_in(nordic10()))))
                      .and(dt())
                      .and(keyword.is_in(murder4()))
                      .and(kind.is_in(["movie", "episode"]))
                      .and(production_year.gt(2005)))
                .select(co().select(company_name)
                        .and(dt().select(data_text))
                        .and(title))
    };

    let q29a = move || {
        db.movie.with(complete_cast.with(subject.eq("cast").and(status.eq("complete+verified")))
                      .and(company.select(country).eq("[us]"))
                      .and(info.with(info_ty.eq("release dates")
                                     .and(info_info.rx(r"^Japan:.*200|^USA:.*200"))))
                      .and(keyword.eq("computer-animation"))
                      .and(title.eq("Shrek 2"))
                      .and(production_year.between(2000, 2010)))
                .select(cast.with(cast_note.is_in(voice3())
                                  .and(role.eq("actress"))
                                  .and(character.eq("Queen"))
                                  .and(person.with(gender.eq("f")
                                                   .and(person_name.rx(r"An"))
                                                   .and(alias)
                                                   .and(bio.select(personinfo_ty).eq("trivia")))))
                            .select(character.and(person.select(person_name)))
                        .and(title))
    };

    let q29b = move || {
        db.movie.with(complete_cast.with(subject.eq("cast").and(status.eq("complete+verified")))
                      .and(company.select(country).eq("[us]"))
                      .and(info.with(info_ty.eq("release dates")
                                     .and(info_info.rx(r"^USA:.*200"))))
                      .and(keyword.eq("computer-animation"))
                      .and(title.eq("Shrek 2"))
                      .and(production_year.between(2000, 2005)))
                .select(cast.with(cast_note.is_in(voice3())
                                  .and(role.eq("actress"))
                                  .and(character.eq("Queen"))
                                  .and(person.with(gender.eq("f")
                                                   .and(person_name.rx(r"An"))
                                                   .and(alias)
                                                   .and(bio.select(personinfo_ty).eq("height")))))
                            .select(character.and(person.select(person_name)))
                        .and(title))
    };

    let q29c = move || {
        db.movie.with(complete_cast.with(subject.eq("cast").and(status.eq("complete+verified")))
                      .and(company.select(country).eq("[us]"))
                      .and(info.with(info_ty.eq("release dates")
                                     .and(info_info.rx(r"^Japan:.*200|^USA:.*200"))))
                      .and(keyword.eq("computer-animation"))
                      .and(production_year.between(2000, 2010)))
                .select(cast.with(cast_note.is_in(voice4())
                                  .and(role.eq("actress"))
                                  .and(person.with(gender.eq("f")
                                                   .and(person_name.rx(r"An"))
                                                   .and(alias)
                                                   .and(bio.select(personinfo_ty).eq("trivia")))))
                            .select(character.and(person.select(person_name)))
                        .and(title))
    };

    let q30a = move || {
        let gf = || info_ty.eq("genres").and(info_info.is_in(horror2()));
        db.movie.with(complete_cast.with(subject.is_in(["cast", "crew"]).and(status.eq("complete+verified")))
                      .and(info.with(gf()))
                      .and(keyword.is_in(kw7()))
                      .and(production_year.gt(2000)))
                .select(info.with(gf()).select(info_info)
                        .and(data.with(data_ty.eq("votes")).select(data_text))
                        .and(title)
                        .and(cast.with(cast_note.is_in(writer5())
                                       .and(person.with(gender.eq("m"))))
                                 .select(person).select(person_name)))
    };

    let q30b = move || {
        let gf = || info_ty.eq("genres").and(info_info.is_in(horror2()));
        db.movie.with(complete_cast.with(subject.is_in(["cast", "crew"]).and(status.eq("complete+verified")))
                      .and(info.with(gf()))
                      .and(keyword.is_in(kw7()))
                      .and(production_year.gt(2000))
                      .and(title.rx(r"Freddy|Jason|^Saw")))
                .select(info.with(gf()).select(info_info)
                        .and(data.with(data_ty.eq("votes")).select(data_text))
                        .and(title)
                        .and(cast.with(cast_note.is_in(writer5())
                                       .and(person.with(gender.eq("m"))))
                                 .select(person).select(person_name)))
    };

    let q30c = move || {
        let gf = || info_ty.eq("genres").and(info_info.is_in(genre6()));
        db.movie.with(complete_cast.with(subject.eq("cast").and(status.eq("complete+verified")))
                      .and(info.with(gf()))
                      .and(keyword.is_in(kw7())))
                .select(info.with(gf()).select(info_info)
                        .and(data.with(data_ty.eq("votes")).select(data_text))
                        .and(title)
                        .and(cast.with(cast_note.is_in(writer5())
                                       .and(person.with(gender.eq("m"))))
                                 .select(person).select(person_name)))
    };

    let q31a = move || {
        let gf = || info_ty.eq("genres").and(info_info.is_in(horror2()));
        db.movie.with(company.select(company_name).rx(r"^Lionsgate")
                      .and(info.with(gf()))
                      .and(keyword.is_in(kw7())))
                .select(info.with(gf()).select(info_info)
                        .and(data.with(data_ty.eq("votes")).select(data_text))
                        .and(title)
                        .and(cast.with(cast_note.is_in(writer5())
                                       .and(person.with(gender.eq("m"))))
                                 .select(person).select(person_name)))
    };

    let q31b = move || {
        let gf = || info_ty.eq("genres").and(info_info.is_in(horror2()));
        db.movie.with(company.with(company_name.rx(r"^Lionsgate")
                                   .and(company_note.rx(r"\(Blu-ray\)")))
                      .and(info.with(gf()))
                      .and(keyword.is_in(kw7()))
                      .and(production_year.gt(2000))
                      .and(title.rx(r"Freddy|Jason|^Saw")))
                .select(info.with(gf()).select(info_info)
                        .and(data.with(data_ty.eq("votes")).select(data_text))
                        .and(title)
                        .and(cast.with(cast_note.is_in(writer5())
                                       .and(person.with(gender.eq("m"))))
                                 .select(person).select(person_name)))
    };

    let q31c = move || {
        let gf = || info_ty.eq("genres").and(info_info.is_in(genre6()));
        db.movie.with(company.select(company_name).rx(r"^Lionsgate")
                      .and(info.with(gf()))
                      .and(keyword.is_in(kw7())))
                .select(info.with(gf()).select(info_info)
                        .and(data.with(data_ty.eq("votes")).select(data_text))
                        .and(title)
                        .and(cast.with(cast_note.is_in(writer5())).select(person).select(person_name)))
    };

    let q32a = move || {
        db.movie.with(link.and(keyword.eq("10,000-mile-club")))
                .select(link.select(link_ty)
                        .and(title)
                        .and(link.select(target).select(title)))
    };

    let q32b = move || {
        db.movie.with(link.and(keyword.eq("character-name-in-title")))
                .select(link.select(link_ty)
                        .and(title)
                        .and(link.select(target).select(title)))
    };

    let q33a = move || {
        let qlink = || link.with(link_ty.is_in(link3())
                                 .and(target.with(kind.eq("tv series")
                                                  .and(company)
                                                  .and(data.with(data_ty.eq("rating").and(data_text.lt("3.0"))))
                                                  .and(production_year.between(2005, 2008)))));
        db.movie.with(kind.eq("tv series")
                      .and(company.select(country).eq("[us]"))
                      .and(qlink()))
                .select(company.with(country.eq("[us]")).select(company_name)
                        .and(qlink().select(target).select(company).select(company_name))
                        .and(data.with(data_ty.eq("rating")).select(data_text))
                        .and(qlink().select(target)
                                    .select(data.with(data_ty.eq("rating")).select(data_text.lt("3.0"))))
                        .and(title)
                        .and(qlink().select(target).select(title)))
    };

    let q33b = move || {
        let qlink = || link.with(link_ty.rx(r"follow")
                                 .and(target.with(kind.eq("tv series")
                                                  .and(company)
                                                  .and(data.with(data_ty.eq("rating").and(data_text.lt("3.0"))))
                                                  .and(production_year.eq(2007)))));
        db.movie.with(kind.eq("tv series")
                      .and(company.select(country).eq("[nl]"))
                      .and(qlink()))
                .select(company.with(country.eq("[nl]")).select(company_name)
                        .and(qlink().select(target).select(company).select(company_name))
                        .and(data.with(data_ty.eq("rating")).select(data_text))
                        .and(qlink().select(target)
                                    .select(data.with(data_ty.eq("rating")).select(data_text.lt("3.0"))))
                        .and(title)
                        .and(qlink().select(target).select(title)))
    };

    let q33c = move || {
        let qlink = || link.with(link_ty.is_in(link3())
                                 .and(target.with(kind.is_in(["tv series", "episode"])
                                                  .and(company)
                                                  .and(data.with(data_ty.eq("rating").and(data_text.lt("3.5"))))
                                                  .and(production_year.between(2000, 2010)))));
        db.movie.with(kind.is_in(["tv series", "episode"])
                      .and(company.select(country).ne("[us]"))
                      .and(qlink()))
                .select(company.with(country.ne("[us]")).select(company_name)
                        .and(qlink().select(target).select(company).select(company_name))
                        .and(data.with(data_ty.eq("rating")).select(data_text))
                        .and(qlink().select(target)
                                    .select(data.with(data_ty.eq("rating")).select(data_text.lt("3.5"))))
                        .and(title)
                        .and(qlink().select(target).select(title)))
    };

    vec![
        entry("1a", "(A Warner Bros.-First National Picture) (presents) || A Clockwork Orange || 1934", move || min_row(q1a())),
        entry("1b", "(Set Decoration Rentals) (uncredited) || Disaster Movie || 2008", move || min_row(q1b())),
        entry("1c", "(co-production) || Intouchables || 2011", move || min_row(q1c())),
        entry("1d", "(Set Decoration Rentals) (uncredited) || Disaster Movie || 2004", move || min_row(q1d())),
        entry("2a", "'Doc'", move || min_row(q2a())),
        entry("2b", "'Doc'", move || min_row(q2b())),
        entry("2c", "(empty)", move || min_row(q2c())),
        entry("2d", "& Teller", move || min_row(q2d())),
        entry("3a", "2 Days in New York", move || min_row(q3a())),
        entry("3b", "300: Rise of an Empire", move || min_row(q3b())),
        entry("3c", "& Teller 2", move || min_row(q3c())),
        entry("4a", "5.1 || & Teller 2", move || min_row(q4a())),
        entry("4b", "9.1 || Batman: Arkham City", move || min_row(q4b())),
        entry("4c", "2.1 || & Teller 2", move || min_row(q4c())),
        entry("5a", "(empty)", move || min_row(q5a())),
        entry("5b", "(empty)", move || min_row(q5b())),
        entry("5c", "11,830,420", move || min_row(q5c())),
        entry("6a", "marvel-cinematic-universe || Iron Man 3 || Downey Jr., Robert", move || min_row(q6a())),
        entry("6b", "based-on-comic || The Avengers 2 || Downey Jr., Robert", move || min_row(q6b())),
        entry("6c", "marvel-cinematic-universe || The Avengers 2 || Downey Jr., Robert", move || min_row(q6c())),
        entry("6d", "based-on-comic || 2008 MTV Movie Awards || Downey Jr., Robert", move || min_row(q6d())),
        entry("6e", "marvel-cinematic-universe || Iron Man 3 || Downey Jr., Robert", move || min_row(q6e())),
        entry("6f", "based-on-comic || & Teller 2 || \"Steff\", Stefanie Oxmann Mcgaha", move || min_row(q6f())),
        entry("7a", "Antonioni, Michelangelo || Dressed to Kill", move || min_row(q7a())),
        entry("7b", "De Palma, Brian || Dressed to Kill", move || min_row(q7b())),
        entry("7c",
              "50 Cent || \"Boo\" Arnold was born Earl Arnold in Hattiesburg, Mississippi in 1966. His father gave him the nickname 'Boo' early in life and it stuck through grade school, high school, and college. He is still known as \"Boo\" to family and friends.  Raised in central Texas, Arnold played baseball at Texas Tech University where he graduated with a BA in Advertising and Marketing. While at Texas Tech he was also a member of the Texas Epsilon chapter of Phi Delta Theta fraternity. After college he worked with Young Life, an outreach to high school students, in San Antonio, Texas.  While with Young Life Arnold began taking extension courses through Fuller Theological Seminary and ultimately went full-time to Gordon-Conwell Theological Seminary in Boston, Massachusetts. At Gordon-Conwell he completed a Master's Degree in Divinity studying Theology, Philosophy, Church History, Biblical Languages (Hebrew & Greek), and Exegetical Methods. Following seminary he was involved with reconciliation efforts in the former Yugoslavia shortly after the war ended there in1995.  Arnold started acting in his early thirties in Texas. After an encouraging visit to Los Angeles where he spent time with childhood friend George Eads (of CSI Las Vegas) he decided to move to Los Angeles in 2001 to pursue acting full-time. While in Los Angeles he has studied acting with Judith Weston at Judith Weston Studio for Actors and Directors.  Arnold's acting career has been one of steady development, booking co-star and guest-star roles in nighttime television. He guest-starred opposite of Jane Seymour on the night time television drama Justice. He played the lead, Michael Hollister, in the film The Seer, written and directed by Patrick Masset (Friday Night Lights).  He was nominated Best Actor in the168 Film Festival for the role of Phil Stevens in the short-film Useless. In Useless he played a US Marshal who must choose between mercy and justice as he confronts the man who murdered his father. Arnold's performance in Useless confirmed his ability to carry lead roles, and he continues to work toward solidifying himself as a male lead in film and television.  Arnold married fellow Texan Stacy Rudd of San Antonio in 2003 and they are now raising their three children in the Los Angeles area.",
              move || min_row(q7c())),
        entry("8a", "Chambers, Linda || .hack//Quantum", move || min_row(q8a())),
        entry("8b", "Chambers, Linda || Dragon Ball Z: Shin Budokai", move || min_row(q8b())),
        entry("8c", "\"A.J.\" || #1 Cheerleader Camp", move || min_row(q8c())),
        entry("8d", "\"Jenny from the Block\" || #1 Cheerleader Camp", move || min_row(q8d())),
        entry("9a", "AJ || Airport Announcer || Blue Harvest", move || min_row(q9a())),
        entry("9b", "AJ || Airport Announcer || Bassett, Angela || Blue Harvest", move || min_row(q9b())),
        entry("9c", "'Annette' || 2nd Balladeer || Alborg, Ana Esther || (1975-01-20)", move || min_row(q9c())),
        entry("9d", "!!!, Toy || Aaron, Caroline || \"Cockamamie's\" Salesgirl || $15,000.00 Error", move || min_row(q9d())),
        entry("10a", "Actor || 12 Rounds", move || min_row(q10a())),
        entry("10b", "(empty)", move || min_row(q10b())),
        entry("10c", "Himself || Evil Eyes: Behind the Scenes", move || min_row(q10c())),
        entry("11a", "Churchill Films || followed by || Batman Beyond", move || min_row(q11a())),
        entry("11b", "Filmlance International AB || follows || The Money Man", move || min_row(q11b())),
        entry("11c", "20th Century Fox Home Entertainment || (1997-2002) (worldwide) (all media) || 24", move || min_row(q11c())),
        entry("11d", "13th Street || (1954) (UK) (TV) || ...denn sie wissen nicht, was sie tun", move || min_row(q11d())),
        entry("12a", "10th Grade Reunion Films || 8.1 || 3:20", move || min_row(q12a())),
        entry("12b", "$10,000 || Birdemic: Shock and Terror", move || min_row(q12b())),
        entry("12c", "\"Oh That Gus!\" || 7.1 || $1.11", move || min_row(q12c())),
        entry("13a", "Afghanistan:24 June 2012 || 1.0 || &Me", move || min_row(q13a())),
        entry("13b", "501audio || 1.8 || 5 Time Champion", move || min_row(q13b())),
        entry("13c", "DL Sites || 1.8 || Champion", move || min_row(q13c())),
        entry("13d", "\"O\" Films || 1.0 || #54 Meets #47", move || min_row(q13d())),
        entry("14a", "1.0 || $lowdown", move || min_row(q14a())),
        entry("14b", "6.4 || Of Dolls and Murder", move || min_row(q14b())),
        entry("14c", "1.0 || $lowdown", move || min_row(q14c())),
        entry("15a", "USA:1 June 2007 || Battlestar Galactica: The Resistance", move || min_row(q15a())),
        entry("15b", "USA:27 April 2007 || RoboCop vs Terminator", move || min_row(q15b())),
        entry("15c", "USA:1 April 2003 || 24: Day Six - Debrief", move || min_row(q15c())),
        entry("15d", "(Not So) Instant Photo || 06/05", move || min_row(q15d())),
        entry("16a", "Adams, Stan || Carol Burnett vs. Anthony Perkins", move || min_row(q16a())),
        entry("16b", "!!!, Toy || & Teller", move || min_row(q16b())),
        entry("16c", "\"Brooklyn\" Tony Danza || (#1.5)", move || min_row(q16c())),
        entry("16d", "\"Brooklyn\" Tony Danza || (#1.5)", move || min_row(q16d())),
        entry("17a", "B, Khaz", move || min_row(q17a())),
        entry("17b", "Z'Dar, Robert", move || min_row(q17b())),
        entry("17c", "X'Volaitis, John", move || min_row(q17c())),
        entry("17d", "Abrahamsson, Bertil", move || min_row(q17d())),
        entry("17e", "$hort, Too", move || min_row(q17e())),
        entry("17f", "'El Galgo PornoStar', Blanquito", move || min_row(q17f())),
        entry("18a", "$1,000 || 10 || 40 Days and 40 Nights", move || min_row(q18a())),
        entry("18b", "Horror || 8.1 || Agorable", move || min_row(q18b())),
        entry("18c", "Action || 10 || #PostModem", move || min_row(q18c())),
        entry("19a", "Angeline, Moriah || Blue Harvest", move || min_row(q19a())),
        entry("19b", "Jolie, Angelina || Kung Fu Panda", move || min_row(q19b())),
        entry("19c", "Alborg, Ana Esther || .hack//Akusei heni vol. 2", move || min_row(q19c())),
        entry("19d", "Aaron, Caroline || $9.99", move || min_row(q19d())),
        entry("20a", "Disaster Movie", move || min_row(q20a())),
        entry("20b", "Iron Man", move || min_row(q20b())),
        entry("20c", "Abell, Alistair || ...And Then I...", move || min_row(q20c())),
        entry("21a", "Det Danske Filminstitut || followed by || Der Serienkiller - Klinge des Todes", move || min_row(q21a())),
        entry("21b", "Filmlance International AB || followed by || Hämndens pris", move || min_row(q21b())),
        entry("21c", "Churchill Films || followed by || Batman Beyond", move || min_row(q21c())),
        entry("22a", "(empty)", move || min_row(q22a())),
        entry("22b", "(empty)", move || min_row(q22b())),
        entry("22c", "(empty)", move || min_row(q22c())),
        entry("22d", "(#1.1) || 2.0 || 13 Productions", move || min_row(q22d())),
        entry("23a", "movie || The Analysts", move || min_row(q23a())),
        entry("23b", "movie || The Big Mope", move || min_row(q23b())),
        entry("23c", "movie || Dirt Merchant", move || min_row(q23c())),
        entry("24a", "Additional Voices || Baker, Andrea || Baiohazâdo 6", move || min_row(q24a())),
        entry("24b", "Tigress || Jolie, Angelina || Kung Fu Panda 2", move || min_row(q24b())),
        entry("25a", "Horror || 10 || -- And Now the Screaming Starts! || Abdallah, Damon", move || min_row(q25a())),
        entry("25b", "Horror || 138 || Vampire Boys || Campbell, Jeremiah", move || min_row(q25b())),
        entry("25c", "Action || 10 || $ || Aakeson, Kim Fupz", move || min_row(q25c())),
        entry("26a", "'Agua' Man || Acereda, Hermie || 7.1 || 3:10 to Yuma", move || min_row(q26a())),
        entry("26b", "Bank Manager || 8.2 || Inception", move || min_row(q26b())),
        entry("26c", "'Agua' Man || 1.9 || 12 Rounds", move || min_row(q26c())),
        entry("27a", "Det Danske Filminstitut || followed by || Spår i mörker", move || min_row(q27a())),
        entry("27b", "Filmlance International AB || followed by || Vita nätter", move || min_row(q27b())),
        entry("27c", "Det Danske Filminstitut || followed by || Spår i mörker", move || min_row(q27c())),
        entry("28a", "01 Distribuzione || 2.9 || (#1.1)", move || min_row(q28a())),
        entry("28b", "20th Century Fox || 6.6 || (#1.1)", move || min_row(q28b())),
        entry("28c", "01 Distribuzione || 1.9 || (#1.1)", move || min_row(q28c())),
        entry("29a", "Queen || Andrews, Julie || Shrek 2", move || min_row(q29a())),
        entry("29b", "Queen || Andrews, Julie || Shrek 2", move || min_row(q29b())),
        entry("29c", "Lola || Andrews, Julie || Hoodwinked!", move || min_row(q29c())),
        entry("30a", "Horror || 100356 || 16 Blocks || Abrams, J.J.", move || min_row(q30a())),
        entry("30b", "Horror || 194782 || Freddy vs. Jason || Shannon, Damian", move || min_row(q30b())),
        entry("30c", "Action || 100356 || $ || Abernathy, Lewis", move || min_row(q30c())),
        entry("31a", "Horror || 1040 || 2001 Maniacs || Agnew, Jim", move || min_row(q31a())),
        entry("31b", "Horror || 129755 || Saw || Bousman, Darren Lynn", move || min_row(q31b())),
        entry("31c", "Action || 1008 || 11:14 || Abraham, Brad", move || min_row(q31c())),
        entry("32a", "(empty)", move || min_row(q32a())),
        entry("32b", "alternate language version of || 12 oz. Mouse || 'Angel': Season 2 Overview", move || min_row(q32b())),
        entry("33a", "495 Productions || 495 Productions || 3.3 || 2.7 || A Double Shot at Love || A Shot at Love with Tila Tequila", move || min_row(q33a())),
        entry("33b", "MTV Netherlands || 495 Productions || 3.3 || 2.7 || A Double Shot at Love || A Shot at Love with Tila Tequila", move || min_row(q33b())),
        entry("33c", "2BE || 495 Productions || 1.3 || 1.0 || A Double Shot at Love || A Double Shot at Love", move || min_row(q33c())),
        entry("6a/method", "marvel-cinematic-universe || Iron Man 3 || Downey Jr., Robert", move || min_row(q6a_methods(db))),
    ]
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
//            navigate: `cast.select(person).select(name)` walks
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
    let Movie { title, production_year, keyword, cast, .. } = &db.movie;
    let Cast { person, .. } = &db.cast;
    let Person { name, .. } = &db.person;
    let kw_marvel = || keyword.eq("marvel-cinematic-universe");
    let q = db.movie.with(production_year.gt(2010)
                          .and(kw_marvel()))
                    .select(kw_marvel()
                            .and(title)
                            .and(cast.select(person).select(name).rx(r"Downey.*Robert")));
    q
}
