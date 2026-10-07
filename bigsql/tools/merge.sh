#!/bin/bash
set -eu
SET=$1; shift
S=/Users/paultalma/projects/prela/bigsql/rust/$SET/src
mkdir -p "$S/concepts" "$S/queries" "$S/wip" "$S/bin"
DEST=concepts
if [ "${1:-}" = "--query" ]; then DEST=queries; shift; fi
for n in "$@"; do
  sed -i '' 's/crate::sofa::/crate::queries::sofa::/g' "$S/wip/$n.rs"
  mv "$S/wip/$n.rs" "$S/$DEST/$n.rs"
  rm -f "$S/bin/wip_$n.rs"
done
for n in "$@"; do
  for f in "$S"/concepts/*.rs "$S"/queries/*.rs "$S"/wip/*.rs; do
    [ -f "$f" ] && sed -i "" -E "s/crate::$n([^a-zA-Z0-9_])/crate::$DEST::$n\\1/g" "$f"
  done
done
mods() { ls "$S/$1" | grep -v '^mod.rs$' | sed 's/\.rs$//' | sort; }
mods concepts | sed 's/.*/pub mod &;/' > "$S/concepts/mod.rs"
mods queries | sed 's/.*/pub mod &;/' > "$S/queries/mod.rs"
{
  echo "use $SET::schema::{self, Db};"
  echo
  echo "static ENTRIES: &[harness::run::Entry<Db>] = &["
  for c in $(mods queries); do echo "    (\"$c\", $SET::queries::$c::q),"; done
  for c in $(mods concepts); do grep -q "pub fn q(" "$S/concepts/$c.rs" && echo "    (\"c_$c\", $SET::concepts::$c::q),"; done
  echo "];"
  echo
  echo "fn main() {"
  echo "    harness::run(\"$SET\", schema::load, ENTRIES)"
  echo "}"
} > "$S/main.rs"
