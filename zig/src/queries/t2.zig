// queries: queries.jl lines ~381-588 (22b, 22c, 22d, 5b, 5c, 15a-d, 11c-d, 13d, 6a-f)
const std = @import("std");
const Io = std.Io;
const Data = @import("../data.zig").Data;
const h = @import("../helpers.zig");
const rx = @import("../regex.zig");
const sets = @import("../sets.zig");
const Entry = @import("all.zig").Entry;

pub const ENTRIES: []const Entry = &.{
    .{ .name = "22d", .oracle = "(#1.1) || 2.0 || 13 Productions", .run = q22d },
    .{ .name = "5b",  .oracle = "(empty)", .run = q5b },
    .{ .name = "5c",  .oracle = "11,830,420", .run = q5c },
    .{ .name = "15a", .oracle = "USA:1 June 2007 || Battlestar Galactica: The Resistance", .run = q15a },
    .{ .name = "15b", .oracle = "USA:27 April 2007 || RoboCop vs Terminator", .run = q15b },
    .{ .name = "15c", .oracle = "USA:1 April 2003 || 24: Day Six - Debrief", .run = q15c },
    .{ .name = "15d", .oracle = "(Not So) Instant Photo || 06/05", .run = q15d },
    .{ .name = "11c", .oracle = "20th Century Fox Home Entertainment || (1997-2002) (worldwide) (all media) || 24", .run = q11c },
    .{ .name = "11d", .oracle = "13th Street || (1954) (UK) (TV) || ...denn sie wissen nicht, was sie tun", .run = q11d },
    .{ .name = "13d", .oracle = "\"O\" Films || 1.0 || #54 Meets #47", .run = q13d },
    .{ .name = "6a",  .oracle = "marvel-cinematic-universe || Iron Man 3 || Downey Jr., Robert", .run = q6a },
    .{ .name = "6b",  .oracle = "based-on-comic || The Avengers 2 || Downey Jr., Robert", .run = q6b },
    .{ .name = "6c",  .oracle = "marvel-cinematic-universe || The Avengers 2 || Downey Jr., Robert", .run = q6c },
    .{ .name = "6d",  .oracle = "based-on-comic || 2008 MTV Movie Awards || Downey Jr., Robert", .run = q6d },
    .{ .name = "6e",  .oracle = "marvel-cinematic-universe || Iron Man 3 || Downey Jr., Robert", .run = q6e },
    .{ .name = "6f",  .oracle = "based-on-comic || & Teller 2 || \"Steff\", Stefanie Oxmann Mcgaha", .run = q6f },
};

fn q22d(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_info.in_s(
            d.info_type.o(d.infotype_info).eq("countries").k()
                .@"and"(d.info_info.in_v(sets.nordic10).k())
        ).k()
            .@"and"(
                d.movie_keyword.o(d.keyword_keyword).in_v(sets.murder4).k()
                    .@"and"(
                        d.movie_production_year.gt(2005).k()
                            .@"and"(d.movie_kind.o(d.kind_kind).in_v(&[_][]const u8{"movie","episode"}).k())
                    )
            )
            .o(
                d.movie_title
                    .x(
                        d.movie_data.o(
                            d.data_data.lt("8.5").k()
                                .@"and"(d.data_type.o(d.infotype_info).eq("rating").k())
                                .o(d.data_data)
                        )
                    )
                    .x(
                        d.movie_company.o(
                            d.company_country.ne("[us]").k()
                                .@"and"(d.company_type.o(d.companytype_kind).eq("production companies").k())
                                .o(d.company_name)
                        )
                    )
            )
    );
    var acc = h.Acc3{};
    q.drive(h.Sink(h.Acc3){ .acc = &acc });
    try h.fmt3(w, acc.m0, acc.m1, acc.m2);
}

fn q5b(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_company.in_s(
            d.company_type.o(d.companytype_kind).eq("production companies").k()
                .@"and"(
                    d.company_note.rx(rx.paren_VHS).k()
                        .@"and"(
                            d.company_note.rx(rx.paren_USA).k()
                                .@"and"(d.company_note.rx(rx.paren_1994).k())
                        )
                )
        ).k()
            .@"and"(
                d.movie_info.o(d.info_info.in_v(&[_][]const u8{"USA","America"})).k()
                    .@"and"(d.movie_production_year.gt(2010).k())
            )
            .o(d.movie_title)
    );
    var acc = h.Acc1{};
    q.drive(h.Sink(h.Acc1){ .acc = &acc });
    try h.fmt1(w, acc.m);
}

fn q5c(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_company.in_s(
            d.company_type.o(d.companytype_kind).eq("production companies").k()
                .@"and"(
                    d.company_note.nrx(rx.paren_TV).k()
                        .@"and"(d.company_note.rx(rx.paren_USA).k())
                )
        ).k()
            .@"and"(
                d.movie_info.o(d.info_info.in_v(sets.nordic10)).k()
                    .@"and"(d.movie_production_year.gt(1990).k())
            )
            .o(d.movie_title)
    );
    var acc = h.Acc1{};
    q.drive(h.Sink(h.Acc1){ .acc = &acc });
    try h.fmt1(w, acc.m);
}

fn q15a(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_production_year.gt(2000).k()
            .@"and"(
                d.movie_company.in_s(
                    d.company_country.eq("[us]").k()
                        .@"and"(
                            d.company_note.rx(rx.paren_200_dot).k()
                                .@"and"(d.company_note.rx(rx.paren_worldwide).k())
                        )
                ).k()
                    .@"and"(
                        d.movie_keyword.k()
                            .@"and"(d.movie_aka.k())
                    )
            )
            .o(
                d.movie_info.o(
                    d.info_type.o(d.infotype_info).eq("release dates").k()
                        .@"and"(
                            d.info_info.rx(rx.usa_dot_space_200).k()
                                .@"and"(d.info_note.rx(rx.internet).k())
                        )
                        .o(d.info_info)
                )
                .x(d.movie_title)
            )
    );
    var acc = h.Acc2{};
    q.drive(h.Sink(h.Acc2){ .acc = &acc });
    try h.fmt2(w, acc.m0, acc.m1);
}

fn q15b(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_company.in_s(
            d.company_country.eq("[us]").k()
                .@"and"(
                    d.company_name.eq("YouTube").k()
                        .@"and"(
                            d.company_note.rx(rx.paren_200_dot).k()
                                .@"and"(d.company_note.rx(rx.paren_worldwide).k())
                        )
                )
        ).k()
            .@"and"(
                d.movie_keyword.k()
                    .@"and"(
                        d.movie_aka.k()
                            .@"and"(
                                d.movie_production_year.ge(2005).k()
                                    .@"and"(d.movie_production_year.le(2010).k())
                            )
                    )
            )
            .o(
                d.movie_info.o(
                    d.info_type.o(d.infotype_info).eq("release dates").k()
                        .@"and"(
                            d.info_info.rx(rx.usa_dot_space_200).k()
                                .@"and"(d.info_note.rx(rx.internet).k())
                        )
                        .o(d.info_info)
                )
                .x(d.movie_title)
            )
    );
    var acc = h.Acc2{};
    q.drive(h.Sink(h.Acc2){ .acc = &acc });
    try h.fmt2(w, acc.m0, acc.m1);
}

fn q15c(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_company.o(d.company_country.eq("[us]")).k()
            .@"and"(
                d.movie_keyword.k()
                    .@"and"(
                        d.movie_aka.k()
                            .@"and"(d.movie_production_year.gt(1990).k())
                    )
            )
            .o(
                d.movie_info.o(
                    d.info_type.o(d.infotype_info).eq("release dates").k()
                        .@"and"(
                            d.info_info.rx(rx.usa_dot_space_199).k()
                                .@"or"(d.info_info.rx(rx.usa_dot_space_200).k())
                                .@"and"(d.info_note.rx(rx.internet).k())
                        )
                        .o(d.info_info)
                )
                .x(d.movie_title)
            )
    );
    var acc = h.Acc2{};
    q.drive(h.Sink(h.Acc2){ .acc = &acc });
    try h.fmt2(w, acc.m0, acc.m1);
}

fn q15d(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_company.o(d.company_country.eq("[us]")).k()
            .@"and"(
                d.movie_keyword.k()
                    .@"and"(
                        d.movie_info.in_s(
                            d.info_type.o(d.infotype_info).eq("release dates").k()
                                .@"and"(d.info_note.rx(rx.internet).k())
                        ).k()
                            .@"and"(d.movie_production_year.gt(1990).k())
                    )
            )
            .o(
                d.movie_aka.o(d.akatitle_title)
                    .x(d.movie_title)
            )
    );
    var acc = h.Acc2{};
    q.drive(h.Sink(h.Acc2){ .acc = &acc });
    try h.fmt2(w, acc.m0, acc.m1);
}

fn q11c(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_keyword.o(d.keyword_keyword).in_v(&[_][]const u8{"sequel","revenge","based-on-novel"}).k()
            .@"and"(
                d.movie_production_year.gt(1950).k()
                    .@"and"(d.movie_link.k())
            )
            .o(
                d.movie_company.o(
                    d.company_country.ne("[pl]").k()
                        .@"and"(
                            d.company_name.rx(rx.pre_20cf).k()
                                .@"or"(d.company_name.rx(rx.pre_twentieth_cf).k())
                                .@"and"(
                                    d.company_type.o(d.companytype_kind).ne("production companies").k()
                                        .@"and"(d.company_note.k())
                                )
                        )
                        .o(d.company_name.x(d.company_note))
                )
                .x(d.movie_title)
            )
    );
    var acc = h.Acc3{};
    q.drive(h.Sink(h.Acc3){ .acc = &acc });
    try h.fmt3(w, acc.m0, acc.m1, acc.m2);
}

fn q11d(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_keyword.o(d.keyword_keyword).in_v(&[_][]const u8{"sequel","revenge","based-on-novel"}).k()
            .@"and"(
                d.movie_production_year.gt(1950).k()
                    .@"and"(d.movie_link.k())
            )
            .o(
                d.movie_company.o(
                    d.company_country.ne("[pl]").k()
                        .@"and"(
                            d.company_type.o(d.companytype_kind).ne("production companies").k()
                                .@"and"(d.company_note.k())
                        )
                        .o(d.company_name.x(d.company_note))
                )
                .x(d.movie_title)
            )
    );
    var acc = h.Acc3{};
    q.drive(h.Sink(h.Acc3){ .acc = &acc });
    try h.fmt3(w, acc.m0, acc.m1, acc.m2);
}

fn q13d(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_kind.o(d.kind_kind).eq("movie").k()
            .@"and"(
                d.movie_info.o(
                    d.info_type.o(d.infotype_info).eq("release dates")
                ).k()
            )
            .o(
                d.movie_company.o(
                    d.company_country.eq("[us]").k()
                        .@"and"(d.company_type.o(d.companytype_kind).eq("production companies").k())
                        .o(d.company_name)
                )
                .x(
                    d.movie_data.o(
                        d.data_type.o(d.infotype_info).eq("rating").k()
                            .o(d.data_data)
                    )
                )
                .x(d.movie_title)
            )
    );
    var acc = h.Acc3{};
    q.drive(h.Sink(h.Acc3){ .acc = &acc });
    try h.fmt3(w, acc.m0, acc.m1, acc.m2);
}

fn q6a(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_production_year.gt(2010).k()
            .@"and"(d.movie_keyword.o(d.keyword_keyword).eq("marvel-cinematic-universe").k())
            .o(
                d.movie_keyword.o(d.keyword_keyword).eq("marvel-cinematic-universe")
                    .x(d.movie_title)
                    .x(
                        d.movie_cast.o(
                            d.cast_person.o(d.person_name.rx(rx.downey_robert))
                        )
                    )
            )
    );
    var acc = h.Acc3{};
    q.drive(h.Sink(h.Acc3){ .acc = &acc });
    try h.fmt3(w, acc.m0, acc.m1, acc.m2);
}

fn q6b(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_production_year.gt(2014).k()
            .@"and"(d.movie_keyword.o(d.keyword_keyword).in_v(sets.kw8).k())
            .o(
                d.movie_keyword.o(d.keyword_keyword).in_v(sets.kw8)
                    .x(d.movie_title)
                    .x(
                        d.movie_cast.o(
                            d.cast_person.o(d.person_name.rx(rx.downey_robert))
                        )
                    )
            )
    );
    var acc = h.Acc3{};
    q.drive(h.Sink(h.Acc3){ .acc = &acc });
    try h.fmt3(w, acc.m0, acc.m1, acc.m2);
}

fn q6c(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_production_year.gt(2014).k()
            .@"and"(d.movie_keyword.o(d.keyword_keyword).eq("marvel-cinematic-universe").k())
            .o(
                d.movie_keyword.o(d.keyword_keyword).eq("marvel-cinematic-universe")
                    .x(d.movie_title)
                    .x(
                        d.movie_cast.o(
                            d.cast_person.o(d.person_name.rx(rx.downey_robert))
                        )
                    )
            )
    );
    var acc = h.Acc3{};
    q.drive(h.Sink(h.Acc3){ .acc = &acc });
    try h.fmt3(w, acc.m0, acc.m1, acc.m2);
}

fn q6d(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_production_year.gt(2000).k()
            .@"and"(d.movie_keyword.o(d.keyword_keyword).in_v(sets.kw8).k())
            .o(
                d.movie_keyword.o(d.keyword_keyword).in_v(sets.kw8)
                    .x(d.movie_title)
                    .x(
                        d.movie_cast.o(
                            d.cast_person.o(d.person_name.rx(rx.downey_robert))
                        )
                    )
            )
    );
    var acc = h.Acc3{};
    q.drive(h.Sink(h.Acc3){ .acc = &acc });
    try h.fmt3(w, acc.m0, acc.m1, acc.m2);
}

fn q6e(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_production_year.gt(2000).k()
            .@"and"(d.movie_keyword.o(d.keyword_keyword).eq("marvel-cinematic-universe").k())
            .o(
                d.movie_keyword.o(d.keyword_keyword).eq("marvel-cinematic-universe")
                    .x(d.movie_title)
                    .x(
                        d.movie_cast.o(
                            d.cast_person.o(d.person_name.rx(rx.downey_robert))
                        )
                    )
            )
    );
    var acc = h.Acc3{};
    q.drive(h.Sink(h.Acc3){ .acc = &acc });
    try h.fmt3(w, acc.m0, acc.m1, acc.m2);
}

fn q6f(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_production_year.gt(2000).k()
            .@"and"(d.movie_keyword.o(d.keyword_keyword).in_v(sets.kw8).k())
            .o(
                d.movie_keyword.o(d.keyword_keyword).in_v(sets.kw8)
                    .x(d.movie_title)
                    .x(
                        d.movie_cast.o(d.cast_person.o(d.person_name))
                    )
            )
    );
    var acc = h.Acc3{};
    q.drive(h.Sink(h.Acc3){ .acc = &acc });
    try h.fmt3(w, acc.m0, acc.m1, acc.m2);
}
