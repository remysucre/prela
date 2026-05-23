// All 113 JOB queries — aggregated from t1.zig..t6.zig.

const std = @import("std");
const Data = @import("../data.zig").Data;
const Io = std.Io;

pub const Entry = struct {
    name: []const u8,
    oracle: []const u8,
    run: *const fn (*const Data, *Io.Writer) anyerror!void,
};

pub const t1 = @import("t1.zig");
pub const t2 = @import("t2.zig");
pub const t3 = @import("t3.zig");
pub const t4 = @import("t4.zig");
pub const t5 = @import("t5.zig");
pub const t6 = @import("t6.zig");

/// Static-len concat of all chunks. Tasks add to it via comptime.
pub const ENTRIES: []const Entry = t1.ENTRIES ++ t2.ENTRIES ++ t3.ENTRIES ++ t4.ENTRIES ++ t5.ENTRIES ++ t6.ENTRIES;
