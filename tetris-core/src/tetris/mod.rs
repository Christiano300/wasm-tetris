mod board;
mod game;

pub use board::*;
pub use game::*;

#[cfg(test)]
mod test {
    use serde_test::{Token, assert_tokens};

    use super::{Board, Mino};

    #[test]
    fn test_ser_de() {
        let mut board = Board::default();
        board.buffer[0][1] = Mino::Garbage;
        board.buffer[0][2] = Mino::I;
        board.buffer[0][3] = Mino::J;
        board.buffer[0][4] = Mino::Z;
        board.buffer[0][5] = Mino::O;

        let mut tokens = vec![
            Token::Tuple { len: 400 },
            Token::UnitVariant {
                name: "Mino",
                variant: "Empty",
            },
            Token::UnitVariant {
                name: "Mino",
                variant: "Garbage",
            },
            Token::UnitVariant {
                name: "Mino",
                variant: "I",
            },
            Token::UnitVariant {
                name: "Mino",
                variant: "J",
            },
            Token::UnitVariant {
                name: "Mino",
                variant: "Z",
            },
            Token::UnitVariant {
                name: "Mino",
                variant: "O",
            },
        ];
        for _ in 0..(400 - 6) {
            tokens.push(Token::UnitVariant {
                name: "Mino",
                variant: "Empty",
            });
        }
        tokens.push(Token::TupleEnd);
        assert_tokens(&board, &tokens);
    }
}
