use std::sync::mpsc::{self, Receiver};

use rustyline::DefaultEditor;
use rustyline::error::ReadlineError;

use crate::game::domain::{PieceType, Side, Square};

#[derive(Debug, PartialEq)]
pub enum Command {
    NewGame(Option<String>),                 // None for the initial position
    Move(Square, Square, Option<PieceType>), // optional promotion piece
    Undo(usize),                             // number of moves to take back
    History,
    Engine(Option<String>), // UCI engine path to play against, None to switch it off
    Side(Side),             // colour played by the human against the engine
    MoveTime(u64),          // engine thinking time per move, in milliseconds
    Exit,
}

/// Parses one terminal line: "ng", "ng <fen>", "mv e2e4", "mv e7e8q" or "exit".
pub fn parse_command(line: &str) -> Result<Command, String> {
    let line = line.trim();
    let (name, rest) = line.split_once(char::is_whitespace).unwrap_or((line, ""));
    let rest = rest.trim();

    match name {
        "ng" if rest.is_empty() => Ok(Command::NewGame(None)),
        "ng" => Ok(Command::NewGame(Some(rest.to_string()))),
        "mv" => parse_move(rest),
        "undo" => parse_undo(rest),
        "engine" if rest.is_empty() => {
            Err("usage: engine <path-to-uci-engine> | engine off".into())
        }
        "engine" if rest == "off" => Ok(Command::Engine(None)),
        "engine" => Ok(Command::Engine(Some(rest.to_string()))),
        "side" => parse_side(rest),
        "movetime" => parse_movetime(rest),
        "history" if rest.is_empty() => Ok(Command::History),
        "exit" | "quit" if rest.is_empty() => Ok(Command::Exit),
        "" => Err("empty command".to_string()),
        _ => Err(format!(
            "unknown command '{name}' (try: ng, ng <fen>, mv e2e4, mv e7e8q, undo [n], history, engine <path>, engine off, side white|black, movetime <ms>, exit)"
        )),
    }
}

fn parse_side(text: &str) -> Result<Command, String> {
    match text.to_ascii_lowercase().as_str() {
        "white" | "w" => Ok(Command::Side(Side::White)),
        "black" | "b" => Ok(Command::Side(Side::Black)),
        _ => Err(format!(
            "invalid side '{text}', expected side white or side black"
        )),
    }
}

fn parse_movetime(text: &str) -> Result<Command, String> {
    match text.parse::<u64>() {
        Ok(ms) if ms > 0 => Ok(Command::MoveTime(ms)),
        _ => Err(format!(
            "invalid time '{text}', expected milliseconds, e.g. movetime 1000"
        )),
    }
}

fn parse_undo(text: &str) -> Result<Command, String> {
    if text.is_empty() {
        return Ok(Command::Undo(1));
    }
    match text.parse::<usize>() {
        Ok(n) if n > 0 => Ok(Command::Undo(n)),
        _ => Err(format!(
            "invalid count '{text}', expected e.g. undo or undo 2"
        )),
    }
}

pub fn parse_move(text: &str) -> Result<Command, String> {
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

/// Reads commands on a background thread so the render loop never blocks.
/// Uses a line editor, so up/down recall history and left/right edit the line.
pub struct Terminal {
    lines: Receiver<String>,
}

impl Terminal {
    pub fn new() -> Self {
        let (tx, lines) = mpsc::channel();
        std::thread::spawn(move || {
            let mut editor = match DefaultEditor::new() {
                Ok(editor) => editor,
                Err(e) => {
                    eprintln!("Terminal input unavailable: {e}");
                    return;
                }
            };
            loop {
                match editor.readline("> ") {
                    Ok(line) => {
                        if !line.trim().is_empty() {
                            let _ = editor.add_history_entry(line.as_str());
                        }
                        if tx.send(line).is_err() {
                            break;
                        }
                    }
                    // Ctrl-C / Ctrl-D ask the app to quit, like the "exit" command.
                    Err(ReadlineError::Interrupted | ReadlineError::Eof) => {
                        let _ = tx.send("exit".to_string());
                        break;
                    }
                    Err(_) => break,
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
    fn parses_undo_and_history() {
        assert_eq!(parse_command("undo"), Ok(Command::Undo(1)));
        assert_eq!(parse_command("undo 3"), Ok(Command::Undo(3)));
        assert!(parse_command("undo 0").is_err());
        assert!(parse_command("undo x").is_err());
        assert_eq!(parse_command("history"), Ok(Command::History));
        assert!(parse_command("history 2").is_err());
    }

    #[test]
    fn parses_engine_commands() {
        assert_eq!(
            parse_command("engine /usr/bin/stockfish"),
            Ok(Command::Engine(Some("/usr/bin/stockfish".into())))
        );
        assert_eq!(parse_command("engine off"), Ok(Command::Engine(None)));
        assert!(parse_command("engine").is_err());
        assert_eq!(parse_command("side black"), Ok(Command::Side(Side::Black)));
        assert!(parse_command("side red").is_err());
        assert_eq!(parse_command("movetime 500"), Ok(Command::MoveTime(500)));
        assert!(parse_command("movetime 0").is_err());
    }

    #[test]
    fn parses_exit() {
        assert_eq!(parse_command("exit"), Ok(Command::Exit));
        assert_eq!(parse_command("quit"), Ok(Command::Exit));
        assert!(parse_command("exit now").is_err());
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
