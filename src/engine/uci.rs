use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver, TryRecvError};

/// What an engine told us, reduced to the lines the GUI cares about.
#[derive(Debug, PartialEq)]
pub enum EngineMessage {
    Name(String),
    UciOk,
    /// UCI move text ("e2e4", "e7e8q"); `None` when the engine has no move ("0000", "(none)").
    BestMove(Option<String>),
    /// The engine process exited or closed its output.
    Closed,
}

pub fn parse_line(line: &str) -> Option<EngineMessage> {
    let line = line.trim();
    let (head, rest) = line.split_once(char::is_whitespace).unwrap_or((line, ""));
    match head {
        "uciok" => Some(EngineMessage::UciOk),
        "id" => rest
            .trim()
            .strip_prefix("name")
            .map(|name| EngineMessage::Name(name.trim().to_string())),
        "bestmove" => {
            let mv = rest.split_whitespace().next()?;
            Some(EngineMessage::BestMove(match mv {
                "0000" | "(none)" => None,
                mv => Some(mv.to_string()),
            }))
        }
        _ => None,
    }
}

/// A UCI engine running as a child process. A reader thread forwards its
/// output over a channel so polling never blocks the render loop.
pub struct UciEngine {
    child: Child,
    stdin: ChildStdin,
    messages: Receiver<EngineMessage>,
    /// Prefix for the logged UCI traffic, e.g. "White".
    label: String,
}

impl UciEngine {
    /// Starts the engine command `path` (program followed by its arguments) and sends the `uci` handshake.
    pub fn spawn(path: &str, label: &str) -> Result<Self, String> {
        let mut words = path.split_whitespace();
        let program = words.next().ok_or("empty engine command")?;
        let mut child = Command::new(program)
            .args(words)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("cannot start engine '{path}': {e}"))?;
        let stdin = child.stdin.take().ok_or("engine stdin unavailable")?;
        let stdout = child.stdout.take().ok_or("engine stdout unavailable")?;

        let (tx, messages) = mpsc::channel();
        let reader_label = label.to_string();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { break };
                println!("[{reader_label} <] {line}");
                if let Some(message) = parse_line(&line) {
                    if tx.send(message).is_err() {
                        return;
                    }
                }
            }
            let _ = tx.send(EngineMessage::Closed);
        });

        let mut engine = Self {
            child,
            stdin,
            messages,
            label: label.to_string(),
        };
        engine.send("uci");
        Ok(engine)
    }

    pub fn send(&mut self, line: &str) {
        println!("[{} >] {line}", self.label);
        // A dead engine is reported through `EngineMessage::Closed`.
        let _ = writeln!(self.stdin, "{line}").and_then(|_| self.stdin.flush());
    }

    /// Messages received since the last call.
    pub fn poll(&mut self) -> Vec<EngineMessage> {
        let mut out = Vec::new();
        loop {
            match self.messages.try_recv() {
                Ok(message) => out.push(message),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => break,
            }
        }
        out
    }
}

impl Drop for UciEngine {
    fn drop(&mut self) {
        self.send("quit");
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_engine_lines() {
        assert_eq!(parse_line("uciok"), Some(EngineMessage::UciOk));
        assert_eq!(
            parse_line("id name Stockfish 17"),
            Some(EngineMessage::Name("Stockfish 17".into()))
        );
        assert_eq!(parse_line("id author someone"), None);
        assert_eq!(
            parse_line("bestmove e2e4 ponder e7e5"),
            Some(EngineMessage::BestMove(Some("e2e4".into())))
        );
        assert_eq!(
            parse_line("bestmove e7e8q"),
            Some(EngineMessage::BestMove(Some("e7e8q".into())))
        );
        assert_eq!(
            parse_line("bestmove (none)"),
            Some(EngineMessage::BestMove(None))
        );
        assert_eq!(parse_line("info depth 5 score cp 12"), None);
    }
}
