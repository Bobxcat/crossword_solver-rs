use std::{array, collections::HashSet, fs, iter};

use itertools::{Itertools, iproduct};
use rayon::prelude::*;

/// The nth bit represents if the number `n` is possible in this cell
#[derive(Clone, Copy, Eq)]
struct Cell(u16);

impl PartialEq for Cell {
    fn eq(&self, other: &Self) -> bool {
        self.important_bits() == other.important_bits()
    }
}

impl std::fmt::Debug for Cell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.possibilities().iter().join(","))
    }
}

impl std::fmt::Display for Cell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", {
            match self.possibilities()[..] {
                [] => "F".into(),
                [num] => format!("{num}"),
                _ => "?".into(),
            }
        })
    }
}

impl Cell {
    fn fixed(num: u8) -> Self {
        Self(1 << num)
    }

    fn any_possible() -> Self {
        Self(u16::MAX)
    }

    fn none_possible() -> Self {
        Self(0)
    }

    /// The internal representation without the bits that don't carry information
    ///
    /// Only the bits `1` through `9` are important
    fn important_bits(&self) -> u16 {
        self.0 & 0b1111111110
    }

    fn is_possible(&self, num: u8) -> bool {
        (self.0 >> num) & 1 == 1
    }

    fn set_possible(&mut self, num: u8, possible: bool) {
        if possible {
            self.0 = self.0 | 1 << num
        } else {
            self.0 = self.0 & !(1 << num);
        }
    }

    /// Which numbers are possible for this cell
    fn possibilities(&self) -> Vec<u8> {
        (1u8..10).filter(|&num| self.is_possible(num)).collect()
    }

    fn num_possibilities(&self) -> usize {
        self.possibilities().len()
    }

    fn both_possible(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }

    fn not_possible(self) -> Self {
        Self(!self.0)
    }

    fn one_possible(self, other: Self) -> Self {
        Self(self.0 ^ other.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SquareIdx {
    TL,
    TM,
    TR,
    ML,
    MM,
    MR,
    BL,
    BM,
    BR,
}

impl SquareIdx {
    fn to_idx(self) -> usize {
        use SquareIdx::*;
        match self {
            TL => 0,
            TM => 1,
            TR => 2,
            ML => 3,
            MM => 4,
            MR => 5,
            BL => 6,
            BM => 7,
            BR => 8,
        }
    }

    fn from_idx(idx: usize) -> Self {
        use SquareIdx::*;
        match idx {
            0 => TL,
            1 => TM,
            2 => TR,
            3 => ML,
            4 => MM,
            5 => MR,
            6 => BL,
            7 => BM,
            8 => BR,
            _ => panic!(),
        }
    }

    fn to_topleft_cell(self) -> BoardIdx {
        use SquareIdx::*;
        match self {
            TL => BoardIdx::new(0, 0),
            TM => BoardIdx::new(3, 0),
            TR => BoardIdx::new(6, 0),
            ML => BoardIdx::new(0, 3),
            MM => BoardIdx::new(3, 3),
            MR => BoardIdx::new(6, 3),
            BL => BoardIdx::new(0, 6),
            BM => BoardIdx::new(3, 6),
            BR => BoardIdx::new(6, 6),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct BoardIdx {
    col: usize,
    row: usize,
    idx: usize,
}

impl BoardIdx {
    pub fn new(col: usize, row: usize) -> Self {
        Self {
            col,
            row,
            idx: col + row * 9,
        }
    }

    #[track_caller]
    fn square(&self) -> SquareIdx {
        use SquareIdx::*;
        match (self.col / 3, self.row / 3) {
            (0, 0) => TL,
            (1, 0) => TM,
            (2, 0) => TR,
            (0, 1) => ML,
            (1, 1) => MM,
            (2, 1) => MR,
            (0, 2) => BL,
            (1, 2) => BM,
            (2, 2) => BR,
            _ => unreachable!("Invalid board idx!"),
        }
    }
}

const BOARD_CELLS: usize = 9 * 9;

#[derive(Clone)]
struct Board {
    cells: [Cell; BOARD_CELLS],
    played: HashSet<BoardIdx>,
}

impl std::fmt::Debug for Board {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = String::new();
        for row in 0..9 {
            for col in 0..9 {
                s.push_str(&format!("{:?} ", self.get(BoardIdx::new(col, row))));
            }
            s.push('\n');
        }

        write!(f, "{s}")
    }
}

impl std::fmt::Display for Board {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = String::new();
        for row in 0..9 {
            for col in 0..9 {
                s.push_str(&format!("{} ", self.get(BoardIdx::new(col, row))));
            }
            s.push('\n');
        }

        write!(f, "{s}")
    }
}

impl Board {
    fn new() -> Self {
        Self {
            cells: array::from_fn(|_| Cell::any_possible()),
            played: HashSet::new(),
        }
    }

    fn from_str(board_str: &str) -> Self {
        let mut board = Board::new();

        let mut cells = board_str
            .chars()
            .into_iter()
            .flat_map(|c| match c.to_ascii_lowercase() {
                '1' => Some(1i32),
                '2' => Some(2),
                '3' => Some(3),
                '4' => Some(4),
                '5' => Some(5),
                '6' => Some(6),
                '7' => Some(7),
                '8' => Some(8),
                '9' => Some(9),
                'x' => Some(-1),
                _ => None,
            })
            .chain(iter::repeat(-1));

        for row in 0..9 {
            for col in 0..9 {
                let idx = BoardIdx::new(col, row);
                match u8::try_from(cells.next().unwrap()) {
                    Ok(num) => board.play_cell(idx, num),
                    Err(_) => (),
                }
            }
        }

        board
    }

    fn get(&self, idx: BoardIdx) -> Cell {
        self.cells[idx.idx]
    }

    fn get_mut(&mut self, idx: BoardIdx) -> &mut Cell {
        &mut self.cells[idx.idx]
    }

    fn set_raw(&mut self, idx: BoardIdx, cell: Cell) {
        self.cells[idx.idx] = cell;
    }

    fn play_cell(&mut self, idx: BoardIdx, num: u8) {
        self.played.insert(idx);
        self.set_raw(idx, Cell::fixed(num));
        for to_update in self
            .iter_square(idx.square())
            .into_iter()
            .chain(self.iter_col(idx.col))
            .chain(self.iter_row(idx.row))
            .unique()
        {
            if to_update == idx {
                continue;
            }
            self.get_mut(to_update).set_possible(num, false);
        }
    }

    fn verify(&self) -> Result<(), ()> {
        for seq in (0..9)
            .map(|col| self.iter_col(col))
            .chain((0..9).map(|row| self.iter_row(row)))
            .chain((0..9).map(|square| self.iter_square(SquareIdx::from_idx(square))))
        {
            let mut seen = HashSet::new();
            for elem in seq {
                match self.get(elem).possibilities()[..] {
                    [] => return Err(()),
                    [num] => {
                        if seen.insert(num) == false {
                            return Err(());
                        }
                    }
                    _ => (),
                }
            }
        }

        Ok(())
    }

    fn iter_square(&self, square: SquareIdx) -> [BoardIdx; 9] {
        let offset = square.to_topleft_cell();
        array::from_fn(|idx| {
            let col = idx / 3;
            let row = idx % 3;
            BoardIdx::new(col + offset.col, row + offset.row)
        })
    }

    /// 0-9, left to right
    fn iter_col(&self, col: usize) -> [BoardIdx; 9] {
        array::from_fn(|row| BoardIdx::new(col, row))
    }

    /// 0-9, top to bottom
    fn iter_row(&self, row: usize) -> [BoardIdx; 9] {
        array::from_fn(|col| BoardIdx::new(col, row))
    }
}

fn solve(mut board: Board) -> Option<Board> {
    if board.verify().is_err() {
        return None;
    }

    let mut least_possibilities_cell = None;
    let mut least_possibilities = usize::MAX;

    for (col, row) in iproduct!(0..9, 0..9) {
        let idx = BoardIdx::new(col, row);
        if board.played.contains(&idx) {
            continue;
        }

        let possibilities = board.get(idx).num_possibilities();
        if possibilities == 0 {
            return None;
        }
        if possibilities == 1 {
            board.play_cell(idx, board.get(idx).possibilities()[0]);
            continue;
        }
        if least_possibilities > possibilities {
            least_possibilities = possibilities;
            least_possibilities_cell = Some(idx);
        }
    }

    let Some(next) = least_possibilities_cell else {
        // This means all cells are played, thus it's solved
        return Some(board);
    };

    let possibilities = board.get(next).possibilities();
    possibilities.par_iter().find_map_any(|&possibility| {
        let mut new_board = board.clone();
        new_board.play_cell(next, possibility);

        solve(new_board)
    })
}

fn main() {
    let filename = std::env::args()
        .skip(1)
        .next()
        .unwrap_or("examples/easy1.txt".into());

    let Ok(board_str) = fs::read_to_string(&filename) else {
        println!(
            "Error finding file '{}'!\nMake sure the path is entered correctly and the file exists.",
            filename
        );
        return;
    };

    let board = Board::from_str(&board_str);
    println!("{board}");
    let ret = solve(board);

    println!("FINISHED!\n======\n");
    match ret {
        Some(board) => println!("{board}"),
        None => println!("Failed"),
    }
}
