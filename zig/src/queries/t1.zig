// queries: queries.jl lines 107..413 (templates 1-5, 11-15, 22 — movie-only)

const std = @import("std");
const Io = std.Io;
const Data = @import("../data.zig").Data;
const h = @import("../helpers.zig");
const rx = @import("../regex.zig");
const sets = @import("../sets.zig");
const Entry = @import("all.zig").Entry;

pub const ENTRIES: []const Entry = &.{
    .{ .name = "2a",  .oracle = "'Doc'",                                                                    .run = q2a },
    .{ .name = "2d",  .oracle = "& Teller",                                                                 .run = q2d },
    .{ .name = "3b",  .oracle = "300: Rise of an Empire",                                                   .run = q3b },
    .{ .name = "4a",  .oracle = "5.1 || & Teller 2",                                                        .run = q4a },
    .{ .name = "13a", .oracle = "Afghanistan:24 June 2012 || 1.0 || &Me",                                   .run = q13a },
    .{ .name = "11a", .oracle = "Churchill Films || followed by || Batman Beyond",                          .run = q11a },
    .{ .name = "22a", .oracle = "(empty)",                                                                  .run = q22a },
    .{ .name = "1a",  .oracle = "(A Warner Bros.-First National Picture) (presents) || A Clockwork Orange || 1934", .run = q1a },
    .{ .name = "5a",  .oracle = "(empty)",                                                                  .run = q5a },
    .{ .name = "12a", .oracle = "10th Grade Reunion Films || 8.1 || 3:20",                                  .run = q12a },
    .{ .name = "14a", .oracle = "1.0 || $lowdown",                                                          .run = q14a },
    .{ .name = "1b",  .oracle = "(Set Decoration Rentals) (uncredited) || Disaster Movie || 2008",          .run = q1b },
    .{ .name = "2b",  .oracle = "'Doc'",                                                                    .run = q2b },
    .{ .name = "2c",  .oracle = "(empty)",                                                                  .run = q2c },
    .{ .name = "3a",  .oracle = "2 Days in New York",                                                       .run = q3a },
    .{ .name = "3c",  .oracle = "& Teller 2",                                                               .run = q3c },
    .{ .name = "4b",  .oracle = "9.1 || Batman: Arkham City",                                               .run = q4b },
    .{ .name = "11b", .oracle = "Filmlance International AB || follows || The Money Man",                   .run = q11b },
    .{ .name = "13b", .oracle = "501audio || 1.8 || 5 Time Champion",                                       .run = q13b },
    .{ .name = "1c",  .oracle = "(co-production) || Intouchables || 2011",                                  .run = q1c },
    .{ .name = "1d",  .oracle = "(Set Decoration Rentals) (uncredited) || Disaster Movie || 2004",          .run = q1d },
    .{ .name = "4c",  .oracle = "2.1 || & Teller 2",                                                        .run = q4c },
    .{ .name = "12b", .oracle = "$10,000 || Birdemic: Shock and Terror",                                    .run = q12b },
    .{ .name = "12c", .oracle = "\"Oh That Gus!\" || 7.1 || $1.11",                                         .run = q12c },
    .{ .name = "13c", .oracle = "DL Sites || 1.8 || Champion",                                              .run = q13c },
    .{ .name = "14b", .oracle = "6.4 || Of Dolls and Murder",                                               .run = q14b },
    .{ .name = "14c", .oracle = "1.0 || $lowdown",                                                          .run = q14c },
    .{ .name = "22b", .oracle = "(empty)",                                                                  .run = q22b },
    .{ .name = "22c", .oracle = "(empty)",                                                                  .run = q22c },
};

fn q2a(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_keyword.o(d.keyword_keyword).eq("character-name-in-title").k()
            .@"and"(d.movie_company.o(d.company_country.eq("[de]")).k())
            .o(d.movie_title)
    );
    var acc = h.Acc1{};
    q.drive(h.Sink(h.Acc1){ .acc = &acc });
    try h.fmt1(w, acc.m);
}

fn q2d(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_keyword.o(d.keyword_keyword).eq("character-name-in-title").k()
            .@"and"(d.movie_company.o(d.company_country.eq("[us]")).k())
            .o(d.movie_title)
    );
    var acc = h.Acc1{};
    q.drive(h.Sink(h.Acc1){ .acc = &acc });
    try h.fmt1(w, acc.m);
}

fn q3b(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_keyword.o(d.keyword_keyword).rx(rx.sequel).k()
            .@"and"(
                d.movie_info.o(d.info_info.eq("Bulgaria")).k()
                    .@"and"(d.movie_production_year.gt(2010).k())
            )
            .o(d.movie_title)
    );
    var acc = h.Acc1{};
    q.drive(h.Sink(h.Acc1){ .acc = &acc });
    try h.fmt1(w, acc.m);
}

fn q4a(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_keyword.o(d.keyword_keyword).rx(rx.sequel).k()
            .@"and"(d.movie_production_year.gt(2005).k())
            .o(
                d.movie_data.o(
                    d.data_type.o(d.infotype_info).eq("rating").k()
                        .@"and"(d.data_data.gt("5.0").k())
                        .o(d.data_data)
                )
                .x(d.movie_title)
            )
    );
    var acc = h.Acc2{};
    q.drive(h.Sink(h.Acc2){ .acc = &acc });
    try h.fmt2(w, acc.m0, acc.m1);
}

fn q13a(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_company.in_s(
            d.company_country.eq("[de]").k()
                .@"and"(d.company_type.o(d.companytype_kind).eq("production companies").k())
        ).k()
            .@"and"(d.movie_kind.o(d.kind_kind).eq("movie").k())
            .o(
                d.movie_info.o(
                    d.info_type.o(d.infotype_info).eq("release dates").k()
                        .o(d.info_info)
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

fn q11a(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_keyword.o(d.keyword_keyword).eq("sequel").k()
            .@"and"(
                d.movie_production_year.ge(1950).k()
                    .@"and"(d.movie_production_year.le(2000).k())
            )
            .o(
                d.movie_company.o(
                    d.company_country.ne("[pl]").k()
                        .@"and"(
                            d.company_name.rx(rx.film).k()
                                .@"or"(d.company_name.rx(rx.warner).k())
                                .@"and"(d.company_type.o(d.companytype_kind).eq("production companies").k())
                        )
                        .minus(d.company_note.k())
                        .o(d.company_name)
                )
                .x(
                    d.movie_link.o(
                        d.movielink_type.o(d.linktype_link).rx(rx.follow).k()
                            .o(d.movielink_type.o(d.linktype_link))
                    )
                )
                .x(d.movie_title)
            )
    );
    var acc = h.Acc3{};
    q.drive(h.Sink(h.Acc3){ .acc = &acc });
    try h.fmt3(w, acc.m0, acc.m1, acc.m2);
}

fn q22a(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_info.in_s(
            d.info_type.o(d.infotype_info).eq("countries").k()
                .@"and"(d.info_info.in_v(&[_][]const u8{"Germany","German","USA","American"}).k())
        ).k()
            .@"and"(
                d.movie_keyword.o(d.keyword_keyword).in_v(sets.murder4).k()
                    .@"and"(
                        d.movie_production_year.gt(2008).k()
                            .@"and"(d.movie_kind.o(d.kind_kind).in_v(&[_][]const u8{"movie","episode"}).k())
                    )
            )
            .o(
                d.movie_title
                    .x(
                        d.movie_data.o(
                            d.data_data.lt("7.0").k()
                                .@"and"(d.data_type.o(d.infotype_info).eq("rating").k())
                                .o(d.data_data)
                        )
                    )
                    .x(
                        d.movie_company.o(
                            d.company_note.nrx(rx.paren_USA).k()
                                .@"and"(
                                    d.company_note.rx(rx.paren_200_dot).k()
                                        .@"and"(
                                            d.company_country.ne("[us]").k()
                                                .@"and"(d.company_type.o(d.companytype_kind).eq("production companies").k())
                                        )
                                )
                                .o(d.company_name)
                        )
                    )
            )
    );
    var acc = h.Acc3{};
    q.drive(h.Sink(h.Acc3){ .acc = &acc });
    try h.fmt3(w, acc.m0, acc.m1, acc.m2);
}

fn q1a(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_data.o(d.data_type.o(d.infotype_info).eq("top 250 rank")).k()
            .o(
                d.movie_company.o(
                    d.company_type.o(d.companytype_kind).eq("production companies").k()
                        .@"and"(
                            d.company_note.nrx(rx.paren_mgm).k()
                                .@"and"(
                                    d.company_note.rx(rx.paren_coprod).k()
                                        .@"or"(d.company_note.rx(rx.paren_presents).k())
                                )
                        )
                        .o(d.company_note)
                )
                .x(d.movie_title)
                .x(d.movie_production_year)
            )
    );
    const Acc = struct {
        m0: ?[]const u8 = null,
        m1: ?[]const u8 = null,
        my: ?i64 = null,
        pub inline fn call(self: *@This(), _: i64, t: anytype) void {
            const c0 = t.a.a; const c1 = t.a.b; const c2 = t.b;
            if (self.m0 == null or std.mem.order(u8, c0, self.m0.?) == .lt) self.m0 = c0;
            if (self.m1 == null or std.mem.order(u8, c1, self.m1.?) == .lt) self.m1 = c1;
            if (self.my == null or c2 < self.my.?) self.my = c2;
        }
    };
    var acc = Acc{};
    q.drive(h.Sink(Acc){ .acc = &acc });
    if (acc.m0 == null) { try w.print("(empty)", .{}); return; }
    try w.print("{s} || {s} || {d}", .{ acc.m0.?, acc.m1.?, acc.my.? });
}

fn q5a(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_company.in_s(
            d.company_type.o(d.companytype_kind).eq("production companies").k()
                .@"and"(
                    d.company_note.rx(rx.paren_theatrical).k()
                        .@"and"(d.company_note.rx(rx.paren_france).k())
                )
        ).k()
            .@"and"(
                d.movie_info.o(d.info_info.in_v(sets.nordic8)).k()
                    .@"and"(d.movie_production_year.gt(2005).k())
            )
            .o(d.movie_title)
    );
    var acc = h.Acc1{};
    q.drive(h.Sink(h.Acc1){ .acc = &acc });
    try h.fmt1(w, acc.m);
}

fn q12a(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_info.in_s(
            d.info_type.o(d.infotype_info).eq("genres").k()
                .@"and"(d.info_info.in_v(&[_][]const u8{"Drama","Horror"}).k())
        ).k()
            .@"and"(
                d.movie_production_year.ge(2005).k()
                    .@"and"(d.movie_production_year.le(2008).k())
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
                            .@"and"(d.data_data.gt("8.0").k())
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

fn q14a(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_keyword.o(d.keyword_keyword).in_v(sets.murder4).k()
            .@"and"(
                d.movie_kind.o(d.kind_kind).eq("movie").k()
                    .@"and"(
                        d.movie_info.in_s(
                            d.info_type.o(d.infotype_info).eq("countries").k()
                                .@"and"(d.info_info.in_v(&[_][]const u8{"Sweden","Norway","Germany","Denmark","Swedish","Denish","Norwegian","German","USA","American"}).k())
                        ).k()
                            .@"and"(d.movie_production_year.gt(2010).k())
                    )
            )
            .o(
                d.movie_data.o(
                    d.data_type.o(d.infotype_info).eq("rating").k()
                        .@"and"(d.data_data.lt("8.5").k())
                        .o(d.data_data)
                )
                .x(d.movie_title)
            )
    );
    var acc = h.Acc2{};
    q.drive(h.Sink(h.Acc2){ .acc = &acc });
    try h.fmt2(w, acc.m0, acc.m1);
}

fn q1b(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_data.o(d.data_type.o(d.infotype_info).eq("bottom 10 rank")).k()
            .@"and"(
                d.movie_production_year.ge(2005).k()
                    .@"and"(d.movie_production_year.le(2010).k())
            )
            .o(
                d.movie_company.o(
                    d.company_type.o(d.companytype_kind).eq("production companies").k()
                        .@"and"(d.company_note.nrx(rx.paren_mgm).k())
                        .o(d.company_note)
                )
                .x(d.movie_title)
                .x(d.movie_production_year)
            )
    );
    const Acc = struct {
        m0: ?[]const u8 = null,
        m1: ?[]const u8 = null,
        my: ?i64 = null,
        pub inline fn call(self: *@This(), _: i64, t: anytype) void {
            const c0 = t.a.a; const c1 = t.a.b; const c2 = t.b;
            if (self.m0 == null or std.mem.order(u8, c0, self.m0.?) == .lt) self.m0 = c0;
            if (self.m1 == null or std.mem.order(u8, c1, self.m1.?) == .lt) self.m1 = c1;
            if (self.my == null or c2 < self.my.?) self.my = c2;
        }
    };
    var acc = Acc{};
    q.drive(h.Sink(Acc){ .acc = &acc });
    if (acc.m0 == null) { try w.print("(empty)", .{}); return; }
    try w.print("{s} || {s} || {d}", .{ acc.m0.?, acc.m1.?, acc.my.? });
}

fn q2b(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_keyword.o(d.keyword_keyword).eq("character-name-in-title").k()
            .@"and"(d.movie_company.o(d.company_country.eq("[nl]")).k())
            .o(d.movie_title)
    );
    var acc = h.Acc1{};
    q.drive(h.Sink(h.Acc1){ .acc = &acc });
    try h.fmt1(w, acc.m);
}

fn q2c(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_keyword.o(d.keyword_keyword).eq("character-name-in-title").k()
            .@"and"(d.movie_company.o(d.company_country.eq("[sm]")).k())
            .o(d.movie_title)
    );
    var acc = h.Acc1{};
    q.drive(h.Sink(h.Acc1){ .acc = &acc });
    try h.fmt1(w, acc.m);
}

fn q3a(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_keyword.o(d.keyword_keyword).rx(rx.sequel).k()
            .@"and"(
                d.movie_info.o(d.info_info.in_v(sets.nordic8)).k()
                    .@"and"(d.movie_production_year.gt(2005).k())
            )
            .o(d.movie_title)
    );
    var acc = h.Acc1{};
    q.drive(h.Sink(h.Acc1){ .acc = &acc });
    try h.fmt1(w, acc.m);
}

fn q3c(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_keyword.o(d.keyword_keyword).rx(rx.sequel).k()
            .@"and"(
                d.movie_info.o(d.info_info.in_v(&[_][]const u8{"Sweden","Norway","Germany","Denmark","Swedish","Denish","Norwegian","German","USA","American"})).k()
                    .@"and"(d.movie_production_year.gt(1990).k())
            )
            .o(d.movie_title)
    );
    var acc = h.Acc1{};
    q.drive(h.Sink(h.Acc1){ .acc = &acc });
    try h.fmt1(w, acc.m);
}

fn q4b(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_keyword.o(d.keyword_keyword).rx(rx.sequel).k()
            .@"and"(d.movie_production_year.gt(2010).k())
            .o(
                d.movie_data.o(
                    d.data_type.o(d.infotype_info).eq("rating").k()
                        .@"and"(d.data_data.gt("9.0").k())
                        .o(d.data_data)
                )
                .x(d.movie_title)
            )
    );
    var acc = h.Acc2{};
    q.drive(h.Sink(h.Acc2){ .acc = &acc });
    try h.fmt2(w, acc.m0, acc.m1);
}

fn q11b(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_keyword.o(d.keyword_keyword).eq("sequel").k()
            .@"and"(
                d.movie_production_year.eq(1998).k()
                    .@"and"(d.movie_title.rx(rx.money).k())
            )
            .o(
                d.movie_company.o(
                    d.company_country.ne("[pl]").k()
                        .@"and"(
                            d.company_name.rx(rx.film).k()
                                .@"or"(d.company_name.rx(rx.warner).k())
                                .@"and"(d.company_type.o(d.companytype_kind).eq("production companies").k())
                        )
                        .minus(d.company_note.k())
                        .o(d.company_name)
                )
                .x(
                    d.movie_link.o(
                        d.movielink_type.o(d.linktype_link).rx(rx.follows).k()
                            .o(d.movielink_type.o(d.linktype_link))
                    )
                )
                .x(d.movie_title)
            )
    );
    var acc = h.Acc3{};
    q.drive(h.Sink(h.Acc3){ .acc = &acc });
    try h.fmt3(w, acc.m0, acc.m1, acc.m2);
}

fn q13b(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_kind.o(d.kind_kind).eq("movie").k()
            .@"and"(
                d.movie_info.o(d.info_type.o(d.infotype_info).eq("release dates")).k()
                    .@"and"(
                        d.movie_title.ne("").k()
                            .@"and"(
                                d.movie_title.rx(rx.champion).k()
                                    .@"or"(d.movie_title.rx(rx.loser).k())
                            )
                    )
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

fn q1c(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_data.o(d.data_type.o(d.infotype_info).eq("top 250 rank")).k()
            .@"and"(d.movie_production_year.gt(2010).k())
            .o(
                d.movie_company.o(
                    d.company_type.o(d.companytype_kind).eq("production companies").k()
                        .@"and"(
                            d.company_note.nrx(rx.paren_mgm).k()
                                .@"and"(d.company_note.rx(rx.paren_coprod).k())
                        )
                        .o(d.company_note)
                )
                .x(d.movie_title)
                .x(d.movie_production_year)
            )
    );
    const Acc = struct {
        m0: ?[]const u8 = null,
        m1: ?[]const u8 = null,
        my: ?i64 = null,
        pub inline fn call(self: *@This(), _: i64, t: anytype) void {
            const c0 = t.a.a; const c1 = t.a.b; const c2 = t.b;
            if (self.m0 == null or std.mem.order(u8, c0, self.m0.?) == .lt) self.m0 = c0;
            if (self.m1 == null or std.mem.order(u8, c1, self.m1.?) == .lt) self.m1 = c1;
            if (self.my == null or c2 < self.my.?) self.my = c2;
        }
    };
    var acc = Acc{};
    q.drive(h.Sink(Acc){ .acc = &acc });
    if (acc.m0 == null) { try w.print("(empty)", .{}); return; }
    try w.print("{s} || {s} || {d}", .{ acc.m0.?, acc.m1.?, acc.my.? });
}

fn q1d(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_data.o(d.data_type.o(d.infotype_info).eq("bottom 10 rank")).k()
            .@"and"(d.movie_production_year.gt(2000).k())
            .o(
                d.movie_company.o(
                    d.company_type.o(d.companytype_kind).eq("production companies").k()
                        .@"and"(d.company_note.nrx(rx.paren_mgm).k())
                        .o(d.company_note)
                )
                .x(d.movie_title)
                .x(d.movie_production_year)
            )
    );
    const Acc = struct {
        m0: ?[]const u8 = null,
        m1: ?[]const u8 = null,
        my: ?i64 = null,
        pub inline fn call(self: *@This(), _: i64, t: anytype) void {
            const c0 = t.a.a; const c1 = t.a.b; const c2 = t.b;
            if (self.m0 == null or std.mem.order(u8, c0, self.m0.?) == .lt) self.m0 = c0;
            if (self.m1 == null or std.mem.order(u8, c1, self.m1.?) == .lt) self.m1 = c1;
            if (self.my == null or c2 < self.my.?) self.my = c2;
        }
    };
    var acc = Acc{};
    q.drive(h.Sink(Acc){ .acc = &acc });
    if (acc.m0 == null) { try w.print("(empty)", .{}); return; }
    try w.print("{s} || {s} || {d}", .{ acc.m0.?, acc.m1.?, acc.my.? });
}

fn q4c(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_keyword.o(d.keyword_keyword).rx(rx.sequel).k()
            .@"and"(d.movie_production_year.gt(1990).k())
            .o(
                d.movie_data.o(
                    d.data_type.o(d.infotype_info).eq("rating").k()
                        .@"and"(d.data_data.gt("2.0").k())
                        .o(d.data_data)
                )
                .x(d.movie_title)
            )
    );
    var acc = h.Acc2{};
    q.drive(h.Sink(h.Acc2){ .acc = &acc });
    try h.fmt2(w, acc.m0, acc.m1);
}

fn q12b(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_company.in_s(
            d.company_country.eq("[us]").k()
                .@"and"(d.company_type.o(d.companytype_kind).in_v(&[_][]const u8{"production companies","distributors"}).k())
        ).k()
            .@"and"(
                d.movie_data.o(d.data_type.o(d.infotype_info).eq("bottom 10 rank")).k()
                    .@"and"(
                        d.movie_production_year.gt(2000).k()
                            .@"and"(
                                d.movie_title.rx(rx.pre_birdemic).k()
                                    .@"or"(d.movie_title.rx(rx.movie).k())
                            )
                    )
            )
            .o(
                d.movie_info.o(
                    d.info_type.o(d.infotype_info).eq("budget").k()
                        .o(d.info_info)
                )
                .x(d.movie_title)
            )
    );
    var acc = h.Acc2{};
    q.drive(h.Sink(h.Acc2){ .acc = &acc });
    try h.fmt2(w, acc.m0, acc.m1);
}

fn q12c(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_info.in_s(
            d.info_type.o(d.infotype_info).eq("genres").k()
                .@"and"(d.info_info.in_v(&[_][]const u8{"Drama","Horror","Western","Family"}).k())
        ).k()
            .@"and"(
                d.movie_production_year.ge(2000).k()
                    .@"and"(d.movie_production_year.le(2010).k())
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
                            .@"and"(d.data_data.gt("7.0").k())
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

fn q13c(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_kind.o(d.kind_kind).eq("movie").k()
            .@"and"(
                d.movie_info.o(d.info_type.o(d.infotype_info).eq("release dates")).k()
                    .@"and"(
                        d.movie_title.ne("").k()
                            .@"and"(
                                d.movie_title.rx(rx.pre_champion).k()
                                    .@"or"(d.movie_title.rx(rx.pre_loser).k())
                            )
                    )
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

fn q14b(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_keyword.o(d.keyword_keyword).in_v(&[_][]const u8{"murder","murder-in-title"}).k()
            .@"and"(
                d.movie_kind.o(d.kind_kind).eq("movie").k()
                    .@"and"(
                        d.movie_info.in_s(
                            d.info_type.o(d.infotype_info).eq("countries").k()
                                .@"and"(d.info_info.in_v(&[_][]const u8{"Sweden","Norway","Germany","Denmark","Swedish","Denish","Norwegian","German","USA","American"}).k())
                        ).k()
                            .@"and"(
                                d.movie_production_year.gt(2010).k()
                                    .@"and"(
                                        d.movie_title.rx(rx.murder_lc).k()
                                            .@"or"(
                                                d.movie_title.rx(rx.murder_uc).k()
                                                    .@"or"(d.movie_title.rx(rx.mord).k())
                                            )
                                    )
                            )
                    )
            )
            .o(
                d.movie_data.o(
                    d.data_type.o(d.infotype_info).eq("rating").k()
                        .@"and"(d.data_data.gt("6.0").k())
                        .o(d.data_data)
                )
                .x(d.movie_title)
            )
    );
    var acc = h.Acc2{};
    q.drive(h.Sink(h.Acc2){ .acc = &acc });
    try h.fmt2(w, acc.m0, acc.m1);
}

fn q14c(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_keyword.o(d.keyword_keyword).in_v(sets.murder4).k()
            .@"and"(
                d.movie_kind.o(d.kind_kind).in_v(&[_][]const u8{"movie","episode"}).k()
                    .@"and"(
                        d.movie_info.in_s(
                            d.info_type.o(d.infotype_info).eq("countries").k()
                                .@"and"(d.info_info.in_v(sets.nordic10).k())
                        ).k()
                            .@"and"(d.movie_production_year.gt(2005).k())
                    )
            )
            .o(
                d.movie_data.o(
                    d.data_type.o(d.infotype_info).eq("rating").k()
                        .@"and"(d.data_data.lt("8.5").k())
                        .o(d.data_data)
                )
                .x(d.movie_title)
            )
    );
    var acc = h.Acc2{};
    q.drive(h.Sink(h.Acc2){ .acc = &acc });
    try h.fmt2(w, acc.m0, acc.m1);
}

fn q22b(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_info.in_s(
            d.info_type.o(d.infotype_info).eq("countries").k()
                .@"and"(d.info_info.in_v(&[_][]const u8{"Germany","German","USA","American"}).k())
        ).k()
            .@"and"(
                d.movie_keyword.o(d.keyword_keyword).in_v(sets.murder4).k()
                    .@"and"(
                        d.movie_production_year.gt(2009).k()
                            .@"and"(d.movie_kind.o(d.kind_kind).in_v(&[_][]const u8{"movie","episode"}).k())
                    )
            )
            .o(
                d.movie_title
                    .x(
                        d.movie_data.o(
                            d.data_data.lt("7.0").k()
                                .@"and"(d.data_type.o(d.infotype_info).eq("rating").k())
                                .o(d.data_data)
                        )
                    )
                    .x(
                        d.movie_company.o(
                            d.company_note.nrx(rx.paren_USA).k()
                                .@"and"(
                                    d.company_note.rx(rx.paren_200_dot).k()
                                        .@"and"(
                                            d.company_country.ne("[us]").k()
                                                .@"and"(d.company_type.o(d.companytype_kind).eq("production companies").k())
                                        )
                                )
                                .o(d.company_name)
                        )
                    )
            )
    );
    var acc = h.Acc3{};
    q.drive(h.Sink(h.Acc3){ .acc = &acc });
    try h.fmt3(w, acc.m0, acc.m1, acc.m2);
}

fn q22c(d: *const Data, w: *Io.Writer) anyerror!void {
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
                            d.company_note.nrx(rx.paren_USA).k()
                                .@"and"(
                                    d.company_note.rx(rx.paren_200_dot).k()
                                        .@"and"(
                                            d.company_country.ne("[us]").k()
                                                .@"and"(d.company_type.o(d.companytype_kind).eq("production companies").k())
                                        )
                                )
                                .o(d.company_name)
                        )
                    )
            )
    );
    var acc = h.Acc3{};
    q.drive(h.Sink(h.Acc3){ .acc = &acc });
    try h.fmt3(w, acc.m0, acc.m1, acc.m2);
}
