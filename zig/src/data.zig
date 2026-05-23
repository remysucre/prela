// JOB data loader for the Zig spike.
//
// Reads the Julia binary cache from ../prela/cache/*.bin. Layouts:
//   bits : [u64 n][n × (i64, i64)]                              (id, id) pairs
//   str  : [u64 n][n × i64 keys][(n+1) × u32 offsets][bytes]   (id, string) pairs
//
// Strings are []const u8 slices into a permanent heap buffer holding the file
// contents — same lifetime story as the Rust port's leaked mmap.

const std = @import("std");
const Io = std.Io;
const engine = @import("engine.zig");

const Vec1 = engine.Vec1;
const Many = engine.Many;
const Universe = engine.Universe;

const cache_dir = "../cache";

const Pair = extern struct { k: i64, v: i64 };

// ---- raw loaders -------------------------------------------------------

fn readWhole(io: Io, allocator: std.mem.Allocator, name: []const u8) ![]u8 {
    var pbuf: [256]u8 = undefined;
    const path = try std.fmt.bufPrint(&pbuf, "{s}/{s}.bin", .{ cache_dir, name });
    const dir = Io.Dir.cwd();
    return try dir.readFileAlloc(io, path, allocator, .unlimited);
}

const BitsPairs = []const Pair;
fn loadBits(io: Io, allocator: std.mem.Allocator, name: []const u8) !BitsPairs {
    const bytes = try readWhole(io, allocator, name);
    const n = std.mem.readInt(u64, bytes[0..8], .little);
    const data_off: usize = 8;
    const total = @as(usize, @intCast(n));
    const slice_bytes = bytes[data_off..(data_off + total * @sizeOf(Pair))];
    const ptr: [*]const Pair = @ptrCast(@alignCast(slice_bytes.ptr));
    return ptr[0..total];
}

const StrPair = struct { k: i64, s: []const u8 };
fn loadStrs(io: Io, allocator: std.mem.Allocator, name: []const u8) ![]const StrPair {
    const bytes = try readWhole(io, allocator, name);
    const n: usize = @intCast(std.mem.readInt(u64, bytes[0..8], .little));
    const keys_off: usize = 8;
    const offsets_off = keys_off + n * 8;
    const bytes_off = offsets_off + (n + 1) * 4;

    const keys_ptr: [*]const i64 = @ptrCast(@alignCast(bytes[keys_off..].ptr));
    const offsets_ptr: [*]const u32 = @ptrCast(@alignCast(bytes[offsets_off..].ptr));
    const data: []const u8 = bytes[bytes_off..];

    const out = try allocator.alloc(StrPair, n);
    for (0..n) |i| {
        const lo: usize = offsets_ptr[i];
        const hi: usize = offsets_ptr[i + 1];
        out[i] = .{ .k = keys_ptr[i], .s = data[lo..hi] };
    }
    return out;
}

// ---- helpers -----------------------------------------------------------

fn maxSrcBits(p: BitsPairs) usize {
    var m: i64 = 0;
    for (p) |pr| if (pr.k > m) { m = pr.k; };
    return @intCast(m);
}
fn maxValBits(p: BitsPairs) usize {
    var m: i64 = 0;
    for (p) |pr| if (pr.v > m) { m = pr.v; };
    return @intCast(m);
}
fn maxSrcStrs(p: []const StrPair) usize {
    var m: i64 = 0;
    for (p) |pr| if (pr.k > m) { m = pr.k; };
    return @intCast(m);
}

fn buildVec1Bits(allocator: std.mem.Allocator, n: usize, pairs: BitsPairs) !Vec1(i64) {
    const arr = try allocator.alloc(i64, n + 1);
    @memset(arr, 0);
    for (pairs) |p| arr[@intCast(p.k)] = p.v;
    return .{ .values = arr };
}
fn buildVec1Str(allocator: std.mem.Allocator, n: usize, pairs: []const StrPair) !Vec1([]const u8) {
    const arr = try allocator.alloc([]const u8, n + 1);
    @memset(arr, "");
    for (pairs) |p| arr[@intCast(p.k)] = p.s;
    return .{ .values = arr };
}
fn buildManyBits(allocator: std.mem.Allocator, n: usize, pairs: BitsPairs) !Many(i64) {
    // first count per slot for one-shot allocation
    var counts = try allocator.alloc(usize, n + 1);
    defer allocator.free(counts);
    @memset(counts, 0);
    for (pairs) |p| {
        if (p.k >= 1 and p.k <= @as(i64, @intCast(n))) counts[@intCast(p.k)] += 1;
    }
    const fwd = try allocator.alloc([]i64, n + 1);
    for (0..n + 1) |i| fwd[i] = try allocator.alloc(i64, counts[i]);
    var idx = try allocator.alloc(usize, n + 1);
    defer allocator.free(idx);
    @memset(idx, 0);
    for (pairs) |p| {
        if (p.k >= 1 and p.k <= @as(i64, @intCast(n))) {
            const k: usize = @intCast(p.k);
            fwd[k][idx[k]] = p.v;
            idx[k] += 1;
        }
    }
    // promote []i64 to []const i64 / []const []const i64
    const fwd_const = try allocator.alloc([]const i64, n + 1);
    for (0..n + 1) |i| fwd_const[i] = fwd[i];
    return .{ .fwd = fwd_const };
}
fn buildManyStr(allocator: std.mem.Allocator, n: usize, pairs: []const StrPair) !Many([]const u8) {
    var counts = try allocator.alloc(usize, n + 1);
    defer allocator.free(counts);
    @memset(counts, 0);
    for (pairs) |p| {
        if (p.k >= 1 and p.k <= @as(i64, @intCast(n))) counts[@intCast(p.k)] += 1;
    }
    const fwd = try allocator.alloc([][]const u8, n + 1);
    for (0..n + 1) |i| fwd[i] = try allocator.alloc([]const u8, counts[i]);
    var idx = try allocator.alloc(usize, n + 1);
    defer allocator.free(idx);
    @memset(idx, 0);
    for (pairs) |p| {
        if (p.k >= 1 and p.k <= @as(i64, @intCast(n))) {
            const k: usize = @intCast(p.k);
            fwd[k][idx[k]] = p.s;
            idx[k] += 1;
        }
    }
    const fwd_const = try allocator.alloc([]const []const u8, n + 1);
    for (0..n + 1) |i| fwd_const[i] = fwd[i];
    return .{ .fwd = fwd_const };
}

// ===== the loaded dataset ===============================================

pub const Data = struct {
    movie: Universe,
    persons: Universe,

    // Movie.*
    movie_title: Vec1([]const u8),
    movie_kind: Vec1(i64),
    movie_production_year: Many(i64),
    movie_episode_nr: Many(i64),
    movie_keyword: Many(i64),
    movie_company: Many(i64),
    movie_cast: Many(i64),
    movie_info: Many(i64),
    movie_data: Many(i64),
    movie_complete_cast: Many(i64),
    movie_link: Many(i64),
    movie_linked_by: Many(i64),
    movie_aka: Many(i64),

    // Cast.*
    cast_person: Vec1(i64),
    cast_role: Vec1(i64),
    cast_note: Many([]const u8),
    cast_character: Many(i64),

    // Person.*
    person_name: Vec1([]const u8),
    person_gender: Many([]const u8),
    person_aka: Many(i64),
    person_info: Many(i64),
    person_name_pcode: Many([]const u8),

    // Keyword/Kind/RoleType/Character
    keyword_keyword: Vec1([]const u8),
    kind_kind: Vec1([]const u8),
    roletype_role: Vec1([]const u8),
    character_name: Vec1([]const u8),

    // Company/CompanyType
    company_country: Many([]const u8),
    company_name: Vec1([]const u8),
    company_note: Many([]const u8),
    company_type: Vec1(i64),
    companytype_kind: Vec1([]const u8),

    // Info/Data/PersonInfo
    info_info: Vec1([]const u8),
    info_type: Vec1(i64),
    info_note: Many([]const u8),
    infotype_info: Vec1([]const u8),
    data_data: Vec1([]const u8),
    data_type: Vec1(i64),
    personinfo_info: Vec1([]const u8),
    personinfo_type: Vec1(i64),
    personinfo_note: Many([]const u8),

    // Aka tables
    akaname_name: Vec1([]const u8),
    akatitle_title: Vec1([]const u8),

    // MovieLink / LinkType
    movielink_target: Vec1(i64),
    movielink_type: Vec1(i64),
    linktype_link: Vec1([]const u8),

    // CompleteCast / CompCastType
    completecast_status: Vec1(i64),
    completecast_subject: Vec1(i64),
    compcasttype_kind: Vec1([]const u8),

    pub fn load(io: Io, allocator: std.mem.Allocator) !Data {
        // ---- bits ----
        const py    = try loadBits(io, allocator, "Movie_production_year");
        const men   = try loadBits(io, allocator, "Movie_episode_nr");
        const mki   = try loadBits(io, allocator, "Movie_kind");
        const mk    = try loadBits(io, allocator, "Movie_keyword");
        const mcmp  = try loadBits(io, allocator, "Movie_company");
        const mcst  = try loadBits(io, allocator, "Movie_cast");
        const mif   = try loadBits(io, allocator, "Movie_info");
        const mdt   = try loadBits(io, allocator, "Movie_data");
        const mcc_  = try loadBits(io, allocator, "Movie_complete_cast");
        const mln   = try loadBits(io, allocator, "Movie_link");
        const mlnby = try loadBits(io, allocator, "Movie_linked_by");
        const mak   = try loadBits(io, allocator, "Movie_aka");
        const cp    = try loadBits(io, allocator, "Cast_person");
        const cr    = try loadBits(io, allocator, "Cast_role");
        const cch   = try loadBits(io, allocator, "Cast_character");
        const pa    = try loadBits(io, allocator, "Person_aka");
        const pif   = try loadBits(io, allocator, "Person_info");
        const cty   = try loadBits(io, allocator, "Company_type");
        const ity   = try loadBits(io, allocator, "Info_type");
        const dty   = try loadBits(io, allocator, "Data_type");
        const pity  = try loadBits(io, allocator, "PersonInfo_type");
        const mlt   = try loadBits(io, allocator, "MovieLink_target");
        const mlty  = try loadBits(io, allocator, "MovieLink_type");
        const ccst  = try loadBits(io, allocator, "CompleteCast_status");
        const ccsub = try loadBits(io, allocator, "CompleteCast_subject");

        // ---- strs ----
        const mt   = try loadStrs(io, allocator, "Movie_title");
        const kk   = try loadStrs(io, allocator, "Keyword_keyword");
        const kik  = try loadStrs(io, allocator, "Kind_kind");
        const rt   = try loadStrs(io, allocator, "RoleType_role");
        const chn  = try loadStrs(io, allocator, "Character_name");
        const cc   = try loadStrs(io, allocator, "Company_country");
        const cmn_ = try loadStrs(io, allocator, "Company_name");
        const cmnt = try loadStrs(io, allocator, "Company_note");
        const cyk  = try loadStrs(io, allocator, "CompanyType_kind");
        const ii   = try loadStrs(io, allocator, "Info_info");
        const in_  = try loadStrs(io, allocator, "Info_note");
        const ityp = try loadStrs(io, allocator, "InfoType_info");
        const dd   = try loadStrs(io, allocator, "Data_data");
        const pi   = try loadStrs(io, allocator, "PersonInfo_info");
        const pin  = try loadStrs(io, allocator, "PersonInfo_note");
        const pn   = try loadStrs(io, allocator, "Person_name");
        const pg   = try loadStrs(io, allocator, "Person_gender");
        const pnp  = try loadStrs(io, allocator, "Person_name_pcode_cf");
        const cnt  = try loadStrs(io, allocator, "Cast_note");
        const an   = try loadStrs(io, allocator, "AkaName_name");
        const at   = try loadStrs(io, allocator, "AkaTitle_title");
        const lty  = try loadStrs(io, allocator, "LinkType_link");
        const cck  = try loadStrs(io, allocator, "CompCastType_kind");

        // ---- sizes ----
        const n_movie     = @max(maxSrcStrs(mt), maxValBits(mlt));
        const n_person    = maxSrcStrs(pn);
        const n_cast      = maxSrcBits(cp);
        const n_keyword   = @max(maxSrcStrs(kk), maxValBits(mk));
        const n_kind      = @max(maxSrcStrs(kik), maxValBits(mki));
        const n_roletype  = @max(maxSrcStrs(rt), maxValBits(cr));
        const n_character = @max(maxSrcStrs(chn), maxValBits(cch));
        const n_company   = @max(maxSrcStrs(cmn_),
                            @max(maxSrcStrs(cc),
                            @max(maxSrcStrs(cmnt),
                            @max(maxSrcBits(cty),
                                 maxValBits(mcmp)))));
        const n_comptype  = @max(maxSrcStrs(cyk), maxValBits(cty));
        const n_info      = @max(maxSrcStrs(ii),
                            @max(maxSrcBits(ity),
                            @max(maxSrcStrs(in_), maxValBits(mif))));
        const n_infotype  = @max(maxSrcStrs(ityp), maxValBits(ity));
        const n_data      = @max(maxSrcStrs(dd),
                            @max(maxSrcBits(dty), maxValBits(mdt)));
        const n_pinfo     = @max(maxSrcStrs(pi),
                            @max(maxSrcBits(pity),
                            @max(maxSrcStrs(pin), maxValBits(pif))));
        const n_akaname   = @max(maxSrcStrs(an), maxValBits(pa));
        const n_akatitle  = @max(maxSrcStrs(at), maxValBits(mak));
        const n_mlink     = @max(maxSrcBits(mlt),
                            @max(maxSrcBits(mlty),
                            @max(maxValBits(mln), maxValBits(mlnby))));
        const n_ltype     = @max(maxSrcStrs(lty), maxValBits(mlty));
        const n_ccast     = @max(maxSrcBits(ccst),
                            @max(maxSrcBits(ccsub), maxValBits(mcc_)));
        const n_ccktype   = @max(maxSrcStrs(cck),
                            @max(maxValBits(ccst), maxValBits(ccsub)));

        return Data{
            .movie   = .{ .n = @intCast(n_movie) },
            .persons = .{ .n = @intCast(n_person) },

            .movie_title           = try buildVec1Str(allocator, n_movie, mt),
            .movie_kind            = try buildVec1Bits(allocator, n_movie, mki),
            .movie_production_year = try buildManyBits(allocator, n_movie, py),
            .movie_episode_nr      = try buildManyBits(allocator, n_movie, men),
            .movie_keyword         = try buildManyBits(allocator, n_movie, mk),
            .movie_company         = try buildManyBits(allocator, n_movie, mcmp),
            .movie_cast            = try buildManyBits(allocator, n_movie, mcst),
            .movie_info            = try buildManyBits(allocator, n_movie, mif),
            .movie_data            = try buildManyBits(allocator, n_movie, mdt),
            .movie_complete_cast   = try buildManyBits(allocator, n_movie, mcc_),
            .movie_link            = try buildManyBits(allocator, n_movie, mln),
            .movie_linked_by       = try buildManyBits(allocator, n_movie, mlnby),
            .movie_aka             = try buildManyBits(allocator, n_movie, mak),

            .cast_person      = try buildVec1Bits(allocator, n_cast, cp),
            .cast_role        = try buildVec1Bits(allocator, n_cast, cr),
            .cast_note        = try buildManyStr(allocator, n_cast, cnt),
            .cast_character   = try buildManyBits(allocator, n_cast, cch),

            .person_name       = try buildVec1Str(allocator, n_person, pn),
            .person_gender     = try buildManyStr(allocator, n_person, pg),
            .person_aka        = try buildManyBits(allocator, n_person, pa),
            .person_info       = try buildManyBits(allocator, n_person, pif),
            .person_name_pcode = try buildManyStr(allocator, n_person, pnp),

            .keyword_keyword = try buildVec1Str(allocator, n_keyword,   kk),
            .kind_kind       = try buildVec1Str(allocator, n_kind,      kik),
            .roletype_role   = try buildVec1Str(allocator, n_roletype,  rt),
            .character_name  = try buildVec1Str(allocator, n_character, chn),

            .company_country  = try buildManyStr(allocator, n_company, cc),
            .company_name     = try buildVec1Str(allocator, n_company, cmn_),
            .company_note     = try buildManyStr(allocator, n_company, cmnt),
            .company_type     = try buildVec1Bits(allocator, n_company, cty),
            .companytype_kind = try buildVec1Str(allocator, n_comptype, cyk),

            .info_info        = try buildVec1Str(allocator, n_info, ii),
            .info_type        = try buildVec1Bits(allocator, n_info, ity),
            .info_note        = try buildManyStr(allocator, n_info, in_),
            .infotype_info    = try buildVec1Str(allocator, n_infotype, ityp),
            .data_data        = try buildVec1Str(allocator, n_data, dd),
            .data_type        = try buildVec1Bits(allocator, n_data, dty),
            .personinfo_info  = try buildVec1Str(allocator, n_pinfo, pi),
            .personinfo_type  = try buildVec1Bits(allocator, n_pinfo, pity),
            .personinfo_note  = try buildManyStr(allocator, n_pinfo, pin),

            .akaname_name    = try buildVec1Str(allocator, n_akaname, an),
            .akatitle_title  = try buildVec1Str(allocator, n_akatitle, at),

            .movielink_target = try buildVec1Bits(allocator, n_mlink, mlt),
            .movielink_type   = try buildVec1Bits(allocator, n_mlink, mlty),
            .linktype_link    = try buildVec1Str(allocator, n_ltype, lty),

            .completecast_status  = try buildVec1Bits(allocator, n_ccast, ccst),
            .completecast_subject = try buildVec1Bits(allocator, n_ccast, ccsub),
            .compcasttype_kind    = try buildVec1Str(allocator, n_ccktype, cck),
        };
    }
};
