const std = @import("std");
const Io = std.Io;
const Data = @import("../data.zig").Data;
const h = @import("../helpers.zig");
const rx = @import("../regex.zig");
const sets = @import("../sets.zig");
const Entry = @import("all.zig").Entry;

pub const ENTRIES: []const Entry = &.{
    .{ .name = "19a", .oracle = "Angeline, Moriah || Blue Harvest", .run = q19a },
    .{ .name = "19b", .oracle = "Jolie, Angelina || Kung Fu Panda", .run = q19b },
    .{ .name = "19c", .oracle = "Alborg, Ana Esther || .hack//Akusei heni vol. 2", .run = q19c },
    .{ .name = "19d", .oracle = "Aaron, Caroline || $9.99", .run = q19d },
    .{ .name = "20a", .oracle = "Disaster Movie", .run = q20a },
    .{ .name = "20b", .oracle = "Iron Man", .run = q20b },
    .{ .name = "20c", .oracle = "Abell, Alistair || ...And Then I...", .run = q20c },
    .{ .name = "21a", .oracle = "Det Danske Filminstitut || followed by || Der Serienkiller - Klinge des Todes", .run = q21a },
    .{ .name = "21b", .oracle = "Filmlance International AB || followed by || Hämndens pris", .run = q21b },
    .{ .name = "21c", .oracle = "Churchill Films || followed by || Batman Beyond", .run = q21c },
    .{ .name = "23a", .oracle = "movie || The Analysts", .run = q23a },
    .{ .name = "23b", .oracle = "movie || The Big Mope", .run = q23b },
    .{ .name = "23c", .oracle = "movie || Dirt Merchant", .run = q23c },
    .{ .name = "24a", .oracle = "Additional Voices || Baker, Andrea || Baiohazâdo 6", .run = q24a },
    .{ .name = "24b", .oracle = "Tigress || Jolie, Angelina || Kung Fu Panda 2", .run = q24b },
    .{ .name = "25a", .oracle = "Horror || 10 || -- And Now the Screaming Starts! || Abdallah, Damon", .run = q25a },
    .{ .name = "25b", .oracle = "Horror || 138 || Vampire Boys || Campbell, Jeremiah", .run = q25b },
    .{ .name = "25c", .oracle = "Action || 10 || $ || Aakeson, Kim Fupz", .run = q25c },
    .{ .name = "26a", .oracle = "'Agua' Man || Acereda, Hermie || 7.1 || 3:10 to Yuma", .run = q26a },
    .{ .name = "26b", .oracle = "Bank Manager || 8.2 || Inception", .run = q26b },
    .{ .name = "26c", .oracle = "'Agua' Man || 1.9 || 12 Rounds", .run = q26c },
};

fn q19a(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_company.in_s(
            d.company_country.eq("[us]").k()
                .@"and"(
                    d.company_note.rx(rx.paren_USA).k()
                        .@"or"(d.company_note.rx(rx.paren_worldwide).k()),
                ),
        ).k()
            .@"and"(
                d.movie_info.in_s(
                    d.info_type.o(d.infotype_info).eq("release dates").k()
                        .@"and"(
                            d.info_info.rx(rx.japan_dot_200).k()
                                .@"or"(d.info_info.rx(rx.usa_dot_200).k()),
                        ),
                ).k()
                    .@"and"(
                        d.movie_production_year.ge(2005).k()
                            .@"and"(d.movie_production_year.le(2009).k()),
                    ),
            )
            .o(
                d.movie_cast.o(
                    d.cast_note.in_v(sets.voice4).k()
                        .@"and"(
                            d.cast_role.o(d.roletype_role).eq("actress").k()
                                .@"and"(
                                    d.cast_character.k()
                                        .@"and"(d.cast_person.in_s(
                                            d.person_gender.eq("f").k()
                                                .@"and"(
                                                    d.person_name.rx(rx.ang).k()
                                                        .@"and"(d.person_aka.k()),
                                                ),
                                        ).k()),
                                ),
                        )
                        .o(d.cast_person.o(d.person_name)),
                )
                    .x(d.movie_title),
            ),
    );
    var acc = h.Acc2{};
    q.drive(h.Sink(h.Acc2){ .acc = &acc });
    try h.fmt2(w, acc.m0, acc.m1);
}

fn q19b(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_company.in_s(
            d.company_country.eq("[us]").k()
                .@"and"(
                    d.company_note.rx(rx.paren_200_dot).k()
                        .@"and"(
                            d.company_note.rx(rx.paren_USA).k()
                                .@"or"(d.company_note.rx(rx.paren_worldwide).k()),
                        ),
                ),
        ).k()
            .@"and"(
                d.movie_info.in_s(
                    d.info_type.o(d.infotype_info).eq("release dates").k()
                        .@"and"(
                            d.info_info.rx(rx.japan_dot_2007).k()
                                .@"or"(d.info_info.rx(rx.usa_dot_2008).k()),
                        ),
                ).k()
                    .@"and"(
                        d.movie_production_year.ge(2007).k()
                            .@"and"(
                                d.movie_production_year.le(2008).k()
                                    .@"and"(d.movie_title.rx(rx.kung_fu_panda_dot).k()),
                            ),
                    ),
            )
            .o(
                d.movie_cast.o(
                    d.cast_note.eq("(voice)").k()
                        .@"and"(
                            d.cast_role.o(d.roletype_role).eq("actress").k()
                                .@"and"(
                                    d.cast_character.k()
                                        .@"and"(d.cast_person.in_s(
                                            d.person_gender.eq("f").k()
                                                .@"and"(
                                                    d.person_name.rx(rx.angel).k()
                                                        .@"and"(d.person_aka.k()),
                                                ),
                                        ).k()),
                                ),
                        )
                        .o(d.cast_person.o(d.person_name)),
                )
                    .x(d.movie_title),
            ),
    );
    var acc = h.Acc2{};
    q.drive(h.Sink(h.Acc2){ .acc = &acc });
    try h.fmt2(w, acc.m0, acc.m1);
}

fn q19c(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_company.o(d.company_country.eq("[us]")).k()
            .@"and"(
                d.movie_info.in_s(
                    d.info_type.o(d.infotype_info).eq("release dates").k()
                        .@"and"(
                            d.info_info.rx(rx.japan_dot_200).k()
                                .@"or"(d.info_info.rx(rx.usa_dot_200).k()),
                        ),
                ).k()
                    .@"and"(d.movie_production_year.gt(2000).k()),
            )
            .o(
                d.movie_cast.o(
                    d.cast_note.in_v(sets.voice4).k()
                        .@"and"(
                            d.cast_role.o(d.roletype_role).eq("actress").k()
                                .@"and"(
                                    d.cast_character.k()
                                        .@"and"(d.cast_person.in_s(
                                            d.person_gender.eq("f").k()
                                                .@"and"(
                                                    d.person_name.rx(rx.an_uc).k()
                                                        .@"and"(d.person_aka.k()),
                                                ),
                                        ).k()),
                                ),
                        )
                        .o(d.cast_person.o(d.person_name)),
                )
                    .x(d.movie_title),
            ),
    );
    var acc = h.Acc2{};
    q.drive(h.Sink(h.Acc2){ .acc = &acc });
    try h.fmt2(w, acc.m0, acc.m1);
}

fn q19d(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_company.o(d.company_country.eq("[us]")).k()
            .@"and"(
                d.movie_info.o(
                    d.info_type.o(d.infotype_info).eq("release dates"),
                ).k()
                    .@"and"(d.movie_production_year.gt(2000).k()),
            )
            .o(
                d.movie_cast.o(
                    d.cast_note.in_v(sets.voice4).k()
                        .@"and"(
                            d.cast_role.o(d.roletype_role).eq("actress").k()
                                .@"and"(
                                    d.cast_character.k()
                                        .@"and"(d.cast_person.in_s(
                                            d.person_gender.eq("f").k()
                                                .@"and"(d.person_aka.k()),
                                        ).k()),
                                ),
                        )
                        .o(d.cast_person.o(d.person_name)),
                )
                    .x(d.movie_title),
            ),
    );
    var acc = h.Acc2{};
    q.drive(h.Sink(h.Acc2){ .acc = &acc });
    try h.fmt2(w, acc.m0, acc.m1);
}

fn q20a(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_complete_cast.in_s(
            d.completecast_subject.o(d.compcasttype_kind).eq("cast").k()
                .@"and"(d.completecast_status.o(d.compcasttype_kind).rx(rx.complete).k()),
        ).k()
            .@"and"(
                d.movie_keyword.o(d.keyword_keyword).in_v(sets.kw8).k()
                    .@"and"(
                        d.movie_kind.o(d.kind_kind).eq("movie").k()
                            .@"and"(
                                d.movie_production_year.gt(1950).k()
                                    .@"and"(d.movie_cast.o(
                                        d.cast_character.in_s(
                                            d.character_name.nrx(rx.sherlock).k()
                                                .@"and"(
                                                    d.character_name.rx(rx.tony_stark).k()
                                                        .@"or"(d.character_name.rx(rx.iron_man).k()),
                                                ),
                                        ),
                                    ).k()),
                            ),
                    ),
            )
            .o(d.movie_title),
    );
    var acc = h.Acc1{};
    q.drive(h.Sink(h.Acc1){ .acc = &acc });
    try h.fmt1(w, acc.m);
}

fn q20b(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_complete_cast.in_s(
            d.completecast_subject.o(d.compcasttype_kind).eq("cast").k()
                .@"and"(d.completecast_status.o(d.compcasttype_kind).rx(rx.complete).k()),
        ).k()
            .@"and"(
                d.movie_keyword.o(d.keyword_keyword).in_v(sets.kw8).k()
                    .@"and"(
                        d.movie_kind.o(d.kind_kind).eq("movie").k()
                            .@"and"(
                                d.movie_production_year.gt(2000).k()
                                    .@"and"(d.movie_cast.in_s(
                                        d.cast_character.in_s(
                                            d.character_name.nrx(rx.sherlock).k()
                                                .@"and"(
                                                    d.character_name.rx(rx.tony_stark).k()
                                                        .@"or"(d.character_name.rx(rx.iron_man).k()),
                                                ),
                                        ).k()
                                            .@"and"(d.cast_person.o(
                                                d.person_name.rx(rx.downey_robert),
                                            ).k()),
                                    ).k()),
                            ),
                    ),
            )
            .o(d.movie_title),
    );
    var acc = h.Acc1{};
    q.drive(h.Sink(h.Acc1){ .acc = &acc });
    try h.fmt1(w, acc.m);
}

fn q20c(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_complete_cast.in_s(
            d.completecast_subject.o(d.compcasttype_kind).eq("cast").k()
                .@"and"(d.completecast_status.o(d.compcasttype_kind).rx(rx.complete).k()),
        ).k()
            .@"and"(
                d.movie_keyword.o(d.keyword_keyword).in_v(sets.kw10).k()
                    .@"and"(
                        d.movie_kind.o(d.kind_kind).eq("movie").k()
                            .@"and"(d.movie_production_year.gt(2000).k()),
                    ),
            )
            .o(
                d.movie_cast.o(
                    d.cast_character.o(d.character_name.rx(rx.class_Man_an)).k()
                        .o(d.cast_person.o(d.person_name)),
                )
                    .x(d.movie_title),
            ),
    );
    var acc = h.Acc2{};
    q.drive(h.Sink(h.Acc2){ .acc = &acc });
    try h.fmt2(w, acc.m0, acc.m1);
}

fn q21a(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_company.in_s(
            d.company_country.ne("[pl]").k()
                .@"and"(
                    d.company_name.rx(rx.film).k()
                        .@"or"(d.company_name.rx(rx.warner).k()),
                )
                .@"and"(
                    d.company_type.o(d.companytype_kind).eq("production companies").k()
                        .minus(d.company_note.k()),
                ),
        ).k()
            .@"and"(
                d.movie_keyword.o(d.keyword_keyword).eq("sequel").k()
                    .@"and"(
                        d.movie_link.in_s(
                            d.movielink_type.o(d.linktype_link).rx(rx.follow).k(),
                        ).k()
                            .@"and"(
                                d.movie_info.o(d.info_info.in_v(sets.nordic8)).k()
                                    .@"and"(
                                        d.movie_production_year.ge(1950).k()
                                            .@"and"(d.movie_production_year.le(2000).k()),
                                    ),
                            ),
                    ),
            )
            .o(
                d.movie_company.in_s(
                    d.company_country.ne("[pl]").k()
                        .@"and"(
                            d.company_name.rx(rx.film).k()
                                .@"or"(d.company_name.rx(rx.warner).k()),
                        )
                        .@"and"(
                            d.company_type.o(d.companytype_kind).eq("production companies").k()
                                .minus(d.company_note.k()),
                        ),
                ).o(d.company_name)
                    .x(d.movie_link.in_s(
                        d.movielink_type.o(d.linktype_link).rx(rx.follow).k(),
                    ).o(d.movielink_type.o(d.linktype_link)))
                    .x(d.movie_title),
            ),
    );
    var acc = h.Acc3{};
    q.drive(h.Sink(h.Acc3){ .acc = &acc });
    try h.fmt3(w, acc.m0, acc.m1, acc.m2);
}

fn q21b(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_company.in_s(
            d.company_country.ne("[pl]").k()
                .@"and"(
                    d.company_name.rx(rx.film).k()
                        .@"or"(d.company_name.rx(rx.warner).k()),
                )
                .@"and"(
                    d.company_type.o(d.companytype_kind).eq("production companies").k()
                        .minus(d.company_note.k()),
                ),
        ).k()
            .@"and"(
                d.movie_keyword.o(d.keyword_keyword).eq("sequel").k()
                    .@"and"(
                        d.movie_link.in_s(
                            d.movielink_type.o(d.linktype_link).rx(rx.follow).k(),
                        ).k()
                            .@"and"(
                                d.movie_info.o(
                                    d.info_info.in_v(&[_][]const u8{ "Germany", "German" }),
                                ).k()
                                    .@"and"(
                                        d.movie_production_year.ge(2000).k()
                                            .@"and"(d.movie_production_year.le(2010).k()),
                                    ),
                            ),
                    ),
            )
            .o(
                d.movie_company.in_s(
                    d.company_country.ne("[pl]").k()
                        .@"and"(
                            d.company_name.rx(rx.film).k()
                                .@"or"(d.company_name.rx(rx.warner).k()),
                        )
                        .@"and"(
                            d.company_type.o(d.companytype_kind).eq("production companies").k()
                                .minus(d.company_note.k()),
                        ),
                ).o(d.company_name)
                    .x(d.movie_link.in_s(
                        d.movielink_type.o(d.linktype_link).rx(rx.follow).k(),
                    ).o(d.movielink_type.o(d.linktype_link)))
                    .x(d.movie_title),
            ),
    );
    var acc = h.Acc3{};
    q.drive(h.Sink(h.Acc3){ .acc = &acc });
    try h.fmt3(w, acc.m0, acc.m1, acc.m2);
}

fn q21c(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_company.in_s(
            d.company_country.ne("[pl]").k()
                .@"and"(
                    d.company_name.rx(rx.film).k()
                        .@"or"(d.company_name.rx(rx.warner).k()),
                )
                .@"and"(
                    d.company_type.o(d.companytype_kind).eq("production companies").k()
                        .minus(d.company_note.k()),
                ),
        ).k()
            .@"and"(
                d.movie_keyword.o(d.keyword_keyword).eq("sequel").k()
                    .@"and"(
                        d.movie_link.in_s(
                            d.movielink_type.o(d.linktype_link).rx(rx.follow).k(),
                        ).k()
                            .@"and"(
                                d.movie_info.o(d.info_info.in_v(sets.nordic9)).k()
                                    .@"and"(
                                        d.movie_production_year.ge(1950).k()
                                            .@"and"(d.movie_production_year.le(2010).k()),
                                    ),
                            ),
                    ),
            )
            .o(
                d.movie_company.in_s(
                    d.company_country.ne("[pl]").k()
                        .@"and"(
                            d.company_name.rx(rx.film).k()
                                .@"or"(d.company_name.rx(rx.warner).k()),
                        )
                        .@"and"(
                            d.company_type.o(d.companytype_kind).eq("production companies").k()
                                .minus(d.company_note.k()),
                        ),
                ).o(d.company_name)
                    .x(d.movie_link.in_s(
                        d.movielink_type.o(d.linktype_link).rx(rx.follow).k(),
                    ).o(d.movielink_type.o(d.linktype_link)))
                    .x(d.movie_title),
            ),
    );
    var acc = h.Acc3{};
    q.drive(h.Sink(h.Acc3){ .acc = &acc });
    try h.fmt3(w, acc.m0, acc.m1, acc.m2);
}

fn q23a(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_complete_cast.o(
            d.completecast_status.o(d.compcasttype_kind).eq("complete+verified"),
        ).k()
            .@"and"(
                d.movie_company.o(d.company_country.eq("[us]")).k()
                    .@"and"(
                        d.movie_info.in_s(
                            d.info_type.o(d.infotype_info).eq("release dates").k()
                                .@"and"(
                                    d.info_note.rx(rx.internet).k()
                                        .@"and"(
                                            d.info_info.rx(rx.usa_dot_space_199).k()
                                                .@"or"(d.info_info.rx(rx.usa_dot_space_200).k()),
                                        ),
                                ),
                        ).k()
                            .@"and"(
                                d.movie_kind.o(d.kind_kind).eq("movie").k()
                                    .@"and"(
                                        d.movie_keyword.k()
                                            .@"and"(d.movie_production_year.gt(2000).k()),
                                    ),
                            ),
                    ),
            )
            .o(d.movie_kind.o(d.kind_kind).eq("movie").x(d.movie_title)),
    );
    var acc = h.Acc2{};
    q.drive(h.Sink(h.Acc2){ .acc = &acc });
    try h.fmt2(w, acc.m0, acc.m1);
}

fn q23b(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_complete_cast.o(
            d.completecast_status.o(d.compcasttype_kind).eq("complete+verified"),
        ).k()
            .@"and"(
                d.movie_company.o(d.company_country.eq("[us]")).k()
                    .@"and"(
                        d.movie_info.in_s(
                            d.info_type.o(d.infotype_info).eq("release dates").k()
                                .@"and"(
                                    d.info_note.rx(rx.internet).k()
                                        .@"and"(d.info_info.rx(rx.usa_dot_space_200).k()),
                                ),
                        ).k()
                            .@"and"(
                                d.movie_kind.o(d.kind_kind).eq("movie").k()
                                    .@"and"(
                                        d.movie_keyword.o(d.keyword_keyword)
                                            .in_v(&[_][]const u8{ "nerd", "loner", "alienation", "dignity" }).k()
                                            .@"and"(d.movie_production_year.gt(2000).k()),
                                    ),
                            ),
                    ),
            )
            .o(d.movie_kind.o(d.kind_kind).eq("movie").x(d.movie_title)),
    );
    var acc = h.Acc2{};
    q.drive(h.Sink(h.Acc2){ .acc = &acc });
    try h.fmt2(w, acc.m0, acc.m1);
}

fn q23c(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_complete_cast.o(
            d.completecast_status.o(d.compcasttype_kind).eq("complete+verified"),
        ).k()
            .@"and"(
                d.movie_company.o(d.company_country.eq("[us]")).k()
                    .@"and"(
                        d.movie_info.in_s(
                            d.info_type.o(d.infotype_info).eq("release dates").k()
                                .@"and"(
                                    d.info_note.rx(rx.internet).k()
                                        .@"and"(
                                            d.info_info.rx(rx.usa_dot_space_199).k()
                                                .@"or"(d.info_info.rx(rx.usa_dot_space_200).k()),
                                        ),
                                ),
                        ).k()
                            .@"and"(
                                d.movie_kind.o(d.kind_kind)
                                    .in_v(&[_][]const u8{ "movie", "tv movie", "video movie", "video game" }).k()
                                    .@"and"(
                                        d.movie_keyword.k()
                                            .@"and"(d.movie_production_year.gt(1990).k()),
                                    ),
                            ),
                    ),
            )
            .o(d.movie_kind.o(d.kind_kind)
                .in_v(&[_][]const u8{ "movie", "tv movie", "video movie", "video game" })
                .x(d.movie_title)),
    );
    var acc = h.Acc2{};
    q.drive(h.Sink(h.Acc2){ .acc = &acc });
    try h.fmt2(w, acc.m0, acc.m1);
}

fn q24a(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_company.o(d.company_country.eq("[us]")).k()
            .@"and"(
                d.movie_info.in_s(
                    d.info_type.o(d.infotype_info).eq("release dates").k()
                        .@"and"(
                            d.info_info.rx(rx.japan_dot_201).k()
                                .@"or"(d.info_info.rx(rx.usa_dot_201).k()),
                        ),
                ).k()
                    .@"and"(
                        d.movie_keyword.o(d.keyword_keyword)
                            .in_v(&[_][]const u8{ "hero", "martial-arts", "hand-to-hand-combat" }).k()
                            .@"and"(d.movie_production_year.gt(2010).k()),
                    ),
            )
            .o(
                d.movie_cast.o(
                    d.cast_note.in_v(sets.voice4).k()
                        .@"and"(
                            d.cast_role.o(d.roletype_role).eq("actress").k()
                                .@"and"(d.cast_person.in_s(
                                    d.person_gender.eq("f").k()
                                        .@"and"(
                                            d.person_name.rx(rx.an_uc).k()
                                                .@"and"(d.person_aka.k()),
                                        ),
                                ).k()),
                        )
                        .o(
                            d.cast_character.o(d.character_name)
                                .x(d.cast_person.o(d.person_name)),
                        ),
                )
                    .x(d.movie_title),
            ),
    );
    var acc = h.Acc3{};
    q.drive(h.Sink(h.Acc3){ .acc = &acc });
    try h.fmt3(w, acc.m0, acc.m1, acc.m2);
}

fn q24b(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_company.in_s(
            d.company_country.eq("[us]").k()
                .@"and"(d.company_name.eq("DreamWorks Animation").k()),
        ).k()
            .@"and"(
                d.movie_info.in_s(
                    d.info_type.o(d.infotype_info).eq("release dates").k()
                        .@"and"(
                            d.info_info.rx(rx.japan_dot_201).k()
                                .@"or"(d.info_info.rx(rx.usa_dot_201).k()),
                        ),
                ).k()
                    .@"and"(
                        d.movie_keyword.o(d.keyword_keyword)
                            .in_v(&[_][]const u8{ "hero", "martial-arts", "hand-to-hand-combat", "computer-animated-movie" }).k()
                            .@"and"(
                                d.movie_production_year.gt(2010).k()
                                    .@"and"(d.movie_title.rx(rx.pre_kung_fu_panda).k()),
                            ),
                    ),
            )
            .o(
                d.movie_cast.o(
                    d.cast_note.in_v(sets.voice4).k()
                        .@"and"(
                            d.cast_role.o(d.roletype_role).eq("actress").k()
                                .@"and"(d.cast_person.in_s(
                                    d.person_gender.eq("f").k()
                                        .@"and"(
                                            d.person_name.rx(rx.an_uc).k()
                                                .@"and"(d.person_aka.k()),
                                        ),
                                ).k()),
                        )
                        .o(
                            d.cast_character.o(d.character_name)
                                .x(d.cast_person.o(d.person_name)),
                        ),
                )
                    .x(d.movie_title),
            ),
    );
    var acc = h.Acc3{};
    q.drive(h.Sink(h.Acc3){ .acc = &acc });
    try h.fmt3(w, acc.m0, acc.m1, acc.m2);
}

fn q25a(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_info.in_s(
            d.info_type.o(d.infotype_info).eq("genres").k()
                .@"and"(d.info_info.eq("Horror").k()),
        ).k()
            .@"and"(
                d.movie_keyword.o(d.keyword_keyword)
                    .in_v(&[_][]const u8{ "murder", "blood", "gore", "death", "female-nudity" }).k(),
            )
            .o(
                d.movie_info.o(
                    d.info_type.o(d.infotype_info).eq("genres").k()
                        .@"and"(d.info_info.eq("Horror").k())
                        .o(d.info_info),
                )
                    .x(d.movie_data.o(
                        d.data_type.o(d.infotype_info).eq("votes").k()
                            .o(d.data_data),
                    ))
                    .x(d.movie_title)
                    .x(d.movie_cast.o(
                        d.cast_note.in_v(sets.writer5).k()
                            .@"and"(d.cast_person.o(d.person_gender.eq("m")).k())
                            .o(d.cast_person.o(d.person_name)),
                    )),
            ),
    );
    var acc = h.Acc4{};
    q.drive(h.Sink(h.Acc4){ .acc = &acc });
    try h.fmt4(w, acc.m0, acc.m1, acc.m2, acc.m3);
}

fn q25b(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_info.in_s(
            d.info_type.o(d.infotype_info).eq("genres").k()
                .@"and"(d.info_info.eq("Horror").k()),
        ).k()
            .@"and"(
                d.movie_keyword.o(d.keyword_keyword)
                    .in_v(&[_][]const u8{ "murder", "blood", "gore", "death", "female-nudity" }).k()
                    .@"and"(
                        d.movie_production_year.gt(2010).k()
                            .@"and"(d.movie_title.rx(rx.pre_vampire).k()),
                    ),
            )
            .o(
                d.movie_info.o(
                    d.info_type.o(d.infotype_info).eq("genres").k()
                        .@"and"(d.info_info.eq("Horror").k())
                        .o(d.info_info),
                )
                    .x(d.movie_data.o(
                        d.data_type.o(d.infotype_info).eq("votes").k()
                            .o(d.data_data),
                    ))
                    .x(d.movie_title)
                    .x(d.movie_cast.o(
                        d.cast_note.in_v(sets.writer5).k()
                            .@"and"(d.cast_person.o(d.person_gender.eq("m")).k())
                            .o(d.cast_person.o(d.person_name)),
                    )),
            ),
    );
    var acc = h.Acc4{};
    q.drive(h.Sink(h.Acc4){ .acc = &acc });
    try h.fmt4(w, acc.m0, acc.m1, acc.m2, acc.m3);
}

fn q25c(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_info.in_s(
            d.info_type.o(d.infotype_info).eq("genres").k()
                .@"and"(d.info_info.in_v(sets.genre6).k()),
        ).k()
            .@"and"(d.movie_keyword.o(d.keyword_keyword).in_v(sets.kw7).k())
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
                    .x(d.movie_title)
                    .x(d.movie_cast.o(
                        d.cast_note.in_v(sets.writer5).k()
                            .@"and"(d.cast_person.o(d.person_gender.eq("m")).k())
                            .o(d.cast_person.o(d.person_name)),
                    )),
            ),
    );
    var acc = h.Acc4{};
    q.drive(h.Sink(h.Acc4){ .acc = &acc });
    try h.fmt4(w, acc.m0, acc.m1, acc.m2, acc.m3);
}

fn q26a(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_complete_cast.in_s(
            d.completecast_subject.o(d.compcasttype_kind).eq("cast").k()
                .@"and"(d.completecast_status.o(d.compcasttype_kind).rx(rx.complete).k()),
        ).k()
            .@"and"(
                d.movie_keyword.o(d.keyword_keyword).in_v(sets.kw10).k()
                    .@"and"(
                        d.movie_kind.o(d.kind_kind).eq("movie").k()
                            .@"and"(d.movie_production_year.gt(2000).k()),
                    ),
            )
            .o(
                d.movie_cast.o(
                    d.cast_character.o(d.character_name.rx(rx.class_Man_an)).k()
                        .o(
                            d.cast_character.o(d.character_name)
                                .x(d.cast_person.o(d.person_name)),
                        ),
                )
                    .x(d.movie_data.o(
                        d.data_type.o(d.infotype_info).eq("rating").k()
                            .@"and"(d.data_data.gt("7.0").k())
                            .o(d.data_data),
                    ))
                    .x(d.movie_title),
            ),
    );
    var acc = h.Acc4{};
    q.drive(h.Sink(h.Acc4){ .acc = &acc });
    try h.fmt4(w, acc.m0, acc.m1, acc.m2, acc.m3);
}

fn q26b(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_complete_cast.in_s(
            d.completecast_subject.o(d.compcasttype_kind).eq("cast").k()
                .@"and"(d.completecast_status.o(d.compcasttype_kind).rx(rx.complete).k()),
        ).k()
            .@"and"(
                d.movie_keyword.o(d.keyword_keyword)
                    .in_v(&[_][]const u8{ "superhero", "marvel-comics", "based-on-comic", "fight" }).k()
                    .@"and"(
                        d.movie_kind.o(d.kind_kind).eq("movie").k()
                            .@"and"(d.movie_production_year.gt(2005).k()),
                    ),
            )
            .o(
                d.movie_cast.o(
                    d.cast_character.o(d.character_name.rx(rx.class_Man_an)).k()
                        .o(d.cast_character.o(d.character_name)),
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

fn q26c(d: *const Data, w: *Io.Writer) anyerror!void {
    const rd = d.movie_data.o(
        d.data_type.o(d.infotype_info).eq("rating").k().o(d.data_data),
    );
    const q = d.movie.o(
        d.movie_complete_cast.in_s(
            d.completecast_subject.o(d.compcasttype_kind).eq("cast").k()
                .@"and"(d.completecast_status.o(d.compcasttype_kind).rx(rx.complete).k()),
        ).k()
            .@"and"(
                d.movie_keyword.o(d.keyword_keyword).in_v(sets.kw10).k()
                    .@"and"(
                        d.movie_kind.o(d.kind_kind).eq("movie").k()
                            .@"and"(d.movie_production_year.gt(2000).k()),
                    ),
            )
            .o(
                d.movie_cast.o(
                    d.cast_character.o(d.character_name.rx(rx.class_Man_an)).k()
                        .o(d.cast_character.o(d.character_name)),
                )
                    .x(rd)
                    .x(d.movie_title),
            ),
    );
    var acc = h.Acc3{};
    q.drive(h.Sink(h.Acc3){ .acc = &acc });
    try h.fmt3(w, acc.m0, acc.m1, acc.m2);
}
