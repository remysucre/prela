"""Schema spec for the SQLStorm StackOverflow corpus.

Maps the 13 SQL tables onto prela entities. Two rules do the real work:

  * ids are remapped to a dense 0..n-1 key (ordered by the SQL Id); the
    original id is kept as `origid` so a query that projects `Id` can print
    what SQL prints.

  * a column that is NULL anywhere is a SET column (CSR, zero-or-one entry
    per key) rather than a dense one. Absence IS the null: `.with(col)` is
    IS NOT NULL, `.minus(col)` is IS NULL, and `.select(col)` drops the null
    rows on its own. Dense vs set is decided from the DATA, not from the
    declared nullability, so a column that is declared nullable but never
    null stays dense.
"""

# rust struct name -> sql table name
ENTITIES = {
    "PostHistoryType": "PostHistoryTypes",
    "LinkType": "LinkTypes",
    "PostType": "PostTypes",
    "CloseReasonType": "CloseReasonTypes",
    "VoteType": "VoteTypes",
    "User": "Users",
    "Badge": "Badges",
    "Post": "Posts",
    "Comment": "Comments",
    "PostHistory": "PostHistory",
    "PostLink": "PostLinks",
    "Tag": "Tags",
    "Vote": "Votes",
}

SQL_TO_ENTITY = {v: k for k, v in ENTITIES.items()}

# (sql table, sql column) -> sql table it references
FKS = {
    ("Badges", "UserId"): "Users",
    ("Posts", "PostTypeId"): "PostTypes",
    ("Posts", "AcceptedAnswerId"): "Posts",
    ("Posts", "ParentId"): "Posts",
    ("Posts", "OwnerUserId"): "Users",
    ("Posts", "LastEditorUserId"): "Users",
    ("Comments", "PostId"): "Posts",
    ("Comments", "UserId"): "Users",
    ("PostHistory", "PostHistoryTypeId"): "PostHistoryTypes",
    ("PostHistory", "PostId"): "Posts",
    ("PostHistory", "UserId"): "Users",
    ("PostLinks", "PostId"): "Posts",
    ("PostLinks", "RelatedPostId"): "Posts",
    ("PostLinks", "LinkTypeId"): "LinkTypes",
    ("Tags", "ExcerptPostId"): "Posts",
    ("Tags", "WikiPostId"): "Posts",
    ("Votes", "PostId"): "Posts",
    ("Votes", "VoteTypeId"): "VoteTypes",
    ("Votes", "UserId"): "Users",
}

# rust field name overrides, keyed the same way. Everything else is the
# snake_case of the SQL column; FK columns additionally lose a trailing
# `_id` (`OwnerUserId` -> `owner_user`), matching how prela's own schemas
# spell a foreign key (`Lineitem_order`, `Nation_region`).
FIELD_OVERRIDES = {
    ("Posts", "Tags"): "tags_str",  # `tags` is the exploded Set<Post, Id<Tag>>
}

RESERVED = {"type", "ref", "match", "move", "box", "self", "crate", "as", "in", "fn", "mod"}


def snake(name: str) -> str:
    out = []
    for i, ch in enumerate(name):
        if ch.isupper() and i and not name[i - 1].isupper():
            out.append("_")
        out.append(ch.lower())
    s = "".join(out)
    return s + "_" if s in RESERVED else s


def field_name(table: str, col: str) -> str:
    if (table, col) in FIELD_OVERRIDES:
        return FIELD_OVERRIDES[(table, col)]
    if col == "Id":
        return "origid"
    s = snake(col)
    if (table, col) in FKS and s.endswith("_id"):
        s = s[: -len("_id")]
    return s


# payload type per SQL type: i64 (ints, bools, timestamps-as-epoch-us) or Str
def payload(sql_type: str) -> str:
    t = sql_type.upper()
    if t.startswith(("VARCHAR", "TEXT", "CHAR")):
        return "Str"
    if t.startswith(("INT", "BIGINT", "SMALLINT", "TINYINT", "HUGEINT", "UINT")):
        return "i64"
    if t.startswith(("TIMESTAMP", "DATE")):
        return "i64"  # epoch microseconds
    if t.startswith(("BOOL",)):
        return "i64"  # 0/1
    if t.startswith(("DOUBLE", "FLOAT", "DECIMAL", "REAL")):
        return "f64"
    raise SystemExit(f"unmapped SQL type {sql_type!r}")


def is_timestamp(sql_type: str) -> bool:
    return sql_type.upper().startswith(("TIMESTAMP", "DATE"))
