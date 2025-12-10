use std::rc::Rc;

use wasm_bindgen::{JsCast, JsValue};
use web_sys::{
    CanvasRenderingContext2d, OffscreenCanvas, OffscreenCanvasRenderingContext2d as CanvasContext,
};

use tetris_core::tetris::{Board, Mino, Tetrimino};

const fn get_base_color(kind: Mino) -> Color {
    match kind {
        Mino::Empty => Color::no_alpha(0, 0, 0),
        Mino::Garbage => Color::no_alpha(100, 100, 100),
        Mino::I => Color::no_alpha(0, 200, 255),
        Mino::O => Color::no_alpha(255, 255, 0),
        Mino::T => Color::no_alpha(127, 0, 127),
        Mino::S => Color::no_alpha(0, 255, 0),
        Mino::Z => Color::no_alpha(255, 0, 0),
        Mino::J => Color::no_alpha(0, 0, 255),
        Mino::L => Color::no_alpha(255, 150, 0),
    }
}

pub struct DrawingContext {
    i: SubImage,
    o: SubImage,
    t: SubImage,
    s: SubImage,
    z: SubImage,
    j: SubImage,
    l: SubImage,
    garbage: SubImage,
    board: SubImage,
}

impl DrawingContext {
    pub fn clear(ctx: &CanvasRenderingContext2d) {
        ctx.clear_rect(0., 0., 1000., 700.);
    }

    pub fn new() -> Self {
        Self {
            i: Self::make_mino(Mino::I),
            o: Self::make_mino(Mino::O),
            t: Self::make_mino(Mino::T),
            s: Self::make_mino(Mino::S),
            z: Self::make_mino(Mino::Z),
            j: Self::make_mino(Mino::J),
            l: Self::make_mino(Mino::L),
            garbage: Self::make_mino(Mino::Garbage),
            board: Self::make_board(),
        }
    }

    fn make_mino(mino: Mino) -> SubImage {
        let base_color = get_base_color(mino);
        SubImage::new(30, 30, |ctx| {
            let gradient = ctx.create_linear_gradient(0., 0., 0., 30.);
            let _ = gradient.add_color_stop(1., &base_color.lighten(0.3).to_css());
            let _ = gradient.add_color_stop(0., &base_color.lighten(0.7).to_css());

            ctx.set_fill_style_canvas_gradient(&gradient);
            ctx.fill_rect(2., 2., 28., 28.);

            ctx.set_stroke_style_str(&base_color.lighten(0.2).darken(0.15).to_css());
            ctx.set_line_width(2.);
            ctx.begin_path();
            let _ = ctx.round_rect_with_f64(2., 2., 27., 27., 2.);
            ctx.stroke();

            ctx.set_stroke_style_str(&base_color.lighten(0.3).darken(0.4).to_css());
            ctx.set_line_width(1.);
            ctx.begin_path();
            let _ = ctx.round_rect_with_f64(1., 1., 29., 29., 2.);
            ctx.stroke();
        })
    }

    fn draw_ghost_mino(ctx: &CanvasContext, base_color: Color) {
        ctx.set_stroke_style_str(&base_color.lighten(0.7).alpha(50).to_css());
        ctx.set_line_width(1.);
        let _ = ctx.round_rect_with_f64(1., 1., 29., 29., 2.);
        ctx.stroke();
    }

    fn make_board() -> SubImage {
        let width = 319;
        let height = 629;
        SubImage::new(width, height, |ctx| {
            let width = f64::from(width);
            let height = f64::from(height);
            ctx.set_fill_style_str(&Color::no_alpha(40, 40, 40).to_css());
            ctx.fill_rect(0., 0., width, height);

            ctx.set_stroke_style_str(&Color::no_alpha(70, 70, 70).to_css());
            ctx.set_line_width(5.);
            ctx.stroke_rect(2., 2., width - 4., height - 4.);

            ctx.set_stroke_style_str(&Color::no_alpha(70, 70, 70).to_css());
            ctx.set_line_width(0.5);
            ctx.begin_path();
            for i in 0..=10 {
                let x = f64::from(i) * 31. + 5.;
                ctx.move_to(x, 5.);
                ctx.line_to(x, height - 5.);
            }

            for i in 0..=20 {
                let y = f64::from(i) * 31. + 5.;
                ctx.move_to(5., y);
                ctx.line_to(width - 5., y);
            }
            ctx.stroke();
        })
    }

    pub fn draw_board(&self, ctx: &CanvasRenderingContext2d, off_x: f64, off_y: f64) {
        let _ = self.board.draw(ctx, off_x, off_y);
    }

    const fn get_mino_image(&self, kind: Mino) -> Option<&SubImage> {
        Some(match kind {
            Mino::Empty => return None,
            Mino::Garbage => &self.garbage,
            Mino::I => &self.i,
            Mino::O => &self.o,
            Mino::J => &self.j,
            Mino::L => &self.l,
            Mino::S => &self.s,
            Mino::Z => &self.z,
            Mino::T => &self.t,
        })
    }

    pub fn draw_field(
        &self,
        ctx: &CanvasRenderingContext2d,
        field: &[[Mino; 10]; 40],
        off_x: f64,
        off_y: f64,
    ) {
        for row in 0..20 {
            #[allow(clippy::needless_range_loop)]
            for col in 0..10 {
                let mino = field[row + 20][col];
                let Some(image) = self.get_mino_image(mino) else {
                    continue;
                };
                let _ = image.draw(ctx, (col as f64) * 31. + off_x, (row as f64) * 31. + off_y);
            }
        }
    }

    pub fn draw_tetrimino(
        &self,
        ctx: &CanvasRenderingContext2d,
        tetrimino: &Tetrimino,
        off_x: f64,
        off_y: f64,
        ghost: bool,
        outside_grid: bool,
    ) {
        let image = if ghost {
            &SubImage::new(30, 30, |ctx| {
                Self::draw_ghost_mino(ctx, get_base_color(tetrimino.kind));
            })
        } else {
            let Some(image) = self.get_mino_image(tetrimino.kind) else {
                return;
            };
            image
        };
        for (y, row) in tetrimino.grid.iter().enumerate() {
            for (x, mino) in row.iter().enumerate() {
                if *mino {
                    let dx = if outside_grid {
                        (x as f64) * 31. + off_x
                    } else {
                        (x + tetrimino.offset_x as usize) as f64 * 31. + off_x
                    };
                    let dy = if outside_grid {
                        (y as f64) * 31. + off_y
                    } else {
                        (y + tetrimino.offset_y as usize - 20) as f64 * 31. + off_y
                    };
                    let _ = image.draw(ctx, dx, dy);
                }
            }
        }
    }

    pub fn draw_score(ctx: &CanvasRenderingContext2d, score: u32, x: f64, y: f64) {
        ctx.clear_rect(x, y, 240., 30.);
        ctx.set_fill_style_str("#099520");
        ctx.set_text_baseline("top");
        ctx.set_font("25px sans-serif");
        let _ = ctx.fill_text_with_max_width(&format!("Score: {score}"), x, y, 240.);
    }

    pub fn draw_hold(
        &self,
        ctx: &CanvasRenderingContext2d,
        hold: Option<&Tetrimino>,
        x: f64,
        y: f64,
    ) {
        ctx.clear_rect(x, y, 31. * 4., 31. * 2.);
        if let Some(hold) = hold {
            self.draw_tetrimino(ctx, hold, x, y, false, true);
        }
    }

    pub fn draw_level(ctx: &CanvasRenderingContext2d, level: u8, x: f64, y: f64) {
        let text_width = 100.;
        ctx.clear_rect(x - text_width, y, text_width, 30.);
        ctx.set_fill_style_str("#099520");
        ctx.set_text_baseline("top");
        ctx.set_font("25px sans-serif");
        ctx.set_text_align("right");
        let _ = ctx.fill_text(&format!("Level {level} "), x, y);
        ctx.set_text_align("start");
    }

    pub fn draw_queue<'a>(
        &self,
        ctx: &CanvasRenderingContext2d,
        next_queue: impl Iterator<Item = &'a Tetrimino>,
        x: f64,
        y: f64,
    ) {
        ctx.clear_rect(x, y, 31. * 4., 450. + 2. * 31.);
        for (i, tetrimino) in next_queue.enumerate() {
            self.draw_tetrimino(ctx, tetrimino, x, y + 85. * i as f64, false, true);
        }
    }

    pub fn draw_opponent_board(ctx: &CanvasRenderingContext2d, board: &Board, x: f64, y: f64) {
        ctx.set_fill_style_str("#333");
        ctx.fill_rect(x, y, 11. * 10., 21. * 10.);
        for row in 0..20 {
            for col in 0..10 {
                let mino = board.buffer[row + 20][col];
                ctx.set_fill_style_str(&get_base_color(mino).to_css());
                ctx.fill_rect(
                    (col as f64) * 10. + x + 5.,
                    (row as f64) * 10. + y + 5.,
                    10.,
                    10.,
                );
            }
        }
    }
}

struct SubImage {
    canvas: Rc<OffscreenCanvas>,
}

impl SubImage {
    fn new(width: u32, height: u32, init: impl FnOnce(&mut CanvasContext)) -> Self {
        let canvas = OffscreenCanvas::new(width, height).unwrap();
        let mut context = canvas
            .get_context("2d")
            .unwrap()
            .unwrap()
            .dyn_into::<CanvasContext>()
            .unwrap();
        // remove half-pixel offset
        let _ = context.translate(-0.5, -0.5);
        init(&mut context);
        Self {
            canvas: Rc::new(canvas),
        }
    }

    fn draw(&self, ctx: &CanvasRenderingContext2d, x: f64, y: f64) -> Result<(), JsValue> {
        ctx.draw_image_with_offscreen_canvas(&self.canvas, x, y)
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Default)]
struct Color {
    r: u8,
    b: u8,
    g: u8,
    alpha: Option<u8>,
}

impl Color {
    const fn no_alpha(r: u8, g: u8, b: u8) -> Self {
        Self {
            r,
            g,
            b,
            alpha: None,
        }
    }

    fn to_css(self) -> String {
        self.alpha.map_or_else(
            || format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b),
            |alpha| format!("rgb({} {} {} / {alpha}%)", self.r, self.g, self.b),
        )
    }

    fn darken(self, amount: f64) -> Self {
        let multi = 1. - amount;
        Self {
            r: (f64::from(self.r) * multi) as u8,
            g: (f64::from(self.g) * multi) as u8,
            b: (f64::from(self.b) * multi) as u8,
            ..self
        }
    }

    #[inline]
    fn lighten_single(v: u8, amount: f64) -> u8 {
        255 - (f64::from(255 - v) * (1. - amount)) as u8
    }

    fn lighten(self, amount: f64) -> Self {
        Self {
            r: Self::lighten_single(self.r, amount),
            g: Self::lighten_single(self.g, amount),
            b: Self::lighten_single(self.b, amount),
            ..self
        }
    }

    const fn alpha(self, value: u8) -> Self {
        Self {
            alpha: Some(value),
            ..self
        }
    }
}
