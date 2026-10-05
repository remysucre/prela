#!/bin/bash
R=/Users/paultalma/projects/prela/sqlstorm/rust
O=/Users/paultalma/projects/prela/sqlstorm/audit/prela_out/final
mkdir -p $O
cargo build --release --manifest-path $R/Cargo.toml $(for i in $(seq 0 113); do printf -- "-p c%02d " $i; done) 2>&1 | grep -E "^(error|warning)" | sort | uniq -c | head
for i in $(seq 0 113); do c=$(printf c%02d $i); (cd $R && ./target/release/$c > $O/$c.txt 2>&1); done
grep -h "DIFF\|SKIP" $O/*.txt
grep -h " ok, " $O/*.txt | awk '{o+=$1;d+=$3;s+=$5} END{print o" ok "d" diff "s" skip"}'
