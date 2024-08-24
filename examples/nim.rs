use std::{
    fmt::Display,
    io::{self, Write},
};

use yamcts::{rng::DefaultRng, GameState, MCTS};

#[derive(PartialEq, Eq, Clone, Copy)]
struct NimMove {
    start_player: bool,
    nums: i32, // 1,2,3
}

#[derive(PartialEq, Eq)]
enum WinCondition {
    StartPlayer,
    NotStartPlayer,
    Invalid,
}

#[derive(Clone, Copy)]
struct NimState {
    start_player: bool,
    current_num: i32,
}

impl Default for NimState {
    fn default() -> Self {
        Self {
            start_player: true,
            current_num: 0,
        }
    }
}

const TARGET_NUMBER: i32 = 21;

impl GameState for NimState {
    type Move = NimMove;
    type UserData = WinCondition;

    fn all_moves(&self) -> Vec<Self::Move> {
        let max = (TARGET_NUMBER - self.current_num).min(3);

        (1..=max)
            .into_iter()
            .map(|n| NimMove {
                start_player: !self.start_player,
                nums: n,
            })
            .collect()
    }

    fn apply_move(&self, action: Self::Move) -> Self {
        NimState {
            start_player: action.start_player,
            current_num: self.current_num + action.nums,
        }
    }

    fn is_terminal_state(&self) -> Option<Self::UserData> {
        use WinCondition::*;
        if self.current_num == TARGET_NUMBER {
            match self.start_player {
                true => Some(NotStartPlayer),
                false => Some(StartPlayer),
            }
        } else if self.current_num > TARGET_NUMBER {
            Some(Invalid)
        } else {
            None
        }
    }

    fn terminal_is_win(&self, condition: &Self::UserData) -> bool {
        match condition {
            WinCondition::StartPlayer => self.start_player,
            WinCondition::NotStartPlayer => !self.start_player,
            WinCondition::Invalid => false,
        }
    }
}

impl Display for NimState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("Current Number: {}", self.current_num))
    }
}

impl Display for NimMove {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.nums))
    }
}

fn readline(prompt: Option<&str>) -> io::Result<String> {
    if let Some(s) = prompt {
        print!("{} ", s);
        io::stdout().flush()?;
    }
    let mut buffer = String::new();
    io::stdin().read_line(&mut buffer)?;
    Ok(buffer)
}

fn get_num_from_player(max: i32) -> i32 {
    let prompt = format!("Avoid number {TARGET_NUMBER}. Enter a number between 1 and {max}:");
    loop {
        if let Ok(s) = readline(Some(&prompt)) {
            if let Ok(val) = s.trim().parse::<i32>() {
                if val >= 1 && val <= max {
                    return val;
                }
            }
        }
    }
}

fn main() -> io::Result<()> {
    let mcts = MCTS::<DefaultRng>::default();
    let mut game = NimState::default();

    loop {
        let best_move = mcts.run_with_duration(game.clone(), chrono::TimeDelta::seconds(1));

        let best_move = best_move.join();

        println!(
            "Computer chooses {} after considering {} moves.",
            best_move.best_move, best_move.iterations
        );

        game = game.apply_move(best_move.best_move);

        println!("{game}");

        if game.is_terminal_state().is_some() {
            println!("You win.");
            break;
        }

        let player_move = NimMove {
            start_player: true,
            nums: get_num_from_player((TARGET_NUMBER - game.current_num).min(3)),
        };

        game = game.apply_move(player_move);

        println!("{game}");

        if game.is_terminal_state().is_some() {
            println!("Computer wins.");
            break;
        }
    }
    Ok(())
}
