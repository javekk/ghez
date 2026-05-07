# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

Chess GUI for testing khez engine versions. Supports human-vs-engine and engine-vs-engine (two simultaneous UCI engines).

## Stack

- **Zig 0.15**
- **Raylib** via Zig package manager (statically linked, zero runtime deps, single binary)
- **raygui** (bundled with raylib) for UI panels

## Commands

```sh
zig build          # build
zig build run      # build and run
zig build test     # run tests
```

## Architecture

```
src/
├── main.zig       # entry point: init raylib window, main loop
├── board.zig      # 8x8 board state, FEN parse/serialize, pseudo-legal move gen, legality, draw detection
├── render.zig     # raylib draw calls: board, pieces (PNG sprites), drag/drop input
├── uci.zig        # UCI engine subprocess: std.process.Child + pipes, async reader thread, parse bestmove/info
├── match.zig      # game state machine, two engine slots, clock
└── ui.zig         # raygui panels: engine config, live info (depth/score/nodes/PV), move list
```

**Key design points:**
- `uci.zig` wraps each engine with its own `std.Thread` reading stdout asynchronously.
- `match.zig` owns two `UciEngine` instances, drives state machine: `IDLE → WHITE_THINKING → BLACK_THINKING → GAME_OVER`.
- `board.zig` implements move generation directly — pseudo-legal moves filtered by king-in-check after applying.
- Piece sprites: cburnett set (public domain), knight replaced with a giraffe. 128px PNGs in `assets/pieces/`.

## Plan

See [PLAN.md](PLAN.md) for the step-by-step build order and current progress.
