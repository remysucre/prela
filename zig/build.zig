const std = @import("std");

pub fn build(b: *std.Build) void {
    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{});
    const no_llvm = b.option(bool, "no-llvm", "Use Zig's self-hosted backend instead of LLVM") orelse false;

    const exe = b.addExecutable(.{
        .name = "prela-zig",
        .root_module = b.createModule(.{
            .root_source_file = b.path("src/main.zig"),
            .target = target,
            .optimize = optimize,
        }),
        .use_llvm = if (no_llvm) false else null,
    });
    b.installArtifact(exe);

    const run_cmd = b.addRunArtifact(exe);
    const run_step = b.step("run", "Run the suite");
    run_step.dependOn(&run_cmd.step);
}
