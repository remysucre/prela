const std = @import("std");
const Io = std.Io;
const Data = @import("../data.zig").Data;
const h = @import("../helpers.zig");
const rx = @import("../regex.zig");
const sets = @import("../sets.zig");
const Entry = @import("all.zig").Entry;

pub const ENTRIES: []const Entry = &.{
    .{ .name = "16a", .oracle = "Adams, Stan || Carol Burnett vs. Anthony Perkins", .run = q16a },
    .{ .name = "16b", .oracle = "!!!, Toy || & Teller", .run = q16b },
    .{ .name = "16c", .oracle = "\"Brooklyn\" Tony Danza || (#1.5)", .run = q16c },
    .{ .name = "16d", .oracle = "\"Brooklyn\" Tony Danza || (#1.5)", .run = q16d },
    .{ .name = "17a", .oracle = "B, Khaz", .run = q17a },
    .{ .name = "17b", .oracle = "Z'Dar, Robert", .run = q17b },
    .{ .name = "17c", .oracle = "X'Volaitis, John", .run = q17c },
    .{ .name = "17d", .oracle = "Abrahamsson, Bertil", .run = q17d },
    .{ .name = "17e", .oracle = "$hort, Too", .run = q17e },
    .{ .name = "17f", .oracle = "'El Galgo PornoStar', Blanquito", .run = q17f },
    .{ .name = "18a", .oracle = "$1,000 || 10 || 40 Days and 40 Nights", .run = q18a },
    .{ .name = "18b", .oracle = "Horror || 8.1 || Agorable", .run = q18b },
    .{ .name = "18c", .oracle = "Action || 10 || #PostModem", .run = q18c },
};

fn q16a(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_company.o(d.company_country.eq("[us]")).k()
            .@"and"(d.movie_keyword.o(d.keyword_keyword).eq("character-name-in-title").k())
            .@"and"(d.movie_episode_nr.ge(50).k())
            .@"and"(d.movie_episode_nr.lt(100).k())
            .o(
                d.movie_cast.o(d.cast_person.o(d.person_aka.o(d.akaname_name)))
                    .x(d.movie_title),
            ),
    );
    var acc = h.Acc2{};
    q.drive(h.Sink(h.Acc2){ .acc = &acc });
    try h.fmt2(w, acc.m0, acc.m1);
}

fn q16b(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_company.o(d.company_country.eq("[us]")).k()
            .@"and"(d.movie_keyword.o(d.keyword_keyword).eq("character-name-in-title").k())
            .o(
                d.movie_cast.o(d.cast_person.o(d.person_aka.o(d.akaname_name)))
                    .x(d.movie_title),
            ),
    );
    var acc = h.Acc2{};
    q.drive(h.Sink(h.Acc2){ .acc = &acc });
    try h.fmt2(w, acc.m0, acc.m1);
}

fn q16c(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_company.o(d.company_country.eq("[us]")).k()
            .@"and"(d.movie_keyword.o(d.keyword_keyword).eq("character-name-in-title").k())
            .@"and"(d.movie_episode_nr.lt(100).k())
            .o(
                d.movie_cast.o(d.cast_person.o(d.person_aka.o(d.akaname_name)))
                    .x(d.movie_title),
            ),
    );
    var acc = h.Acc2{};
    q.drive(h.Sink(h.Acc2){ .acc = &acc });
    try h.fmt2(w, acc.m0, acc.m1);
}

fn q16d(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_company.o(d.company_country.eq("[us]")).k()
            .@"and"(d.movie_keyword.o(d.keyword_keyword).eq("character-name-in-title").k())
            .@"and"(d.movie_episode_nr.ge(5).k())
            .@"and"(d.movie_episode_nr.lt(100).k())
            .o(
                d.movie_cast.o(d.cast_person.o(d.person_aka.o(d.akaname_name)))
                    .x(d.movie_title),
            ),
    );
    var acc = h.Acc2{};
    q.drive(h.Sink(h.Acc2){ .acc = &acc });
    try h.fmt2(w, acc.m0, acc.m1);
}

fn q17a(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_company.o(d.company_country.eq("[us]")).k()
            .@"and"(d.movie_keyword.o(d.keyword_keyword).eq("character-name-in-title").k())
            .o(
                d.movie_cast.o(d.cast_person.o(d.person_name.rx(rx.pre_B))),
            ),
    );
    var acc = h.Acc1{};
    q.drive(h.Sink(h.Acc1){ .acc = &acc });
    try h.fmt1(w, acc.m);
}

fn q17b(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_company.k()
            .@"and"(d.movie_keyword.o(d.keyword_keyword).eq("character-name-in-title").k())
            .o(
                d.movie_cast.o(d.cast_person.o(d.person_name.rx(rx.pre_Z))),
            ),
    );
    var acc = h.Acc1{};
    q.drive(h.Sink(h.Acc1){ .acc = &acc });
    try h.fmt1(w, acc.m);
}

fn q17c(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_company.k()
            .@"and"(d.movie_keyword.o(d.keyword_keyword).eq("character-name-in-title").k())
            .o(
                d.movie_cast.o(d.cast_person.o(d.person_name.rx(rx.pre_X))),
            ),
    );
    var acc = h.Acc1{};
    q.drive(h.Sink(h.Acc1){ .acc = &acc });
    try h.fmt1(w, acc.m);
}

fn q17d(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_company.k()
            .@"and"(d.movie_keyword.o(d.keyword_keyword).eq("character-name-in-title").k())
            .o(
                d.movie_cast.o(d.cast_person.o(d.person_name.rx(rx.bert))),
            ),
    );
    var acc = h.Acc1{};
    q.drive(h.Sink(h.Acc1){ .acc = &acc });
    try h.fmt1(w, acc.m);
}

fn q17e(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_company.o(d.company_country.eq("[us]")).k()
            .@"and"(d.movie_keyword.o(d.keyword_keyword).eq("character-name-in-title").k())
            .o(
                d.movie_cast.o(d.cast_person.o(d.person_name)),
            ),
    );
    var acc = h.Acc1{};
    q.drive(h.Sink(h.Acc1){ .acc = &acc });
    try h.fmt1(w, acc.m);
}

fn q17f(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_company.k()
            .@"and"(d.movie_keyword.o(d.keyword_keyword).eq("character-name-in-title").k())
            .o(
                d.movie_cast.o(d.cast_person.o(d.person_name.rx(rx.b_uc))),
            ),
    );
    var acc = h.Acc1{};
    q.drive(h.Sink(h.Acc1){ .acc = &acc });
    try h.fmt1(w, acc.m);
}

fn q18a(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_info.o(
            d.info_type.o(d.infotype_info).eq("budget").k()
                .o(d.info_info),
        ).k()
            .@"and"(d.movie_cast.in_s(
                d.cast_note.in_v(&[_][]const u8{ "(producer)", "(executive producer)" }).k()
                    .@"and"(d.cast_person.in_s(
                        d.person_gender.eq("m").k()
                            .@"and"(d.person_name.rx(rx.tim).k()),
                    ).k()),
            ).k())
            .o(
                d.movie_info.o(
                    d.info_type.o(d.infotype_info).eq("budget").k()
                        .o(d.info_info),
                )
                    .x(d.movie_data.o(
                        d.data_type.o(d.infotype_info).eq("votes").k()
                            .o(d.data_data),
                    ))
                    .x(d.movie_title),
            ),
    );
    var acc = h.Acc3{};
    q.drive(h.Sink(h.Acc3){ .acc = &acc });
    try h.fmt3(w, acc.m0, acc.m1, acc.m2);
}

fn q18b(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_info.in_s(
            d.info_type.o(d.infotype_info).eq("genres").k()
                .@"and"(d.info_info.in_v(&[_][]const u8{ "Horror", "Thriller" }).k())
                .minus(d.info_note.k()),
        ).k()
            .@"and"(d.movie_production_year.ge(2008).k())
            .@"and"(d.movie_production_year.le(2014).k())
            .@"and"(d.movie_cast.in_s(
                d.cast_note.in_v(sets.writer5).k()
                    .@"and"(d.cast_person.in_s(d.person_gender.eq("f").k()).k()),
            ).k())
            .o(
                d.movie_info.o(
                    d.info_type.o(d.infotype_info).eq("genres").k()
                        .@"and"(d.info_info.in_v(&[_][]const u8{ "Horror", "Thriller" }).k())
                        .minus(d.info_note.k())
                        .o(d.info_info),
                )
                    .x(d.movie_data.o(
                        d.data_type.o(d.infotype_info).eq("rating").k()
                            .@"and"(d.data_data.gt("8.0").k())
                            .o(d.data_data),
                    ))
                    .x(d.movie_title),
            ),
    );
    var acc = h.Acc3{};
    q.drive(h.Sink(h.Acc3){ .acc = &acc });
    try h.fmt3(w, acc.m0, acc.m1, acc.m2);
}

fn q18c(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_info.in_s(
            d.info_type.o(d.infotype_info).eq("genres").k()
                .@"and"(d.info_info.in_v(sets.genre6).k()),
        ).k()
            .@"and"(d.movie_cast.in_s(
                d.cast_note.in_v(sets.writer5).k()
                    .@"and"(d.cast_person.in_s(d.person_gender.eq("m").k()).k()),
            ).k())
            .o(
                d.movie_info.o(
                    d.info_type.o(d.infotype_info).eq("genres").k()
                        .@"and"(d.info_info.in_v(sets.genre6).k())
                        .o(d.info_info),
                )
                    .x(d.movie_data.o(
                        d.data_type.o(d.infotype_info).eq("votes").k()
                            .o(d.data_data),
                    ))
                    .x(d.movie_title),
            ),
    );
    var acc = h.Acc3{};
    q.drive(h.Sink(h.Acc3){ .acc = &acc });
    try h.fmt3(w, acc.m0, acc.m1, acc.m2);
}
