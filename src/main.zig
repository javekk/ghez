const rl = @cImport(@cInclude("raylib.h"));
const board = @import("board.zig");
const render = @import("render.zig");

const START_FEN = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

pub fn main() void {
    rl.InitWindow(render.SQUARE_SIZE * 8, render.SQUARE_SIZE * 8, "ghez");
    defer rl.CloseWindow();
    rl.SetTargetFPS(60);

    var game_board = board.Board.fromFen(START_FEN);
    const renderer = render.Renderer.init();
    defer renderer.deinit();

    var drag = render.DragState{};

    while (!rl.WindowShouldClose()) {
        if (game_board.gameResult() == .ongoing) {
            if (renderer.handleInput(game_board, &drag)) |m| {
                game_board = game_board.applyMove(m);
            }
        }

        rl.BeginDrawing();
        rl.ClearBackground(rl.RAYWHITE);
        renderer.drawBoard(drag);
        renderer.drawPieces(game_board, drag);
        rl.EndDrawing();
    }
}
