mod utils;
use std::cmp::PartialEq;
use std::convert::TryInto;
use wasm_bindgen::prelude::*;
use std::fmt;
use wasm_timer::Instant;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = window)]
    pub fn alert(s: &str);
    #[wasm_bindgen(js_namespace = Math)]
    fn random() -> f64;
}

#[wasm_bindgen]
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cell {
    Alive = 1,
    Dead = 0,
}

#[wasm_bindgen]
#[derive(Debug, Clone, Copy)]
pub enum DirectionName {
    Up,
    Down,
    Left,
    Right,
}

#[wasm_bindgen]
#[derive(Clone)]
pub struct Position {
    x: u32,
    y: u32,
}

#[wasm_bindgen]
pub struct Direction {
    vx: i32,
    vy: i32,
}

#[wasm_bindgen]
pub struct Snake {
    // head: Position,
    body: Vec<Position>,
    direction: Direction,
}

#[wasm_bindgen]
impl Snake {
    pub fn new() -> Snake {
        Snake {
            body: vec![
                Position { x: 5, y: 6 },
                Position { x: 4, y: 6 },
                Position { x: 3, y: 6 },
                Position { x: 2, y: 6 },
            ],
            direction: Direction { vx: 1, vy: 0 },
        }
    }

    fn set_direction(&mut self, vx: i32, vy: i32) {
        self.direction = Direction { vx, vy };
    }

    pub fn set_direction_name(&mut self, direction: DirectionName) {
        match direction {
            DirectionName::Up => self.set_direction(0, -1),
            DirectionName::Down => self.set_direction(0, 1),
            DirectionName::Left => self.set_direction(-1, 0),
            DirectionName::Right => self.set_direction(1, 0),
        }
    }

    pub fn has_index(&self, index: u32, universe_width: u32) -> bool {
        self.body.iter().any(|p| {
            let idx = p.y * universe_width + p.x;

            idx == index
        })
    }
}

#[wasm_bindgen]
#[derive(Debug, Clone, Copy)]
pub enum UniverseTopology {
    Flat,
    Toroidal,
}

pub struct FpsCounter {
    last_frame: Instant,
    frames: u32,
    fps: f64,
}

impl FpsCounter {
    pub fn new(fps_target: f64) -> FpsCounter {
        FpsCounter {
            last_frame: Instant::now(),
            frames: 0,
            fps: fps_target,
        }
    }

    pub fn tick(&mut self, fps_measurements: u32) {
        const AVG_LEARNING_RATE: f64 = 0.01;

        let now = Instant::now();
        let elapsed = now.duration_since(self.last_frame).as_nanos() as f64 / 1_000_000_000.0; // Convert to seconds
        self.last_frame = now;

        if self.frames != 0 {
            self.fps = self.fps * (1.0-AVG_LEARNING_RATE) + ((fps_measurements as f64) / elapsed) * AVG_LEARNING_RATE; // Exponential moving average
        }

        self.frames += 1;
    }
}

#[wasm_bindgen]
pub struct Universe {
    width: u32,
    height: u32,
    cells: Vec<Cell>,
    snake: Snake,
    apple: Option<Position>,
    game_over: bool,
    topology: UniverseTopology,
    counter: FpsCounter,
}

impl PartialEq for Position {
    fn eq(&self, other: &Self) -> bool {
        (self.x == other.x) && (self.y == other.y)
    }
}

#[wasm_bindgen]
impl Universe {
    fn add_u32_i32(&self, u: u32, i: i32, modulo: u32) -> u32 {
        (u as i64 + i as i64).rem_euclid(modulo as i64) as u32
    }

    fn randomize_apple(&mut self) {
        let apple_x = random_position(self.width.try_into().unwrap()) as u32;
        let apple_y = random_position(self.height.try_into().unwrap()) as u32;

        let apple_index = self.get_index(apple_y.try_into().unwrap(), apple_x.try_into().unwrap());
        if self.cells[apple_index] == Cell::Dead {
            self.cells[apple_index] = Cell::Alive; // Place an apple
        } else {
            // If the cell is already occupied, try again
            self.randomize_apple();
        }

        self.apple = Some(Position {
            x: apple_x,
            y: apple_y,
        });
    }

    fn get_index(&self, row: u32, column: u32) -> usize {
        (row * self.width + column) as usize
    }

    pub fn tick(&mut self, fps_measurements: u32) {
        if self.game_over {
            return;
        }

        let new_head = match self.topology {
            UniverseTopology::Flat => {
                let head = self.snake.body.first().unwrap();

                let new_x = head.x as i32 + self.snake.direction.vx;
                let new_y = head.y as i32 + self.snake.direction.vy;

                // Game over if out of bounds
                if new_x < 0 || new_y < 0 || new_x >= self.width as i32 || new_y >= self.height as i32 {
                    self.game_over = true;
                    return;
                }

                Position {
                    x: new_x as u32,
                    y: new_y as u32,
                }
            },
            UniverseTopology::Toroidal => {
                Position {
                    x: self.add_u32_i32(
                        self.snake.body.first().unwrap().x,
                        self.snake.direction.vx,
                        self.width,
                    ),
                    y: self.add_u32_i32(
                        self.snake.body.first().unwrap().y,
                        self.snake.direction.vy,
                        self.height,
                    ),
                }
            },
        };

        // Collision with body
        if self.snake.body.contains(&new_head) {
            self.game_over = true;
            return;
        }

        let mut next = self.cells.clone();

        if let Some(apple) = &self.apple {
            if new_head.eq(apple) {
                self.randomize_apple();
                let apple = self.apple.clone().unwrap();
                let apple_idx = self.get_index(apple.y, apple.x);
                next[apple_idx] = Cell::Alive;
            } else {
                let last = self.snake.body.pop().unwrap();
                let old_idx = self.get_index(last.y, last.x);
                next[old_idx] = Cell::Dead;
            }
        }

        self.snake.body.insert(0, new_head);
        let new_idx = self.get_index(
                    self.snake.body.first().unwrap().y,
                    self.snake.body.first().unwrap().x,
        );
        next[new_idx] = Cell::Alive;

        self.cells = next;

        if self.apple.is_none() {
            self.randomize_apple();
        }

        if fps_measurements > 0 {
            self.counter.tick(fps_measurements);
        }
    }


    pub fn on_click(&mut self, direction: DirectionName) {
        self.snake.set_direction_name(direction);
    }

    pub fn new(snake: Snake, fps_target: f64) -> Universe {
        utils::set_panic_hook();

        let width: u32 = 64;
        let height: u32 = 64;

        let cells = (0..width * height)
            .map(|i| {
                if snake.has_index(i, width) {
                    Cell::Alive
                } else {
                    Cell::Dead
                }
            })
            .collect();

        Universe {
            width,
            height,
            cells,
            snake,
            apple: None,
            game_over: false,
            topology: UniverseTopology::Toroidal,
            counter: FpsCounter::new(fps_target),
        }
    }

    pub fn render(&self) -> String {
        self.to_string()
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn cells(&self) -> *const Cell {
        self.cells.as_ptr()
    }

    pub fn snake_mut(&mut self) -> *mut Snake {
        &mut self.snake
    }

    pub fn is_game_over(&self) -> bool {
        self.game_over
    }

    pub fn fps(&self) -> f64 {
        self.counter.fps
    }

    pub fn topology(&self) -> UniverseTopology {
        self.topology.clone()
    }

    pub fn toggle_topology(&mut self) {
        self.topology = match self.topology {
            UniverseTopology::Flat => UniverseTopology::Toroidal,
            UniverseTopology::Toroidal => UniverseTopology::Flat,
        };
    }
}

impl fmt::Display for Universe {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for line in self.cells.as_slice().chunks(self.width as usize) {
            for &cell in line {
                let symbol = if cell == Cell::Dead { '◻' } else { '◼' };
                write!(f, "{}", symbol)?;
            }
            write!(f, "\n")?;
        }

        Ok(())
    }
}

#[wasm_bindgen]
pub fn random_position(max: i32) -> i32 {
    (random() * (max as f64)).floor() as i32
}
