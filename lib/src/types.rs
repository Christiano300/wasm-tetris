use core::panic;

#[repr(u8)]
#[derive(Default, Copy, Clone, PartialEq, Eq)]
pub enum Mino {
    #[default]
    Empty,
    I,
    O,
    J,
    L,
    S,
    Z,
    T,
}

#[repr(u8)]
#[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Rotation {
    #[default]
    O,
    R,
    T,
    L,
}

impl Rotation {
    #[inline]
    fn cw(&self) -> Self {
        match *self {
            Self::O => Self::R,
            Self::R => Self::T,
            Self::T => Self::L,
            Self::L => Self::O,
        }
    }

    #[inline]
    fn ccw(&self) -> Self {
        match *self {
            Self::O => Self::L,
            Self::R => Self::O,
            Self::T => Self::R,
            Self::L => Self::T,
        }
    }

    pub fn rotate(&self, direction: Direction) -> Self {
        match direction {
            Direction::Cw => self.cw(),
            Direction::Ccw => self.ccw(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Cw,
    Ccw,
}

pub struct Board {
    pub buffer: [[Mino; 10]; 40],
}

impl Default for Board {
    fn default() -> Self {
        Self {
            buffer: [[Mino::Empty; 10]; 40],
        }
    }
}

impl Board {
    pub fn place(&mut self, tetrimino: &Tetrimino) {
        for (y, row) in tetrimino.grid.iter().enumerate() {
            for (x, mino) in row.iter().enumerate() {
                if *mino {
                    self.buffer[y + tetrimino.offset_y as usize][x + tetrimino.offset_x as usize] =
                        tetrimino.kind;
                }
            }
        }
    }

    fn check_collisions(&self, tetrimino: &Tetrimino) -> bool {
        for (y, row) in tetrimino.grid.iter().enumerate() {
            for (x, mino) in row.iter().enumerate() {
                if *mino
                    && self.buffer[y + tetrimino.offset_y as usize][x + tetrimino.offset_x as usize]
                        != Mino::Empty
                {
                    return true;
                }
            }
        }
        false
    }

    fn check_oob(&self, tetrimino: &Tetrimino) -> bool {
        for (y, row) in tetrimino.grid.iter().enumerate() {
            for (x, mino) in row.iter().enumerate() {
                if !*mino {
                    continue;
                }
                let x = x as i8 + tetrimino.offset_x;
                if !(0..10).contains(&x) {
                    return true;
                }
                let y = y as i8 + tetrimino.offset_y;
                if !(0..40).contains(&y) {
                    return true;
                }
            }
        }
        false
    }

    pub fn can_place(&self, tetrimino: &Tetrimino) -> bool {
        !self.check_oob(tetrimino) && !self.check_collisions(tetrimino)
    }

    pub fn drop(&mut self, tetrimino: &mut Tetrimino) {
        while self.can_place(tetrimino) {
            tetrimino.offset_y += 1;
        }
        tetrimino.offset_y -= 1;
    }

    /// Rotates the tetrimino with wall-kicks. Returns if the rotation was successful.
    pub fn rotate(&self, tetrimino: &mut Tetrimino, direction: Direction) -> bool {
        let offsets = match tetrimino.kind {
            Mino::O | Mino::Empty => return true,
            Mino::J | Mino::L | Mino::S | Mino::Z | Mino::T => {
                Board::get_three_offsets(tetrimino.rotation, direction)
            }
            Mino::I => Board::get_i_offsets(tetrimino.rotation, direction),
        };

        let mut clone = tetrimino.clone();
        clone.rotate_grid(direction);

        for (x, y) in offsets {
            clone.offset_x += x;
            clone.offset_y += y;
            if !self.can_place(&clone) {
                clone.offset_x -= x;
                clone.offset_y -= y;
                continue;
            }
            tetrimino.rotate_grid(direction);
            tetrimino.offset_x += x;
            tetrimino.offset_y += y;
            return true;
        }

        false
    }

    fn get_three_offsets(rot: Rotation, dir: Direction) -> [(i8, i8); 5] {
        let to = rot.rotate(dir);
        use Rotation as R;
        match (rot, to) {
            (R::O, R::R) => [(0, 0), (-1, 0), (-1, -1), (0, 2), (-1, 2)],
            (R::R, R::O) => [(0, 0), (1, 0), (1, 1), (0, -2), (1, -2)],
            (R::R, R::T) => [(0, 0), (1, 0), (1, 1), (0, -2), (1, -2)],
            (R::T, R::R) => [(0, 0), (-1, 0), (-1, -1), (0, 2), (-1, 2)],
            (R::T, R::L) => [(0, 0), (1, 0), (1, -1), (0, 2), (1, 2)],
            (R::L, R::T) => [(0, 0), (-1, 0), (-1, 1), (0, -2), (-1, -2)],
            (R::L, R::O) => [(0, 0), (-1, 0), (-1, 1), (0, -2), (-1, -2)],
            (R::O, R::L) => [(0, 0), (1, 0), (1, -1), (0, 2), (1, 2)],
            _ => panic!("Tried to invalid rotation from: {rot:?}, to: {to:?}"),
        }
    }

    fn get_i_offsets(rot: Rotation, dir: Direction) -> [(i8, i8); 5] {
        let to = rot.rotate(dir);
        use Rotation as R;
        match (rot, to) {
            (R::O, R::R) => [(0, 0), (-2, 0), (1, 0), (-2, 1), (1, -2)],
            (R::R, R::O) => [(0, 0), (2, 0), (-1, 0), (2, -1), (-1, 2)],
            (R::R, R::T) => [(0, 0), (-1, 0), (2, 0), (-1, -2), (2, 1)],
            (R::T, R::R) => [(0, 0), (1, 0), (-2, 0), (1, 2), (-2, -1)],
            (R::T, R::L) => [(0, 0), (2, 0), (-1, 0), (2, -1), (-1, 2)],
            (R::L, R::T) => [(0, 0), (-2, 0), (1, 0), (-2, 1), (1, -2)],
            (R::L, R::O) => [(0, 0), (1, 0), (-2, 0), (1, 2), (-2, -1)],
            (R::O, R::L) => [(0, 0), (-1, 0), (2, 0), (-1, -2), (2, 1)],
            _ => panic!("Tried invalid rotation from: {rot:?}, to: {to:?}"),
        }
    }

    /// Move along the x-axis. Returns if movement was a success
    pub fn move_x(&self, tetrimino: &mut Tetrimino, offset: i8) -> bool {
        tetrimino.offset_x += offset;
        if self.can_place(tetrimino) {
            return true;
        }
        tetrimino.offset_x -= offset;
        false
    }

    /// Move along the x-axis. Returns if movement was a success
    pub fn move_down(&self, tetrimino: &mut Tetrimino) -> bool {
        tetrimino.offset_y += 1;
        if self.can_place(tetrimino) {
            return true;
        }
        tetrimino.offset_y -= 1;
        false
    }

    pub fn can_move_down(&self, tetrimino: &mut Tetrimino) -> bool {
        tetrimino.offset_y += 1;
        let can_move = self.can_place(tetrimino);
        tetrimino.offset_y -= 1;
        can_move
    }

    pub fn clear_lines(&mut self) -> u8 {
        let mut count = 0;
        for i in 0..40 {
            let row = self.buffer[i];
            if row.iter().all(|x| *x != Mino::Empty) {
                count += 1;
                for j in (1..=i).rev() {
                    let copy = self.buffer[j - 1];
                    self.buffer[j] = copy;
                }
                self.buffer[0] = [Mino::Empty; 10]
            }
        }
        count
    }
}

/// Represents a Tetrimino currently being dropped, or a ghost, or a "shadow" used for rotation
/// testing or in the preview queue
#[derive(Clone)]
pub struct Tetrimino {
    pub kind: Mino,
    pub rotation: Rotation,
    pub grid: Vec<Vec<bool>>,
    pub offset_x: i8,
    pub offset_y: i8,
}

impl Tetrimino {
    pub fn new(kind: Mino, x: i8, y: i8) -> Self {
        Tetrimino {
            kind,
            rotation: Rotation::O,
            offset_x: x,
            offset_y: y,
            grid: match kind {
                Mino::Empty => panic!("Tried to create an empty Tetrimino"),
                Mino::O => vec![vec![true, true], vec![true, true]],
                Mino::I => vec![
                    vec![false, false, false, false],
                    vec![true, true, true, true],
                    vec![false, false, false, false],
                    vec![false, false, false, false],
                ],
                Mino::T => vec![
                    vec![false, true, false],
                    vec![true, true, true],
                    vec![false, false, false],
                ],
                Mino::L => vec![
                    vec![false, false, true],
                    vec![true, true, true],
                    vec![false, false, false],
                ],
                Mino::J => vec![
                    vec![true, false, false],
                    vec![true, true, true],
                    vec![false, false, false],
                ],
                Mino::S => vec![
                    vec![false, true, true],
                    vec![true, true, false],
                    vec![false, false, false],
                ],
                Mino::Z => vec![
                    vec![true, true, false],
                    vec![false, true, true],
                    vec![false, false, false],
                ],
            },
        }
    }

    pub fn rotate_grid(&mut self, direction: Direction) {
        match direction {
            Direction::Cw => self.rotate_grid_cw(),
            Direction::Ccw => self.rotate_grid_ccw(),
        }
    }

    fn rotate_grid_cw(&mut self) {
        if self.kind == Mino::O {
            return;
        }
        let n = self.grid.len();
        for i in 0..(n / 2) {
            for j in i..(n - i - 1) {
                let temp = self.grid[i][j];
                self.grid[i][j] = self.grid[n - 1 - j][i];
                self.grid[n - 1 - j][i] = self.grid[n - 1 - i][n - 1 - j];
                self.grid[n - 1 - i][n - 1 - j] = self.grid[j][n - 1 - i];
                self.grid[j][n - 1 - i] = temp;
            }
        }
    }

    fn rotate_grid_ccw(&mut self) {
        if self.kind == Mino::O {
            return;
        }
        let n = self.grid.len();
        for i in 0..(n / 2) {
            for j in i..(n - i - 1) {
                let temp = self.grid[i][j];
                self.grid[i][j] = self.grid[j][n - 1 - i];
                self.grid[j][n - 1 - i] = self.grid[n - 1 - i][n - 1 - j];
                self.grid[n - 1 - i][n - 1 - j] = self.grid[n - 1 - j][i];
                self.grid[n - 1 - j][i] = temp;
            }
        }
    }
}
