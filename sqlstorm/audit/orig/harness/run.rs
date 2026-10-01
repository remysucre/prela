// The batch runner. A batch crate's `main` is one call:
//
//     fn main() { harness::run(&[("10117", q10117)]) }
//
// Each entry's result is compared against `oracles/<name>.txt`, which
// `tools/oracle.py` produced by running the same SQL through DuckDB and
// formatting it by the rules in `fmt`. Rows are compared as a sorted
// multiset — a port may emit them in any order (see fmt).
//
// Env:
//   SQLSTORM_DATA=<dir>   root holding cache/ and oracles/
//                         (default /Users/paultalma/projects/sqlstorm_data)
//   SQLSTORM_ONLY=a,b     run only these queries
//   SQLSTORM_OUT=<dir>    write every result there, match or not

use crate::schema::{self, So};
use std::path::PathBuf;

pub type Entry = (&'static str, fn(&'static So) -> String);

fn data_dir() -> PathBuf {
    std::env::var_os("SQLSTORM_DATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/Users/paultalma/projects/sqlstorm_data"))
}

fn sorted(s: &str) -> Vec<&str> {
    let mut v: Vec<&str> = if s.is_empty() {
        Vec::new()
    } else {
        s.lines().collect()
    };
    v.sort_unstable();
    v
}

pub fn run(entries: &[Entry]) {
    let data = data_dir();
    let t = std::time::Instant::now();
    let db: &'static So = Box::leak(Box::new(schema::load(&data.join("cache"))));
    eprintln!(
        "load: {:.2}s  (posts n={}, users n={}, votes n={})",
        t.elapsed().as_secs_f32(),
        db.post.id.n,
        db.user.id.n,
        db.vote.id.n
    );

    let only: Option<Vec<String>> = std::env::var("SQLSTORM_ONLY")
        .ok()
        .map(|s| s.split(',').map(|x| x.trim().to_string()).collect());
    let out = std::env::var_os("SQLSTORM_OUT").map(PathBuf::from);
    if let Some(d) = &out {
        let _ = std::fs::create_dir_all(d);
    }

    let (mut ok, mut bad, mut skipped) = (0usize, 0usize, 0usize);
    for (name, f) in entries {
        if let Some(keep) = &only
            && !keep.iter().any(|k| k == name)
        {
            continue;
        }
        let q_t = std::time::Instant::now();
        let got = f(db);
        let dt = q_t.elapsed().as_secs_f64();
        if let Some(d) = &out {
            let _ = std::fs::write(d.join(format!("{name}.txt")), &got);
        }

        let path = data.join("oracles").join(format!("{name}.txt"));
        let Ok(oracle) = std::fs::read_to_string(&path) else {
            println!("{name:<8} SKIP  no oracle at {path:?}");
            skipped += 1;
            continue;
        };
        let oracle = oracle.strip_suffix('\n').unwrap_or(&oracle);
        let (g, o) = (sorted(&got), sorted(oracle));
        if g == o {
            ok += 1;
            println!("{name:<8} ok    {dt:>7.3}s  {} rows", g.len());
        } else {
            bad += 1;
            println!(
                "{name:<8} DIFF  {dt:>7.3}s  got {} rows, oracle {} rows",
                g.len(),
                o.len()
            );
            for (i, (a, b)) in g.iter().zip(o.iter()).enumerate() {
                if a != b {
                    println!("           first differing row (sorted) {i}:");
                    println!("             got:    {a}");
                    println!("             oracle: {b}");
                    break;
                }
            }
            if g.len() != o.len() {
                let (more, side) = if g.len() > o.len() {
                    (&g[o.len().min(g.len())..], "got")
                } else {
                    (&o[g.len().min(o.len())..], "oracle")
                };
                for r in more.iter().take(3) {
                    println!("           extra in {side}: {r}");
                }
            }
        }
    }
    eprintln!("{ok} ok, {bad} diff, {skipped} skipped");
    if bad > 0 {
        std::process::exit(1);
    }
}
