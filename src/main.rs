use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use rand::{prelude::ThreadRng, Rng};
use std::{
    error::Error,
    ops::{Add, Mul, Sub},
};
use std::{
    io,
    time::{Duration, Instant},
};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::Color,
    widgets::{
        canvas::{Canvas, Points},
        Block,
    },
    Frame, Terminal,
};

#[derive(Debug, Copy, Clone, PartialEq)]
struct Vector {
    x: f64,
    y: f64,
}

impl Vector {
    fn zero() -> Vector {
        Vector { x: 0.0, y: 0.0 }
    }
}

impl Add for Vector {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

impl Sub for Vector {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}

impl Mul<f64> for Vector {
    type Output = Self;

    fn mul(self, scalar: f64) -> Self {
        Self {
            x: self.x * scalar,
            y: self.y * scalar,
        }
    }
}

fn random_unit_vector(rng: &mut ThreadRng) -> Vector {
    let x: f64 = rng.gen_range(-1.0..1.0);
    let y: f64 = rng.gen_range(-1.0..1.0);
    let abs = (x.powi(2) + y.powi(2)).powf(0.5);
    Vector {
        x: x / abs,
        y: y / abs,
    }
}

#[derive(Copy, Clone, Debug)]
struct Particle {
    pos: Vector,
    vel: Vector,
    acc: Vector,
    dont_delete: bool,
    exploded: bool,
    subparticle: bool,
}

impl Particle {
    fn new(is_sub: bool, ipos: Vector, ivel: Vector) -> Particle {
        Particle {
            pos: ipos,
            vel: ivel,
            acc: Vector { x: 0.0, y: 0.0 },
            dont_delete: true,
            exploded: false,
            subparticle: is_sub,
        }
    }

    fn apply_force(&mut self, force: Vector) {
        self.acc = self.acc + force;
    }

    fn update(&mut self) {
        if (!self.dont_delete || self.vel.y <= -0.05) && !self.subparticle {
            self.dont_delete = false;
            return;
        }
        self.vel = self.vel + self.acc;
        self.pos = self.pos + self.vel;
        self.acc = self.acc * 0.0;
        if self.subparticle {
            self.vel = self.vel * 0.98;
        }
    }
}

const COLORS: [Color; 6] = [
    Color::Blue,
    Color::Green,
    Color::Magenta,
    Color::Red,
    Color::Yellow,
    Color::White,
];
const MAX_PARTICLES_COLOR: usize = 1000;

#[derive(Copy, Clone, Debug)]
struct ParticleGroup {
    pos: [(f64, f64); MAX_PARTICLES_COLOR],
    add_at: usize,
    particles: [Particle; MAX_PARTICLES_COLOR],
    color: Color,
}

impl ParticleGroup {
    fn new(color: Color) -> ParticleGroup {
        ParticleGroup {
            pos: [(-9999.9, -9999.9); MAX_PARTICLES_COLOR],
            add_at: 0,
            particles: [Particle::new(
                false,
                Vector {
                    x: -999.9,
                    y: -999.9,
                },
                Vector::zero(),
            ); MAX_PARTICLES_COLOR],
            color: color,
        }
    }
}

struct App {
    particle_groups: [ParticleGroup; COLORS.len()],
    gravity: Vector,
    rng: ThreadRng,
}

impl App {
    fn new() -> App {
        let mut tmp: [ParticleGroup; COLORS.len()] =
            [ParticleGroup::new(Color::Black); COLORS.len()];
        for (i, c) in COLORS.iter().enumerate() {
            tmp[i].color = c.clone();
        }
        App {
            particle_groups: tmp,
            gravity: Vector { x: 0.0, y: -0.004 },
            rng: rand::thread_rng(),
        }
    }

    fn on_tick(&mut self) {
        for particle_group in self.particle_groups.iter_mut() {
            for i in 0..MAX_PARTICLES_COLOR {
                particle_group.particles[i].apply_force(self.gravity);
                particle_group.particles[i].update();
                particle_group.pos[i] = (
                    particle_group.particles[i].pos.x,
                    particle_group.particles[i].pos.y,
                );

                if !particle_group.particles[i].dont_delete {
                    particle_group.pos[i] = (9999.9, 9999.9);
                    if !particle_group.particles[i].exploded
                        && !particle_group.particles[i].subparticle
                    {
                        for _ in 1..20 {
                            create_particle(
                                particle_group,
                                true,
                                particle_group.particles[i].pos,
                                random_unit_vector(&mut self.rng) * self.rng.gen_range(0.2..0.4),
                            );
                        }
                        particle_group.particles[i].exploded = true;
                        continue;
                    }
                    continue;
                }
            }
        }
    }
}

fn create_particle(pgroup: &mut ParticleGroup, is_sub: bool, pos: Vector, vel: Vector) {
    let p = Particle::new(is_sub, pos, vel);
    pgroup.particles[pgroup.add_at] = p;
    pgroup.pos[pgroup.add_at] = (pos.x, pos.y);
    pgroup.add_at += 1;
    if pgroup.add_at == MAX_PARTICLES_COLOR {
        pgroup.add_at = 0;
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let tick_rate = Duration::from_millis(10);
    let app = App::new();
    let res = run_app(&mut terminal, app, tick_rate);

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    mut app: App,
    tick_rate: Duration,
) -> io::Result<()> {
    let mut rng = rand::thread_rng();
    let w_int = terminal.get_frame().size().width;
    let h_int = terminal.get_frame().size().height;
    let w_float = f64::from(w_int);
    let h_float = f64::from(h_int);

    let max_speed: f64 = 0.08 * h_float.powf(0.5);

    let mut last_tick = Instant::now();
    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => {
                        return Ok(());
                    }
                    KeyCode::Char('m') => {
                        for i in -10..10 {
                            let pos_x = f64::from(i) * w_float / 20.0;
                            let speed_y = rng.gen_range(max_speed * 0.8..max_speed);
                            let speed_x = rng.gen_range(-0.08..0.08);
                            create_particle(
                                &mut app.particle_groups[rng.gen_range(0..COLORS.len())],
                                false,
                                Vector {
                                    x: pos_x,
                                    y: -h_float / 2.0,
                                },
                                Vector {
                                    x: speed_x,
                                    y: speed_y,
                                },
                            );
                        }
                    }
                    KeyCode::Char('f') => {
                        let pos_x = rng.gen_range(-w_float / 2.0..w_float / 2.0);
                        let speed_y = rng.gen_range(max_speed * 0.8..max_speed);
                        let speed_x = rng.gen_range(-0.08..0.08);
                        create_particle(
                            &mut app.particle_groups[rng.gen_range(0..COLORS.len())],
                            false,
                            Vector {
                                x: pos_x,
                                y: -h_float / 2.0,
                            },
                            Vector {
                                x: speed_x,
                                y: speed_y,
                            },
                        );
                    }
                    _ => {}
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            app.on_tick();
            last_tick = Instant::now();
        }
    }
}

fn ui<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(100)].as_ref())
        .split(f.size());
    let canvas = Canvas::default()
        .block(Block::default())
        .paint(|ctx| {
            for particle_group in app.particle_groups {
                ctx.draw(&Points {
                    color: particle_group.color,
                    coords: &particle_group.pos,
                });
            }
        })
        .x_bounds([
            -f64::from(f.size().width) / 2.0,
            f64::from(f.size().width) / 2.0,
        ])
        .y_bounds([
            -f64::from(f.size().height) / 2.0,
            f64::from(f.size().height) / 2.0,
        ]);
    f.render_widget(canvas, chunks[0]);
}
