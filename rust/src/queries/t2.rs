// queries: queries.jl lines ~381-588 (22d, 5b, 5c, 15a-d, 11c-d, 13d, 6a-f)

use crate::engine::*;
use crate::job_schema::*;
use crate::queries::helpers::{min_row, Row};
use crate::queries::sets::{kw8, murder4, nordic10};

pub const ENTRIES: &[super::Entry] = &[
    ("22d", "(#1.1) || 2.0 || 13 Productions", |db| min_row(q22d(db))),
    ("5b",  "(empty)", |db| min_row(q5b(db))),
    ("5c",  "11,830,420", |db| min_row(q5c(db))),
    ("15a", "USA:1 June 2007 || Battlestar Galactica: The Resistance", |db| min_row(q15a(db))),
    ("15b", "USA:27 April 2007 || RoboCop vs Terminator", |db| min_row(q15b(db))),
    ("15c", "USA:1 April 2003 || 24: Day Six - Debrief", |db| min_row(q15c(db))),
    ("15d", "(Not So) Instant Photo || 06/05", |db| min_row(q15d(db))),
    ("11c", "20th Century Fox Home Entertainment || (1997-2002) (worldwide) (all media) || 24", |db| min_row(q11c(db))),
    ("11d", "13th Street || (1954) (UK) (TV) || ...denn sie wissen nicht, was sie tun", |db| min_row(q11d(db))),
    ("13d", "\"O\" Films || 1.0 || #54 Meets #47", |db| min_row(q13d(db))),
    ("6a",  "marvel-cinematic-universe || Iron Man 3 || Downey Jr., Robert", |db| min_row(q6a(db))),
    ("6b",  "based-on-comic || The Avengers 2 || Downey Jr., Robert", |db| min_row(q6b(db))),
    ("6c",  "marvel-cinematic-universe || The Avengers 2 || Downey Jr., Robert", |db| min_row(q6c(db))),
    ("6d",  "based-on-comic || 2008 MTV Movie Awards || Downey Jr., Robert", |db| min_row(q6d(db))),
    ("6e",  "marvel-cinematic-universe || Iron Man 3 || Downey Jr., Robert", |db| min_row(q6e(db))),
    ("6f",  "based-on-comic || & Teller 2 || \"Steff\", Stefanie Oxmann Mcgaha", |db| min_row(q6f(db))),
];

fn q22d(db: &'static Job) -> impl Drive<R: Row> {
    let Movie { title, kind, production_year, keyword, company, info, data, .. } = &db.movie;
    let Company { name: company_name, country, ty: company_ty, .. } = &db.company;
    let CompanyType { text: companytype_text, .. } = &db.company_type;
    let Data { text: data_text, ty: data_ty, .. } = &db.data;
    let Info { info: info_info, ty: info_ty, .. } = &db.info;
    let InfoType { text: infotype_text, .. } = &db.info_type;
    let Keyword { text: keyword_text, .. } = &db.keyword;
    let Kind { text: kind_text, .. } = &db.kind;
    let movie = db.movie.all();
    movie.with(info.select(info_ty.select(infotype_text).eq("countries")
                      .and(info_info.is_in(nordic10())))
          .and(keyword.select(keyword_text).is_in(murder4()))
          .and(production_year.gt(2005))
          .and(kind.select(kind_text).is_in(["movie", "episode"])))
       .select(title
          .and(data.with(data_text.lt("8.5")
                    .and(data_ty.select(infotype_text).eq("rating"))).select(data_text))
          .and(company.with(country.ne("[us]")
                       .and(company_ty.select(companytype_text).eq("production companies"))).select(company_name)))
}

fn q5b(db: &'static Job) -> impl Drive<R: Row> {
    let Movie { title, production_year, company, info, .. } = &db.movie;
    let Company { note: company_note, ty: company_ty, .. } = &db.company;
    let CompanyType { text: companytype_text, .. } = &db.company_type;
    let Info { info: info_info, .. } = &db.info;
    let movie = db.movie.all();
    movie.with(company.select(company_ty.select(companytype_text).eq("production companies")
                         .and(company_note.rx(r"\(VHS\)"))
                         .and(company_note.rx(r"\(USA\)"))
                         .and(company_note.rx(r"\(1994\)")))
          .and(info.select(info_info).is_in(["USA", "America"]))
          .and(production_year.gt(2010)))
        .select(title)
}

fn q5c(db: &'static Job) -> impl Drive<R: Row> {
    let Movie { title, production_year, company, info, .. } = &db.movie;
    let Company { note: company_note, ty: company_ty, .. } = &db.company;
    let CompanyType { text: companytype_text, .. } = &db.company_type;
    let Info { info: info_info, .. } = &db.info;
    let movie = db.movie.all();
    movie.with(company.select(company_ty.select(companytype_text).eq("production companies")
                         .and(company_note.nrx(r"\(TV\)"))
                         .and(company_note.rx(r"\(USA\)")))
          .and(info.select(info_info).is_in(nordic10()))
          .and(production_year.gt(1990)))
        .select(title)
}

fn q15a(db: &'static Job) -> impl Drive<R: Row> {
    let Movie { title, production_year, keyword, company, info, aka, .. } = &db.movie;
    let Company { country, note: company_note, .. } = &db.company;
    let Info { info: info_info, ty: info_ty, note: info_note, .. } = &db.info;
    let InfoType { text: infotype_text, .. } = &db.info_type;
    let movie = db.movie.all();
    movie.with(production_year.gt(2000)
          .and(company.select(country.eq("[us]")
                         .and(company_note.rx(r"\(200.*\)"))
                         .and(company_note.rx(r"\(worldwide\)"))))
          .and(keyword)
          .and(aka))
       .select(info.with(info_ty.select(infotype_text).eq("release dates")
                    .and(info_info.rx(r"^USA:.* 200"))
                    .and(info_note.rx(r"internet"))).select(info_info)
          .and(title))
}

fn q15b(db: &'static Job) -> impl Drive<R: Row> {
    let Movie { title, production_year, keyword, company, info, aka, .. } = &db.movie;
    let Company { name: company_name, country, note: company_note, .. } = &db.company;
    let Info { info: info_info, ty: info_ty, note: info_note, .. } = &db.info;
    let InfoType { text: infotype_text, .. } = &db.info_type;
    let movie = db.movie.all();
    movie.with(company.select(country.eq("[us]")
                         .and(company_name.eq("YouTube"))
                         .and(company_note.rx(r"\(200.*\)"))
                         .and(company_note.rx(r"\(worldwide\)")))
          .and(keyword)
          .and(aka)
          .and(production_year.ge(2005))
          .and(production_year.le(2010)))
       .select(info.with(info_ty.select(infotype_text).eq("release dates")
                    .and(info_info.rx(r"^USA:.* 200"))
                    .and(info_note.rx(r"internet"))).select(info_info)
          .and(title))
}

fn q15c(db: &'static Job) -> impl Drive<R: Row> {
    let Movie { title, production_year, keyword, company, info, aka, .. } = &db.movie;
    let Company { country, .. } = &db.company;
    let Info { info: info_info, ty: info_ty, note: info_note, .. } = &db.info;
    let InfoType { text: infotype_text, .. } = &db.info_type;
    let movie = db.movie.all();
    movie.with(company.select(country).eq("[us]")
          .and(keyword)
          .and(aka)
          .and(production_year.gt(1990)))
       .select(info.with(info_ty.select(infotype_text).eq("release dates")
                    .and(info_info.rx(r"^USA:.* 199|^USA:.* 200"))
                    .and(info_note.rx(r"internet"))).select(info_info)
          .and(title))
}

fn q15d(db: &'static Job) -> impl Drive<R: Row> {
    let Movie { title, production_year, keyword, company, info, aka, .. } = &db.movie;
    let AkaTitle { text: akatitle_text, .. } = &db.aka_title;
    let Company { country, .. } = &db.company;
    let Info { ty: info_ty, note: info_note, .. } = &db.info;
    let InfoType { text: infotype_text, .. } = &db.info_type;
    let movie = db.movie.all();
    movie.with(company.select(country).eq("[us]")
          .and(keyword)
          .and(info.select(info_ty.select(infotype_text).eq("release dates")
                      .and(info_note.rx(r"internet"))))
          .and(production_year.gt(1990)))
       .select(aka.select(akatitle_text)
          .and(title))
}

fn q11c(db: &'static Job) -> impl Drive<R: Row> {
    let Movie { title, production_year, keyword, company, link, .. } = &db.movie;
    let Company { name: company_name, country, note: company_note, ty: company_ty, .. } = &db.company;
    let CompanyType { text: companytype_text, .. } = &db.company_type;
    let Keyword { text: keyword_text, .. } = &db.keyword;
    let movie = db.movie.all();
    movie.with(keyword.select(keyword_text).is_in(["sequel", "revenge", "based-on-novel"])
          .and(production_year.gt(1950))
          .and(link))
       .select(company.with(country.ne("[pl]")
                       .and(company_name.rx(r"^20th Century Fox|^Twentieth Century Fox"))
                       .and(company_ty.select(companytype_text).ne("production companies"))
                       .and(company_note)).select(company_name.and(company_note))
          .and(title))
}

fn q11d(db: &'static Job) -> impl Drive<R: Row> {
    let Movie { title, production_year, keyword, company, link, .. } = &db.movie;
    let Company { name: company_name, country, note: company_note, ty: company_ty, .. } = &db.company;
    let CompanyType { text: companytype_text, .. } = &db.company_type;
    let Keyword { text: keyword_text, .. } = &db.keyword;
    let movie = db.movie.all();
    movie.with(keyword.select(keyword_text).is_in(["sequel", "revenge", "based-on-novel"])
          .and(production_year.gt(1950))
          .and(link))
       .select(company.with(country.ne("[pl]")
                       .and(company_ty.select(companytype_text).ne("production companies"))
                       .and(company_note)).select(company_name.and(company_note))
          .and(title))
}

fn q13d(db: &'static Job) -> impl Drive<R: Row> {
    let Movie { title, kind, company, info, data, .. } = &db.movie;
    let Company { name: company_name, country, ty: company_ty, .. } = &db.company;
    let CompanyType { text: companytype_text, .. } = &db.company_type;
    let Data { text: data_text, ty: data_ty, .. } = &db.data;
    let Info { ty: info_ty, .. } = &db.info;
    let InfoType { text: infotype_text, .. } = &db.info_type;
    let Kind { text: kind_text, .. } = &db.kind;
    let movie = db.movie.all();
    movie.with(kind.select(kind_text).eq("movie")
          .and(info.select(info_ty).select(infotype_text).eq("release dates")))
       .select(company.with(country.eq("[us]")
                       .and(company_ty.select(companytype_text).eq("production companies"))).select(company_name)
          .and(data.with(data_ty.select(infotype_text).eq("rating")).select(data_text))
          .and(title))
}

// q6a/c/e share the marvel-cinematic-universe keyword and q6b/d the kw8
// list; within each pair only the year cutoff varies.
fn q6_marvel(db: &'static Job, year: i64) -> impl Drive<R: Row> {
    let Movie { title, production_year, keyword, cast, .. } = &db.movie;
    let Cast { person, .. } = &db.cast;
    let Keyword { text: keyword_text, .. } = &db.keyword;
    let Person { name: person_name, .. } = &db.person;
    let movie = db.movie.all();
    let kw = || keyword.select(keyword_text).eq("marvel-cinematic-universe");
    let downey = cast.select(person).select(person_name).rx(r"Downey.*Robert");
    movie.with(production_year.gt(year).and(kw()))
       .select(kw().and(title).and(downey))
}

fn q6_comic(db: &'static Job, year: i64) -> impl Drive<R: Row> {
    let Movie { title, production_year, keyword, cast, .. } = &db.movie;
    let Cast { person, .. } = &db.cast;
    let Keyword { text: keyword_text, .. } = &db.keyword;
    let Person { name: person_name, .. } = &db.person;
    let movie = db.movie.all();
    let kw = || keyword.select(keyword_text).is_in(kw8());
    let downey = cast.select(person).select(person_name).rx(r"Downey.*Robert");
    movie.with(production_year.gt(year).and(kw()))
       .select(kw().and(title).and(downey))
}

fn q6a(db: &'static Job) -> impl Drive<R: Row> { q6_marvel(db, 2010) }
fn q6b(db: &'static Job) -> impl Drive<R: Row> { q6_comic(db, 2014) }
fn q6c(db: &'static Job) -> impl Drive<R: Row> { q6_marvel(db, 2014) }
fn q6d(db: &'static Job) -> impl Drive<R: Row> { q6_comic(db, 2000) }
fn q6e(db: &'static Job) -> impl Drive<R: Row> { q6_marvel(db, 2000) }

fn q6f(db: &'static Job) -> impl Drive<R: Row> {
    let Movie { title, production_year, keyword, cast, .. } = &db.movie;
    let Cast { person, .. } = &db.cast;
    let Keyword { text: keyword_text, .. } = &db.keyword;
    let Person { name: person_name, .. } = &db.person;
    let movie = db.movie.all();
    let kw = || keyword.select(keyword_text).is_in(kw8());
    let cast_name = cast.select(person).select(person_name);
    movie.with(production_year.gt(2000).and(kw()))
       .select(kw().and(title).and(cast_name))
}
