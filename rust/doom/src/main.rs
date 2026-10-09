mod db;
mod info;
mod mobj;
mod physics;
mod player;
mod render;
mod tic;
mod wad;
mod world;

use db::Db;
use minifb::{Key, Scale, Window, WindowOptions};
use player::{Input, Player};
use prela::engine::*;
use render::{H, W};
use std::io::Write;
use std::time::{Duration, Instant};
use tic::State;

const TIC: Duration = Duration::from_nanos(1_000_000_000 / 35);

fn player(s: &State) -> Player {
    s.player.get(0).unwrap()
}

fn write_ppm(path: &str, buf: &[u32]) {
    let mut f = std::io::BufWriter::new(std::fs::File::create(path).unwrap());
    write!(f, "P6\n{W} {H}\n255\n").unwrap();
    for &p in buf {
        f.write_all(&[(p >> 16) as u8, (p >> 8) as u8, p as u8]).unwrap();
    }
}

fn read_input(w: &Window) -> Input {
    let down = |ks: &[Key]| ks.iter().any(|&k| w.is_key_down(k));
    let axis = |pos: &[Key], neg: &[Key]| down(pos) as i32 as f64 - down(neg) as i32 as f64;
    let alt = down(&[Key::LeftAlt, Key::RightAlt]);
    let turn = axis(&[Key::Left], &[Key::Right]);
    Input {
        forward: axis(&[Key::Up, Key::W], &[Key::Down, Key::S]),
        strafe: axis(&[Key::D, Key::Period], &[Key::A, Key::Comma]) + if alt { -turn } else { 0.0 },
        turn: if alt { 0.0 } else { turn },
        use_: down(&[Key::Space, Key::E]),
        fire: down(&[Key::LeftCtrl, Key::RightCtrl, Key::F]),
        run: down(&[Key::LeftShift, Key::RightShift]),
        weapon: [Key::Key1, Key::Key2, Key::Key3, Key::Key4].iter().position(|&k| w.is_key_down(k)),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let wad = std::env::var("DOOM_WAD").unwrap_or("../data/doom/freedoom-0.13.0/freedoom1.wad".into());
    let t0 = Instant::now();
    let (db, sectors) = Db::new(wad::Game::load(&wad, "E1M1"));
    let fresh = |sectors: &VecRel<usize, wad::Sector>| tic::initial(&db, sectors.map(|s| s).collect());
    let mut state = fresh(&sectors);
    if let Some(i) = args.iter().position(|a| a == "--at") {
        let v: Vec<f64> = args[i + 1..i + 4].iter().map(|a| a.parse().unwrap()).collect();
        state = tic::place(&db, state, v[0], v[1], v[2].to_radians());
    }
    eprintln!("loaded in {:?}, {} mobjs, start {:?}", t0.elapsed(), state.mobjs.idx.len(), render::pose(player(&state)));
    let mut buf = vec![0u32; W * H];

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
        if std::env::var_os("DOOM_DEBUG").is_some() {
            let p = player(&state);
            (&state.mobjs).filt(move |m: mobj::Mobj| (m.x - p.x).hypot(m.y - p.y) < 400.0).drive(|_, m| eprintln!("{m:?}"));
        }
        write_ppm(&args[i + 1], &buf);
        return;
    }

    let mut window = Window::new("Prela DOOM", W, H, WindowOptions { scale: Scale::X4, ..WindowOptions::default() }).unwrap();
    let mut next = Instant::now();
    let (mut frames, mut tics, mut tic_time, mut frame_time, mut last) = (0, 0, Duration::ZERO, Duration::ZERO, Instant::now());
    while window.is_open() && !window.is_key_down(Key::Escape) {
        while Instant::now() >= next {
            let inp = read_input(&window);
            let t = Instant::now();
            let p = player(&state);
            state = if (p.dead || p.exited) && inp.use_ && !p.usedown { fresh(&sectors) } else { tic::tic(&db, &state, inp) };
            tic_time += t.elapsed();
            tics += 1;
            next += TIC;
        }
        let t = Instant::now();
        render::render(&db, &state, &mut buf);
        frame_time += t.elapsed();
        frames += 1;
        window.update_with_buffer(&buf, W, H).unwrap();
        if last.elapsed() > Duration::from_secs(2) {
            window.set_title(&format!(
                "Prela DOOM — {:.0} fps, frame {:.1} ms, tic {:.2} ms",
                frames as f64 / last.elapsed().as_secs_f64(),
                frame_time.as_secs_f64() * 1000.0 / frames as f64,
                tic_time.as_secs_f64() * 1000.0 / tics.max(1) as f64,
            ));
            (frames, tics, tic_time, frame_time, last) = (0, 0, Duration::ZERO, Duration::ZERO, Instant::now());
        }
    }
}
