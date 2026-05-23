mod engine;
mod data;
mod queries;

use data::Data;

fn main() {
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
