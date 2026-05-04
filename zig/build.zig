const std = @import("std");

pub fn build(b: *std.build.Builder) void {
    const mode = b.standardReleaseOptions();
    const lib = b.addStaticLibrary("rnim_zig_support", "src/lib.zig");
    lib.setBuildMode(mode);
    lib.install();

    const tests = b.addTest("src/lib.zig");
    tests.setBuildMode(mode);
    const test_step = b.step("test", "Run tests");
    test_step.dependOn(&tests.step);
}