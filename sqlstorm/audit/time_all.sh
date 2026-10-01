#!/bin/bash
cd "$(dirname "$0")/../rust"
cargo build --release $(for i in $(seq 0 113); do printf -- "-p c%02d "  $i; done) 2>&1 | grep -E "^error" && exit 1
for c in $(ls -d c[0-9]* | sort -V); do
  n=${c#c}; [ "$n" -gt 113 ] && continue
  ./target/release/$c > ../audit/prela_out/$c.txt 2>&1
done
cd ..
/Users/paultalma/projects/sqlstorm_data/venv/bin/python audit/time_duck.py 8
