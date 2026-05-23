// Per-arity accumulators (lex-min per output column) and writers.
//
// Each query picks the AccN matching its output column count, drives the
// query into `Sink(AccN){ .acc = &acc }`, then `fmtN(w, acc.m0, ...)`.

const std = @import("std");
const Io = std.Io;

// ===== accumulators =====================================================

pub const Acc1 = struct {
    m: ?[]const u8 = null,
    pub inline fn call(self: *@This(), _: i64, v: []const u8) void {
        if (self.m == null or std.mem.order(u8, v, self.m.?) == .lt) self.m = v;
    }
};

// 2-col Prod: t = struct { a, b }
pub const Acc2 = struct {
    m0: ?[]const u8 = null,
    m1: ?[]const u8 = null,
    pub inline fn call(self: *@This(), _: i64, t: anytype) void {
        if (self.m0 == null or std.mem.order(u8, t.a, self.m0.?) == .lt) self.m0 = t.a;
        if (self.m1 == null or std.mem.order(u8, t.b, self.m1.?) == .lt) self.m1 = t.b;
    }
};

// 3-col Prod = Prod(Prod(a,b),c) → t = { a = { a, b }, b }
pub const Acc3 = struct {
    m0: ?[]const u8 = null,
    m1: ?[]const u8 = null,
    m2: ?[]const u8 = null,
    pub inline fn call(self: *@This(), _: i64, t: anytype) void {
        const c0 = t.a.a; const c1 = t.a.b; const c2 = t.b;
        if (self.m0 == null or std.mem.order(u8, c0, self.m0.?) == .lt) self.m0 = c0;
        if (self.m1 == null or std.mem.order(u8, c1, self.m1.?) == .lt) self.m1 = c1;
        if (self.m2 == null or std.mem.order(u8, c2, self.m2.?) == .lt) self.m2 = c2;
    }
};

// 4-col Prod = Prod(Prod(Prod(a,b),c),d) → t = { a = { a = {a,b}, b }, b }
pub const Acc4 = struct {
    m0: ?[]const u8 = null,
    m1: ?[]const u8 = null,
    m2: ?[]const u8 = null,
    m3: ?[]const u8 = null,
    pub inline fn call(self: *@This(), _: i64, t: anytype) void {
        const c0 = t.a.a.a; const c1 = t.a.a.b; const c2 = t.a.b; const c3 = t.b;
        if (self.m0 == null or std.mem.order(u8, c0, self.m0.?) == .lt) self.m0 = c0;
        if (self.m1 == null or std.mem.order(u8, c1, self.m1.?) == .lt) self.m1 = c1;
        if (self.m2 == null or std.mem.order(u8, c2, self.m2.?) == .lt) self.m2 = c2;
        if (self.m3 == null or std.mem.order(u8, c3, self.m3.?) == .lt) self.m3 = c3;
    }
};

// 5-col Prod = left-nested = { a = { a = { a = {a,b}, b }, b }, b }
pub const Acc5 = struct {
    m0: ?[]const u8 = null, m1: ?[]const u8 = null, m2: ?[]const u8 = null,
    m3: ?[]const u8 = null, m4: ?[]const u8 = null,
    pub inline fn call(self: *@This(), _: i64, t: anytype) void {
        const c0 = t.a.a.a.a; const c1 = t.a.a.a.b;
        const c2 = t.a.a.b;   const c3 = t.a.b; const c4 = t.b;
        if (self.m0 == null or std.mem.order(u8, c0, self.m0.?) == .lt) self.m0 = c0;
        if (self.m1 == null or std.mem.order(u8, c1, self.m1.?) == .lt) self.m1 = c1;
        if (self.m2 == null or std.mem.order(u8, c2, self.m2.?) == .lt) self.m2 = c2;
        if (self.m3 == null or std.mem.order(u8, c3, self.m3.?) == .lt) self.m3 = c3;
        if (self.m4 == null or std.mem.order(u8, c4, self.m4.?) == .lt) self.m4 = c4;
    }
};

pub const Acc6 = struct {
    m0: ?[]const u8 = null, m1: ?[]const u8 = null, m2: ?[]const u8 = null,
    m3: ?[]const u8 = null, m4: ?[]const u8 = null, m5: ?[]const u8 = null,
    pub inline fn call(self: *@This(), _: i64, t: anytype) void {
        const c0 = t.a.a.a.a.a; const c1 = t.a.a.a.a.b;
        const c2 = t.a.a.a.b;   const c3 = t.a.a.b;
        const c4 = t.a.b;       const c5 = t.b;
        if (self.m0 == null or std.mem.order(u8, c0, self.m0.?) == .lt) self.m0 = c0;
        if (self.m1 == null or std.mem.order(u8, c1, self.m1.?) == .lt) self.m1 = c1;
        if (self.m2 == null or std.mem.order(u8, c2, self.m2.?) == .lt) self.m2 = c2;
        if (self.m3 == null or std.mem.order(u8, c3, self.m3.?) == .lt) self.m3 = c3;
        if (self.m4 == null or std.mem.order(u8, c4, self.m4.?) == .lt) self.m4 = c4;
        if (self.m5 == null or std.mem.order(u8, c5, self.m5.?) == .lt) self.m5 = c5;
    }
};

// ===== sink wrapper =====================================================
//
// The acc is held by pointer so the chain can pass the Sink BY VALUE through
// nested Mid/Inner structs while mutation lands at the single Acc.

pub fn Sink(comptime Acc: type) type {
    return struct {
        acc: *Acc,
        pub inline fn call(self: @This(), x: i64, v: anytype) void {
            self.acc.call(x, v);
        }
    };
}

// ===== writers ==========================================================

pub fn fmt1(w: *Io.Writer, m: ?[]const u8) !void {
    if (m) |s| try w.print("{s}", .{s}) else try w.print("(empty)", .{});
}
pub fn fmt2(w: *Io.Writer, a: ?[]const u8, b: ?[]const u8) !void {
    if (a == null) { try w.print("(empty)", .{}); return; }
    try w.print("{s} || {s}", .{ a.?, b.? });
}
pub fn fmt3(w: *Io.Writer, a: ?[]const u8, b: ?[]const u8, c: ?[]const u8) !void {
    if (a == null) { try w.print("(empty)", .{}); return; }
    try w.print("{s} || {s} || {s}", .{ a.?, b.?, c.? });
}
pub fn fmt4(w: *Io.Writer, a: ?[]const u8, b: ?[]const u8, c: ?[]const u8, d: ?[]const u8) !void {
    if (a == null) { try w.print("(empty)", .{}); return; }
    try w.print("{s} || {s} || {s} || {s}", .{ a.?, b.?, c.?, d.? });
}
pub fn fmt5(w: *Io.Writer, a: ?[]const u8, b: ?[]const u8, c: ?[]const u8, d: ?[]const u8, e: ?[]const u8) !void {
    if (a == null) { try w.print("(empty)", .{}); return; }
    try w.print("{s} || {s} || {s} || {s} || {s}", .{ a.?, b.?, c.?, d.?, e.? });
}
pub fn fmt6(w: *Io.Writer, a: ?[]const u8, b: ?[]const u8, c: ?[]const u8, d: ?[]const u8, e: ?[]const u8, f: ?[]const u8) !void {
    if (a == null) { try w.print("(empty)", .{}); return; }
    try w.print("{s} || {s} || {s} || {s} || {s} || {s}", .{ a.?, b.?, c.?, d.?, e.?, f.? });
}
