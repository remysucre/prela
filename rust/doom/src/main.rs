mod app;
mod db;
mod info;
mod mobj;
mod physics;
mod player;
mod render;
mod rules;
mod tic;
mod wad;
mod world;

use db::Db;
use player::{Input, Player};
use prela::engine::*;
use render::{H, W};
use std::io::Write;
use std::time::Instant;
use tic::State;

fn player(s: &State) -> Player {
    s.player.get(0).unwrap()
}

// Writes the framebuffer as a PPM image (for --shot).
fn write_ppm(path: &str, buf: &[u32]) {
    let mut f = std::io::BufWriter::new(std::fs::File::create(path).unwrap());
    write!(f, "P6\n{W} {H}\n255\n").unwrap();
    for &p in buf {
        f.write_all(&[(p >> 16) as u8, (p >> 8) as u8, p as u8]).unwrap();
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let wad = std::env::var("DOOM_WAD").unwrap_or("../data/doom/freedoom-0.13.0/freedoom1.wad".into());
    let t0 = Instant::now();
    // Load the WAD into relations and build the starting state.
    let (db, sectors) = Db::new(wad::Game::load(&wad, "E1M1"));
    let fresh = |sectors: &VecRel<usize, wad::Sector>| tic::initial(&db, sectors.map(|s| s).collect());
    let mut state = fresh(&sectors);
    // --at x y angle: start somewhere else.
    if let Some(i) = args.iter().position(|a| a == "--at") {
        let v: Vec<f64> = args[i + 1..i + 4].iter().map(|a| a.parse().unwrap()).collect();
        state = tic::place(&db, state, v[0], v[1], v[2].to_radians());
    }
    eprintln!("loaded in {:?}, {} mobjs, start {:?}", t0.elapsed(), state.mobjs.idx.len(), render::pose(player(&state)));
    let mut buf = vec![0u32; W * H];

    // --shot out.ppm SCRIPT: run one tic per script letter, render one frame, write it, exit.
    if let Some(i) = args.iter().position(|a| a == "--shot") {
        let script = args.get(i + 2).map(String::as_str).unwrap_or("");
        let t = Instant::now();
        for c in script.chars() {
            let inp = match c {
                'f' => Input { forward: 1.0, ..Default::default() },
                'b' => Input { forward: -1.0, ..Default::default() },
                'l' => Input { turn: 1.0, ..Default::default() },
                'r' => Input { turn: -1.0, ..Default::default() },
                'u' => Input { use_: true, ..Default::default() },
                'x' => Input { fire: true, ..Default::default() },
                '3' => Input { weapon: Some(2), ..Default::default() },
                _ => Input::default(),
            };
            state = tic::tic(&db, &state, inp);
        }
        let tics = t.elapsed();
        let t = Instant::now();
        render::render(&db, &state, &mut buf);
        eprintln!("{} tics in {:?}, frame in {:?}, {:?}", script.len(), tics, t.elapsed(), player(&state));
        // Print nearby things and active movers.
        if std::env::var_os("DOOM_DEBUG").is_some() {
            let p = player(&state);
            (&state.mobjs).filt(move |m: mobj::Mobj| (m.x - p.x).hypot(m.y - p.y) < 400.0).drive(|_, m| eprintln!("{m:?}"));
            (&state.movers).and(&state.sectors).filt(|(m, _): (world::Mover, wad::Sector)| m.kind != 0).drive(|k, v| eprintln!("mover {k} {v:?}"));
        }
        write_ppm(&args[i + 1], &buf);
        return;
    }

    // Otherwise open the window and play.
    app::run(&db, &sectors, state);
}
