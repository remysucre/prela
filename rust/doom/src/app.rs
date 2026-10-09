use crate::db::Db;
use crate::player::Input;
use crate::render::{self, H, W};
use crate::tic::{self, State};
use std::collections::HashSet;
use std::num::NonZeroU32;
use std::rc::Rc;
use std::time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{DeviceEvent, DeviceId, ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{CursorGrabMode, Window, WindowId};

// Doom runs game logic at 35 tics per second.
const TIC: Duration = Duration::from_nanos(1_000_000_000 / 35);

// Timing shown in the window title.
struct Stats {
    frames: u32,
    tics: u32,
    tic_time: Duration,
    frame_time: Duration,
    since: Instant,
}

// Window, input state, and the current game state.
struct App<'a> {
    db: &'a Db,
    state: State,
    buf: Vec<u32>,
    window: Option<Rc<Window>>,
    surface: Option<softbuffer::Surface<Rc<Window>, Rc<Window>>>,
    keys: HashSet<KeyCode>,
    fire: bool,
    use_: bool,
    look: f64,
    sensitivity: f64,
    grabbed: bool,
    next: Instant,
    stats: Stats,
}

impl App<'_> {
    // Locks/hides the mouse cursor for mouselook.
    fn grab(&mut self, on: bool) {
        let Some(w) = &self.window else { return };
        let ok = !on
            || w.set_cursor_grab(CursorGrabMode::Locked).is_ok()
            || w.set_cursor_grab(CursorGrabMode::Confined).is_ok();
        if !on {
            let _ = w.set_cursor_grab(CursorGrabMode::None);
        }
        w.set_cursor_visible(!(on && ok));
        self.grabbed = on && ok;
    }

    // Turns held keys and mouse movement into this tic's Input.
    fn input(&mut self) -> Input {
        let down = |ks: &[KeyCode]| ks.iter().any(|k| self.keys.contains(k));
        let axis = |pos: &[KeyCode], neg: &[KeyCode]| down(pos) as i32 as f64 - down(neg) as i32 as f64;
        let alt = down(&[KeyCode::AltLeft, KeyCode::AltRight]);
        let turn = axis(&[KeyCode::ArrowLeft], &[KeyCode::ArrowRight]);
        let digits = [KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3, KeyCode::Digit4];
        let inp = Input {
            forward: axis(&[KeyCode::ArrowUp, KeyCode::KeyW], &[KeyCode::ArrowDown, KeyCode::KeyS]),
            strafe: axis(&[KeyCode::KeyD, KeyCode::Period], &[KeyCode::KeyA, KeyCode::Comma]) + if alt { -turn } else { 0.0 },
            turn: if alt { 0.0 } else { turn },
            look: -self.look * self.sensitivity,
            use_: self.use_ || down(&[KeyCode::Space, KeyCode::KeyE]),
            fire: self.fire || down(&[KeyCode::ControlLeft, KeyCode::ControlRight, KeyCode::KeyF]),
            run: down(&[KeyCode::ShiftLeft, KeyCode::ShiftRight]),
            weapon: digits.iter().position(|k| self.keys.contains(k)),
        };
        self.look = 0.0;
        inp
    }

    // Runs as many 35 Hz tics as are due.
    fn step(&mut self) {
        while Instant::now() >= self.next {
            let inp = self.input();
            let t = Instant::now();
            self.state = tic::tic(self.db, &self.state, inp);
            self.stats.tic_time += t.elapsed();
            self.stats.tics += 1;
            self.next += TIC;
        }
    }

    // Renders a frame and scales it to the window (nearest neighbour).
    fn draw(&mut self) {
        let (Some(w), Some(surface)) = (&self.window, &mut self.surface) else { return };
        let t = Instant::now();
        render::render(self.db, &self.state, &mut self.buf);
        self.stats.frame_time += t.elapsed();
        self.stats.frames += 1;
        let size = w.inner_size();
        let (Some(sw), Some(sh)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) else { return };
        surface.resize(sw, sh).unwrap();
        let mut out = surface.buffer_mut().unwrap();
        let (ow, oh) = (size.width as usize, size.height as usize);
        for y in 0..oh {
            let src = &self.buf[(y * H / oh) * W..];
            for x in 0..ow {
                out[y * ow + x] = src[x * W / ow];
            }
        }
        out.present().unwrap();
        if self.stats.since.elapsed() > Duration::from_secs(2) {
            let s = &self.stats;
            w.set_title(&format!(
                "Prela DOOM — {:.0} fps, frame {:.1} ms, tic {:.2} ms",
                s.frames as f64 / s.since.elapsed().as_secs_f64(),
                s.frame_time.as_secs_f64() * 1000.0 / s.frames.max(1) as f64,
                s.tic_time.as_secs_f64() * 1000.0 / s.tics.max(1) as f64,
            ));
            self.stats = Stats { frames: 0, tics: 0, tic_time: Duration::ZERO, frame_time: Duration::ZERO, since: Instant::now() };
        }
    }
}

impl ApplicationHandler for App<'_> {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attrs = Window::default_attributes().with_title("Prela DOOM").with_inner_size(LogicalSize::new(W as f64 * 3.0, H as f64 * 3.0));
        let w = Rc::new(el.create_window(attrs).unwrap());
        let ctx = softbuffer::Context::new(w.clone()).unwrap();
        self.surface = Some(softbuffer::Surface::new(&ctx, w.clone()).unwrap());
        self.window = Some(w);
        self.grab(true);
        self.next = Instant::now();
    }

    // Keyboard, mouse buttons, focus, and redraw.
    fn window_event(&mut self, el: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => el.exit(),
            WindowEvent::Focused(false) => {
                self.keys.clear();
                (self.fire, self.use_) = (false, false);
                self.grab(false);
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let PhysicalKey::Code(code) = event.physical_key else { return };
                match (code, event.state) {
                    (KeyCode::Escape, ElementState::Pressed) if self.grabbed => self.grab(false),
                    (KeyCode::Escape, ElementState::Pressed) => el.exit(),
                    (_, ElementState::Pressed) => {
                        self.keys.insert(code);
                    }
                    (_, ElementState::Released) => {
                        self.keys.remove(&code);
                    }
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let down = state == ElementState::Pressed;
                if !self.grabbed {
                    if down {
                        self.grab(true);
                    }
                    return;
                }
                match button {
                    MouseButton::Left => self.fire = down,
                    MouseButton::Right => self.use_ = down,
                    _ => {}
                }
            }
            WindowEvent::RedrawRequested => {
                self.step();
                self.draw();
            }
            _ => {}
        }
    }

    // Raw mouse motion for turning.
    fn device_event(&mut self, _: &ActiveEventLoop, _: DeviceId, event: DeviceEvent) {
        if let DeviceEvent::MouseMotion { delta: (dx, _) } = event {
            if self.grabbed {
                self.look += dx;
            }
        }
    }

    fn about_to_wait(&mut self, _: &ActiveEventLoop) {
        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }
}

// Opens the window and runs the event loop.
pub fn run(db: &Db, state: State) {
    let sensitivity = std::env::var("DOOM_MOUSE").ok().and_then(|s| s.parse().ok()).unwrap_or(0.003);
    let mut app = App {
        db,
        state,
        buf: vec![0; W * H],
        window: None,
        surface: None,
        keys: HashSet::new(),
        fire: false,
        use_: false,
        look: 0.0,
        sensitivity,
        grabbed: false,
        next: Instant::now(),
        stats: Stats { frames: 0, tics: 0, tic_time: Duration::ZERO, frame_time: Duration::ZERO, since: Instant::now() },
    };
    EventLoop::new().unwrap().run_app(&mut app).unwrap();
}
