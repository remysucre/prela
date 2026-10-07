use std::path::{Path, PathBuf};

pub type Entry<D> = (&'static str, fn(&'static D) -> String);

pub fn data_dir(set: &str) -> PathBuf {
    std::env::var_os("BIGSQL_DATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/Users/paultalma/projects/bigsql_data"))
        .join(set)
}

fn sorted(s: &str) -> Vec<&str> {
    let mut v: Vec<&str> = if s.is_empty() { Vec::new() } else { s.lines().collect() };
    v.sort_unstable();
    v
}

pub fn run<D: 'static>(set: &str, load: fn(&Path) -> D, entries: &[Entry<D>]) {
    let data = data_dir(set);
    let t = std::time::Instant::now();
    let db: &'static D = Box::leak(Box::new(load(&data.join("cache"))));
    eprintln!("load: {:.3}s", t.elapsed().as_secs_f64());

    let only: Option<Vec<String>> = std::env::var("BIGSQL_ONLY")
        .ok()
        .map(|s| s.split(',').map(|x| x.trim().to_string()).collect());
    let out = std::env::var_os("BIGSQL_OUT").map(PathBuf::from);
    if let Some(d) = &out {
        let _ = std::fs::create_dir_all(d);
    }
    let reps: usize = std::env::var("BIGSQL_REPS").ok().and_then(|s| s.parse().ok()).unwrap_or(1);

    let (mut ok, mut bad, mut skipped) = (0usize, 0usize, 0usize);
    for (name, f) in entries {
        if let Some(keep) = &only
            && !keep.iter().any(|k| k == name)
        {
            continue;
        }
        let mut times = Vec::with_capacity(reps);
        let mut got = String::new();
        for _ in 0..reps {
            let q_t = std::time::Instant::now();
            got = f(db);
            times.push(q_t.elapsed().as_secs_f64());
        }
        times.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let dt = times[times.len() / 2];
        if let Some(d) = &out {
            let _ = std::fs::write(d.join(format!("{name}.txt")), &got);
        }

        let path = data.join("oracles").join(format!("{name}.txt"));
        let Ok(oracle) = std::fs::read_to_string(&path) else {
            println!("{name:<16} SKIP  no oracle at {path:?}");
            skipped += 1;
            continue;
        };
        let oracle = oracle.strip_suffix('\n').unwrap_or(&oracle);
        let (g, o) = (sorted(&got), sorted(oracle));
        let tie_col: Option<usize> = std::fs::read_to_string(data.join("oracles").join(format!("{name}.ties")))
            .ok()
            .and_then(|s| s.trim().parse().ok());
        let drop = |v: &[&str], c: usize| -> Vec<String> {
            let mut r: Vec<String> = v
                .iter()
                .map(|l| l.split('\t').enumerate().filter(|(i, _)| *i != c).map(|(_, f)| f).collect::<Vec<_>>().join("\t"))
                .collect();
            r.sort_unstable();
            r
        };
        let tie_ok = g != o && tie_col.is_some_and(|c| drop(&g, c) == drop(&o, c));
        if tie_ok {
            ok += 1;
            println!("{name:<16} ok    {dt:>9.4}s  {} rows (column {} ignored: tie in the SQL's ROW_NUMBER)", g.len(), tie_col.unwrap());
        } else if g == o {
            ok += 1;
            println!("{name:<16} ok    {dt:>9.4}s  {} rows", g.len());
        } else {
            bad += 1;
            println!("{name:<16} DIFF  {dt:>9.4}s  got {} rows, oracle {} rows", g.len(), o.len());
            let mut shown = 0;
            for (i, (a, b)) in g.iter().zip(o.iter()).enumerate() {
                if a != b {
                    println!("           differing row (sorted) {i}:");
                    println!("             got:    {a}");
                    println!("             oracle: {b}");
                    shown += 1;
                    if shown == 3 {
                        break;
                    }
                }
            }
            if g.len() != o.len() {
                let (more, side) = if g.len() > o.len() { (&g[o.len()..], "got") } else { (&o[g.len()..], "oracle") };
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
