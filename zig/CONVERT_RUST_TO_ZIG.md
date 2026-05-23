# Rust → Zig query translation guide

For each query in `/Users/remywang/projects/prela-rs/src/queries/tN.rs`, produce
`/Users/remywang/projects/prela-zig/src/queries/tN.zig` with the same fn names,
same ENTRIES, same oracles, same drive destructuring. The translation is
mechanical.

## File template

```zig
const std = @import("std");
const Io = std.Io;
const Data = @import("../data.zig").Data;
const h = @import("../helpers.zig");
const rx = @import("../regex.zig");
const sets = @import("../sets.zig");
const Entry = @import("all.zig").Entry;

pub const ENTRIES: []const Entry = &.{
    .{ .name = "2a", .oracle = "'Doc'", .run = q2a },
    // … one entry per query, same order as the Rust ENTRIES
};

fn q2a(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = /* ... */;
    var acc = h.Acc1{};
    q.drive(h.Sink(h.Acc1){ .acc = &acc });
    try h.fmt1(w, acc.m);
}
```

## Receiver convention

Rust requires explicit borrows because methods take `self` by value:
`(&d.movie_keyword).o(...)`. Zig auto-copies field values on method call:
**drop the `&` and the parens** — just write `d.movie_keyword.o(...)`.

`d.movie` (Universe — `Copy` in Rust, plain value in Zig) is bare in both.

## Operator name map

| Rust            | Zig                |
|-----------------|--------------------|
| `.o(b)`         | `.o(b)`            |
| `.k()`          | `.k()`             |
| `.x(b)`         | `.x(b)`            |
| `.and(b)`       | `.@"and"(b)`       |
| `.or(b)`        | `.@"or"(b)`        |
| `.minus(b)`     | `.minus(b)`        |
| `.eq(v)`        | `.eq(v)`           |
| `.ne / .gt / .lt / .ge / .le` | same |
| `.in_v(vs)`     | `.in_v(vs)`        |
| `.in_s(s)`      | `.in_s(s)`         |
| `.rx(r"...")`   | `.rx(rx.<name>)`   |
| `.nrx(r"...")`  | `.nrx(rx.<name>)`  |

Zig keywords `and` / `or` require `@"and"` / `@"or"` escapes.

## Constants

| Rust                  | Zig             |
|-----------------------|-----------------|
| `vec!["a", "b"]`      | `&[_][]const u8{"a","b"}` |
| `kw8()`               | `sets.kw8`      |
| `voice3()`            | `sets.voice3`   |
| `writer5()`, etc.     | `sets.writer5`  |
| etc.                  |                 |

The named sets in `sets.zig` are already-built `[]const []const u8` constants
(no fn call needed).

## Regex → named Match

Every `r"..."` maps to a `pub const` in `/Users/remywang/projects/prela-zig/src/regex.zig`.
Use `rx.<name>`. Examples:

| Rust regex                     | Zig                  |
|--------------------------------|----------------------|
| `r"^B"`                        | `rx.pre_B`           |
| `r"Downey.*Robert"`            | `rx.downey_robert`   |
| `r"\(USA\)"`                   | `rx.paren_USA`       |
| `r"\(Japan\)"`                 | `rx.paren_japan`     |
| `r"^USA:.* 200"`               | `rx.usa_dot_space_200` |
| `r"a|^A"`                      | `rx.a_or_pre_A`      |
| `r"[Mm]an"`                    | `rx.class_Man_an`    |
| `r"\(200.*\)"`                 | `rx.paren_200_dot`   |
| etc.                           | see regex.zig        |

If a regex pattern doesn't have a named const yet, ADD one at the bottom of
regex.zig (with a clear name) and then use it.

## Output: write to *Io.Writer instead of returning String

Rust:
```rust
fn q2a(d: &Data) -> String {
    let q = ...;
    let mut m: Option<&'static str> = None;
    q.drive(|_, t| update(&mut m, t));
    fmt1(m)
}
```

Zig:
```zig
fn q2a(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = ...;
    var acc = h.Acc1{};
    q.drive(h.Sink(h.Acc1){ .acc = &acc });
    try h.fmt1(w, acc.m);
}
```

The arity follows the Rust `[Option<&'static str>; K]`:
- 1 col → `Acc1`, `fmt1(w, acc.m)`
- 2 col → `Acc2`, `fmt2(w, acc.m0, acc.m1)`
- 3 col → `Acc3`, `fmt3(w, acc.m0, acc.m1, acc.m2)`
- 4 col → `Acc4`, `fmt4(w, acc.m0, acc.m1, acc.m2, acc.m3)`
- 5 col → `Acc5` (q33a etc.)
- 6 col → `Acc6` (q33b etc.)

## Numeric-output queries (q1a..q1d)

A few queries (templates 1a/1b/1c/1d) output `production_year` as i64. The
Rust uses a manual `format!`. In Zig, follow the same idiom — use a hand-rolled
accumulator + writer call. Adapt from Rust as needed; you may need to create
a custom `Acc1i` (i64) accumulator inline or reuse the existing helpers
by formatting the year to a buffer.

You can write a small local accumulator in the query function for these
i64-output cases, like:
```zig
const Acc = struct { tmin: ?[]const u8 = null, ymin: ?i64 = null,
    pub inline fn call(...) ... };
```
Then write the final formatted result manually.

## let-binding / helpers

Rust:
```rust
fn co_21<'d>(d: &'d Data) -> impl Query<R = i64> + 'd { ... }
fn q21a(d: &Data) -> String {
    let q = ... .and(co_21(d).k()) ... .o(co_21(d).o(&d.company_name)) ...;
}
```

Zig: **inline the helper expression both places** — duplicate ~5 lines is
fine. Don't try to write the return type for an `impl` Zig fn; it gets ugly
fast. Pattern:

```zig
fn q21a(d: *const Data, w: *Io.Writer) anyerror!void {
    const q = ... .@"and"(/* co_21 expression */.k())
                  .o(/* co_21 expression again */.o(d.company_name)) ...;
}
```

The cost is mechanical duplication, not behavioral.

## Nested conj/and chains

Rust `(a.and(b).and(c).and(d))` translates 1-to-1 to Zig
`(a.@"and"(b).@"and"(c).@"and"(d))`. Same shape.

## Disj `or` flatten / nesting

`.or(b)` → `.@"or"(b)`. Same shape, with the keyword escape.

## Style

- One blank line between fns.
- No per-query doc comments.
- Mirror the Rust file's ordering.
- Preserve oracle strings byte-for-byte (note escapes like `\"` are the same
  in Zig string literals).

## Verification

Don't run cargo or zig. Once all 6 chunks are written, the integration step
builds and verifies.
