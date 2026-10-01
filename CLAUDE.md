# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

Chess GUI for testing khez engine versions. Supports human-vs-engine and engine-vs-engine (two simultaneous UCI engines).

## Stack

- **Rust** (edition 2024)
- **macroquad 0.4** — windowing, input, 2D rendering (single dep)

## Commands

```sh
cargo build         # build
cargo run           # build and run
cargo test          # run all tests
cargo test <name>   # run a single test by name substring
cargo check         # fast type-check without codegen
cargo clippy        # lint
```

## Architecture

```
src/
├── main.rs                 # entry: declares module tree, macroquad main loop
├── game/
│   ├── domain.rs           # Square, Side, PieceType, Piece, Board = [Option<Piece>; 64]
│   ├── game_state.rs       # GameState: board + side to move + castling rights + en passant + counters
│   └── game.rs             # Game wrapper: constructors (incl. FEN parsing), drives game progression
├── engine/
│   ├── uci.rs              # UciEngine: child process + reader thread, parses uciok/id name/bestmove
│   └── engine_match.rs     # EngineMatch: human-vs-engine driver (position/go movetime, stale-search handling)
└── render/
    ├── renderer.rs         # macroquad draw calls: board, pieces from sprite sheet
    └── theme.rs            # sizing constants, colors
```

**Key design points:**

- `main.rs` declares the full module tree inline (`mod render { pub mod ... }`). Adding a new file means adding a `pub mod` line here.
- `Renderer::new()` is async — loads the sprite sheet via `load_texture`. Must be `.await`ed before the main loop.
- `Renderer` owns GPU/asset state (`Texture2D`, sprite rects). It takes `&GameState` to draw — never mutates game state.
- Piece sprites come from a single sheet `assets/pieces/chess_sprites.png` with rows for color variants and columns for piece types. Slicing happens in `Renderer::new`.
- `Board` is a flat `[Option<Piece>; 64]` indexed `a1..h1, a2..h2, ..., a8..h8` (rank-major, white's back rank first). FEN parsing in `Game::from_fen` walks ranks 8 → 1.
- `GameState` is `Copy` — keep it that way (no heap-allocating fields like `HashMap`/`Vec`). Cheap clones matter once search/perft land.

- Human vs engine: `ghez --engine <path> [--color white|black] [--movetime <ms>]`, or at runtime `engine <path>`, `engine off`, `side white|black`, `movetime <ms>`. ghez is the UCI *client*; the engine runs as a child process and never blocks the render loop.

## Plan

See [PLAN.md](PLAN.md) for the step-by-step build order and current progress.
