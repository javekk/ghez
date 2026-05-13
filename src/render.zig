const rl = @cImport(@cInclude("raylib.h"));
const board = @import("board.zig");

pub const PIECE_PIXELS = 16;
pub const SCALE = 6;
pub const PIECE_SIZE = PIECE_PIXELS * SCALE;
pub const BORDER = SCALE;
pub const SQUARE_SIZE = PIECE_SIZE + BORDER;
pub const CELL = PIECE_SIZE;

const LIGHT = rl.Color{ .r = 213, .g = 191, .b = 180, .a = 255 };
const DARK = rl.Color{ .r = 225, .g = 173, .b = 1, .a = 255 };
const BORDER_COLOR = rl.Color{ .r = 30, .g = 20, .b = 10, .a = 255 };
const HIGHLIGHT = rl.Color{ .r = 100, .g = 200, .b = 100, .a = 160 };
const LEGAL_DOT = rl.Color{ .r = 0, .g = 0, .b = 0, .a = 60 };

const Col = struct {
    const pawn = 0;
    const rook = 1;
    const knight = 2;
    const bishop = 3;
    const queen = 4;
    const king = 5;
};

// Drag state: which square is being dragged (null = nothing held).
pub const DragState = struct {
    from_sq: ?u8 = null,
    legal: [256]board.Move = undefined,
    legal_count: usize = 0,
};

pub const Renderer = struct {
    spritesheet: rl.Texture2D,

    pub fn init() Renderer {
        return .{ .spritesheet = rl.LoadTexture("assets/pieces/fresh.png") };
    }

    pub fn deinit(self: Renderer) void {
        rl.UnloadTexture(self.spritesheet);
    }

    // Handle mouse input. Returns a completed move or null each frame.
    pub fn handleInput(_: Renderer, b: board.Board, drag: *DragState) ?board.Move {
        const mx = rl.GetMouseX();
        const my = rl.GetMouseY();
        const hovered = screenToSquare(mx, my);

        if (rl.IsMouseButtonPressed(rl.MOUSE_BUTTON_LEFT)) {
            if (hovered) |sq| {
                const piece = b.squares[sq];
                if (piece != .empty and piece.isColor(b.side_to_move)) {
                    drag.from_sq = sq;
                    drag.legal_count = b.legalMoves(&drag.legal);
                }
            }
        }

        if (rl.IsMouseButtonReleased(rl.MOUSE_BUTTON_LEFT)) {
            if (drag.from_sq) |from| {
                drag.from_sq = null;
                if (hovered) |to| {
                    // Find matching legal move (prefer queen promotion).
                    for (drag.legal[0..drag.legal_count]) |m| {
                        if (m.from == from and m.to == to) {
                            if (m.promotion == null or isQueenPromo(m, b.side_to_move)) {
                                return m;
                            }
                        }
                    }
                }
            }
        }
        return null;
    }

    pub fn drawBoard(_: Renderer, drag: DragState) void {
        rl.DrawRectangle(0, 0, SQUARE_SIZE * 8, SQUARE_SIZE * 8, BORDER_COLOR);
        for (0..8) |row| {
            for (0..8) |col| {
                const x: c_int = @intCast(col * SQUARE_SIZE);
                const y: c_int = @intCast(row * SQUARE_SIZE);
                const dark = (row + col) % 2 == 0;
                rl.DrawRectangle(x, y, CELL, CELL, if (dark) DARK else LIGHT);
            }
        }
        // Highlight selected square.
        if (drag.from_sq) |sq| {
            const sc: usize = sq % 8;
            const sr: usize = sq / 8;
            const x: c_int = @intCast(sc * SQUARE_SIZE);
            const y: c_int = @intCast(sr * SQUARE_SIZE);
            rl.DrawRectangle(x, y, CELL, CELL, HIGHLIGHT);
            for (drag.legal[0..drag.legal_count]) |m| {
                if (m.from != sq) continue;
                const tc: usize = m.to % 8;
                const tr: usize = m.to / 8;
                const tx: c_int = @intCast(tc * SQUARE_SIZE + CELL / 2);
                const ty: c_int = @intCast(tr * SQUARE_SIZE + CELL / 2);
                rl.DrawCircle(tx, ty, CELL / 6, LEGAL_DOT);
            }
        }
    }

    pub fn drawPieces(self: Renderer, b: board.Board, drag: DragState) void {
        for (0..64) |index| {
            const piece = b.squares[index];
            if (piece == .empty) continue;
            // Skip the dragged piece — draw it floating instead.
            if (drag.from_sq) |from| {
                if (index == from) continue;
            }
            const dest = squareRect(@intCast(index));
            rl.DrawTexturePro(self.spritesheet, pieceSource(piece), dest, .{ .x = 0, .y = 0 }, 0, rl.WHITE);
        }
        // Draw dragged piece centered on cursor.
        if (drag.from_sq) |from| {
            const piece = b.squares[from];
            const mx: f32 = @floatFromInt(rl.GetMouseX());
            const my: f32 = @floatFromInt(rl.GetMouseY());
            const dest = rl.Rectangle{
                .x = mx - CELL / 2,
                .y = my - CELL / 2,
                .width = CELL,
                .height = CELL,
            };
            rl.DrawTexturePro(self.spritesheet, pieceSource(piece), dest, .{ .x = 0, .y = 0 }, 0, rl.WHITE);
        }
    }

    fn pieceSource(piece: board.Piece) rl.Rectangle {
        const col: f32 = @floatFromInt(pieceCol(piece));
        const row: f32 = if (piece.isWhite()) 1.0 else 0.0;
        return .{
            .x = col * PIECE_SIZE,
            .y = row * PIECE_SIZE,
            .width = PIECE_SIZE,
            .height = PIECE_SIZE,
        };
    }

    fn pieceCol(piece: board.Piece) usize {
        return switch (piece) {
            .white_pawn, .black_pawn => Col.pawn,
            .white_rook, .black_rook => Col.rook,
            .white_knight, .black_knight => Col.knight,
            .white_bishop, .black_bishop => Col.bishop,
            .white_queen, .black_queen => Col.queen,
            .white_king, .black_king => Col.king,
            .empty => 0,
        };
    }
};

fn squareRect(sq: u8) rl.Rectangle {
    const col: usize = sq % 8;
    const row: usize = sq / 8;
    return .{
        .x = @floatFromInt(col * SQUARE_SIZE),
        .y = @floatFromInt(row * SQUARE_SIZE),
        .width = CELL,
        .height = CELL,
    };
}

fn screenToSquare(mx: c_int, my: c_int) ?u8 {
    if (mx < 0 or my < 0) return null;
    const col: usize = @intCast(@divTrunc(mx, SQUARE_SIZE));
    const row: usize = @intCast(@divTrunc(my, SQUARE_SIZE));
    if (col >= 8 or row >= 8) return null;
    return @intCast(row * 8 + col);
}

fn isQueenPromo(m: board.Move, c: board.Color) bool {
    return m.promotion == (if (c == .white) board.Piece.white_queen else board.Piece.black_queen);
}
