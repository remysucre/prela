// queries: 7a-c, 8a-d, 9a-d, 10a-c (queries.jl lines 591-753)
const std = @import("std");
const Io = std.Io;
const Data = @import("../data.zig").Data;
const h = @import("../helpers.zig");
const rx = @import("../regex.zig");
const sets = @import("../sets.zig");
const Entry = @import("all.zig").Entry;

pub const ENTRIES: []const Entry = &.{
    .{ .name = "7a",  .oracle = "Antonioni, Michelangelo || Dressed to Kill",                                 .run = q7a },
    .{ .name = "7b",  .oracle = "De Palma, Brian || Dressed to Kill",                                         .run = q7b },
    .{ .name = "7c",  .oracle = "50 Cent || \"Boo\" Arnold was born Earl Arnold in Hattiesburg, Mississippi in 1966. His father gave him the nickname 'Boo' early in life and it stuck through grade school, high school, and college. He is still known as \"Boo\" to family and friends.  Raised in central Texas, Arnold played baseball at Texas Tech University where he graduated with a BA in Advertising and Marketing. While at Texas Tech he was also a member of the Texas Epsilon chapter of Phi Delta Theta fraternity. After college he worked with Young Life, an outreach to high school students, in San Antonio, Texas.  While with Young Life Arnold began taking extension courses through Fuller Theological Seminary and ultimately went full-time to Gordon-Conwell Theological Seminary in Boston, Massachusetts. At Gordon-Conwell he completed a Master's Degree in Divinity studying Theology, Philosophy, Church History, Biblical Languages (Hebrew & Greek), and Exegetical Methods. Following seminary he was involved with reconciliation efforts in the former Yugoslavia shortly after the war ended there in1995.  Arnold started acting in his early thirties in Texas. After an encouraging visit to Los Angeles where he spent time with childhood friend George Eads (of CSI Las Vegas) he decided to move to Los Angeles in 2001 to pursue acting full-time. While in Los Angeles he has studied acting with Judith Weston at Judith Weston Studio for Actors and Directors.  Arnold's acting career has been one of steady development, booking co-star and guest-star roles in nighttime television. He guest-starred opposite of Jane Seymour on the night time television drama Justice. He played the lead, Michael Hollister, in the film The Seer, written and directed by Patrick Masset (Friday Night Lights).  He was nominated Best Actor in the168 Film Festival for the role of Phil Stevens in the short-film Useless. In Useless he played a US Marshal who must choose between mercy and justice as he confronts the man who murdered his father. Arnold's performance in Useless confirmed his ability to carry lead roles, and he continues to work toward solidifying himself as a male lead in film and television.  Arnold married fellow Texan Stacy Rudd of San Antonio in 2003 and they are now raising their three children in the Los Angeles area.", .run = q7c },
    .{ .name = "8a",  .oracle = "Chambers, Linda || .hack//Quantum",                                          .run = q8a },
    .{ .name = "8b",  .oracle = "Chambers, Linda || Dragon Ball Z: Shin Budokai",                             .run = q8b },
    .{ .name = "8c",  .oracle = "\"A.J.\" || #1 Cheerleader Camp",                                            .run = q8c },
    .{ .name = "8d",  .oracle = "\"Jenny from the Block\" || #1 Cheerleader Camp",                            .run = q8d },
    .{ .name = "9a",  .oracle = "AJ || Airport Announcer || Blue Harvest",                                    .run = q9a },
    .{ .name = "9b",  .oracle = "AJ || Airport Announcer || Bassett, Angela || Blue Harvest",                 .run = q9b },
    .{ .name = "9c",  .oracle = "'Annette' || 2nd Balladeer || Alborg, Ana Esther || (1975-01-20)",           .run = q9c },
    .{ .name = "9d",  .oracle = "!!!, Toy || Aaron, Caroline || \"Cockamamie's\" Salesgirl || $15,000.00 Error", .run = q9d },
    .{ .name = "10a", .oracle = "Actor || 12 Rounds",                                                         .run = q10a },
    .{ .name = "10b", .oracle = "(empty)",                                                                    .run = q10b },
    .{ .name = "10c", .oracle = "Himself || Evil Eyes: Behind the Scenes",                                    .run = q10c },
};

fn q7a(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_production_year.ge(1980).k()
            .@"and"(d.movie_production_year.le(1995).k())
            .@"and"(d.movie_linked_by.o(
                d.movielink_type.o(d.linktype_link).eq("features")).k())
            .o(
                d.movie_cast.o(
                    d.cast_person.in_s(
                        d.person_aka.o(d.akaname_name).rx(rx.a_lc).k()
                            .@"and"(d.person_name_pcode.ge("A").k())
                            .@"and"(d.person_name_pcode.le("F").k())
                            .@"and"(
                                d.person_gender.eq("m").k()
                                    .@"or"(
                                        d.person_gender.eq("f").k()
                                            .@"and"(d.person_name.rx(rx.pre_B).k())
                                    )
                            )
                            .@"and"(d.person_info.in_s(
                                d.personinfo_type.o(d.infotype_info).eq("mini biography").k()
                                    .@"and"(d.personinfo_note.eq("Volker Boehm").k())
                            ).k())
                    ).k().o(d.cast_person.o(d.person_name))
                ).x(d.movie_title)
            )
    );
    var acc = h.Acc2{};
    q.drive(h.Sink(h.Acc2){ .acc = &acc });
    try h.fmt2(w, acc.m0, acc.m1);
}

fn q7b(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_production_year.ge(1980).k()
            .@"and"(d.movie_production_year.le(1984).k())
            .@"and"(d.movie_linked_by.o(
                d.movielink_type.o(d.linktype_link).eq("features")).k())
            .o(
                d.movie_cast.o(
                    d.cast_person.in_s(
                        d.person_aka.o(d.akaname_name).rx(rx.a_lc).k()
                            .@"and"(d.person_name_pcode.rx(rx.pre_D).k())
                            .@"and"(d.person_gender.eq("m").k())
                            .@"and"(d.person_info.in_s(
                                d.personinfo_type.o(d.infotype_info).eq("mini biography").k()
                                    .@"and"(d.personinfo_note.eq("Volker Boehm").k())
                            ).k())
                    ).k().o(d.cast_person.o(d.person_name))
                ).x(d.movie_title)
            )
    );
    var acc = h.Acc2{};
    q.drive(h.Sink(h.Acc2){ .acc = &acc });
    try h.fmt2(w, acc.m0, acc.m1);
}

fn q7c(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_production_year.ge(1980).k()
            .@"and"(d.movie_production_year.le(2010).k())
            .@"and"(d.movie_linked_by.o(
                d.movielink_type.o(d.linktype_link).in_v(
                    &[_][]const u8{ "references", "referenced in", "features", "featured in" })).k())
            .o(
                d.movie_cast.o(
                    d.cast_person.in_s(
                        d.person_aka.o(d.akaname_name).rx(rx.a_or_pre_A).k()
                            .@"and"(d.person_name_pcode.ge("A").k())
                            .@"and"(d.person_name_pcode.le("F").k())
                            .@"and"(
                                d.person_gender.eq("m").k()
                                    .@"or"(
                                        d.person_gender.eq("f").k()
                                            .@"and"(d.person_name.rx(rx.pre_A).k())
                                    )
                            )
                            .@"and"(d.person_info.in_s(
                                d.personinfo_type.o(d.infotype_info).eq("mini biography").k()
                                    .@"and"(d.personinfo_note.k())
                            ).k())
                    ).k().o(
                        d.cast_person.o(d.person_name)
                            .x(d.cast_person.o(
                                d.person_info.o(
                                    d.personinfo_type.o(d.infotype_info).eq("mini biography").k()
                                        .@"and"(d.personinfo_note.k())
                                        .o(d.personinfo_info))))
                    )
                )
            )
    );
    var acc = h.Acc2{};
    q.drive(h.Sink(h.Acc2){ .acc = &acc });
    try h.fmt2(w, acc.m0, acc.m1);
}

fn q8a(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_company.in_s(
            d.company_country.eq("[jp]").k()
                .@"and"(d.company_note.rx(rx.paren_japan).k())
                .@"and"(d.company_note.nrx(rx.paren_USA).k())
        ).k()
            .o(
                d.movie_cast.o(
                    d.cast_note.eq("(voice: English version)").k()
                        .@"and"(d.cast_role.o(d.roletype_role).eq("actress").k())
                        .@"and"(d.cast_person.in_s(
                            d.person_name.rx(rx.yo).k()
                                .@"and"(d.person_name.nrx(rx.yu).k())
                        ).k())
                        .o(d.cast_person.o(d.person_aka.o(d.akaname_name)))
                ).x(d.movie_title)
            )
    );
    var acc = h.Acc2{};
    q.drive(h.Sink(h.Acc2){ .acc = &acc });
    try h.fmt2(w, acc.m0, acc.m1);
}

fn q8b(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_company.in_s(
            d.company_country.eq("[jp]").k()
                .@"and"(d.company_note.rx(rx.paren_japan).k())
                .@"and"(d.company_note.nrx(rx.paren_USA).k())
                .@"and"(
                    d.company_note.rx(rx.paren_2006).k()
                        .@"or"(d.company_note.rx(rx.paren_2007).k())
                )
        ).k()
            .@"and"(d.movie_production_year.ge(2006).k())
            .@"and"(d.movie_production_year.le(2007).k())
            .@"and"(
                d.movie_title.rx(rx.pre_one_piece).k()
                    .@"or"(d.movie_title.rx(rx.pre_dragon_ball_z).k())
            )
            .o(
                d.movie_cast.o(
                    d.cast_note.eq("(voice: English version)").k()
                        .@"and"(d.cast_role.o(d.roletype_role).eq("actress").k())
                        .@"and"(d.cast_person.in_s(
                            d.person_name.rx(rx.yo).k()
                                .@"and"(d.person_name.nrx(rx.yu).k())
                        ).k())
                        .o(d.cast_person.o(d.person_aka.o(d.akaname_name)))
                ).x(d.movie_title)
            )
    );
    var acc = h.Acc2{};
    q.drive(h.Sink(h.Acc2){ .acc = &acc });
    try h.fmt2(w, acc.m0, acc.m1);
}

fn q8c(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_company.o(d.company_country.eq("[us]")).k()
            .o(
                d.movie_cast.o(
                    d.cast_role.o(d.roletype_role).eq("writer").k()
                        .o(d.cast_person.o(d.person_aka.o(d.akaname_name)))
                ).x(d.movie_title)
            )
    );
    var acc = h.Acc2{};
    q.drive(h.Sink(h.Acc2){ .acc = &acc });
    try h.fmt2(w, acc.m0, acc.m1);
}

fn q8d(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_company.o(d.company_country.eq("[us]")).k()
            .o(
                d.movie_cast.o(
                    d.cast_role.o(d.roletype_role).eq("costume designer").k()
                        .o(d.cast_person.o(d.person_aka.o(d.akaname_name)))
                ).x(d.movie_title)
            )
    );
    var acc = h.Acc2{};
    q.drive(h.Sink(h.Acc2){ .acc = &acc });
    try h.fmt2(w, acc.m0, acc.m1);
}

fn q9a(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_company.in_s(
            d.company_country.eq("[us]").k()
                .@"and"(
                    d.company_note.rx(rx.paren_USA).k()
                        .@"or"(d.company_note.rx(rx.paren_worldwide).k())
                )
        ).k()
            .@"and"(d.movie_production_year.ge(2005).k())
            .@"and"(d.movie_production_year.le(2015).k())
            .o(
                d.movie_cast.o(
                    d.cast_note.in_v(sets.voice4).k()
                        .@"and"(d.cast_role.o(d.roletype_role).eq("actress").k())
                        .@"and"(d.cast_person.in_s(
                            d.person_gender.eq("f").k()
                                .@"and"(d.person_name.rx(rx.ang).k())
                        ).k())
                        .o(
                            d.cast_person.o(d.person_aka.o(d.akaname_name))
                                .x(d.cast_character.o(d.character_name))
                        )
                ).x(d.movie_title)
            )
    );
    var acc = h.Acc3{};
    q.drive(h.Sink(h.Acc3){ .acc = &acc });
    try h.fmt3(w, acc.m0, acc.m1, acc.m2);
}

fn q9b(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_company.in_s(
            d.company_country.eq("[us]").k()
                .@"and"(d.company_note.rx(rx.paren_200_dot).k())
                .@"and"(
                    d.company_note.rx(rx.paren_USA).k()
                        .@"or"(d.company_note.rx(rx.paren_worldwide).k())
                )
        ).k()
            .@"and"(d.movie_production_year.ge(2007).k())
            .@"and"(d.movie_production_year.le(2010).k())
            .o(
                d.movie_cast.o(
                    d.cast_note.eq("(voice)").k()
                        .@"and"(d.cast_role.o(d.roletype_role).eq("actress").k())
                        .@"and"(d.cast_person.in_s(
                            d.person_gender.eq("f").k()
                                .@"and"(d.person_name.rx(rx.angel).k())
                        ).k())
                        .o(
                            d.cast_person.o(d.person_aka.o(d.akaname_name))
                                .x(d.cast_character.o(d.character_name))
                                .x(d.cast_person.o(d.person_name))
                        )
                ).x(d.movie_title)
            )
    );
    var acc = h.Acc4{};
    q.drive(h.Sink(h.Acc4){ .acc = &acc });
    try h.fmt4(w, acc.m0, acc.m1, acc.m2, acc.m3);
}

fn q9c(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_company.o(d.company_country.eq("[us]")).k()
            .o(
                d.movie_cast.o(
                    d.cast_note.in_v(sets.voice4).k()
                        .@"and"(d.cast_role.o(d.roletype_role).eq("actress").k())
                        .@"and"(d.cast_person.in_s(
                            d.person_gender.eq("f").k()
                                .@"and"(d.person_name.rx(rx.an_uc).k())
                        ).k())
                        .o(
                            d.cast_person.o(d.person_aka.o(d.akaname_name))
                                .x(d.cast_character.o(d.character_name))
                                .x(d.cast_person.o(d.person_name))
                        )
                ).x(d.movie_title)
            )
    );
    var acc = h.Acc4{};
    q.drive(h.Sink(h.Acc4){ .acc = &acc });
    try h.fmt4(w, acc.m0, acc.m1, acc.m2, acc.m3);
}

fn q9d(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_company.o(d.company_country.eq("[us]")).k()
            .o(
                d.movie_cast.o(
                    d.cast_note.in_v(sets.voice4).k()
                        .@"and"(d.cast_role.o(d.roletype_role).eq("actress").k())
                        .@"and"(d.cast_person.o(d.person_gender.eq("f")).k())
                        .o(
                            d.cast_person.o(d.person_aka.o(d.akaname_name))
                                .x(d.cast_person.o(d.person_name))
                                .x(d.cast_character.o(d.character_name))
                        )
                ).x(d.movie_title)
            )
    );
    var acc = h.Acc4{};
    q.drive(h.Sink(h.Acc4){ .acc = &acc });
    try h.fmt4(w, acc.m0, acc.m1, acc.m2, acc.m3);
}

fn q10a(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_company.o(d.company_country.eq("[ru]")).k()
            .@"and"(d.movie_production_year.gt(2005).k())
            .o(
                d.movie_cast.o(
                    d.cast_note.rx(rx.paren_voice).k()
                        .@"and"(d.cast_note.rx(rx.paren_uncredited).k())
                        .@"and"(d.cast_role.o(d.roletype_role).eq("actor").k())
                        .o(d.cast_character.o(d.character_name))
                ).x(d.movie_title)
            )
    );
    var acc = h.Acc2{};
    q.drive(h.Sink(h.Acc2){ .acc = &acc });
    try h.fmt2(w, acc.m0, acc.m1);
}

fn q10b(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_company.o(d.company_country.eq("[ru]")).k()
            .@"and"(d.movie_production_year.gt(2010).k())
            .o(
                d.movie_cast.o(
                    d.cast_note.rx(rx.paren_producer).k()
                        .@"and"(d.cast_role.o(d.roletype_role).eq("actor").k())
                        .o(d.cast_character.o(d.character_name))
                ).x(d.movie_title)
            )
    );
    var acc = h.Acc2{};
    q.drive(h.Sink(h.Acc2){ .acc = &acc });
    try h.fmt2(w, acc.m0, acc.m1);
}

fn q10c(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = d.movie.o(
        d.movie_company.o(d.company_country.eq("[us]")).k()
            .@"and"(d.movie_production_year.gt(1990).k())
            .o(
                d.movie_cast.o(
                    d.cast_note.rx(rx.paren_producer).k()
                        .o(d.cast_character.o(d.character_name))
                ).x(d.movie_title)
            )
    );
    var acc = h.Acc2{};
    q.drive(h.Sink(h.Acc2){ .acc = &acc });
    try h.fmt2(w, acc.m0, acc.m1);
}
