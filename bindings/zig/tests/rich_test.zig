const std = @import("std");
const rich = @import("rich");

fn expectUtf8(bytes: []const u8) !void {
    try std.testing.expect(std.unicode.utf8ValidateSlice(bytes));
}

fn plainConsole() !rich.Console {
    var console = try rich.Console.init();
    errdefer console.deinit();
    try console.setForceTerminal(false);
    try console.setSize(80, 24);
    try console.setColorSystem(.truecolor);
    try console.setMarkupEnabled(true);
    try console.setEmojiEnabled(true);
    return console;
}

fn textRenderable(value: []const u8, style: ?[]const u8) !rich.Renderable {
    var text = try rich.Text.init(value);
    defer text.deinit();
    if (style) |description| try text.setStyle(description);
    return text.finish();
}

test "ABI metadata, exact errors, strings, and buffers" {
    try rich.ensureAbi();
    try rich.check(rich.raw.RICH_OK);
    try std.testing.expectError(error.NullArgument, rich.check(rich.raw.RICH_ERROR_NULL_ARGUMENT));
    try std.testing.expectError(error.InvalidUtf8, rich.check(rich.raw.RICH_ERROR_INVALID_UTF8));
    try std.testing.expectError(error.InvalidArgument, rich.check(rich.raw.RICH_ERROR_INVALID_ARGUMENT));
    try std.testing.expectError(error.InvalidState, rich.check(rich.raw.RICH_ERROR_INVALID_STATE));
    try std.testing.expectError(error.Io, rich.check(rich.raw.RICH_ERROR_IO));
    try std.testing.expectError(error.Render, rich.check(rich.raw.RICH_ERROR_RENDER));
    try std.testing.expectError(error.NativePanic, rich.check(rich.raw.RICH_ERROR_PANIC));
    try std.testing.expectError(error.AbiMismatch, rich.check(0xffff_fffe));

    const allocator = std.testing.allocator;
    const package_version = try rich.version(allocator);
    defer allocator.free(package_version);
    try std.testing.expectEqualStrings("1.2.2", package_version);

    var console = try plainConsole();
    defer console.deinit();
    try std.testing.expectEqual(@as(u32, 80), try console.width());

    const empty = try console.renderMarkup(allocator, "");
    defer allocator.free(empty);
    try std.testing.expectEqual(@as(usize, 0), empty.len);

    const unicode = try console.renderMarkup(allocator, "Héllo 世界 🦀");
    defer allocator.free(unicode);
    try expectUtf8(unicode);
    try std.testing.expectEqualStrings("Héllo 世界 🦀", unicode);

    const embedded_nul = try console.renderMarkup(allocator, "a\x00b");
    defer allocator.free(embedded_nul);
    try std.testing.expectEqualSlices(u8, "a\x00b", embedded_nul);

    const invalid = [_]u8{0xff};
    try std.testing.expectError(error.InvalidUtf8, console.renderMarkup(allocator, &invalid));

    const no_extra_terminator = try console.renderMarkup(allocator, "abc");
    defer allocator.free(no_extra_terminator);
    try std.testing.expectEqual(@as(usize, 3), no_extra_terminator.len);
    try std.testing.expectEqualStrings("abc", no_extra_terminator);

    console.deinit();
    console.deinit();
    try std.testing.expectError(error.InvalidState, console.width());
    try std.testing.expectError(error.InvalidState, console.renderMarkup(allocator, "x"));
}

test "plain and terminal markup honor ANSI and OSC policy" {
    const allocator = std.testing.allocator;
    var console = try plainConsole();
    defer console.deinit();

    const plain = try console.renderMarkup(allocator, "[bold link=https://example.com]hello[/]");
    defer allocator.free(plain);
    try std.testing.expect(std.mem.indexOf(u8, plain, "\x1b[") == null);
    try std.testing.expect(std.mem.indexOf(u8, plain, "\x1b]") == null);

    try console.setForceTerminal(true);
    const terminal = try console.renderMarkup(allocator, "[bold]hello[/]");
    defer allocator.free(terminal);
    try expectUtf8(terminal);
    try std.testing.expect(std.mem.indexOf(u8, terminal, "\x1b[") != null);
}

test "owners are idempotent, finish invalidates, and composition borrows" {
    const allocator = std.testing.allocator;
    var console = try plainConsole();
    defer console.deinit();

    var style = try rich.Style.parse("bold red");
    style.deinit();
    style.deinit();
    try std.testing.expectError(error.InvalidArgument, rich.Style.parse("not-a-real-style-token"));

    var text = try rich.Text.fromMarkup(&console, "[bold]reused[/]");
    defer text.deinit();

    // A native finish error before ownership transfer leaves the builder live.
    var raw_text: ?*rich.raw.RichText = @ptrCast(text.handle.?);
    try std.testing.expectError(error.NullArgument, rich.check(rich.raw.rich_text_finish(&raw_text, null)));
    try std.testing.expect(raw_text != null);
    text.handle = @ptrCast(raw_text.?);

    var reusable = try text.finish();
    defer reusable.deinit();
    try std.testing.expect(text.handle == null);
    try std.testing.expectError(error.InvalidState, text.setStyle("blue"));
    try std.testing.expectError(error.InvalidState, text.finish());

    var columns = try rich.Columns.init();
    defer columns.deinit();
    try columns.add(&reusable);
    try columns.add(&reusable);
    try columns.addString("plain child");
    try columns.setEqual(true);
    try columns.setExpand(false);
    try columns.setPadding(0, 1);
    var columns_renderable = try columns.finish();
    defer columns_renderable.deinit();

    var panel = try rich.Panel.init(&reusable);
    defer panel.deinit();
    try panel.setTitle("Borrowed");
    try panel.setSubtitle("still reusable");
    try panel.setBox(.rounded);
    try panel.setExpand(false);
    try panel.setWidth(40);
    try panel.setPadding(0, 1, 0, 1);
    try panel.setStyle("white");
    try panel.setBorderStyle("blue");
    var panel_renderable = try panel.finish();
    defer panel_renderable.deinit();

    const original_output = try console.render(allocator, &reusable);
    defer allocator.free(original_output);
    try std.testing.expect(std.mem.indexOf(u8, original_output, "reused") != null);
    const columns_output = try console.render(allocator, &columns_renderable);
    defer allocator.free(columns_output);
    try expectUtf8(columns_output);
    const panel_output = try console.render(allocator, &panel_renderable);
    defer allocator.free(panel_output);
    try expectUtf8(panel_output);

    // Local validation rejects a finished renderable before allocating/calling C.
    var dead = try textRenderable("dead", null);
    dead.deinit();
    var table = try rich.Table.init();
    defer table.deinit();
    const bad_cells = [_]*const rich.Renderable{&dead};
    try std.testing.expectError(
        error.InvalidState,
        table.addRowRenderables(allocator, &bad_cells),
    );
    const invalid_utf8 = [_]u8{0xff};
    const bad_strings = [_][]const u8{&invalid_utf8};
    try std.testing.expectError(
        error.InvalidUtf8,
        table.addRowStrings(allocator, &bad_strings),
    );
    try table.addColumn("after failed temporary array");
}

test "every owning type has idempotent deinit" {
    var console = try rich.Console.init();
    console.deinit();
    console.deinit();

    var text = try rich.Text.init("text");
    text.deinit();
    text.deinit();

    var style = try rich.Style.parse("bold");
    style.deinit();
    style.deinit();

    var reusable = try textRenderable("content", null);
    defer reusable.deinit();

    var renderable_owner = try textRenderable("renderable", null);
    renderable_owner.deinit();
    renderable_owner.deinit();

    var table = try rich.Table.init();
    table.deinit();
    table.deinit();

    var panel = try rich.Panel.init(&reusable);
    panel.deinit();
    panel.deinit();

    var tree = try rich.Tree.init("root");
    tree.deinit();
    tree.deinit();
    var renderable_tree = try rich.Tree.initRenderable(&reusable);
    renderable_tree.deinit();
    renderable_tree.deinit();

    var rule = try rich.Rule.init();
    rule.deinit();
    rule.deinit();

    var columns = try rich.Columns.init();
    columns.deinit();
    columns.deinit();

    var alignment = try rich.Align.init(&reusable, .left);
    alignment.deinit();
    alignment.deinit();

    var padding = try rich.Padding.init(&reusable, 0, 0, 0, 0);
    padding.deinit();
    padding.deinit();

    var syntax = try rich.Syntax.init("x", "text");
    syntax.deinit();
    syntax.deinit();

    var markdown = try rich.Markdown.init("text");
    markdown.deinit();
    markdown.deinit();

    var json = try rich.Json.init("{}", .{});
    json.deinit();
    json.deinit();

    var progress = try rich.Progress.init();
    progress.deinit();
    progress.deinit();

    var status = try rich.Status.init("status");
    status.deinit();
    status.deinit();

    var spinner = try rich.Spinner.init("dots");
    spinner.deinit();
    spinner.deinit();
}

test "table, panel, and tree expose the full composition surface" {
    const allocator = std.testing.allocator;
    var console = try plainConsole();
    defer console.deinit();

    var header = try textRenderable("renderable header", "bold cyan");
    defer header.deinit();
    var cell = try textRenderable("renderable cell", "green");
    defer cell.deinit();

    var table = try rich.Table.init();
    defer table.deinit();
    try table.setTitle("title");
    try table.setCaption("caption");
    try table.setTitleMarkup(&console, "[bold]markup title[/]");
    try table.setCaptionMarkup(&console, "[italic]markup caption[/]");
    try table.setBox(.square);
    try table.setShowHeader(true);
    try table.setShowLines(true);
    try table.setShowEdge(true);
    try table.setExpand(false);
    try table.setPadding(1, 1);
    try table.setStyle("white");
    try table.addColumn("plain header");
    try table.addColumnRenderable(&header);
    const strings = [_][]const u8{ "alpha", "βeta" };
    try table.addRowStrings(allocator, &strings);
    const renderables = [_]*const rich.Renderable{ &cell, &cell };
    try table.addRowRenderables(allocator, &renderables);
    var table_renderable = try table.finish();
    defer table_renderable.deinit();
    const table_output = try console.render(allocator, &table_renderable);
    defer allocator.free(table_output);
    try expectUtf8(table_output);
    try std.testing.expect(std.mem.indexOf(u8, table_output, "alpha") != null);

    var tree = try rich.Tree.init("root");
    defer tree.deinit();
    const first = try tree.add(rich.rootNode, "first");
    const second = try tree.add(rich.rootNode, "second");
    _ = second;
    const deep = try tree.add(first, "deep");
    _ = try tree.add(deep, "deeper");
    _ = try tree.addRenderable(first, &cell);
    try tree.setStyle("white");
    try tree.setGuideStyle("blue");
    try tree.setHideRoot(false);
    try std.testing.expectError(error.InvalidState, tree.add(999_999, "unknown"));
    _ = try tree.add(first, "still valid after sibling/error");
    var tree_renderable = try tree.finish();
    defer tree_renderable.deinit();
    const tree_output = try console.render(allocator, &tree_renderable);
    defer allocator.free(tree_output);
    try expectUtf8(tree_output);
    try std.testing.expect(std.mem.indexOf(u8, tree_output, "deeper") != null);
    try std.testing.expectError(error.InvalidState, tree.add(rich.rootNode, "finished"));
}

test "layout and content families render valid UTF-8" {
    const allocator = std.testing.allocator;
    var console = try plainConsole();
    defer console.deinit();
    var content = try textRenderable("layout content", "bold magenta");
    defer content.deinit();

    var rule = try rich.Rule.init();
    defer rule.deinit();
    try rule.setTitle("Rule");
    try rule.setCharacters("-");
    try rule.setStyle("blue");
    try rule.setAlign(.center);
    var rule_renderable = try rule.finish();
    defer rule_renderable.deinit();

    var alignment = try rich.Align.init(&content, .right);
    defer alignment.deinit();
    try alignment.setVertical(.middle);
    try alignment.setWidth(30);
    var aligned = try alignment.finish();
    defer aligned.deinit();

    var centered_builder = try rich.Align.center(&content);
    defer centered_builder.deinit();
    var centered = try centered_builder.finish();
    defer centered.deinit();

    var padding = try rich.Padding.init(&content, 1, 2, 1, 2);
    defer padding.deinit();
    try padding.setStyle("on blue");
    try padding.setExpand(false);
    var padded = try padding.finish();
    defer padded.deinit();

    const layouts = [_]*const rich.Renderable{ &rule_renderable, &aligned, &centered, &padded };
    for (layouts) |layout| {
        const output = try console.render(allocator, layout);
        defer allocator.free(output);
        try expectUtf8(output);
    }

    var syntax = try rich.Syntax.init("const answer: u32 = 42;", "rust");
    defer syntax.deinit();
    try syntax.setTheme("monokai");
    try syntax.setLineNumbers(true);
    try syntax.setWordWrap(true);
    try syntax.setLineRange(1, null);
    var syntax_renderable = try syntax.finish();
    defer syntax_renderable.deinit();

    var temporary = std.testing.tmpDir(.{});
    defer temporary.cleanup();
    try temporary.dir.writeFile(std.testing.io, .{
        .sub_path = "sample.rs",
        .data = "fn main() {}\n",
    });
    var temporary_path_buffer: [std.Io.Dir.max_path_bytes]u8 = undefined;
    const temporary_path_len = try temporary.dir.realPath(std.testing.io, &temporary_path_buffer);
    const source_path = try std.fs.path.join(allocator, &.{
        temporary_path_buffer[0..temporary_path_len],
        "sample.rs",
    });
    defer allocator.free(source_path);
    var syntax_file = try rich.Syntax.fromPath(source_path);
    defer syntax_file.deinit();
    var syntax_file_renderable = try syntax_file.finish();
    defer syntax_file_renderable.deinit();
    try std.testing.expectError(error.Io, rich.Syntax.fromPath("/definitely/not/a/real/rich-zig-file"));

    var markdown = try rich.Markdown.init("# Header\n\n`code`");
    defer markdown.deinit();
    try markdown.setCodeTheme("monokai");
    try markdown.setHyperlinks(false);
    try markdown.setJustify(.left);
    var markdown_renderable = try markdown.finish();
    defer markdown_renderable.deinit();

    var json = try rich.Json.init("{\"b\":1,\"a\":2}", .{ .sort_keys = true });
    defer json.deinit();
    var json_renderable = try json.finish();
    defer json_renderable.deinit();
    try std.testing.expectError(error.InvalidArgument, rich.Json.init("{broken", .{}));

    const contents = [_]*const rich.Renderable{
        &syntax_renderable,
        &syntax_file_renderable,
        &markdown_renderable,
        &json_renderable,
    };
    for (contents) |item| {
        const output = try console.render(allocator, item);
        defer allocator.free(output);
        try expectUtf8(output);
    }
}

test "progress, status, and spinner are caller-driven frames" {
    const allocator = std.testing.allocator;
    var console = try plainConsole();
    defer console.deinit();

    var progress = try rich.Progress.init();
    defer progress.deinit();
    const task = try progress.addTask("work", 100);
    try std.testing.expectEqual(@as(u64, 0), task);
    try progress.update(task, 50);
    try std.testing.expectError(error.InvalidState, progress.update(999_999, 1));
    const progress_frame = try progress.renderFrame(allocator, &console);
    defer allocator.free(progress_frame);
    try expectUtf8(progress_frame);

    var status = try rich.Status.init("working");
    defer status.deinit();
    const status_frame = try status.renderFrame(allocator, &console);
    defer allocator.free(status_frame);
    try expectUtf8(status_frame);
    try std.testing.expect(status_frame.len != 0);

    var spinner = try rich.Spinner.init("dots");
    defer spinner.deinit();
    const spinner_frame = try spinner.renderFrame(allocator, &console, 0.25);
    defer allocator.free(spinner_frame);
    try expectUtf8(spinner_frame);
    try std.testing.expect(spinner_frame.len != 0);
    try std.testing.expectError(error.InvalidArgument, spinner.renderFrame(allocator, &console, std.math.nan(f64)));
    try std.testing.expectError(error.InvalidArgument, rich.Spinner.init("not-a-spinner"));

    progress.deinit();
    progress.deinit();
    try std.testing.expectError(error.InvalidState, progress.addTask("finished", 1));
}
