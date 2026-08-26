const std = @import("std");
const rich = @import("rich");

pub fn main(init: std.process.Init) !void {
    var gpa: std.heap.DebugAllocator(.{}) = .init;
    defer {
        const status = gpa.deinit();
        if (status == .leak) @panic("rich Zig demo leaked memory");
    }
    const allocator = gpa.allocator();

    var console = try rich.Console.init();
    defer console.deinit();
    try console.setForceTerminal(false);
    try console.setSize(80, 24);

    var text = try rich.Text.init("Reusable rich-zig content");
    defer text.deinit();
    try text.setStyle("bold cyan");
    var reused = try text.finish();
    defer reused.deinit();

    var table = try rich.Table.init();
    defer table.deinit();
    try table.setTitle("Zig 0.16 binding");
    try table.addColumnRenderable(&reused);
    try table.addColumn("State");
    const row = [_][]const u8{ "borrowed + reusable", "safe" };
    try table.addRowStrings(allocator, &row);
    var table_renderable = try table.finish();
    defer table_renderable.deinit();

    var panel = try rich.Panel.init(&table_renderable);
    defer panel.deinit();
    try panel.setTitle("rich-rs from Zig");
    try panel.setExpand(false);
    var composed = try panel.finish();
    defer composed.deinit();

    // Both inputs remain valid because composition only borrows renderables.
    const reused_output = try console.render(allocator, &reused);
    defer allocator.free(reused_output);
    const output = try console.render(allocator, &composed);
    defer allocator.free(output);

    var stdout_buffer: [4096]u8 = undefined;
    var stdout_writer = std.Io.File.stdout().writer(init.io, &stdout_buffer);
    const stdout = &stdout_writer.interface;
    try stdout.writeAll(reused_output);
    try stdout.writeByte('\n');
    try stdout.writeAll(output);
    try stdout.flush();
}
