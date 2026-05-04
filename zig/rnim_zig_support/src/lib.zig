const std = @import("std");

/// RnimAllocator - A simple allocator wrapper for Chimera-Nim
/// Used for compact allocator implementations in the runtime
pub const RnimAllocator = struct {
    /// Initialize the allocator
    pub fn init() void {
        std.debug.print("RnimAllocator initialized\n", .{});
    }

    /// Allocate memory of given size
    pub fn alloc(size: usize) []u8 {
        // Simple fixed-size buffer for testing
        const buffer: [1024]u8 = undefined;
        return buffer[0..size];
    }

    /// Deallocate memory
    pub fn dealloc(_: []u8) void {
        std.debug.print("RnimAllocator dealloc called\n", .{});
    }
};

test "rnim allocator init" {
    RnimAllocator.init();
    const buf = RnimAllocator.alloc(10);
    try std.testing.expect(buf.len == 10);
    RnimAllocator.dealloc(buf);
}

test "rnim allocator zero size" {
    const buf = RnimAllocator.alloc(0);
    try std.testing.expect(buf.len == 0);
}