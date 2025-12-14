#![allow(dead_code)]
use std::{collections::VecDeque, time::Duration};

use crossterm::event::{self, KeyCode};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Paragraph},
};
use tetris_core::tetris::{Game, Mino};

use crate::environment::Environment;

const IMG_WIDTH: usize = 5 + 10 + 2 + 5;
const IMG_HEIGHT: usize = 1 + 20 + 2;

type Image = [[Color; IMG_WIDTH]; IMG_HEIGHT];

const fn get_base_color(kind: Mino) -> Color {
    match kind {
        Mino::Empty => Color::Rgb(0, 0, 0),
        Mino::Garbage => Color::Rgb(100, 100, 100),
        Mino::I => Color::Rgb(0, 200, 255),
        Mino::O => Color::Rgb(255, 255, 0),
        Mino::T => Color::Rgb(127, 0, 127),
        Mino::S => Color::Rgb(0, 255, 0),
        Mino::Z => Color::Rgb(255, 0, 0),
        Mino::J => Color::Rgb(0, 0, 255),
        Mino::L => Color::Rgb(255, 150, 0),
    }
}

fn render_as_text(img: &Image) -> Text<'_> {
    Text::from(
        img.iter()
            .map(|row| {
                Line::from(
                    row.iter()
                        .map(|color| Span::styled("  ", Style::default().bg(*color)))
                        .collect::<Vec<_>>(),
                )
            })
            .collect::<Vec<_>>(),
    )
}

pub fn draw_game(game: &Game, frame: &mut Frame, area: Rect) {
    let mut img = [[Color::Reset; IMG_WIDTH]; IMG_HEIGHT];

    // Border
    for y in 1..IMG_HEIGHT - 1 {
        for x in 5..IMG_WIDTH - 5 {
            if x == 5 || x == IMG_WIDTH - 6 || y == 1 || y == IMG_HEIGHT - 2 {
                img[y][x] = Color::Rgb(50, 50, 50);
            }
        }
    }

    if let Some(hold) = &game.hold {
        for (y, row) in hold.grid.iter().enumerate() {
            for (x, cell) in row.iter().enumerate() {
                if *cell {
                    img[y][x] = get_base_color(hold.kind);
                }
            }
        }
    }

    for (i, next) in game.next_queue.iter().enumerate() {
        for (y, row) in next.grid.iter().enumerate() {
            for (x, cell) in row.iter().enumerate() {
                if *cell {
                    img[y + i * 4][x + IMG_WIDTH - 5] = get_base_color(next.kind);
                }
            }
        }
    }

    for y in 0..20 {
        for x in 0..10 {
            let mino = game.board.buffer[y + 20][x];
            img[y + 1][x + 6] = get_base_color(mino);
        }
    }

    for (y, row) in game.piece.grid.iter().enumerate() {
        for (x, cell) in row.iter().enumerate() {
            if *cell {
                let board_x = game.piece.offset_x as isize + x as isize;
                let board_y = game.piece.offset_y as isize + y as isize - 20;
                if board_x >= 0 && board_x < 10 && board_y >= 0 && board_y < 20 {
                    img[board_y as usize + 1][board_x as usize + 6] =
                        get_base_color(game.piece.kind);
                }
            }
        }
    }

    frame.render_widget(
        Paragraph::new(render_as_text(&img))
            .block(Block::bordered().border_type(BorderType::Rounded)),
        area,
    );
}

pub enum Event {
    Quit,
}

pub struct Stat {
    pub episode: usize,
    pub reward: f32,
    pub steps: usize,
    pub reward_per_step: f32,
    pub loss: f32,
}

pub struct Tui {
    terminal: DefaultTerminal,
    stats: VecDeque<Stat>,
}

impl Tui {
    pub fn init() -> Self {
        let terminal = ratatui::init();
        // Set panic hook
        fn panic_hook(info: &std::panic::PanicHookInfo) {
            ratatui::restore();
            eprintln!("Panic occurred: {}", info);
        }
        std::panic::set_hook(Box::new(panic_hook));
        Tui {
            terminal,
            stats: VecDeque::new(),
        }
    }

    pub fn new_stat(&mut self, stat: Stat) {
        if self.stats.len() >= 10 {
            self.stats.pop_front();
        }
        self.stats.push_back(stat);
    }

    pub fn render(&mut self, env: &Environment, loss: Option<f32>, reward: f32) -> Option<Event> {
        self.terminal
            .draw(|frame| {
                Self::draw(frame, env, &self.stats, loss, reward);
            })
            .expect("Failed to draw frame");
        self.handle_events()
    }

    pub fn end(self) {
        ratatui::restore();
    }

    fn draw(
        frame: &mut Frame,
        env: &Environment,
        stats: &VecDeque<Stat>,
        loss: Option<f32>,
        reward: f32,
    ) {
        let chunks = Layout::horizontal([
            Constraint::Fill(1),
            Constraint::Min(IMG_WIDTH as u16 * 2 + 2),
        ])
        .split(frame.area());
        draw_game(&env.game, frame, chunks[1]);
        let mut lines = vec![
            Line::from(format!("Current Reward: {:>7.2}", reward)),
            Line::from(format!(
                "Current Loss:    {}",
                match loss {
                    Some(l) => format!("{:>7.5}", l),
                    None => "N/A".to_string(),
                }
            )),
            Line::from("Last episodes: "),
            Line::from("Episode |  Reward  | Steps |   RPS   | Loss "),
        ];
        lines.extend(stats.iter().rev().map(|stat| {
            Line::from(format!(
                "{:>7} | {:>7.2} | {:>5} | {:>6.3} | {:>6.3}",
                stat.episode, stat.reward, stat.steps, stat.reward_per_step, stat.loss
            ))
        }));
        lines.extend_from_slice(&[
            Line::from(""),
            Line::from("Controls: "),
            Line::from(" q: Quit Training"),
        ]);
        let paragraph = Paragraph::new(Text::from(lines)).block(
            Block::bordered()
                .title("Tetris AI")
                .border_type(BorderType::Rounded),
        );
        frame.render_widget(paragraph, chunks[0]);
    }

    fn handle_events(&self) -> Option<Event> {
        while event::poll(Duration::ZERO).expect("Failed to poll event") {
            let event = event::read().expect("Failed to read event");
            if let event::Event::Key(event) = event {
                if !event.kind.is_press() {
                    continue;
                }
                return Some(match event.code {
                    KeyCode::Char('q') => Event::Quit,
                    _ => continue,
                });
            }
        }
        None
    }
}
