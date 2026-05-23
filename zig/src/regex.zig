// Named regex patterns — one `pub const` per unique Rust `r"..."` in the JOB
// suite. Match values are static (slices live in rodata), no allocation.
//
// Convention for names:
//   - alphanumerics from the pattern with `_` separators
//   - `pre_X`        for `^X` (prefix)
//   - `X_dot_Y`      for `X.*Y`
//   - `paren_X`      for `\(X\)`
//   - `caret_X` only if disambiguation needed

const Match = @import("engine.zig").Match;

// ---- single substring ---------------------------------------------------
pub const film: Match           = .{ .seq = &.{"Film"} };
pub const warner: Match         = .{ .seq = &.{"Warner"} };
pub const loser: Match          = .{ .seq = &.{"Loser"} };
pub const champion: Match       = .{ .seq = &.{"Champion"} };
pub const complete: Match       = .{ .seq = &.{"complete"} };
pub const follow: Match         = .{ .seq = &.{"follow"} };
pub const follows: Match        = .{ .seq = &.{"follows"} };
pub const internet: Match       = .{ .seq = &.{"internet"} };
pub const sequel: Match         = .{ .seq = &.{"sequel"} };
pub const murder_lc: Match      = .{ .seq = &.{"murder"} };
pub const murder_uc: Match      = .{ .seq = &.{"Murder"} };
pub const mord: Match           = .{ .seq = &.{"Mord"} };
pub const money: Match          = .{ .seq = &.{"Money"} };
pub const movie: Match          = .{ .seq = &.{"Movie"} };
pub const tim: Match            = .{ .seq = &.{"Tim"} };
pub const yo: Match             = .{ .seq = &.{"Yo"} };
pub const yu: Match             = .{ .seq = &.{"Yu"} };
pub const ang: Match            = .{ .seq = &.{"Ang"} };
pub const angel: Match          = .{ .seq = &.{"Angel"} };
pub const an_uc: Match          = .{ .seq = &.{"An"} };
pub const a_lc: Match           = .{ .seq = &.{"a"} };
pub const b_uc: Match           = .{ .seq = &.{"B"} };
pub const bert: Match           = .{ .seq = &.{"Bert"} };
pub const freddy: Match         = .{ .seq = &.{"Freddy"} };
pub const jason: Match          = .{ .seq = &.{"Jason"} };
pub const sherlock: Match       = .{ .seq = &.{"Sherlock"} };
pub const comma: Match          = .{ .seq = &.{","} };
pub const comma_space: Match    = .{ .seq = &.{", "} };

// ---- escaped-parens substrings -----------------------------------------
pub const paren_1994: Match     = .{ .seq = &.{"(1994)"} };
pub const paren_2006: Match     = .{ .seq = &.{"(2006)"} };
pub const paren_2007: Match     = .{ .seq = &.{"(2007)"} };
pub const paren_blu_ray: Match  = .{ .seq = &.{"(Blu-ray)"} };
pub const paren_coprod: Match   = .{ .seq = &.{"(co-production)"} };
pub const paren_france: Match   = .{ .seq = &.{"(France)"} };
pub const paren_japan: Match    = .{ .seq = &.{"(Japan)"} };
pub const paren_presents: Match = .{ .seq = &.{"(presents)"} };
pub const paren_producer: Match = .{ .seq = &.{"(producer)"} };
pub const paren_theatrical: Match = .{ .seq = &.{"(theatrical)"} };
pub const paren_TV: Match       = .{ .seq = &.{"(TV)"} };
pub const paren_uncredited: Match = .{ .seq = &.{"(uncredited)"} };
pub const paren_USA: Match      = .{ .seq = &.{"(USA)"} };
pub const paren_VHS: Match      = .{ .seq = &.{"(VHS)"} };
pub const paren_voice: Match    = .{ .seq = &.{"(voice)"} };
pub const paren_worldwide: Match = .{ .seq = &.{"(worldwide)"} };
pub const paren_mgm: Match      = .{ .seq = &.{"(as Metro-Goldwyn-Mayer Pictures)"} };

// `\(200.*\)` → "(200" then ")"
pub const paren_200_dot: Match  = .{ .seq = &.{ "(200", ")" } };

// ---- prefix `^X` --------------------------------------------------------
pub const pre_A: Match          = .{ .pre = "A" };
pub const pre_B: Match          = .{ .pre = "B" };
pub const pre_D: Match          = .{ .pre = "D" };
pub const pre_X: Match          = .{ .pre = "X" };
pub const pre_Z: Match          = .{ .pre = "Z" };
pub const pre_birdemic: Match   = .{ .pre = "Birdemic" };
pub const pre_champion: Match   = .{ .pre = "Champion" };
pub const pre_complete: Match   = .{ .pre = "complete" };
pub const pre_dragon_ball_z: Match = .{ .pre = "Dragon Ball Z" };
pub const pre_kung_fu_panda: Match = .{ .pre = "Kung Fu Panda" };
pub const pre_lionsgate: Match  = .{ .pre = "Lionsgate" };
pub const pre_loser: Match      = .{ .pre = "Loser" };
pub const pre_one_piece: Match  = .{ .pre = "One Piece" };
pub const pre_saw: Match        = .{ .pre = "Saw" };
pub const pre_20cf: Match       = .{ .pre = "20th Century Fox" };
pub const pre_twentieth_cf: Match = .{ .pre = "Twentieth Century Fox" };
pub const pre_vampire: Match    = .{ .pre = "Vampire" };

// ---- two-substring sequence `X.*Y` --------------------------------------
pub const downey_robert: Match  = .{ .seq = &.{ "Downey", "Robert" } };
pub const iron_man: Match       = .{ .seq = &.{ "Iron", "Man" } };
pub const tony_stark: Match     = .{ .seq = &.{ "Tony", "Stark" } };

// ---- three-substring sequence -------------------------------------------
pub const kung_fu_panda_dot: Match = .{ .seq = &.{ "Kung", "Fu", "Panda" } };

// ---- prefix-then-sub `^X:.* Y` -----------------------------------------
pub const usa_dot_space_199: Match  = .{ .pre_seq = .{ .pre = "USA:", .seq = &.{" 199"} } };
pub const usa_dot_space_200: Match  = .{ .pre_seq = .{ .pre = "USA:", .seq = &.{" 200"} } };
pub const usa_dot_200: Match        = .{ .pre_seq = .{ .pre = "USA:", .seq = &.{"200"} } };
pub const usa_dot_2008: Match       = .{ .pre_seq = .{ .pre = "USA:", .seq = &.{"2008"} } };
pub const usa_dot_201: Match        = .{ .pre_seq = .{ .pre = "USA:", .seq = &.{"201"} } };
pub const japan_dot_200: Match      = .{ .pre_seq = .{ .pre = "Japan:", .seq = &.{"200"} } };
pub const japan_dot_2007: Match     = .{ .pre_seq = .{ .pre = "Japan:", .seq = &.{"2007"} } };
pub const japan_dot_201: Match      = .{ .pre_seq = .{ .pre = "Japan:", .seq = &.{"201"} } };

// ---- disjunctions -------------------------------------------------------
pub const a_or_pre_A: Match = .{ .any_of = &.{
    .{ .seq = &.{"a"} },
    .{ .pre = "A" },
} };
// `[Mm]an` = substring "Man" or "man"
pub const class_Man_an: Match = .{ .any_of = &.{
    .{ .seq = &.{"Man"} },
    .{ .seq = &.{"man"} },
} };
