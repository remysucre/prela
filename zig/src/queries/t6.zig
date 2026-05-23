const std = @import("std");
const Io = std.Io;
const Data = @import("../data.zig").Data;
const h = @import("../helpers.zig");
const rx = @import("../regex.zig");
const sets = @import("../sets.zig");
const Entry = @import("all.zig").Entry;

pub const ENTRIES: []const Entry = &.{
    .{ .name = "27a", .oracle = "Det Danske Filminstitut || followed by || Spår i mörker", .run = q27a },
    .{ .name = "27b", .oracle = "Filmlance International AB || followed by || Vita nätter", .run = q27b },
    .{ .name = "27c", .oracle = "Det Danske Filminstitut || followed by || Spår i mörker", .run = q27c },
    .{ .name = "28a", .oracle = "01 Distribuzione || 2.9 || (#1.1)", .run = q28a },
    .{ .name = "28b", .oracle = "20th Century Fox || 6.6 || (#1.1)", .run = q28b },
    .{ .name = "28c", .oracle = "01 Distribuzione || 1.9 || (#1.1)", .run = q28c },
    .{ .name = "29a", .oracle = "Queen || Andrews, Julie || Shrek 2", .run = q29a },
    .{ .name = "29b", .oracle = "Queen || Andrews, Julie || Shrek 2", .run = q29b },
    .{ .name = "29c", .oracle = "Lola || Andrews, Julie || Hoodwinked!", .run = q29c },
    .{ .name = "30a", .oracle = "Horror || 100356 || 16 Blocks || Abrams, J.J.", .run = q30a },
    .{ .name = "30b", .oracle = "Horror || 194782 || Freddy vs. Jason || Shannon, Damian", .run = q30b },
    .{ .name = "30c", .oracle = "Action || 100356 || $ || Abernathy, Lewis", .run = q30c },
    .{ .name = "31a", .oracle = "Horror || 1040 || 2001 Maniacs || Agnew, Jim", .run = q31a },
    .{ .name = "31b", .oracle = "Horror || 129755 || Saw || Bousman, Darren Lynn", .run = q31b },
    .{ .name = "31c", .oracle = "Action || 1008 || 11:14 || Abraham, Brad", .run = q31c },
    .{ .name = "32a", .oracle = "(empty)", .run = q32a },
    .{ .name = "32b", .oracle = "alternate language version of || 12 oz. Mouse || 'Angel': Season 2 Overview", .run = q32b },
    .{ .name = "33a", .oracle = "495 Productions || 495 Productions || 3.3 || 2.7 || A Double Shot at Love || A Shot at Love with Tila Tequila", .run = q33a },
    .{ .name = "33b", .oracle = "MTV Netherlands || 495 Productions || 3.3 || 2.7 || A Double Shot at Love || A Shot at Love with Tila Tequila", .run = q33b },
    .{ .name = "33c", .oracle = "2BE || 495 Productions || 1.3 || 1.0 || A Double Shot at Love || A Double Shot at Love", .run = q33c },
};

fn q27a(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_complete_cast.in_s(
            d.completecast_subject.o(d.compcasttype_kind).in_v(&[_][]const u8{ "cast", "crew" }).k()
                .@"and"(d.completecast_status.o(d.compcasttype_kind).eq("complete").k())
        ).k()
            .@"and"(d.movie_company.in_s(
                d.company_country.ne("[pl]").k()
                    .@"and"(
                        d.company_name.rx(rx.film).k()
                            .@"or"(d.company_name.rx(rx.warner).k())
                    )
                    .@"and"(
                        d.company_type.o(d.companytype_kind).eq("production companies").k()
                            .minus(d.company_note.k())
                    )
            ).k())
            .@"and"(d.movie_keyword.o(d.keyword_keyword).eq("sequel").k())
            .@"and"(d.movie_link.in_s(d.movielink_type.o(d.linktype_link).rx(rx.follow).k()).k())
            .@"and"(d.movie_info.in_s(d.info_info.in_v(&[_][]const u8{ "Sweden", "Germany", "Swedish", "German" }).k()).k())
            .@"and"(d.movie_production_year.ge(1950).k())
            .@"and"(d.movie_production_year.le(2000).k())
            .o(
                d.movie_company.in_s(
                    d.company_country.ne("[pl]").k()
                        .@"and"(
                            d.company_name.rx(rx.film).k()
                                .@"or"(d.company_name.rx(rx.warner).k())
                        )
                        .@"and"(
                            d.company_type.o(d.companytype_kind).eq("production companies").k()
                                .minus(d.company_note.k())
                        )
                ).o(d.company_name)
                    .x(d.movie_link.in_s(d.movielink_type.o(d.linktype_link).rx(rx.follow).k()).o(d.movielink_type.o(d.linktype_link)))
                    .x(d.movie_title)
            )
    );
    var acc = h.Acc3{};
    q.drive(h.Sink(h.Acc3){ .acc = &acc });
    try h.fmt3(w, acc.m0, acc.m1, acc.m2);
}

fn q27b(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_complete_cast.in_s(
            d.completecast_subject.o(d.compcasttype_kind).in_v(&[_][]const u8{ "cast", "crew" }).k()
                .@"and"(d.completecast_status.o(d.compcasttype_kind).eq("complete").k())
        ).k()
            .@"and"(d.movie_company.in_s(
                d.company_country.ne("[pl]").k()
                    .@"and"(
                        d.company_name.rx(rx.film).k()
                            .@"or"(d.company_name.rx(rx.warner).k())
                    )
                    .@"and"(
                        d.company_type.o(d.companytype_kind).eq("production companies").k()
                            .minus(d.company_note.k())
                    )
            ).k())
            .@"and"(d.movie_keyword.o(d.keyword_keyword).eq("sequel").k())
            .@"and"(d.movie_link.in_s(d.movielink_type.o(d.linktype_link).rx(rx.follow).k()).k())
            .@"and"(d.movie_info.in_s(d.info_info.in_v(&[_][]const u8{ "Sweden", "Germany", "Swedish", "German" }).k()).k())
            .@"and"(d.movie_production_year.eq(1998).k())
            .o(
                d.movie_company.in_s(
                    d.company_country.ne("[pl]").k()
                        .@"and"(
                            d.company_name.rx(rx.film).k()
                                .@"or"(d.company_name.rx(rx.warner).k())
                        )
                        .@"and"(
                            d.company_type.o(d.companytype_kind).eq("production companies").k()
                                .minus(d.company_note.k())
                        )
                ).o(d.company_name)
                    .x(d.movie_link.in_s(d.movielink_type.o(d.linktype_link).rx(rx.follow).k()).o(d.movielink_type.o(d.linktype_link)))
                    .x(d.movie_title)
            )
    );
    var acc = h.Acc3{};
    q.drive(h.Sink(h.Acc3){ .acc = &acc });
    try h.fmt3(w, acc.m0, acc.m1, acc.m2);
}

fn q27c(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_complete_cast.in_s(
            d.completecast_subject.o(d.compcasttype_kind).eq("cast").k()
                .@"and"(d.completecast_status.o(d.compcasttype_kind).rx(rx.pre_complete).k())
        ).k()
            .@"and"(d.movie_company.in_s(
                d.company_country.ne("[pl]").k()
                    .@"and"(
                        d.company_name.rx(rx.film).k()
                            .@"or"(d.company_name.rx(rx.warner).k())
                    )
                    .@"and"(
                        d.company_type.o(d.companytype_kind).eq("production companies").k()
                            .minus(d.company_note.k())
                    )
            ).k())
            .@"and"(d.movie_keyword.o(d.keyword_keyword).eq("sequel").k())
            .@"and"(d.movie_link.in_s(d.movielink_type.o(d.linktype_link).rx(rx.follow).k()).k())
            .@"and"(d.movie_info.in_s(d.info_info.in_v(sets.nordic9).k()).k())
            .@"and"(d.movie_production_year.ge(1950).k())
            .@"and"(d.movie_production_year.le(2010).k())
            .o(
                d.movie_company.in_s(
                    d.company_country.ne("[pl]").k()
                        .@"and"(
                            d.company_name.rx(rx.film).k()
                                .@"or"(d.company_name.rx(rx.warner).k())
                        )
                        .@"and"(
                            d.company_type.o(d.companytype_kind).eq("production companies").k()
                                .minus(d.company_note.k())
                        )
                ).o(d.company_name)
                    .x(d.movie_link.in_s(d.movielink_type.o(d.linktype_link).rx(rx.follow).k()).o(d.movielink_type.o(d.linktype_link)))
                    .x(d.movie_title)
            )
    );
    var acc = h.Acc3{};
    q.drive(h.Sink(h.Acc3){ .acc = &acc });
    try h.fmt3(w, acc.m0, acc.m1, acc.m2);
}

fn q28a(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_complete_cast.in_s(
            d.completecast_subject.o(d.compcasttype_kind).eq("crew").k()
                .@"and"(d.completecast_status.o(d.compcasttype_kind).ne("complete+verified").k())
        ).k()
            .@"and"(d.movie_company.in_s(
                d.company_country.ne("[us]").k()
                    .@"and"(d.company_note.nrx(rx.paren_USA).k())
                    .@"and"(d.company_note.rx(rx.paren_200_dot).k())
            ).k())
            .@"and"(d.movie_info.in_s(
                d.info_type.o(d.infotype_info).eq("countries").k()
                    .@"and"(d.info_info.in_v(sets.nordic10).k())
            ).k())
            .@"and"(d.movie_data.in_s(
                d.data_type.o(d.infotype_info).eq("rating").k()
                    .@"and"(d.data_data.lt("8.5").k())
            ).k())
            .@"and"(d.movie_keyword.o(d.keyword_keyword).in_v(sets.murder4).k())
            .@"and"(d.movie_kind.o(d.kind_kind).in_v(&[_][]const u8{ "movie", "episode" }).k())
            .@"and"(d.movie_production_year.gt(2000).k())
            .o(
                d.movie_company.in_s(
                    d.company_country.ne("[us]").k()
                        .@"and"(d.company_note.nrx(rx.paren_USA).k())
                        .@"and"(d.company_note.rx(rx.paren_200_dot).k())
                ).o(d.company_name)
                    .x(d.movie_data.in_s(
                        d.data_type.o(d.infotype_info).eq("rating").k()
                            .@"and"(d.data_data.lt("8.5").k())
                    ).o(d.data_data))
                    .x(d.movie_title)
            )
    );
    var acc = h.Acc3{};
    q.drive(h.Sink(h.Acc3){ .acc = &acc });
    try h.fmt3(w, acc.m0, acc.m1, acc.m2);
}

fn q28b(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_complete_cast.in_s(
            d.completecast_subject.o(d.compcasttype_kind).eq("crew").k()
                .@"and"(d.completecast_status.o(d.compcasttype_kind).ne("complete+verified").k())
        ).k()
            .@"and"(d.movie_company.in_s(
                d.company_country.ne("[us]").k()
                    .@"and"(d.company_note.nrx(rx.paren_USA).k())
                    .@"and"(d.company_note.rx(rx.paren_200_dot).k())
            ).k())
            .@"and"(d.movie_info.in_s(
                d.info_type.o(d.infotype_info).eq("countries").k()
                    .@"and"(d.info_info.in_v(&[_][]const u8{ "Sweden", "Germany", "Swedish", "German" }).k())
            ).k())
            .@"and"(d.movie_data.in_s(
                d.data_type.o(d.infotype_info).eq("rating").k()
                    .@"and"(d.data_data.gt("6.5").k())
            ).k())
            .@"and"(d.movie_keyword.o(d.keyword_keyword).in_v(sets.murder4).k())
            .@"and"(d.movie_kind.o(d.kind_kind).in_v(&[_][]const u8{ "movie", "episode" }).k())
            .@"and"(d.movie_production_year.gt(2005).k())
            .o(
                d.movie_company.in_s(
                    d.company_country.ne("[us]").k()
                        .@"and"(d.company_note.nrx(rx.paren_USA).k())
                        .@"and"(d.company_note.rx(rx.paren_200_dot).k())
                ).o(d.company_name)
                    .x(d.movie_data.in_s(
                        d.data_type.o(d.infotype_info).eq("rating").k()
                            .@"and"(d.data_data.gt("6.5").k())
                    ).o(d.data_data))
                    .x(d.movie_title)
            )
    );
    var acc = h.Acc3{};
    q.drive(h.Sink(h.Acc3){ .acc = &acc });
    try h.fmt3(w, acc.m0, acc.m1, acc.m2);
}

fn q28c(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_complete_cast.in_s(
            d.completecast_subject.o(d.compcasttype_kind).eq("cast").k()
                .@"and"(d.completecast_status.o(d.compcasttype_kind).eq("complete").k())
        ).k()
            .@"and"(d.movie_company.in_s(
                d.company_country.ne("[us]").k()
                    .@"and"(d.company_note.nrx(rx.paren_USA).k())
                    .@"and"(d.company_note.rx(rx.paren_200_dot).k())
            ).k())
            .@"and"(d.movie_info.in_s(
                d.info_type.o(d.infotype_info).eq("countries").k()
                    .@"and"(d.info_info.in_v(sets.nordic10).k())
            ).k())
            .@"and"(d.movie_data.in_s(
                d.data_type.o(d.infotype_info).eq("rating").k()
                    .@"and"(d.data_data.lt("8.5").k())
            ).k())
            .@"and"(d.movie_keyword.o(d.keyword_keyword).in_v(sets.murder4).k())
            .@"and"(d.movie_kind.o(d.kind_kind).in_v(&[_][]const u8{ "movie", "episode" }).k())
            .@"and"(d.movie_production_year.gt(2005).k())
            .o(
                d.movie_company.in_s(
                    d.company_country.ne("[us]").k()
                        .@"and"(d.company_note.nrx(rx.paren_USA).k())
                        .@"and"(d.company_note.rx(rx.paren_200_dot).k())
                ).o(d.company_name)
                    .x(d.movie_data.in_s(
                        d.data_type.o(d.infotype_info).eq("rating").k()
                            .@"and"(d.data_data.lt("8.5").k())
                    ).o(d.data_data))
                    .x(d.movie_title)
            )
    );
    var acc = h.Acc3{};
    q.drive(h.Sink(h.Acc3){ .acc = &acc });
    try h.fmt3(w, acc.m0, acc.m1, acc.m2);
}

fn q29a(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_complete_cast.in_s(
            d.completecast_subject.o(d.compcasttype_kind).eq("cast").k()
                .@"and"(d.completecast_status.o(d.compcasttype_kind).eq("complete+verified").k())
        ).k()
            .@"and"(d.movie_company.o(d.company_country.eq("[us]")).k())
            .@"and"(d.movie_info.in_s(
                d.info_type.o(d.infotype_info).eq("release dates").k()
                    .@"and"(
                        d.info_info.rx(rx.japan_dot_200).k()
                            .@"or"(d.info_info.rx(rx.usa_dot_200).k())
                    )
            ).k())
            .@"and"(d.movie_keyword.o(d.keyword_keyword).eq("computer-animation").k())
            .@"and"(d.movie_title.eq("Shrek 2").k())
            .@"and"(d.movie_production_year.ge(2000).k())
            .@"and"(d.movie_production_year.le(2010).k())
            .o(
                d.movie_cast.o(
                    d.cast_note.in_v(sets.voice3).k()
                        .@"and"(d.cast_role.o(d.roletype_role).eq("actress").k())
                        .@"and"(d.cast_character.o(d.character_name.eq("Queen")).k())
                        .@"and"(d.cast_person.in_s(
                            d.person_gender.eq("f").k()
                                .@"and"(d.person_name.rx(rx.an_uc).k())
                                .@"and"(d.person_aka.k())
                                .@"and"(d.person_info.in_s(d.personinfo_type.o(d.infotype_info).eq("trivia").k()).k())
                        ).k())
                        .o(
                            d.cast_character.o(d.character_name)
                                .x(d.cast_person.o(d.person_name))
                        )
                )
                .x(d.movie_title)
            )
    );
    var acc = h.Acc3{};
    q.drive(h.Sink(h.Acc3){ .acc = &acc });
    try h.fmt3(w, acc.m0, acc.m1, acc.m2);
}

fn q29b(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_complete_cast.in_s(
            d.completecast_subject.o(d.compcasttype_kind).eq("cast").k()
                .@"and"(d.completecast_status.o(d.compcasttype_kind).eq("complete+verified").k())
        ).k()
            .@"and"(d.movie_company.o(d.company_country.eq("[us]")).k())
            .@"and"(d.movie_info.in_s(
                d.info_type.o(d.infotype_info).eq("release dates").k()
                    .@"and"(d.info_info.rx(rx.usa_dot_200).k())
            ).k())
            .@"and"(d.movie_keyword.o(d.keyword_keyword).eq("computer-animation").k())
            .@"and"(d.movie_title.eq("Shrek 2").k())
            .@"and"(d.movie_production_year.ge(2000).k())
            .@"and"(d.movie_production_year.le(2005).k())
            .o(
                d.movie_cast.o(
                    d.cast_note.in_v(sets.voice3).k()
                        .@"and"(d.cast_role.o(d.roletype_role).eq("actress").k())
                        .@"and"(d.cast_character.o(d.character_name.eq("Queen")).k())
                        .@"and"(d.cast_person.in_s(
                            d.person_gender.eq("f").k()
                                .@"and"(d.person_name.rx(rx.an_uc).k())
                                .@"and"(d.person_aka.k())
                                .@"and"(d.person_info.in_s(d.personinfo_type.o(d.infotype_info).eq("height").k()).k())
                        ).k())
                        .o(
                            d.cast_character.o(d.character_name)
                                .x(d.cast_person.o(d.person_name))
                        )
                )
                .x(d.movie_title)
            )
    );
    var acc = h.Acc3{};
    q.drive(h.Sink(h.Acc3){ .acc = &acc });
    try h.fmt3(w, acc.m0, acc.m1, acc.m2);
}

fn q29c(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_complete_cast.in_s(
            d.completecast_subject.o(d.compcasttype_kind).eq("cast").k()
                .@"and"(d.completecast_status.o(d.compcasttype_kind).eq("complete+verified").k())
        ).k()
            .@"and"(d.movie_company.o(d.company_country.eq("[us]")).k())
            .@"and"(d.movie_info.in_s(
                d.info_type.o(d.infotype_info).eq("release dates").k()
                    .@"and"(
                        d.info_info.rx(rx.japan_dot_200).k()
                            .@"or"(d.info_info.rx(rx.usa_dot_200).k())
                    )
            ).k())
            .@"and"(d.movie_keyword.o(d.keyword_keyword).eq("computer-animation").k())
            .@"and"(d.movie_production_year.ge(2000).k())
            .@"and"(d.movie_production_year.le(2010).k())
            .o(
                d.movie_cast.o(
                    d.cast_note.in_v(sets.voice4).k()
                        .@"and"(d.cast_role.o(d.roletype_role).eq("actress").k())
                        .@"and"(d.cast_person.in_s(
                            d.person_gender.eq("f").k()
                                .@"and"(d.person_name.rx(rx.an_uc).k())
                                .@"and"(d.person_aka.k())
                                .@"and"(d.person_info.in_s(d.personinfo_type.o(d.infotype_info).eq("trivia").k()).k())
                        ).k())
                        .o(
                            d.cast_character.o(d.character_name)
                                .x(d.cast_person.o(d.person_name))
                        )
                )
                .x(d.movie_title)
            )
    );
    var acc = h.Acc3{};
    q.drive(h.Sink(h.Acc3){ .acc = &acc });
    try h.fmt3(w, acc.m0, acc.m1, acc.m2);
}

fn q30a(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_complete_cast.in_s(
            d.completecast_subject.o(d.compcasttype_kind).in_v(&[_][]const u8{ "cast", "crew" }).k()
                .@"and"(d.completecast_status.o(d.compcasttype_kind).eq("complete+verified").k())
        ).k()
            .@"and"(d.movie_info.in_s(
                d.info_type.o(d.infotype_info).eq("genres").k()
                    .@"and"(d.info_info.in_v(&[_][]const u8{ "Horror", "Thriller" }).k())
            ).k())
            .@"and"(d.movie_keyword.o(d.keyword_keyword).in_v(sets.kw7).k())
            .@"and"(d.movie_production_year.gt(2000).k())
            .o(
                d.movie_info.o(
                    d.info_type.o(d.infotype_info).eq("genres").k()
                        .@"and"(d.info_info.in_v(&[_][]const u8{ "Horror", "Thriller" }).k())
                        .o(d.info_info)
                )
                    .x(d.movie_data.o(d.data_type.o(d.infotype_info).eq("votes").k().o(d.data_data)))
                    .x(d.movie_title)
                    .x(d.movie_cast.o(
                        d.cast_note.in_v(sets.writer5).k()
                            .@"and"(d.cast_person.o(d.person_gender.eq("m")).k())
                            .o(d.cast_person.o(d.person_name))
                    ))
            )
    );
    var acc = h.Acc4{};
    q.drive(h.Sink(h.Acc4){ .acc = &acc });
    try h.fmt4(w, acc.m0, acc.m1, acc.m2, acc.m3);
}

fn q30b(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_complete_cast.in_s(
            d.completecast_subject.o(d.compcasttype_kind).in_v(&[_][]const u8{ "cast", "crew" }).k()
                .@"and"(d.completecast_status.o(d.compcasttype_kind).eq("complete+verified").k())
        ).k()
            .@"and"(d.movie_info.in_s(
                d.info_type.o(d.infotype_info).eq("genres").k()
                    .@"and"(d.info_info.in_v(&[_][]const u8{ "Horror", "Thriller" }).k())
            ).k())
            .@"and"(d.movie_keyword.o(d.keyword_keyword).in_v(sets.kw7).k())
            .@"and"(d.movie_production_year.gt(2000).k())
            .@"and"(
                d.movie_title.rx(rx.freddy).k()
                    .@"or"(
                        d.movie_title.rx(rx.jason).k()
                            .@"or"(d.movie_title.rx(rx.pre_saw).k())
                    )
            )
            .o(
                d.movie_info.o(
                    d.info_type.o(d.infotype_info).eq("genres").k()
                        .@"and"(d.info_info.in_v(&[_][]const u8{ "Horror", "Thriller" }).k())
                        .o(d.info_info)
                )
                    .x(d.movie_data.o(d.data_type.o(d.infotype_info).eq("votes").k().o(d.data_data)))
                    .x(d.movie_title)
                    .x(d.movie_cast.o(
                        d.cast_note.in_v(sets.writer5).k()
                            .@"and"(d.cast_person.o(d.person_gender.eq("m")).k())
                            .o(d.cast_person.o(d.person_name))
                    ))
            )
    );
    var acc = h.Acc4{};
    q.drive(h.Sink(h.Acc4){ .acc = &acc });
    try h.fmt4(w, acc.m0, acc.m1, acc.m2, acc.m3);
}

fn q30c(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_complete_cast.in_s(
            d.completecast_subject.o(d.compcasttype_kind).eq("cast").k()
                .@"and"(d.completecast_status.o(d.compcasttype_kind).eq("complete+verified").k())
        ).k()
            .@"and"(d.movie_info.in_s(
                d.info_type.o(d.infotype_info).eq("genres").k()
                    .@"and"(d.info_info.in_v(sets.genre6).k())
            ).k())
            .@"and"(d.movie_keyword.o(d.keyword_keyword).in_v(sets.kw7).k())
            .o(
                d.movie_info.o(
                    d.info_type.o(d.infotype_info).eq("genres").k()
                        .@"and"(d.info_info.in_v(sets.genre6).k())
                        .o(d.info_info)
                )
                    .x(d.movie_data.o(d.data_type.o(d.infotype_info).eq("votes").k().o(d.data_data)))
                    .x(d.movie_title)
                    .x(d.movie_cast.o(
                        d.cast_note.in_v(sets.writer5).k()
                            .@"and"(d.cast_person.o(d.person_gender.eq("m")).k())
                            .o(d.cast_person.o(d.person_name))
                    ))
            )
    );
    var acc = h.Acc4{};
    q.drive(h.Sink(h.Acc4){ .acc = &acc });
    try h.fmt4(w, acc.m0, acc.m1, acc.m2, acc.m3);
}

fn q31a(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_company.o(d.company_name.rx(rx.pre_lionsgate)).k()
            .@"and"(d.movie_info.in_s(
                d.info_type.o(d.infotype_info).eq("genres").k()
                    .@"and"(d.info_info.in_v(&[_][]const u8{ "Horror", "Thriller" }).k())
            ).k())
            .@"and"(d.movie_keyword.o(d.keyword_keyword).in_v(sets.kw7).k())
            .o(
                d.movie_info.o(
                    d.info_type.o(d.infotype_info).eq("genres").k()
                        .@"and"(d.info_info.in_v(&[_][]const u8{ "Horror", "Thriller" }).k())
                        .o(d.info_info)
                )
                    .x(d.movie_data.o(d.data_type.o(d.infotype_info).eq("votes").k().o(d.data_data)))
                    .x(d.movie_title)
                    .x(d.movie_cast.o(
                        d.cast_note.in_v(sets.writer5).k()
                            .@"and"(d.cast_person.o(d.person_gender.eq("m")).k())
                            .o(d.cast_person.o(d.person_name))
                    ))
            )
    );
    var acc = h.Acc4{};
    q.drive(h.Sink(h.Acc4){ .acc = &acc });
    try h.fmt4(w, acc.m0, acc.m1, acc.m2, acc.m3);
}

fn q31b(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_company.in_s(
            d.company_name.rx(rx.pre_lionsgate).k()
                .@"and"(d.company_note.rx(rx.paren_blu_ray).k())
        ).k()
            .@"and"(d.movie_info.in_s(
                d.info_type.o(d.infotype_info).eq("genres").k()
                    .@"and"(d.info_info.in_v(&[_][]const u8{ "Horror", "Thriller" }).k())
            ).k())
            .@"and"(d.movie_keyword.o(d.keyword_keyword).in_v(sets.kw7).k())
            .@"and"(d.movie_production_year.gt(2000).k())
            .@"and"(
                d.movie_title.rx(rx.freddy).k()
                    .@"or"(
                        d.movie_title.rx(rx.jason).k()
                            .@"or"(d.movie_title.rx(rx.pre_saw).k())
                    )
            )
            .o(
                d.movie_info.o(
                    d.info_type.o(d.infotype_info).eq("genres").k()
                        .@"and"(d.info_info.in_v(&[_][]const u8{ "Horror", "Thriller" }).k())
                        .o(d.info_info)
                )
                    .x(d.movie_data.o(d.data_type.o(d.infotype_info).eq("votes").k().o(d.data_data)))
                    .x(d.movie_title)
                    .x(d.movie_cast.o(
                        d.cast_note.in_v(sets.writer5).k()
                            .@"and"(d.cast_person.o(d.person_gender.eq("m")).k())
                            .o(d.cast_person.o(d.person_name))
                    ))
            )
    );
    var acc = h.Acc4{};
    q.drive(h.Sink(h.Acc4){ .acc = &acc });
    try h.fmt4(w, acc.m0, acc.m1, acc.m2, acc.m3);
}

fn q31c(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_company.o(d.company_name.rx(rx.pre_lionsgate)).k()
            .@"and"(d.movie_info.in_s(
                d.info_type.o(d.infotype_info).eq("genres").k()
                    .@"and"(d.info_info.in_v(sets.genre6).k())
            ).k())
            .@"and"(d.movie_keyword.o(d.keyword_keyword).in_v(sets.kw7).k())
            .o(
                d.movie_info.o(
                    d.info_type.o(d.infotype_info).eq("genres").k()
                        .@"and"(d.info_info.in_v(sets.genre6).k())
                        .o(d.info_info)
                )
                    .x(d.movie_data.o(d.data_type.o(d.infotype_info).eq("votes").k().o(d.data_data)))
                    .x(d.movie_title)
                    .x(d.movie_cast.o(
                        d.cast_note.in_v(sets.writer5).k()
                            .o(d.cast_person.o(d.person_name))
                    ))
            )
    );
    var acc = h.Acc4{};
    q.drive(h.Sink(h.Acc4){ .acc = &acc });
    try h.fmt4(w, acc.m0, acc.m1, acc.m2, acc.m3);
}

fn q32a(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_keyword.o(d.keyword_keyword).eq("10,000-mile-club").k()
            .@"and"(d.movie_link.k())
            .o(
                d.movie_link.o(d.movielink_type.o(d.linktype_link))
                    .x(d.movie_title)
                    .x(d.movie_link.o(d.movielink_target.o(d.movie_title)))
            )
    );
    var acc = h.Acc3{};
    q.drive(h.Sink(h.Acc3){ .acc = &acc });
    try h.fmt3(w, acc.m0, acc.m1, acc.m2);
}

fn q32b(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_keyword.o(d.keyword_keyword).eq("character-name-in-title").k()
            .@"and"(d.movie_link.k())
            .o(
                d.movie_link.o(d.movielink_type.o(d.linktype_link))
                    .x(d.movie_title)
                    .x(d.movie_link.o(d.movielink_target.o(d.movie_title)))
            )
    );
    var acc = h.Acc3{};
    q.drive(h.Sink(h.Acc3){ .acc = &acc });
    try h.fmt3(w, acc.m0, acc.m1, acc.m2);
}

fn q33a(d: *const Data, w: *Io.Writer) anyerror!void {
    const co = d.movie_company.o(d.company_country.eq("[us]").k().o(d.company_name));
    const rd = d.movie_data.o(d.data_type.o(d.infotype_info).eq("rating").k().o(d.data_data));
    const rdlt = d.movie_data.o(
        d.data_type.o(d.infotype_info).eq("rating").k()
            .@"and"(d.data_data.lt("3.0").k())
            .o(d.data_data),
    );
    const t2f = d.movie_kind.o(d.kind_kind).eq("tv series").k()
        .@"and"(d.movie_company.k())
        .@"and"(d.movie_data.in_s(
            d.data_type.o(d.infotype_info).eq("rating").k()
                .@"and"(d.data_data.lt("3.0").k())
        ).k())
        .@"and"(d.movie_production_year.ge(2005).k())
        .@"and"(d.movie_production_year.le(2008).k());
    const qlk = d.movie_link.in_s(
        d.movielink_type.o(d.linktype_link).in_v(sets.link3).k()
            .@"and"(d.movielink_target.in_s(t2f).k()),
    );
    const t2 = qlk.o(d.movielink_target);
    const q = d.movie.o(
        d.movie_kind.o(d.kind_kind).eq("tv series").k()
            .@"and"(d.movie_company.o(d.company_country.eq("[us]")).k())
            .@"and"(qlk.k())
            .o(
                co.x(t2.o(d.movie_company.o(d.company_name)))
                  .x(rd).x(t2.o(rdlt))
                  .x(d.movie_title).x(t2.o(d.movie_title)),
            )
    );
    var acc = h.Acc6{};
    q.drive(h.Sink(h.Acc6){ .acc = &acc });
    try h.fmt6(w, acc.m0, acc.m1, acc.m2, acc.m3, acc.m4, acc.m5);
}

fn q33b(d: *const Data, w: *Io.Writer) anyerror!void {
    const co = d.movie_company.o(d.company_country.eq("[nl]").k().o(d.company_name));
    const rd = d.movie_data.o(d.data_type.o(d.infotype_info).eq("rating").k().o(d.data_data));
    const rdlt = d.movie_data.o(
        d.data_type.o(d.infotype_info).eq("rating").k()
            .@"and"(d.data_data.lt("3.0").k())
            .o(d.data_data),
    );
    const t2f = d.movie_kind.o(d.kind_kind).eq("tv series").k()
        .@"and"(d.movie_company.k())
        .@"and"(d.movie_data.in_s(
            d.data_type.o(d.infotype_info).eq("rating").k()
                .@"and"(d.data_data.lt("3.0").k())
        ).k())
        .@"and"(d.movie_production_year.eq(2007).k());
    const qlk = d.movie_link.in_s(
        d.movielink_type.o(d.linktype_link).rx(rx.follow).k()
            .@"and"(d.movielink_target.in_s(t2f).k()),
    );
    const t2 = qlk.o(d.movielink_target);
    const q = d.movie.o(
        d.movie_kind.o(d.kind_kind).eq("tv series").k()
            .@"and"(d.movie_company.o(d.company_country.eq("[nl]")).k())
            .@"and"(qlk.k())
            .o(
                co.x(t2.o(d.movie_company.o(d.company_name)))
                  .x(rd).x(t2.o(rdlt))
                  .x(d.movie_title).x(t2.o(d.movie_title)),
            )
    );
    var acc = h.Acc6{};
    q.drive(h.Sink(h.Acc6){ .acc = &acc });
    try h.fmt6(w, acc.m0, acc.m1, acc.m2, acc.m3, acc.m4, acc.m5);
}

fn q33c(d: *const Data, w: *Io.Writer) anyerror!void {
    const tv_or_ep = &[_][]const u8{ "tv series", "episode" };
    const co = d.movie_company.o(d.company_country.ne("[us]").k().o(d.company_name));
    const rd = d.movie_data.o(d.data_type.o(d.infotype_info).eq("rating").k().o(d.data_data));
    const rdlt = d.movie_data.o(
        d.data_type.o(d.infotype_info).eq("rating").k()
            .@"and"(d.data_data.lt("3.5").k())
            .o(d.data_data),
    );
    const t2f = d.movie_kind.o(d.kind_kind).in_v(tv_or_ep).k()
        .@"and"(d.movie_company.k())
        .@"and"(d.movie_data.in_s(
            d.data_type.o(d.infotype_info).eq("rating").k()
                .@"and"(d.data_data.lt("3.5").k())
        ).k())
        .@"and"(d.movie_production_year.ge(2000).k())
        .@"and"(d.movie_production_year.le(2010).k());
    const qlk = d.movie_link.in_s(
        d.movielink_type.o(d.linktype_link).in_v(sets.link3).k()
            .@"and"(d.movielink_target.in_s(t2f).k()),
    );
    const t2 = qlk.o(d.movielink_target);
    const q = d.movie.o(
        d.movie_kind.o(d.kind_kind).in_v(tv_or_ep).k()
            .@"and"(d.movie_company.o(d.company_country.ne("[us]")).k())
            .@"and"(qlk.k())
            .o(
                co.x(t2.o(d.movie_company.o(d.company_name)))
                  .x(rd).x(t2.o(rdlt))
                  .x(d.movie_title).x(t2.o(d.movie_title)),
            )
    );
    var acc = h.Acc6{};
    q.drive(h.Sink(h.Acc6){ .acc = &acc });
    try h.fmt6(w, acc.m0, acc.m1, acc.m2, acc.m3, acc.m4, acc.m5);
}
