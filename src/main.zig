const std = @import("std");
const rl = @cImport(@cInclude("raylib.h"));

const WINDOW_W = 640;
const WINDOW_H = 640;
const SQUARE_SIZE = WINDOW_W / 8;

const LIGHT = rl.Color{ .r = 240, .g = 217, .b = 181, .a = 255 };
const DARK = rl.Color{ .r = 181, .g = 136, .b = 99, .a = 255 };

pub fn main() void {
    rl.InitWindow(WINDOW_W, WINDOW_H, "ghez");
    defer rl.CloseWindow();
    rl.SetTargetFPS(60);

    while (!rl.WindowShouldClose()) {
        rl.BeginDrawing();
        rl.ClearBackground(rl.RAYWHITE);
        drawBoard();
        rl.EndDrawing();
    }
}

fn drawBoard() void {
    for (0..8) |row| {
        for (0..8) |col| {
            const x: c_int = @intCast(col * SQUARE_SIZE);
            const y: c_int = @intCast(row * SQUARE_SIZE);
            const light = (row + col) % 2 == 0;
            rl.DrawRectangle(x, y, SQUARE_SIZE, SQUARE_SIZE, if (light) LIGHT else DARK);
        }
    }
}
