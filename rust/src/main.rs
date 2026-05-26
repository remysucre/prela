mod engine;
mod data;
mod queries;
mod tpch_data;
mod tpch_queries_idiomatic;
mod tpch_queries_optimized;
mod tpch_queries_ddbcheat;

use data::Data;
use tpch_data::TpchData;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let suite = args.get(1).map(|s| s.as_str()).unwrap_or("job");

    if suite == "tpch" {
        run_tpch();
    } else {
        run_job();
    }
}

fn run_job() {
    let t = std::time::Instant::now();
    let d = Data::load();
    eprintln!("load: {:.2}s  (movie n={}, person n={})",
              t.elapsed().as_secs_f32(), d.movie.n, d.persons.n);

    let qs = queries::all_queries();
    eprintln!("{} queries registered", qs.len());

    for round in 1..=2 {
        eprintln!("--- run {round} ---");
        let mut ok = 0usize;
        let mut bad: Vec<(&'static str, &'static str, String)> = Vec::new();
        let t = std::time::Instant::now();
        for (name, oracle, f) in &qs {
            let q_t = std::time::Instant::now();
            let got = f(&d);
            let dt = q_t.elapsed().as_secs_f32();
            let pass = got == *oracle;
            if pass {
                ok += 1;
                if round == 2 || dt > 0.5 {
                    println!("{:<5} ok  {:>6.2}s  {}", name, dt, got);
                }
            } else {
                bad.push((name, oracle, got.clone()));
                println!("{:<5} DIFF {:>6.2}s  {}", name, dt, got);
                println!("        oracle: {oracle}");
            }
        }
        eprintln!("run {round}: {}/{} ok  total {:.2}s",
                  ok, qs.len(), t.elapsed().as_secs_f32());
        if round == 2 && !bad.is_empty() {
            eprintln!("\n{} diffs:", bad.len());
            for (n, o, g) in &bad {
                eprintln!("  {n}: got {g:?}  oracle {o:?}");
            }
        }
    }
}

fn run_tpch() {
    let t = std::time::Instant::now();
    let d = TpchData::load();
    eprintln!("load: {:.2}s  (li n={}, ord n={}, ps n={})",
              t.elapsed().as_secs_f32(), d.lineitem.n, d.orders.n, d.partsupp.n);

    // QS=idiomatic|optimized|ddbcheat (default optimized)
    let variant = std::env::var("QS").unwrap_or_else(|_| "optimized".to_string());
    let qs = match variant.as_str() {
        "idiomatic" => tpch_queries_idiomatic::all_queries(),
        "optimized" => tpch_queries_optimized::all_queries(),
        "ddbcheat"  => tpch_queries_ddbcheat::all_queries(),
        other => panic!("unknown QS variant: {other:?} (use idiomatic|optimized|ddbcheat)"),
    };
    eprintln!("{} TPC-H queries registered ({} variant)", qs.len(), variant);

    for round in 1..=2 {
        eprintln!("--- run {round} ---");
        let mut ok = 0usize;
        let t = std::time::Instant::now();
        for (name, oracle, f) in &qs {
            let q_t = std::time::Instant::now();
            let got = f(&d);
            let dt = q_t.elapsed().as_secs_f32();
            let pass = got == *oracle;
            if pass {
                ok += 1;
                println!("{:<5} ok    {:>6.2}s", name, dt);
            } else {
                println!("{:<5} DIFF  {:>6.2}s", name, dt);
                // Write to /tmp for offline diff
                let _ = std::fs::write(format!("/tmp/tpch_got_{name}.txt"), &got);
                let _ = std::fs::write(format!("/tmp/tpch_oracle_{name}.txt"), oracle);
                let got_lines: Vec<_> = got.lines().collect();
                let oracle_lines: Vec<_> = oracle.lines().collect();
                let n = got_lines.len().max(oracle_lines.len());
                for i in 0..n {
                    let g = got_lines.get(i).unwrap_or(&"");
                    let o = oracle_lines.get(i).unwrap_or(&"");
                    if g != o {
                        println!("        line {}:", i + 1);
                        println!("          got:    {g:?}");
                        println!("          oracle: {o:?}");
                        if i >= 3 { break; }
                    }
                }
            }
        }
        eprintln!("run {round}: {}/{} ok  total {:.2}s",
                  ok, qs.len(), t.elapsed().as_secs_f32());
    }
}
