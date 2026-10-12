#!/bin/sh
set -e
cd "$(dirname "$0")/../.."
cargo build --release -q -p prela-doom
REF=doom/ref
OUT=$(mktemp -d)
UPDATE=${1:-}
r() { printf "%${2}s" | tr ' ' "$1"; }
status=0
shot() {
  name=$1; shift
  ./target/release/prela-doom "$@" >/dev/null 2>&1
  if [ "$UPDATE" = "--update" ]; then
    cp "$OUT/$name.ppm" "$REF/$name.ppm"
    echo "updated $name"
  elif cmp -s "$OUT/$name.ppm" "$REF/$name.ppm"; then
    echo "ok      $name"
  else
    echo "DIFFERS $name"
    status=1
  fi
}
shot walk --shot "$OUT/walk.ppm" "$(r f 30)$(r l 8)$(r f 20)$(r r 16)$(r b 20)"
shot door --at 832 592 -90 --shot "$OUT/door.ppm" "u$(r . 35)"
shot lift --at 240 256 -180 --shot "$OUT/lift.ppm" "u$(r . 40)"
shot puffs --at 608 1100 0 --shot "$OUT/puffs.ppm" "$(r x 12)"
shot shoot --at 608 1100 0 --shot "$OUT/shoot.ppm" "$(r x 30)$(r . 60)"
shot pickup --at 608 940 90 --shot "$OUT/pickup.ppm" "$(r f 20)"
shot fight --at 2240 850 0 --shot "$OUT/fight.ppm" "$(r . 600)"
shot exit --at -352 1296 -180 --shot "$OUT/exit.ppm" "u$(r . 10)"
rm -rf "$OUT"
exit $status
