mod app;
mod db;
mod info;
mod mobj;
mod physics;
mod player;
mod render;
mod rules;
mod schema;
mod tic;
mod wad;
mod world;

use db::Db;
use player::Input;
use schema::one;
use prela::engine::*;
use render::{H, W};
use std::io::Write;
use std::time::Instant;
use tic::State;

// The player's main fields, for logging.
fn player(s: &State) -> String {
    let p = &s.player;
    format!(
        "player x {:.3} y {:.3} z {:.1} ang {:.4} sector {} health {} armor {} weapon {} dead {} exited {}",
        one(&p.x),
        one(&p.y),
        one(&p.z),
        one(&p.ang),
        one(&p.sector).0,
        one(&p.health),
        one(&p.armor),
        one(&p.weapon).0,
        one(&p.dead),
        one(&p.exited)
    )
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
    let db = Db::new(wad::Game::load(&wad, "E1M1"));
    let mut state = tic::initial(&db);
    // --at x y angle: start somewhere else.
    if let Some(i) = args.iter().position(|a| a == "--at") {
        let v: Vec<f64> = args[i + 1..i + 4].iter().map(|a| a.parse().unwrap()).collect();
        state = tic::place(&db, state, v[0], v[1], v[2].to_radians());
    }
    eprintln!("loaded in {:?}, {} mobjs, start {:?}", t0.elapsed(), state.mobj.id.n, render::pose(&db, &state.player));
    let mut buf = vec![0u32; W * H];

    // --shot out.ppm SCRIPT: run one tic per script letter, render one frame, write it, exit.
    if let Some(i) = args.iter().position(|a| a == "--shot") {
        let script = args.get(i + 2).map(String::as_str).unwrap_or("");
        let t = Instant::now();
        // Each letter is one tic of held input.
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
        eprintln!("{} tics in {:?}, frame in {:?}, {}", script.len(), tics, t.elapsed(), player(&state));
        // Print nearby things and active movers.
        if std::env::var_os("DOOM_DEBUG").is_some() {
            let (m, sec) = (&state.mobj, &state.sector);
            let (px, py) = (one(&state.player.x), one(&state.player.y));
            (&*m.uid)
                .and((&*m.x).and(&*m.y))
                .and((&*m.kind).and(&*m.state).and(&*m.health))
                .filt(move |((_, (x, y)), _): ((usize, (f64, f64)), ((schema::Id<schema::ThingType>, schema::Id<schema::AnimState>), i32))| (x - px).hypot(y - py) < 400.0)
                .drive(|_, v| eprintln!("mobj {v:?}"));
            (&*sec.kind).and((&*sec.floor).and(&*sec.ceil)).filt(|(k, _): (u8, (f64, f64))| k != 0).drive(|s, v| eprintln!("mover {s:?} {v:?}"));
        }
        write_ppm(&args[i + 1], &buf);
        return;
    }

    // Otherwise open the window and play.
    app::run(&db, state);
}
