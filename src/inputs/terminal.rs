use std::io::BufRead;
use std::sync::mpsc::{self, Receiver};

use crate::game::domain::{PieceType, Square};

#[derive(Debug, PartialEq)]
pub enum Command {
    NewGame(Option<String>),                 // None for the initial position
    Move(Square, Square, Option<PieceType>), // optional promotion piece
}

/// Parses one terminal line: "ng", "ng <fen>", "mv e2e4" or "mv e7e8q".
pub fn parse_command(line: &str) -> Result<Command, String> {
    let line = line.trim();
    let (name, rest) = line.split_once(char::is_whitespace).unwrap_or((line, ""));
    let rest = rest.trim();

    match name {
        "ng" if rest.is_empty() => Ok(Command::NewGame(None)),
        "ng" => Ok(Command::NewGame(Some(rest.to_string()))),
        "mv" => parse_move(rest),
        "" => Err("empty command".to_string()),
        _ => Err(format!(
            "unknown command '{name}' (try: ng, ng <fen>, mv e2e4, mv e7e8q)"
        )),
    }
}

fn parse_move(text: &str) -> Result<Command, String> {
    let text = text.trim();
    if !text.is_ascii() || !(4..=5).contains(&text.len()) {
        return Err(format!(
            "invalid move '{text}', expected e.g. mv e2e4 or mv e7e8q"
        ));
    }
    let promotion = match text.as_bytes().get(4) {
        None => None,
        Some(b'q' | b'Q') => Some(PieceType::Queen),
        Some(b'r' | b'R') => Some(PieceType::Rook),
        Some(b'b' | b'B') => Some(PieceType::Bishop),
        Some(b'n' | b'N') => Some(PieceType::Knight),
        Some(_) => {
            return Err(format!(
                "invalid promotion piece '{}', use q, r, b or n",
                &text[4..]
            ));
        }
    };
    let from = text[..2]
        .parse::<Square>()
        .map_err(|_| format!("invalid square '{}'", &text[..2]))?;
    let to = text[2..4]
        .parse::<Square>()
        .map_err(|_| format!("invalid square '{}'", &text[2..4]))?;
    Ok(Command::Move(from, to, promotion))
}

/// Reads stdin on a background thread so the render loop never blocks.
pub struct Terminal {
    lines: Receiver<String>,
}

impl Terminal {
    pub fn new() -> Self {
        let (tx, lines) = mpsc::channel();
        std::thread::spawn(move || {
            for line in std::io::stdin().lock().lines().map_while(Result::ok) {
                if tx.send(line).is_err() {
                    break;
                }
            }
        });
        Self { lines }
    }

    /// Lines typed since the last call.
    pub fn poll(&self) -> Vec<String> {
        self.lines.try_iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_new_game() {
        assert_eq!(parse_command("ng"), Ok(Command::NewGame(None)));
        assert_eq!(
            parse_command("ng 8/8/8/8/8/8/8/K6k w - - 0 1"),
            Ok(Command::NewGame(Some("8/8/8/8/8/8/8/K6k w - - 0 1".into())))
        );
    }

    #[test]
    fn parses_move() {
        assert_eq!(
            parse_command("mv e2e4"),
            Ok(Command::Move(Square::E2, Square::E4, None))
        );
        assert_eq!(
            parse_command("mv e7e8n"),
            Ok(Command::Move(
                Square::E7,
                Square::E8,
                Some(PieceType::Knight)
            ))
        );
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse_command("mv e2").is_err());
        assert!(parse_command("mv e2e9").is_err());
        assert!(parse_command("mv e7e8k").is_err());
        assert!(parse_command("foo").is_err());
        assert!(parse_command("").is_err());
    }
}
