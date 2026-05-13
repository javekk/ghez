const std = @import("std");

pub const Piece = enum(u8) {
    empty,
    white_pawn,
    white_knight,
    white_bishop,
    white_rook,
    white_queen,
    white_king,
    black_pawn,
    black_knight,
    black_bishop,
    black_rook,
    black_queen,
    black_king,

    pub fn color(self: Piece) ?Color {
        return switch (self) {
            .empty => null,
            .white_pawn, .white_knight, .white_bishop, .white_rook, .white_queen, .white_king => .white,
            .black_pawn, .black_knight, .black_bishop, .black_rook, .black_queen, .black_king => .black,
        };
    }

    pub fn isWhite(self: Piece) bool {
        return self.color() == .white;
    }

    pub fn isBlack(self: Piece) bool {
        return self.color() == .black;
    }

    pub fn isColor(self: Piece, c: Color) bool {
        return self.color() == c;
    }
};

pub const Color = enum { white, black };

pub const Move = struct {
    from: u8,
    to: u8,
    promotion: ?Piece = null, // non-null only for pawn promotions
};

// Game state flags packed alongside the board.
pub const CastleRights = packed struct {
    white_king_side: bool = true,
    white_queen_side: bool = true,
    black_king_side: bool = true,
    black_queen_side: bool = true,
};

pub const GameResult = enum { ongoing, white_wins, black_wins, draw };

// The board is a flat array of 64 squares, index 0 = a8 (top-left), 63 = h1 (bottom-right).
pub const Board = struct {
    squares: [64]Piece = [_]Piece{.empty} ** 64,
    side_to_move: Color = .white,
    castle: CastleRights = .{},
    en_passant: ?u8 = null,
    halfmove_clock: u8 = 0, // for 50-move rule
    fullmove: u16 = 1,
    // Position history for threefold repetition (stores Zobrist-like hashes).
    // We use a simple hash of squares+side for this GUI (not cryptographically strong).
    history: [500]u64 = [_]u64{0} ** 500,
    history_len: u8 = 0,

    pub fn fromFen(fen: []const u8) Board {
        var board = Board{};
        var it = std.mem.splitScalar(u8, fen, ' ');

        const placement = it.next() orelse return board;
        var square: usize = 0;
        for (placement) |ch| {
            switch (ch) {
                '/' => {},
                '1'...'8' => square += ch - '0',
                else => {
                    board.squares[square] = charToPiece(ch);
                    square += 1;
                },
            }
        }

        const side_str = it.next() orelse "w";
        board.side_to_move = if (side_str[0] == 'b') .black else .white;

        const castle_str = it.next() orelse "-";
        board.castle = .{
            .white_king_side = std.mem.indexOfScalar(u8, castle_str, 'K') != null,
            .white_queen_side = std.mem.indexOfScalar(u8, castle_str, 'Q') != null,
            .black_king_side = std.mem.indexOfScalar(u8, castle_str, 'k') != null,
            .black_queen_side = std.mem.indexOfScalar(u8, castle_str, 'q') != null,
        };

        const ep_str = it.next() orelse "-";
        board.en_passant = if (ep_str[0] != '-') algebraicToIndex(ep_str) else null;

        const hm_str = it.next() orelse "0";
        board.halfmove_clock = std.fmt.parseInt(u8, hm_str, 10) catch 0;

        const fm_str = it.next() orelse "1";
        board.fullmove = std.fmt.parseInt(u16, fm_str, 10) catch 1;

        board.history[0] = board.computeHash();
        board.history_len = 1;

        return board;
    }

    // Apply a move and return the new board state.
    pub fn applyMove(self: Board, m: Move) Board {
        var next = self;
        const piece = next.squares[m.from];
        const captured = next.squares[m.to];

        // Update halfmove clock.
        if (piece == .white_pawn or piece == .black_pawn or captured != .empty) {
            next.halfmove_clock = 0;
        } else {
            next.halfmove_clock += 1;
        }

        // En passant capture.
        if ((piece == .white_pawn or piece == .black_pawn) and next.en_passant != null and m.to == next.en_passant.?) {
            const ep_capture_sq: u8 = if (self.side_to_move == .white) m.to + 8 else m.to - 8;
            next.squares[ep_capture_sq] = .empty;
        }

        // Set en passant target square.
        next.en_passant = null;
        if (piece == .white_pawn and m.from >= 48 and m.to == m.from - 16) {
            next.en_passant = m.from - 8;
        } else if (piece == .black_pawn and m.from <= 15 and m.to == m.from + 16) {
            next.en_passant = m.from + 8;
        }

        // Castling: move rook too.
        if (piece == .white_king) {
            if (m.from == 60 and m.to == 62) { // O-O
                next.squares[63] = .empty;
                next.squares[61] = .white_rook;
            } else if (m.from == 60 and m.to == 58) { // O-O-O
                next.squares[56] = .empty;
                next.squares[59] = .white_rook;
            }
            next.castle.white_king_side = false;
            next.castle.white_queen_side = false;
        }
        if (piece == .black_king) {
            if (m.from == 4 and m.to == 6) {
                next.squares[7] = .empty;
                next.squares[5] = .black_rook;
            } else if (m.from == 4 and m.to == 2) {
                next.squares[0] = .empty;
                next.squares[3] = .black_rook;
            }
            next.castle.black_king_side = false;
            next.castle.black_queen_side = false;
        }
        // Rook moves revoke castling rights.
        if (m.from == 63 or m.to == 63) next.castle.white_king_side = false;
        if (m.from == 56 or m.to == 56) next.castle.white_queen_side = false;
        if (m.from == 7 or m.to == 7) next.castle.black_king_side = false;
        if (m.from == 0 or m.to == 0) next.castle.black_queen_side = false;

        // Move the piece.
        next.squares[m.to] = if (m.promotion) |promo| promo else piece;
        next.squares[m.from] = .empty;

        // Flip side.
        next.side_to_move = if (self.side_to_move == .white) .black else .white;
        if (self.side_to_move == .black) next.fullmove += 1;

        // Record history.
        if (next.history_len < 150) {
            next.history[next.history_len] = next.computeHash();
            next.history_len += 1;
        }

        return next;
    }

    // Generate all legal moves for the side to move.
    pub fn legalMoves(self: Board, out: []Move) usize {
        var pseudo: [256]Move = undefined;
        const n = self.pseudoLegal(&pseudo);
        var count: usize = 0;
        for (pseudo[0..n]) |m| {
            const after = self.applyMove(m);
            if (!after.isInCheck(self.side_to_move)) {
                out[count] = m;
                count += 1;
            }
        }
        return count;
    }

    // Is the given color's king currently in check?
    pub fn isInCheck(self: Board, c: Color) bool {
        const king: Piece = if (c == .white) .white_king else .black_king;
        var king_sq: u8 = 0;
        for (self.squares, 0..) |p, i| {
            if (p == king) {
                king_sq = @intCast(i);
                break;
            }
        }
        const opp: Color = if (c == .white) .black else .white;
        return self.isAttackedBy(king_sq, opp);
    }

    pub fn gameResult(self: Board) GameResult {
        var moves: [256]Move = undefined;
        const n = self.legalMoves(&moves);

        if (n == 0) {
            if (self.isInCheck(self.side_to_move)) {
                return if (self.side_to_move == .white) .black_wins else .white_wins;
            }
            return .draw; // stalemate
        }

        if (self.halfmove_clock >= 100) return .draw; // 50-move rule
        if (self.isInsufficientMaterial()) return .draw;
        if (self.isThreefoldRepetition()) return .draw;

        return .ongoing;
    }

    // --- internals ---

    fn isInsufficientMaterial(self: Board) bool {
        var white_pieces: u8 = 0;
        var black_pieces: u8 = 0;
        var white_bishops: u8 = 0;
        var black_bishops: u8 = 0;
        var white_knights: u8 = 0;
        var black_knights: u8 = 0;

        for (self.squares) |p| {
            switch (p) {
                .white_pawn, .white_rook, .white_queen => return false,
                .black_pawn, .black_rook, .black_queen => return false,
                .white_bishop => {
                    white_bishops += 1;
                    white_pieces += 1;
                },
                .white_knight => {
                    white_knights += 1;
                    white_pieces += 1;
                },
                .black_bishop => {
                    black_bishops += 1;
                    black_pieces += 1;
                },
                .black_knight => {
                    black_knights += 1;
                    black_pieces += 1;
                },
                else => {},
            }
        }
        // K vs K, K+B vs K, K+N vs K
        if (white_pieces == 0 and black_pieces == 0) return true;
        if (white_pieces == 1 and black_pieces == 0 and (white_bishops == 1 or white_knights == 1)) return true;
        if (black_pieces == 1 and white_pieces == 0 and (black_bishops == 1 or black_knights == 1)) return true;
        return false;
    }

    fn isThreefoldRepetition(self: Board) bool {
        const current = self.computeHash();
        var count: u8 = 0;
        for (self.history[0..self.history_len]) |h| {
            if (h == current) count += 1;
        }
        return count >= 3;
    }

    fn computeHash(self: Board) u64 {
        var h: u64 = 0;
        for (self.squares, 0..) |p, i| {
            h ^= (@as(u64, @intFromEnum(p)) +% 1) *% (i *% 0x9e3779b97f4a7c15 +% 0x6c62272e07bb0142);
        }
        if (self.side_to_move == .black) h ^= 0xf1a5c3e7d9b82460;
        if (self.en_passant) |ep| h ^= @as(u64, ep) *% 0x1234567890abcdef;
        return h;
    }

    // Is square `sq` attacked by any piece of color `c`?
    fn isAttackedBy(self: Board, sq: u8, c: Color) bool {
        for (self.squares, 0..) |p, i| {
            if (!p.isColor(c)) continue;
            if (self.attacks(p, @intCast(i), sq)) return true;
        }
        return false;
    }

    // Does piece `p` at `from` attack square `to` (ignoring pins)?
    fn attacks(self: Board, p: Piece, from: u8, to: u8) bool {
        return switch (p) {
            .white_pawn => pawnAttacks(.white, from, to),
            .black_pawn => pawnAttacks(.black, from, to),
            .white_knight, .black_knight => knightAttacks(from, to),
            .white_bishop, .black_bishop => self.slidingAttacks(from, to, true, false),
            .white_rook, .black_rook => self.slidingAttacks(from, to, false, true),
            .white_queen, .black_queen => self.slidingAttacks(from, to, true, true),
            .white_king, .black_king => kingAttacks(from, to),
            .empty => false,
        };
    }

    fn pseudoLegal(self: Board, out: []Move) usize {
        var count: usize = 0;
        for (self.squares, 0..) |piece, i| {
            if (!piece.isColor(self.side_to_move)) continue;
            count += self.movesFor(piece, @intCast(i), out[count..]);
        }
        return count;
    }

    fn movesFor(self: Board, p: Piece, from: u8, out: []Move) usize {
        return switch (p) {
            .white_pawn => self.pawnMoves(from, .white, out),
            .black_pawn => self.pawnMoves(from, .black, out),
            .white_knight, .black_knight => self.knightMoves(from, out),
            .white_bishop, .black_bishop => self.slidingMoves(from, true, false, out),
            .white_rook, .black_rook => self.slidingMoves(from, false, true, out),
            .white_queen, .black_queen => self.slidingMoves(from, true, true, out),
            .white_king, .black_king => self.kingMoves(from, out),
            .empty => 0,
        };
    }

    fn pawnMoves(self: Board, from: u8, c: Color, out: []Move) usize {
        var n: usize = 0;
        const row = from / 8;
        const col = from % 8;

        if (c == .white) {
            // Forward one
            if (row > 0 and self.squares[from - 8] == .empty) {
                n += addPawnMove(from, from - 8, c, out[n..]);
                // Forward two from starting rank (rank 7 in 0-indexed = row 6)
                if (row == 6 and self.squares[from - 16] == .empty) {
                    out[n] = .{ .from = from, .to = from - 16 };
                    n += 1;
                }
            }
            // Captures
            if (row > 0 and col > 0) {
                const t = from - 9;
                if (self.squares[t].isBlack() or (self.en_passant != null and self.en_passant.? == t))
                    n += addPawnMove(from, t, c, out[n..]);
            }
            if (row > 0 and col < 7) {
                const t = from - 7;
                if (self.squares[t].isBlack() or (self.en_passant != null and self.en_passant.? == t))
                    n += addPawnMove(from, t, c, out[n..]);
            }
        } else {
            if (row < 7 and self.squares[from + 8] == .empty) {
                n += addPawnMove(from, from + 8, c, out[n..]);
                if (row == 1 and self.squares[from + 16] == .empty) {
                    out[n] = .{ .from = from, .to = from + 16 };
                    n += 1;
                }
            }
            if (row < 7 and col > 0) {
                const t = from + 7;
                if (self.squares[t].isWhite() or (self.en_passant != null and self.en_passant.? == t))
                    n += addPawnMove(from, t, c, out[n..]);
            }
            if (row < 7 and col < 7) {
                const t = from + 9;
                if (self.squares[t].isWhite() or (self.en_passant != null and self.en_passant.? == t))
                    n += addPawnMove(from, t, c, out[n..]);
            }
        }
        return n;
    }

    fn knightMoves(self: Board, from: u8, out: []Move) usize {
        const c = self.squares[from].color().?;
        const row: i16 = @intCast(from / 8);
        const col: i16 = @intCast(from % 8);
        const deltas = [8][2]i16{ .{ -2, -1 }, .{ -2, 1 }, .{ -1, -2 }, .{ -1, 2 }, .{ 1, -2 }, .{ 1, 2 }, .{ 2, -1 }, .{ 2, 1 } };
        var n: usize = 0;
        for (deltas) |d| {
            const r = row + d[0];
            const cl = col + d[1];
            if (r < 0 or r > 7 or cl < 0 or cl > 7) continue;
            const to: u8 = @intCast(r * 8 + cl);
            if (!self.squares[to].isColor(c)) {
                out[n] = .{ .from = from, .to = to };
                n += 1;
            }
        }
        return n;
    }

    fn slidingMoves(self: Board, from: u8, diag: bool, straight: bool, out: []Move) usize {
        const c = self.squares[from].color().?;
        var n: usize = 0;
        const dirs: []const [2]i16 = if (diag and straight)
            &[_][2]i16{ .{ -1, 0 }, .{ 1, 0 }, .{ 0, -1 }, .{ 0, 1 }, .{ -1, -1 }, .{ -1, 1 }, .{ 1, -1 }, .{ 1, 1 } }
        else if (diag)
            &[_][2]i16{ .{ -1, -1 }, .{ -1, 1 }, .{ 1, -1 }, .{ 1, 1 } }
        else
            &[_][2]i16{ .{ -1, 0 }, .{ 1, 0 }, .{ 0, -1 }, .{ 0, 1 } };

        for (dirs) |d| {
            var r: i16 = @intCast(from / 8);
            var cl: i16 = @intCast(from % 8);
            while (true) {
                r += d[0];
                cl += d[1];
                if (r < 0 or r > 7 or cl < 0 or cl > 7) break;
                const to: u8 = @intCast(r * 8 + cl);
                if (self.squares[to].isColor(c)) break;
                out[n] = .{ .from = from, .to = to };
                n += 1;
                if (self.squares[to] != .empty) break; // capture stops ray
            }
        }
        return n;
    }

    fn kingMoves(self: Board, from: u8, out: []Move) usize {
        const c = self.squares[from].color().?;
        const row: i16 = @intCast(from / 8);
        const col: i16 = @intCast(from % 8);
        var n: usize = 0;
        var dr: i16 = -1;
        while (dr <= 1) : (dr += 1) {
            var dc: i16 = -1;
            while (dc <= 1) : (dc += 1) {
                if (dr == 0 and dc == 0) continue;
                const r = row + dr;
                const cl = col + dc;
                if (r < 0 or r > 7 or cl < 0 or cl > 7) continue;
                const to: u8 = @intCast(r * 8 + cl);
                if (!self.squares[to].isColor(c)) {
                    out[n] = .{ .from = from, .to = to };
                    n += 1;
                }
            }
        }
        // Castling
        if (c == .white and from == 60) {
            if (self.castle.white_king_side and
                self.squares[61] == .empty and self.squares[62] == .empty and
                !self.isAttackedBy(60, .black) and !self.isAttackedBy(61, .black) and !self.isAttackedBy(62, .black))
            {
                out[n] = .{ .from = 60, .to = 62 };
                n += 1;
            }
            if (self.castle.white_queen_side and
                self.squares[59] == .empty and self.squares[58] == .empty and self.squares[57] == .empty and
                !self.isAttackedBy(60, .black) and !self.isAttackedBy(59, .black) and !self.isAttackedBy(58, .black))
            {
                out[n] = .{ .from = 60, .to = 58 };
                n += 1;
            }
        }
        if (c == .black and from == 4) {
            if (self.castle.black_king_side and
                self.squares[5] == .empty and self.squares[6] == .empty and
                !self.isAttackedBy(4, .white) and !self.isAttackedBy(5, .white) and !self.isAttackedBy(6, .white))
            {
                out[n] = .{ .from = 4, .to = 6 };
                n += 1;
            }
            if (self.castle.black_queen_side and
                self.squares[3] == .empty and self.squares[2] == .empty and self.squares[1] == .empty and
                !self.isAttackedBy(4, .white) and !self.isAttackedBy(3, .white) and !self.isAttackedBy(2, .white))
            {
                out[n] = .{ .from = 4, .to = 2 };
                n += 1;
            }
        }
        return n;
    }

    // --- attack helpers (no board scan, pure geometry for check detection) ---

    fn pawnAttacks(c: Color, from: u8, to: u8) bool {
        const row: i16 = @intCast(from / 8);
        const col: i16 = @intCast(from % 8);
        const tr: i16 = @intCast(to / 8);
        const tc: i16 = @intCast(to % 8);
        if (c == .white) {
            return tr == row - 1 and (tc == col - 1 or tc == col + 1);
        } else {
            return tr == row + 1 and (tc == col - 1 or tc == col + 1);
        }
    }

    fn knightAttacks(from: u8, to: u8) bool {
        const dr = @abs(@as(i16, @intCast(to / 8)) - @as(i16, @intCast(from / 8)));
        const dc = @abs(@as(i16, @intCast(to % 8)) - @as(i16, @intCast(from % 8)));
        return (dr == 2 and dc == 1) or (dr == 1 and dc == 2);
    }

    fn kingAttacks(from: u8, to: u8) bool {
        const dr = @abs(@as(i16, @intCast(to / 8)) - @as(i16, @intCast(from / 8)));
        const dc = @abs(@as(i16, @intCast(to % 8)) - @as(i16, @intCast(from % 8)));
        return dr <= 1 and dc <= 1 and (dr + dc > 0);
    }

    fn slidingAttacks(self: Board, from: u8, to: u8, diag: bool, straight: bool) bool {
        const fr: i16 = @intCast(from / 8);
        const fc: i16 = @intCast(from % 8);
        const tr: i16 = @intCast(to / 8);
        const tc: i16 = @intCast(to % 8);
        const dr = tr - fr;
        const dc = tc - fc;

        const is_diag = @abs(dr) == @abs(dc) and dr != 0;
        const is_straight = (dr == 0) != (dc == 0);

        if (diag and is_diag) {
            const sr: i16 = if (dr > 0) 1 else -1;
            const sc: i16 = if (dc > 0) 1 else -1;
            var r = fr + sr;
            var c = fc + sc;
            while (r != tr or c != tc) {
                if (self.squares[@intCast(r * 8 + c)] != .empty) return false;
                r += sr;
                c += sc;
            }
            return true;
        }
        if (straight and is_straight) {
            const sr: i16 = std.math.sign(dr);
            const sc: i16 = std.math.sign(dc);
            var r = fr + sr;
            var c = fc + sc;
            while (r != tr or c != tc) {
                if (self.squares[@intCast(r * 8 + c)] != .empty) return false;
                r += sr;
                c += sc;
            }
            return true;
        }
        return false;
    }
};

fn addPawnMove(from: u8, to: u8, c: Color, out: []Move) usize {
    const promo_row: u8 = if (c == .white) 0 else 7;
    if (to / 8 == promo_row) {
        const promos: [4]Piece = if (c == .white)
            .{ .white_queen, .white_rook, .white_bishop, .white_knight }
        else
            .{ .black_queen, .black_rook, .black_bishop, .black_knight };
        for (promos, 0..) |promo, i| {
            out[i] = .{ .from = from, .to = to, .promotion = promo };
        }
        return 4;
    }
    out[0] = .{ .from = from, .to = to };
    return 1;
}

fn charToPiece(ch: u8) Piece {
    return switch (ch) {
        'P' => .white_pawn,
        'N' => .white_knight,
        'B' => .white_bishop,
        'R' => .white_rook,
        'Q' => .white_queen,
        'K' => .white_king,
        'p' => .black_pawn,
        'n' => .black_knight,
        'b' => .black_bishop,
        'r' => .black_rook,
        'q' => .black_queen,
        'k' => .black_king,
        else => .empty,
    };
}

fn algebraicToIndex(s: []const u8) u8 {
    if (s.len < 2) return 0;
    const file = s[0] - 'a'; // 0..7
    const rank = s[1] - '1'; // 0..7, rank 1 = index 56
    return @intCast((7 - rank) * 8 + file);
}

// --- tests ---

test "starting position has 20 legal moves" {
    const start = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    const b = Board.fromFen(start);
    var moves: [256]Move = undefined;
    const n = b.legalMoves(&moves);
    try std.testing.expectEqual(@as(usize, 20), n);
}

test "scholars mate is checkmate" {
    // Position after 1.e4 e5 2.Bc4 Nc6 3.Qh5 Nf6?? 4.Qxf7#
    const fen = "r1bqkb1r/pppp1Qpp/2n2n2/4p3/2B1P3/8/PPPP1PPP/RNB1K1NR b KQkq - 0 4";
    const b = Board.fromFen(fen);
    try std.testing.expectEqual(GameResult.white_wins, b.gameResult());
}

test "stalemate" {
    const fen = "k7/8/1Q6/8/8/8/8/7K b - - 0 1";
    const b = Board.fromFen(fen);
    try std.testing.expectEqual(GameResult.draw, b.gameResult());
}
