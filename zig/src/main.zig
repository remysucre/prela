// Prela-Zig — full 113-query JOB suite runner.

const std = @import("std");
const Io = std.Io;
const Data = @import("data.zig").Data;
const all = @import("queries/all.zig");

pub fn main(init: std.process.Init) !void {
    const arena = init.arena.allocator();
    const io = init.io;

    var stdout_buf: [4096]u8 = undefined;
    var stdout_fw: Io.File.Writer = .init(.stdout(), io, &stdout_buf);
    const w = &stdout_fw.interface;

    const Clock = Io.Clock;
    const tsNow = struct {
        fn f(i: Io) Clock.Timestamp { return Clock.Timestamp.now(i, .awake); }
    }.f;
    const secsBetween = struct {
        fn f(a: Clock.Timestamp, b: Clock.Timestamp) f64 {
            const dur: i96 = a.raw.durationTo(b.raw).nanoseconds;
            return @as(f64, @floatFromInt(@as(i64, @intCast(dur)))) / 1e9;
        }
    }.f;

    const t_load = tsNow(io);
    const d = try Data.load(io, arena);
    try w.print("load: {d:.2}s  (movie n={d}, person n={d})\n",
        .{ secsBetween(t_load, tsNow(io)), d.movie.n, d.persons.n });
    try w.print("{d} queries registered\n", .{all.ENTRIES.len});

    var round: usize = 1;
    while (round <= 2) : (round += 1) {
        try w.print("--- run {d} ---\n", .{round});
        var ok: usize = 0;
        const t_run = tsNow(io);
        for (all.ENTRIES) |e| {
            const t_q = tsNow(io);
            var qbuf: [8192]u8 = undefined;
            var qw = Io.Writer.fixed(&qbuf);
            try e.run(&d, &qw);
            const out = qw.buffered();
            const dt_q = secsBetween(t_q, tsNow(io));
            const pass = std.mem.eql(u8, out, e.oracle);
            if (pass) ok += 1;
            // run 1 prints only diffs / slow; run 2 prints everything.
            const show = !pass or round == 2 or dt_q > 0.5;
            if (show) {
                try w.print("{s:<5} {s} {d:>6.2}s  {s}\n",
                    .{ e.name, if (pass) "ok  " else "DIFF", dt_q, out });
                if (!pass) try w.print("        oracle: {s}\n", .{e.oracle});
            }
        }
        const dt_run = secsBetween(t_run, tsNow(io));
        try w.print("run {d}: {d}/{d} ok  total {d:.2}s\n",
            .{ round, ok, all.ENTRIES.len, dt_run });
    }
    try w.flush();
}
