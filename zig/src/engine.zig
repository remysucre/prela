// Prela engine — Zig 0.16
//
// Top-down CPS over typed query nodes. Continuations are "sink structs": tiny
// values with a `pub inline fn call(self, ...)` method, passed via `anytype`.
// With every method `inline fn` and every type comptime-known, the chain
// fuses into one flat loop nest at `zig build -Doptimize=ReleaseFast`.
//
// Two trait-shaped contracts (no actual traits in Zig — duck typing via
// `anytype` + `comptime @hasDecl`):
//
//   Query  — has `R: type`, `drive(self, sink)`, `probe(self, key, sink)`,
//            `probeAny(self, key, pred) bool`. Yields (key, value) pairs.
//            The sink's `call` takes (i64, R).
//   SetQ   — has `driveKeys(self, sink)`, `member(self, key) bool`.
//            The sink's `callKey` takes i64.
//
// Method dispatch:
//   on a Query receiver:  `.o` = compose ; `.k` = keys ; `.x` = prod ;
//                          `.eq/.ne/.gt/.lt/.ge/.le/.in_v/.in_s/.rx/.nrx`
//   on a SetQ receiver:   `.o` = restrict ; `.and/.or/.minus`
//
// `.o` is the single algebraic composition: Query∘Query or SetQ∘Query, picked
// by which method is defined on `self`'s type. Same insight as the Rust port.

const std = @import("std");

// ===== leaf: Vec1<R> — total 1:1 dense ==================================
//
// `values[i]` is the single value for entity id `i`. Slot 0 is sentinel.

pub fn Vec1(comptime R_: type) type {
    return struct {
        values: []const R_,
        const Self = @This();
        pub const isQuery = true;
        pub const R = R_;

        pub fn drive(self: *const Self, sink: anytype) void {
            var i: usize = 1;
            while (i < self.values.len) : (i += 1) {
                sink.call(@as(i64, @intCast(i)), self.values[i]);
            }
        }
        pub fn probe(self: *const Self, key: i64, sink: anytype) void {
            const i: usize = @intCast(key);
            if (i >= 1 and i < self.values.len) {
                sink.call(self.values[i]);
            }
        }
        pub fn probeAny(self: *const Self, key: i64, pred: anytype) bool {
            const i: usize = @intCast(key);
            return i >= 1 and i < self.values.len and pred.call(self.values[i]);
        }

        // ---- Query method-chain operators ----
        pub inline fn k(self: Self) Keys(Self) { return .{ .q = self }; }
        pub inline fn o(self: Self, b: anytype) Compose(Self, @TypeOf(b)) { return .{ .a = self, .b = b }; }
        pub inline fn x(self: Self, b: anytype) Prod(Self, @TypeOf(b)) { return .{ .a = self, .b = b }; }
        pub inline fn eq(self: Self, v: R) Filter(Self, Eq(R)) { return .{ .a = self, .p = .{ .v = v } }; }
        pub inline fn ne(self: Self, v: R) Filter(Self, Ne(R)) { return .{ .a = self, .p = .{ .v = v } }; }
        pub inline fn gt(self: Self, v: R) Filter(Self, Gt(R)) { return .{ .a = self, .p = .{ .v = v } }; }
        pub inline fn lt(self: Self, v: R) Filter(Self, Lt(R)) { return .{ .a = self, .p = .{ .v = v } }; }
        pub inline fn ge(self: Self, v: R) Filter(Self, Ge(R)) { return .{ .a = self, .p = .{ .v = v } }; }
        pub inline fn le(self: Self, v: R) Filter(Self, Le(R)) { return .{ .a = self, .p = .{ .v = v } }; }
        pub inline fn in_v(self: Self, vs: []const R) Filter(Self, InVec(R)) { return .{ .a = self, .p = .{ .vs = vs } }; }
        pub inline fn in_s(self: Self, s: anytype) Filter(Self, InSet(@TypeOf(s))) { return .{ .a = self, .p = .{ .s = s } }; }
        pub inline fn rx(self: Self, m: Match) Filter(Self, Rx) { return .{ .a = self, .p = .{ .m = m } }; }
        pub inline fn nrx(self: Self, m: Match) Filter(Self, Nrx) { return .{ .a = self, .p = .{ .m = m } }; }
    };
}

// ===== leaf: Many<R> — multi-valued / partial, dense forward index ======

pub fn Many(comptime R_: type) type {
    return struct {
        fwd: []const []const R_,
        const Self = @This();
        pub const isQuery = true;
        pub const R = R_;

        pub fn drive(self: *const Self, sink: anytype) void {
            var i: usize = 1;
            while (i < self.fwd.len) : (i += 1) {
                for (self.fwd[i]) |v| {
                    sink.call(@as(i64, @intCast(i)), v);
                }
            }
        }
        pub fn probe(self: *const Self, key: i64, sink: anytype) void {
            const i: usize = @intCast(key);
            if (i >= 1 and i < self.fwd.len) {
                for (self.fwd[i]) |v| {
                    sink.call(v);
                }
            }
        }
        pub fn probeAny(self: *const Self, key: i64, pred: anytype) bool {
            const i: usize = @intCast(key);
            if (i >= 1 and i < self.fwd.len) {
                for (self.fwd[i]) |v| {
                    if (pred.call(v)) return true;
                }
            }
            return false;
        }

        pub inline fn k(self: Self) Keys(Self) { return .{ .q = self }; }
        pub inline fn o(self: Self, b: anytype) Compose(Self, @TypeOf(b)) { return .{ .a = self, .b = b }; }
        pub inline fn x(self: Self, b: anytype) Prod(Self, @TypeOf(b)) { return .{ .a = self, .b = b }; }
        pub inline fn eq(self: Self, v: R) Filter(Self, Eq(R)) { return .{ .a = self, .p = .{ .v = v } }; }
        pub inline fn ne(self: Self, v: R) Filter(Self, Ne(R)) { return .{ .a = self, .p = .{ .v = v } }; }
        pub inline fn gt(self: Self, v: R) Filter(Self, Gt(R)) { return .{ .a = self, .p = .{ .v = v } }; }
        pub inline fn lt(self: Self, v: R) Filter(Self, Lt(R)) { return .{ .a = self, .p = .{ .v = v } }; }
        pub inline fn ge(self: Self, v: R) Filter(Self, Ge(R)) { return .{ .a = self, .p = .{ .v = v } }; }
        pub inline fn le(self: Self, v: R) Filter(Self, Le(R)) { return .{ .a = self, .p = .{ .v = v } }; }
        pub inline fn in_v(self: Self, vs: []const R) Filter(Self, InVec(R)) { return .{ .a = self, .p = .{ .vs = vs } }; }
        pub inline fn in_s(self: Self, s: anytype) Filter(Self, InSet(@TypeOf(s))) { return .{ .a = self, .p = .{ .s = s } }; }
        pub inline fn rx(self: Self, m: Match) Filter(Self, Rx) { return .{ .a = self, .p = .{ .m = m } }; }
        pub inline fn nrx(self: Self, m: Match) Filter(Self, Nrx) { return .{ .a = self, .p = .{ .m = m } }; }
    };
}

// ===== Universe — SetQ over [1, n] ======================================

pub const Universe = struct {
    n: i64,
    pub const isSetQ = true;

    pub fn driveKeys(self: Universe, sink: anytype) void {
        var i: i64 = 1;
        while (i <= self.n) : (i += 1) {
            sink.callKey(i);
        }
    }
    pub fn member(self: Universe, key: i64) bool {
        return key >= 1 and key <= self.n;
    }

    // SetQ method-chain operators
    pub inline fn o(self: Universe, b: anytype) Restrict(Universe, @TypeOf(b)) { return .{ .a = self, .b = b }; }
    pub inline fn @"and"(self: Universe, b: anytype) Conj(Universe, @TypeOf(b)) { return .{ .a = self, .b = b }; }
    pub inline fn @"or"(self: Universe, b: anytype) Disj(Universe, @TypeOf(b)) { return .{ .a = self, .b = b }; }
    pub inline fn minus(self: Universe, b: anytype) SetDiff(Universe, @TypeOf(b)) { return .{ .a = self, .b = b }; }
};

// ===== Compose<A, B> — Query ∘ Query, bridge = value ====================

pub fn Compose(comptime A: type, comptime B: type) type {
    return struct {
        a: A,
        b: B,
        const Self = @This();
        pub const isQuery = true;
        pub const R = B.R;

        pub fn drive(self: *const Self, sink: anytype) void {
            const Sink = @TypeOf(sink);
            const Mid = struct {
                outer: Sink,
                b_ref: *const B,
                pub inline fn call(s: @This(), key: i64, m: A.R) void {
                    const Inner = struct {
                        outer2: Sink,
                        key_: i64,
                        pub inline fn call(t: @This(), r: B.R) void {
                            t.outer2.call(t.key_, r);
                        }
                    };
                    s.b_ref.probe(m, Inner{ .outer2 = s.outer, .key_ = key });
                }
            };
            self.a.drive(Mid{ .outer = sink, .b_ref = &self.b });
        }

        pub fn probe(self: *const Self, key: i64, sink: anytype) void {
            const Sink = @TypeOf(sink);
            const Mid = struct {
                outer: Sink,
                b_ref: *const B,
                pub inline fn call(s: @This(), m: A.R) void {
                    s.b_ref.probe(m, s.outer);
                }
            };
            self.a.probe(key, Mid{ .outer = sink, .b_ref = &self.b });
        }

        pub fn probeAny(self: *const Self, key: i64, pred: anytype) bool {
            const P = @TypeOf(pred);
            const Mid = struct {
                outer: P,
                b_ref: *const B,
                pub inline fn call(s: @This(), m: A.R) bool {
                    return s.b_ref.probeAny(m, s.outer);
                }
            };
            return self.a.probeAny(key, Mid{ .outer = pred, .b_ref = &self.b });
        }

        pub inline fn k(self: Self) Keys(Self) { return .{ .q = self }; }
        pub inline fn o(self: Self, b: anytype) Compose(Self, @TypeOf(b)) { return .{ .a = self, .b = b }; }
        pub inline fn x(self: Self, b: anytype) Prod(Self, @TypeOf(b)) { return .{ .a = self, .b = b }; }
        pub inline fn eq(self: Self, v: R) Filter(Self, Eq(R)) { return .{ .a = self, .p = .{ .v = v } }; }
        pub inline fn ne(self: Self, v: R) Filter(Self, Ne(R)) { return .{ .a = self, .p = .{ .v = v } }; }
        pub inline fn gt(self: Self, v: R) Filter(Self, Gt(R)) { return .{ .a = self, .p = .{ .v = v } }; }
        pub inline fn lt(self: Self, v: R) Filter(Self, Lt(R)) { return .{ .a = self, .p = .{ .v = v } }; }
        pub inline fn ge(self: Self, v: R) Filter(Self, Ge(R)) { return .{ .a = self, .p = .{ .v = v } }; }
        pub inline fn le(self: Self, v: R) Filter(Self, Le(R)) { return .{ .a = self, .p = .{ .v = v } }; }
        pub inline fn in_v(self: Self, vs: []const R) Filter(Self, InVec(R)) { return .{ .a = self, .p = .{ .vs = vs } }; }
        pub inline fn in_s(self: Self, s: anytype) Filter(Self, InSet(@TypeOf(s))) { return .{ .a = self, .p = .{ .s = s } }; }
        pub inline fn rx(self: Self, m: Match) Filter(Self, Rx) { return .{ .a = self, .p = .{ .m = m } }; }
        pub inline fn nrx(self: Self, m: Match) Filter(Self, Nrx) { return .{ .a = self, .p = .{ .m = m } }; }
    };
}

// ===== Filter<A, P> — value-side filter via predicate ===================

pub fn Filter(comptime A: type, comptime P: type) type {
    return struct {
        a: A,
        p: P,
        const Self = @This();
        pub const isQuery = true;
        pub const R = A.R;

        pub fn drive(self: *const Self, sink: anytype) void {
            const Sink = @TypeOf(sink);
            const Mid = struct {
                outer: Sink,
                p_ref: *const P,
                pub inline fn call(s: @This(), key: i64, v: A.R) void {
                    if (s.p_ref.test_(v)) s.outer.call(key, v);
                }
            };
            self.a.drive(Mid{ .outer = sink, .p_ref = &self.p });
        }

        pub fn probe(self: *const Self, key: i64, sink: anytype) void {
            const Sink = @TypeOf(sink);
            const Mid = struct {
                outer: Sink,
                p_ref: *const P,
                pub inline fn call(s: @This(), v: A.R) void {
                    if (s.p_ref.test_(v)) s.outer.call(v);
                }
            };
            self.a.probe(key, Mid{ .outer = sink, .p_ref = &self.p });
        }

        pub fn probeAny(self: *const Self, key: i64, pred: anytype) bool {
            const Pr = @TypeOf(pred);
            const Mid = struct {
                outer: Pr,
                p_ref: *const P,
                pub inline fn call(s: @This(), v: A.R) bool {
                    return s.p_ref.test_(v) and s.outer.call(v);
                }
            };
            return self.a.probeAny(key, Mid{ .outer = pred, .p_ref = &self.p });
        }

        pub inline fn k(self: Self) Keys(Self) { return .{ .q = self }; }
        pub inline fn o(self: Self, b: anytype) Compose(Self, @TypeOf(b)) { return .{ .a = self, .b = b }; }
        pub inline fn x(self: Self, b: anytype) Prod(Self, @TypeOf(b)) { return .{ .a = self, .b = b }; }
        pub inline fn eq(self: Self, v: R) Filter(Self, Eq(R)) { return .{ .a = self, .p = .{ .v = v } }; }
        pub inline fn ne(self: Self, v: R) Filter(Self, Ne(R)) { return .{ .a = self, .p = .{ .v = v } }; }
        pub inline fn gt(self: Self, v: R) Filter(Self, Gt(R)) { return .{ .a = self, .p = .{ .v = v } }; }
        pub inline fn lt(self: Self, v: R) Filter(Self, Lt(R)) { return .{ .a = self, .p = .{ .v = v } }; }
        pub inline fn ge(self: Self, v: R) Filter(Self, Ge(R)) { return .{ .a = self, .p = .{ .v = v } }; }
        pub inline fn le(self: Self, v: R) Filter(Self, Le(R)) { return .{ .a = self, .p = .{ .v = v } }; }
        pub inline fn in_v(self: Self, vs: []const R) Filter(Self, InVec(R)) { return .{ .a = self, .p = .{ .vs = vs } }; }
        pub inline fn in_s(self: Self, s: anytype) Filter(Self, InSet(@TypeOf(s))) { return .{ .a = self, .p = .{ .s = s } }; }
        pub inline fn rx(self: Self, m: Match) Filter(Self, Rx) { return .{ .a = self, .p = .{ .m = m } }; }
        pub inline fn nrx(self: Self, m: Match) Filter(Self, Nrx) { return .{ .a = self, .p = .{ .m = m } }; }
    };
}

// ===== Restrict<A:SetQ, B:Query> — SetQ ∘ Query, bridge = key ==========

pub fn Restrict(comptime A: type, comptime B: type) type {
    return struct {
        a: A,
        b: B,
        const Self = @This();
        pub const isQuery = true;
        pub const R = B.R;

        pub fn drive(self: *const Self, sink: anytype) void {
            const Sink = @TypeOf(sink);
            const Mid = struct {
                outer: Sink,
                b_ref: *const B,
                pub inline fn callKey(s: @This(), key: i64) void {
                    const Inner = struct {
                        outer2: Sink,
                        key_: i64,
                        pub inline fn call(t: @This(), r: B.R) void {
                            t.outer2.call(t.key_, r);
                        }
                    };
                    s.b_ref.probe(key, Inner{ .outer2 = s.outer, .key_ = key });
                }
            };
            self.a.driveKeys(Mid{ .outer = sink, .b_ref = &self.b });
        }

        pub fn probe(self: *const Self, key: i64, sink: anytype) void {
            if (self.a.member(key)) self.b.probe(key, sink);
        }

        pub fn probeAny(self: *const Self, key: i64, pred: anytype) bool {
            return self.a.member(key) and self.b.probeAny(key, pred);
        }

        pub inline fn k(self: Self) Keys(Self) { return .{ .q = self }; }
        pub inline fn o(self: Self, b: anytype) Compose(Self, @TypeOf(b)) { return .{ .a = self, .b = b }; }
        pub inline fn x(self: Self, b: anytype) Prod(Self, @TypeOf(b)) { return .{ .a = self, .b = b }; }
        pub inline fn eq(self: Self, v: R) Filter(Self, Eq(R)) { return .{ .a = self, .p = .{ .v = v } }; }
        pub inline fn ne(self: Self, v: R) Filter(Self, Ne(R)) { return .{ .a = self, .p = .{ .v = v } }; }
        pub inline fn gt(self: Self, v: R) Filter(Self, Gt(R)) { return .{ .a = self, .p = .{ .v = v } }; }
        pub inline fn lt(self: Self, v: R) Filter(Self, Lt(R)) { return .{ .a = self, .p = .{ .v = v } }; }
        pub inline fn ge(self: Self, v: R) Filter(Self, Ge(R)) { return .{ .a = self, .p = .{ .v = v } }; }
        pub inline fn le(self: Self, v: R) Filter(Self, Le(R)) { return .{ .a = self, .p = .{ .v = v } }; }
        pub inline fn in_v(self: Self, vs: []const R) Filter(Self, InVec(R)) { return .{ .a = self, .p = .{ .vs = vs } }; }
        pub inline fn in_s(self: Self, s: anytype) Filter(Self, InSet(@TypeOf(s))) { return .{ .a = self, .p = .{ .s = s } }; }
        pub inline fn rx(self: Self, m: Match) Filter(Self, Rx) { return .{ .a = self, .p = .{ .m = m } }; }
        pub inline fn nrx(self: Self, m: Match) Filter(Self, Nrx) { return .{ .a = self, .p = .{ .m = m } }; }
    };
}

// ===== Keys<Q> — Query → SetQ (forget value) ============================

pub fn Keys(comptime Q: type) type {
    return struct {
        q: Q,
        const Self = @This();
        pub const isSetQ = true;

        pub fn driveKeys(self: *const Self, sink: anytype) void {
            const Sink = @TypeOf(sink);
            const Mid = struct {
                outer: Sink,
                pub inline fn call(s: @This(), key: i64, _: Q.R) void { s.outer.callKey(key); }
            };
            self.q.drive(Mid{ .outer = sink });
        }

        pub fn member(self: *const Self, key: i64) bool {
            const T = struct {
                pub inline fn call(_: @This(), _: Q.R) bool { return true; }
            };
            return self.q.probeAny(key, T{});
        }

        pub inline fn o(self: Self, b: anytype) Restrict(Self, @TypeOf(b)) { return .{ .a = self, .b = b }; }
        pub inline fn @"and"(self: Self, b: anytype) Conj(Self, @TypeOf(b)) { return .{ .a = self, .b = b }; }
        pub inline fn @"or"(self: Self, b: anytype) Disj(Self, @TypeOf(b)) { return .{ .a = self, .b = b }; }
        pub inline fn minus(self: Self, b: anytype) SetDiff(Self, @TypeOf(b)) { return .{ .a = self, .b = b }; }
    };
}

// ===== Conj<A:SetQ, B:SetQ> — SetQ ∧ SetQ =============================

pub fn Conj(comptime A: type, comptime B: type) type {
    return struct {
        a: A,
        b: B,
        const Self = @This();
        pub const isSetQ = true;

        pub fn driveKeys(self: *const Self, sink: anytype) void {
            const Sink = @TypeOf(sink);
            const Mid = struct {
                outer: Sink,
                b_ref: *const B,
                pub inline fn callKey(s: @This(), key: i64) void {
                    if (s.b_ref.member(key)) s.outer.callKey(key);
                }
            };
            self.a.driveKeys(Mid{ .outer = sink, .b_ref = &self.b });
        }

        pub fn member(self: *const Self, key: i64) bool {
            return self.a.member(key) and self.b.member(key);
        }

        pub inline fn o(self: Self, b: anytype) Restrict(Self, @TypeOf(b)) { return .{ .a = self, .b = b }; }
        pub inline fn @"and"(self: Self, b: anytype) Conj(Self, @TypeOf(b)) { return .{ .a = self, .b = b }; }
        pub inline fn @"or"(self: Self, b: anytype) Disj(Self, @TypeOf(b)) { return .{ .a = self, .b = b }; }
        pub inline fn minus(self: Self, b: anytype) SetDiff(Self, @TypeOf(b)) { return .{ .a = self, .b = b }; }
    };
}

// ===== Disj<A:SetQ, B:SetQ> — SetQ ∨ SetQ =============================

pub fn Disj(comptime A: type, comptime B: type) type {
    return struct {
        a: A,
        b: B,
        const Self = @This();
        pub const isSetQ = true;

        pub fn driveKeys(self: *const Self, sink: anytype) void {
            const Sink = @TypeOf(sink);
            // drive A, then drive B emitting only keys not already in A.
            self.a.driveKeys(sink);
            const Mid = struct {
                outer: Sink,
                a_ref: *const A,
                pub inline fn callKey(s: @This(), key: i64) void {
                    if (!s.a_ref.member(key)) s.outer.callKey(key);
                }
            };
            self.b.driveKeys(Mid{ .outer = sink, .a_ref = &self.a });
        }

        pub fn member(self: *const Self, key: i64) bool {
            return self.a.member(key) or self.b.member(key);
        }

        pub inline fn o(self: Self, b: anytype) Restrict(Self, @TypeOf(b)) { return .{ .a = self, .b = b }; }
        pub inline fn @"and"(self: Self, b: anytype) Conj(Self, @TypeOf(b)) { return .{ .a = self, .b = b }; }
        pub inline fn @"or"(self: Self, b: anytype) Disj(Self, @TypeOf(b)) { return .{ .a = self, .b = b }; }
        pub inline fn minus(self: Self, b: anytype) SetDiff(Self, @TypeOf(b)) { return .{ .a = self, .b = b }; }
    };
}

// ===== SetDiff<A:SetQ, B:SetQ> — SetQ - SetQ ===========================

pub fn SetDiff(comptime A: type, comptime B: type) type {
    return struct {
        a: A,
        b: B,
        const Self = @This();
        pub const isSetQ = true;

        pub fn driveKeys(self: *const Self, sink: anytype) void {
            const Sink = @TypeOf(sink);
            const Mid = struct {
                outer: Sink,
                b_ref: *const B,
                pub inline fn callKey(s: @This(), key: i64) void {
                    if (!s.b_ref.member(key)) s.outer.callKey(key);
                }
            };
            self.a.driveKeys(Mid{ .outer = sink, .b_ref = &self.b });
        }

        pub fn member(self: *const Self, key: i64) bool {
            return self.a.member(key) and !self.b.member(key);
        }

        pub inline fn o(self: Self, b: anytype) Restrict(Self, @TypeOf(b)) { return .{ .a = self, .b = b }; }
        pub inline fn @"and"(self: Self, b: anytype) Conj(Self, @TypeOf(b)) { return .{ .a = self, .b = b }; }
        pub inline fn @"or"(self: Self, b: anytype) Disj(Self, @TypeOf(b)) { return .{ .a = self, .b = b }; }
        pub inline fn minus(self: Self, b: anytype) SetDiff(Self, @TypeOf(b)) { return .{ .a = self, .b = b }; }
    };
}

// ===== Prod<A:Query, B:Query> — × (Cartesian per key) ==================

pub fn Prod(comptime A: type, comptime B: type) type {
    return struct {
        a: A,
        b: B,
        const Self = @This();
        pub const isQuery = true;
        pub const R = struct { a: A.R, b: B.R };  // anonymous tuple-as-struct

        pub fn drive(self: *const Self, sink: anytype) void {
            const Sink = @TypeOf(sink);
            const Mid = struct {
                outer: Sink,
                b_ref: *const B,
                pub inline fn call(s: @This(), key: i64, va: A.R) void {
                    const Inner = struct {
                        outer2: Sink,
                        key_: i64,
                        va_: A.R,
                        pub inline fn call(t: @This(), vb: B.R) void {
                            t.outer2.call(t.key_, R{ .a = t.va_, .b = vb });
                        }
                    };
                    s.b_ref.probe(key, Inner{ .outer2 = s.outer, .key_ = key, .va_ = va });
                }
            };
            self.a.drive(Mid{ .outer = sink, .b_ref = &self.b });
        }

        pub fn probe(self: *const Self, key: i64, sink: anytype) void {
            const Sink = @TypeOf(sink);
            const RR = Self.R;
            const Mid = struct {
                outer: Sink,
                b_ref: *const B,
                key_: i64,
                pub inline fn call(s: @This(), va: A.R) void {
                    const Inner = struct {
                        outer2: Sink,
                        va_: A.R,
                        pub inline fn call(t: @This(), vb: B.R) void {
                            t.outer2.call(RR{ .a = t.va_, .b = vb });
                        }
                    };
                    s.b_ref.probe(s.key_, Inner{ .outer2 = s.outer, .va_ = va });
                }
            };
            self.a.probe(key, Mid{ .outer = sink, .b_ref = &self.b, .key_ = key });
        }

        pub fn probeAny(self: *const Self, key: i64, pred: anytype) bool {
            const Pr = @TypeOf(pred);
            const RR = Self.R;
            const Mid = struct {
                outer: Pr,
                b_ref: *const B,
                key_: i64,
                pub inline fn call(s: @This(), va: A.R) bool {
                    const Inner = struct {
                        outer2: Pr,
                        va_: A.R,
                        pub inline fn call(t: @This(), vb: B.R) bool {
                            return t.outer2.call(RR{ .a = t.va_, .b = vb });
                        }
                    };
                    return s.b_ref.probeAny(s.key_, Inner{ .outer2 = s.outer, .va_ = va });
                }
            };
            return self.a.probeAny(key, Mid{ .outer = pred, .b_ref = &self.b, .key_ = key });
        }

        pub inline fn k(self: Self) Keys(Self) { return .{ .q = self }; }
        pub inline fn x(self: Self, b: anytype) Prod(Self, @TypeOf(b)) { return .{ .a = self, .b = b }; }
    };
}

// ===== Predicates =======================================================

pub fn Eq(comptime R: type) type {
    return struct {
        v: R,
        pub inline fn test_(self: @This(), val: R) bool {
            return if (R == []const u8) std.mem.eql(u8, val, self.v) else val == self.v;
        }
    };
}
pub fn Ne(comptime R: type) type {
    return struct {
        v: R,
        pub inline fn test_(self: @This(), val: R) bool {
            return if (R == []const u8) !std.mem.eql(u8, val, self.v) else val != self.v;
        }
    };
}
pub fn Gt(comptime R: type) type {
    return struct {
        v: R,
        pub inline fn test_(self: @This(), val: R) bool {
            return if (R == []const u8) std.mem.order(u8, val, self.v) == .gt else val > self.v;
        }
    };
}
pub fn Lt(comptime R: type) type {
    return struct {
        v: R,
        pub inline fn test_(self: @This(), val: R) bool {
            return if (R == []const u8) std.mem.order(u8, val, self.v) == .lt else val < self.v;
        }
    };
}
pub fn Ge(comptime R: type) type {
    return struct {
        v: R,
        pub inline fn test_(self: @This(), val: R) bool {
            return if (R == []const u8) std.mem.order(u8, val, self.v) != .lt else val >= self.v;
        }
    };
}
pub fn Le(comptime R: type) type {
    return struct {
        v: R,
        pub inline fn test_(self: @This(), val: R) bool {
            return if (R == []const u8) std.mem.order(u8, val, self.v) != .gt else val <= self.v;
        }
    };
}
pub fn InVec(comptime R: type) type {
    return struct {
        vs: []const R,
        pub inline fn test_(self: @This(), val: R) bool {
            for (self.vs) |vi| {
                if (R == []const u8) {
                    if (std.mem.eql(u8, val, vi)) return true;
                } else {
                    if (val == vi) return true;
                }
            }
            return false;
        }
    };
}
pub fn InSet(comptime S: type) type {
    return struct {
        s: S,
        pub inline fn test_(self: @This(), val: i64) bool {
            return self.s.member(val);
        }
    };
}

// ===== Match — minimal regex matcher ====================================
//
// Tagged union covering every pattern shape in the JOB suite:
//   .seq      — contains all parts in order        (`r"X"`, `r"X.*Y"`, `r"X.*Y.*Z"`, `r"\(USA\)"`)
//   .pre      — starts with this prefix             (`r"^X"`)
//   .pre_seq  — starts with .pre AND contains .seq after  (`r"^USA:.* 200"`)
//   .any_of   — disjunction (recursive)             (`r"a|^A"`, `r"[Mm]an"`)
// Patterns are static const Match values in `regex.zig`; the matcher is a
// runtime switch with `std.mem.indexOf`/`startsWith` calls.

pub const Match = union(enum) {
    seq: []const []const u8,
    pre: []const u8,
    pre_seq: struct { pre: []const u8, seq: []const []const u8 },
    any_of: []const Match,

    pub fn matches(self: Match, s: []const u8) bool {
        return switch (self) {
            .seq => |parts| containsSeq(s, parts),
            .pre => |p| std.mem.startsWith(u8, s, p),
            .pre_seq => |x| std.mem.startsWith(u8, s, x.pre) and
                containsSeq(s[x.pre.len..], x.seq),
            .any_of => |ms| blk: {
                for (ms) |m| if (m.matches(s)) break :blk true;
                break :blk false;
            },
        };
    }
};

inline fn containsSeq(s: []const u8, parts: []const []const u8) bool {
    var off: usize = 0;
    for (parts) |part| {
        const idx = std.mem.indexOf(u8, s[off..], part) orelse return false;
        off += idx + part.len;
    }
    return true;
}

// Builders used by the spike's inline patterns; the full 113 use named const
// patterns declared in `regex.zig`.
pub inline fn sub(a: []const u8) Match { return .{ .seq = &.{a} }; }
pub inline fn pre(a: []const u8) Match { return .{ .pre = a }; }
pub inline fn two(a: []const u8, b: []const u8) Match { return .{ .seq = &.{ a, b } }; }

pub const Rx = struct {
    m: Match,
    pub inline fn test_(self: @This(), val: []const u8) bool { return self.m.matches(val); }
};
pub const Nrx = struct {
    m: Match,
    pub inline fn test_(self: @This(), val: []const u8) bool { return !self.m.matches(val); }
};
