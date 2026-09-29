#!/bin/bash
# tools/newcrate.sh cNN bNNN: a new crate holding one empty batch, added to
# the workspace. Copies the Cargo.toml of c61.
set -e
cd "$(dirname "$0")/../rust"
c=$1; b=$2
mkdir -p $c/src
sed "s/name = \"c61\"/name = \"$c\"/" c61/Cargo.toml > $c/Cargo.toml
cat > $c/src/main.rs <<EOT
mod $b;

fn main() {
    let mut all: Vec<harness::Entry> = Vec::new();
    for e in [$b::ENTRIES] {
        all.extend_from_slice(e);
    }
    harness::run(&all)
}
EOT
cat > $c/src/$b.rs <<EOT
use harness::prelude::*;
use std::cmp::Reverse;

pub static ENTRIES: &[harness::Entry] = &[
];
EOT
python3 - "$c" <<'PY'
import sys, re
c = sys.argv[1]
p = 'Cargo.toml'; s = open(p).read()
last = re.findall(r'"c\d+"', s)[-1]
s = s.replace(last + "]", last + f', "{c}"]', 1)
open(p, 'w').write(s)
PY
