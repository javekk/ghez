const std = @import("std");

// The build function is the entry point for `zig build`.
// `b` is a builder that accumulates a dependency graph — nothing is compiled here,
// the runner executes the graph after this function returns.
pub fn build(b: *std.Build) void {
    // Allow the caller to choose target (e.g. -Dtarget=x86_64-linux) and
    // optimization level (e.g. -Doptimize=ReleaseFast). Defaults: native CPU, Debug.
    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{});

    // Fetch and compile the raylib dependency declared in build.zig.zon.
    // Passing target/optimize ensures raylib is built with the same settings as our exe.
    const raylib_dep = b.dependency("raylib", .{
        .target = target,
        .optimize = optimize,
    });
    // Get the compiled static library artifact produced by raylib's own build.zig.
    const raylib = raylib_dep.artifact("raylib");

    // A module is a collection of Zig source files with shared compile options.
    // root_source_file is the entry point; other files are reached via @import from it.
    const exe_mod = b.createModule(.{
        .root_source_file = b.path("src/main.zig"),
        .target = target,
        .optimize = optimize,
    });

    // Define the executable. root_module owns the Zig source; the name becomes the binary name.
    const exe = b.addExecutable(.{
        .name = "ghez",
        .root_module = exe_mod,
    });
    // Link the compiled raylib static library into our executable.
    exe.root_module.linkLibrary(raylib);
    // Add raylib's `src/` folder to the C include search path so that
    // `@cInclude("raylib.h")` resolves correctly.
    exe.root_module.addIncludePath(raylib_dep.path("src"));

    // Register the executable to be copied into zig-out/bin/ when running `zig build`.
    b.installArtifact(exe);

    // Create a step that runs the compiled executable.
    const run_cmd = b.addRunArtifact(exe);
    // Make sure the binary is installed before we try to run it.
    run_cmd.step.dependOn(b.getInstallStep());
    // Forward any extra CLI args: `zig build run -- arg1 arg2`
    if (b.args) |args| run_cmd.addArgs(args);

    // Expose `zig build run` as a named top-level step.
    const run_step = b.step("run", "Run the app");
    run_step.dependOn(&run_cmd.step);

    // Separate module for tests — same source, but the test runner needs its own module instance.
    const test_mod = b.createModule(.{
        .root_source_file = b.path("src/main.zig"),
        .target = target,
        .optimize = optimize,
    });
    // Build a test executable that runs all `test` blocks in the module.
    const exe_tests = b.addTest(.{ .root_module = test_mod });
    const run_tests = b.addRunArtifact(exe_tests);
    // Expose `zig build test` as a named top-level step.
    const test_step = b.step("test", "Run tests");
    test_step.dependOn(&run_tests.step);
}
