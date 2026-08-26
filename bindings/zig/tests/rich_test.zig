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

fn renderColumnProbe(
    allocator: std.mem.Allocator,
    console: *rich.Console,
    options: rich.ColumnOptions,
    expand: bool,
) ![]u8 {
    var table = try rich.Table.initGrid();
    defer table.deinit();
    try table.setPadEdge(false);
    try table.setExpand(expand);
    try table.addColumnOptions(null, options);
    try table.addColumnOptions(null, .{ .ratio = 1 });
    const cells = [_][]const u8{ "alpha beta gamma\nlast", "one\ntwo\nthree" };
    try table.addRowStrings(allocator, &cells);
    var renderable = try table.finish();
    defer renderable.deinit();
    return console.render(allocator, &renderable);
}

fn renderPadEdgeProbe(
    allocator: std.mem.Allocator,
    console: *rich.Console,
    pad_edge: bool,
) ![]u8 {
    var table = try rich.Table.initGrid();
    defer table.deinit();
    try table.setPadEdge(pad_edge);
    try table.setExpand(false);
    try table.setPadding(1, 1);
    try table.addColumnOptions(null, .{ .width = 8 });
    try table.addRowStrings(allocator, &.{"edge"});
    var renderable = try table.finish();
    defer renderable.deinit();
    return console.render(allocator, &renderable);
}

fn renderBoxProbe(
    allocator: std.mem.Allocator,
    console: *rich.Console,
    box: rich.Box,
) ![]u8 {
    var table = try rich.Table.init();
    defer table.deinit();
    try table.setShowHeader(false);
    try table.setExpand(false);
    try table.setBox(box);
    try table.addColumnOptions(null, .{ .width = 8 });
    try table.addRowStrings(allocator, &.{"box"});
    var renderable = try table.finish();
    defer renderable.deinit();
    return console.render(allocator, &renderable);
}

const RowStyleProbe = enum { none, red, green, replace, clear };

fn renderRowStyleProbe(
    allocator: std.mem.Allocator,
    console: *rich.Console,
    mode: RowStyleProbe,
) ![]u8 {
    var table = try rich.Table.initGrid();
    defer table.deinit();
    try table.setPadEdge(false);
    try table.setExpand(false);
    switch (mode) {
        .none => {},
        .red => try table.setRowStyles(allocator, &.{"red"}),
        .green => try table.setRowStyles(allocator, &.{"green"}),
        .replace => {
            try table.setRowStyles(allocator, &.{"red"});
            try table.setRowStyles(allocator, &.{"green"});
        },
        .clear => {
            try table.setRowStyles(allocator, &.{"red"});
            try table.setRowStyles(allocator, &.{});
        },
    }
    try table.addColumnOptions(null, .{ .width = 12 });
    try table.addRowStrings(allocator, &.{"row style"});
    var renderable = try table.finish();
    defer renderable.deinit();
    return console.render(allocator, &renderable);
}

const CallbackProbe = struct {
    render_count: usize = 0,
    measure_count: usize = 0,
    drop_count: usize = 0,
    last_width: u32 = 0,
    height: u32 = 1,
    retained_sink: ?*rich.SegmentSink = null,
    prior_sink_rejected: bool = false,
    console: ?*rich.Console = null,
    reentry_rejected: bool = false,
};

fn probeRender(
    context: ?*anyopaque,
    options: rich.RenderOptions,
    sink: *rich.SegmentSink,
) !void {
    const probe: *CallbackProbe = @ptrCast(@alignCast(context.?));
    probe.render_count += 1;
    probe.last_width = options.max_width;
    if (probe.retained_sink) |prior| {
        _ = prior.line() catch |failure| {
            probe.prior_sink_rejected = failure == error.InvalidState;
        };
    }
    probe.retained_sink = sink;
    if (probe.console) |console| {
        _ = console.width() catch |failure| {
            probe.reentry_rejected = failure == error.InvalidState;
        };
    }
    for (0..probe.height) |_| {
        for (0..options.max_width) |_| try sink.push("▄", "rgb(255,0,0)");
        try sink.line();
    }
}

fn probeMeasure(context: ?*anyopaque, options: rich.RenderOptions) !rich.Measurement {
    const probe: *CallbackProbe = @ptrCast(@alignCast(context.?));
    probe.measure_count += 1;
    probe.last_width = options.max_width;
    return .{ .minimum = 1, .maximum = options.max_width };
}

fn probeDrop(context: ?*anyopaque) void {
    const probe: *CallbackProbe = @ptrCast(@alignCast(context.?));
    probe.drop_count += 1;
}

fn failingRender(
    context: ?*anyopaque,
    options: rich.RenderOptions,
    sink: *rich.SegmentSink,
) !void {
    _ = context;
    _ = options;
    try sink.push("partial", "red");
    return error.InvalidArgument;
}

fn defaultRender(
    context: ?*anyopaque,
    options: rich.RenderOptions,
    sink: *rich.SegmentSink,
) !void {
    _ = context;
    _ = options;
    try sink.push("default measurement", "cyan");
    try sink.line();
}

fn malformedMeasure(context: ?*anyopaque, options: rich.RenderOptions) !rich.Measurement {
    _ = context;
    _ = options;
    return .{ .minimum = 2, .maximum = 1 };
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

    var pretty = try rich.Pretty.init("{'x': [1, 2]}");
    pretty.deinit();
    pretty.deinit();
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

test "ABI v2 table options, Pretty, and syntax guides are native features" {
    try std.testing.expectEqual(@as(u32, 2), rich.raw.RICH_FFI_ABI_VERSION);
    try std.testing.expectEqual(@as(u32, 14), rich.raw.RICH_BOX_NONE);
    try std.testing.expectEqual(@as(usize, 24), @sizeOf(rich.raw.RichRenderOptions));
    try std.testing.expectEqual(@as(usize, 40), @sizeOf(rich.raw.RichTableColumnOptions));

    const allocator = std.testing.allocator;
    var console = try plainConsole();
    defer console.deinit();
    try console.setSize(48, 24);

    var header = try textRenderable("Header", "bold cyan");
    defer header.deinit();
    var left = try textRenderable("left value wraps here", "green");
    defer left.deinit();
    var right = try textRenderable("42", "magenta");
    defer right.deinit();

    var table = try rich.Table.initGrid();
    defer table.deinit();
    try table.setBox(.none);
    try table.setPadEdge(false);
    try table.setRowStyles(allocator, &.{ "", "dim" });
    try table.addColumnOptions(&header, .{
        .justify = .left,
        .vertical = .top,
        .ratio = 1,
        .style = "green",
    });
    try table.addColumnOptions(null, .{
        .justify = .right,
        .min_width = 8,
        .no_wrap = true,
        .style = "magenta",
    });
    const row = [_]*const rich.Renderable{ &left, &right };
    try table.addRowRenderables(allocator, &row);
    var table_renderable = try table.finish();
    defer table_renderable.deinit();
    const table_output = try console.render(allocator, &table_renderable);
    defer allocator.free(table_output);
    try std.testing.expect(std.mem.indexOf(u8, table_output, "Header") == null);
    try std.testing.expect(std.mem.indexOf(u8, table_output, "left value") != null);

    var invalid = try rich.Table.initGrid();
    defer invalid.deinit();
    try std.testing.expectError(
        error.InvalidArgument,
        invalid.addColumnOptions(null, .{ .ratio = 0 }),
    );
    try invalid.addColumnOptions(null, .{});

    try console.setSize(32, 24);
    const repr = "{'foo': [3.1427, ('Paul Atreides', 'Thufir Hawat')], 'atomic': (False, None)}";
    var pretty_plain = try rich.Pretty.init(repr);
    defer pretty_plain.deinit();
    var pretty_plain_renderable = try pretty_plain.finish();
    defer pretty_plain_renderable.deinit();
    var pretty_guided = try rich.Pretty.init(repr);
    defer pretty_guided.deinit();
    try pretty_guided.setIndentGuides(true);
    var pretty_guided_renderable = try pretty_guided.finish();
    defer pretty_guided_renderable.deinit();
    const pretty_without = try console.render(allocator, &pretty_plain_renderable);
    defer allocator.free(pretty_without);
    const pretty_with = try console.render(allocator, &pretty_guided_renderable);
    defer allocator.free(pretty_with);
    try std.testing.expect(!std.mem.eql(u8, pretty_without, pretty_with));
    try std.testing.expect(std.mem.indexOf(u8, pretty_with, "│") != null);
    try std.testing.expectError(error.InvalidState, pretty_guided.setIndentGuides(false));

    const unicode_nul_repr = "{'日本語': ['nul\x00tail', {'emoji': '🦀', '値': [1, 2, 3, 4, 5]}]}";
    var pretty_unicode_nul_plain = try rich.Pretty.init(unicode_nul_repr);
    defer pretty_unicode_nul_plain.deinit();
    var pretty_unicode_nul_plain_renderable = try pretty_unicode_nul_plain.finish();
    defer pretty_unicode_nul_plain_renderable.deinit();
    var pretty_unicode_nul_guided = try rich.Pretty.init(unicode_nul_repr);
    defer pretty_unicode_nul_guided.deinit();
    try pretty_unicode_nul_guided.setIndentGuides(true);
    var pretty_unicode_nul_guided_renderable = try pretty_unicode_nul_guided.finish();
    defer pretty_unicode_nul_guided_renderable.deinit();
    const pretty_unicode_nul_without = try console.render(allocator, &pretty_unicode_nul_plain_renderable);
    defer allocator.free(pretty_unicode_nul_without);
    const pretty_unicode_nul_with = try console.render(allocator, &pretty_unicode_nul_guided_renderable);
    defer allocator.free(pretty_unicode_nul_with);
    try expectUtf8(pretty_unicode_nul_without);
    try std.testing.expect(std.mem.indexOf(u8, pretty_unicode_nul_without, "日本語") != null);
    try std.testing.expect(std.mem.indexOfScalar(u8, pretty_unicode_nul_without, 0) != null);
    try std.testing.expect(!std.mem.eql(u8, pretty_unicode_nul_without, pretty_unicode_nul_with));

    const distinct_repr = "[Point(x=1, y=2), {'enabled': True, 'items': [10, 20, 30, 40]}]";
    var pretty_distinct_plain = try rich.Pretty.init(distinct_repr);
    defer pretty_distinct_plain.deinit();
    var pretty_distinct_plain_renderable = try pretty_distinct_plain.finish();
    defer pretty_distinct_plain_renderable.deinit();
    var pretty_distinct_guided = try rich.Pretty.init(distinct_repr);
    defer pretty_distinct_guided.deinit();
    try pretty_distinct_guided.setIndentGuides(true);
    var pretty_distinct_guided_renderable = try pretty_distinct_guided.finish();
    defer pretty_distinct_guided_renderable.deinit();
    const pretty_distinct_without = try console.render(allocator, &pretty_distinct_plain_renderable);
    defer allocator.free(pretty_distinct_without);
    const pretty_distinct_with = try console.render(allocator, &pretty_distinct_guided_renderable);
    defer allocator.free(pretty_distinct_with);
    try std.testing.expect(std.mem.indexOf(u8, pretty_distinct_without, "Point") != null);
    try std.testing.expect(!std.mem.eql(u8, pretty_distinct_without, pretty_distinct_with));
    try std.testing.expect(!std.mem.eql(u8, pretty_unicode_nul_without, pretty_distinct_without));

    const syntax_source = "fn main() {\n    if true {\n        work();\n    }\n}";
    var syntax_plain = try rich.Syntax.init(syntax_source, "rust");
    defer syntax_plain.deinit();
    var syntax_plain_renderable = try syntax_plain.finish();
    defer syntax_plain_renderable.deinit();
    var syntax_guided = try rich.Syntax.init(syntax_source, "rust");
    defer syntax_guided.deinit();
    try syntax_guided.setIndentGuides(true);
    var syntax_guided_renderable = try syntax_guided.finish();
    defer syntax_guided_renderable.deinit();
    const syntax_without = try console.render(allocator, &syntax_plain_renderable);
    defer allocator.free(syntax_without);
    const syntax_with = try console.render(allocator, &syntax_guided_renderable);
    defer allocator.free(syntax_with);
    try std.testing.expect(!std.mem.eql(u8, syntax_without, syntax_with));
    try std.testing.expect(std.mem.indexOf(u8, syntax_with, "│") != null);
}

test "configured table inputs are call-scoped and allocation failures leave it usable" {
    const allocator = std.testing.allocator;
    var console = try plainConsole();
    defer console.deinit();

    var header = try textRenderable("retained header", "bold cyan");
    defer header.deinit();
    var cell = try textRenderable("retained cell", null);
    defer cell.deinit();
    var table = try rich.Table.init();
    defer table.deinit();

    var failing = std.testing.FailingAllocator.init(allocator, .{ .fail_index = 0 });
    try std.testing.expectError(
        error.OutOfMemory,
        table.setRowStyles(failing.allocator(), &.{"red"}),
    );

    const temporary_style = try allocator.dupe(u8, "green");
    try table.setRowStyles(allocator, &.{temporary_style});
    try table.addColumnOptions(&header, .{ .style = temporary_style });
    header.deinit();
    @memset(temporary_style, 'x');
    allocator.free(temporary_style);

    const cells = [_]*const rich.Renderable{&cell};
    try table.addRowRenderables(allocator, &cells);
    var renderable = try table.finish();
    defer renderable.deinit();
    const output = try console.render(allocator, &renderable);
    defer allocator.free(output);
    try std.testing.expect(std.mem.indexOf(u8, output, "retained header") != null);
    try std.testing.expect(std.mem.indexOf(u8, output, "retained cell") != null);

    var panel = try rich.Panel.init(&cell);
    defer panel.deinit();
    try std.testing.expectError(error.InvalidArgument, panel.setBox(.none));
    try panel.setBox(.rounded);
}

test "configured column options vary independently" {
    const allocator = std.testing.allocator;
    var console = try plainConsole();
    defer console.deinit();
    try console.setForceTerminal(true);
    try console.setSize(36, 24);

    const justify_left = try renderColumnProbe(
        allocator,
        &console,
        .{ .justify = .left, .width = 18, .no_wrap = true },
        false,
    );
    defer allocator.free(justify_left);
    const justify_right = try renderColumnProbe(
        allocator,
        &console,
        .{ .justify = .right, .width = 18, .no_wrap = true },
        false,
    );
    defer allocator.free(justify_right);
    try std.testing.expect(!std.mem.eql(u8, justify_left, justify_right));

    const vertical_top = try renderColumnProbe(
        allocator,
        &console,
        .{ .vertical = .top, .width = 18, .no_wrap = true },
        false,
    );
    defer allocator.free(vertical_top);
    const vertical_bottom = try renderColumnProbe(
        allocator,
        &console,
        .{ .vertical = .bottom, .width = 18, .no_wrap = true },
        false,
    );
    defer allocator.free(vertical_bottom);
    try std.testing.expect(!std.mem.eql(u8, vertical_top, vertical_bottom));

    const narrow_width = try renderColumnProbe(allocator, &console, .{ .width = 8 }, false);
    defer allocator.free(narrow_width);
    const wide_width = try renderColumnProbe(allocator, &console, .{ .width = 18 }, false);
    defer allocator.free(wide_width);
    try std.testing.expect(!std.mem.eql(u8, narrow_width, wide_width));

    const small_minimum = try renderColumnProbe(allocator, &console, .{ .min_width = 4 }, false);
    defer allocator.free(small_minimum);
    const large_minimum = try renderColumnProbe(allocator, &console, .{ .min_width = 18 }, false);
    defer allocator.free(large_minimum);
    try std.testing.expect(!std.mem.eql(u8, small_minimum, large_minimum));

    const equal_ratio = try renderColumnProbe(allocator, &console, .{ .ratio = 1 }, true);
    defer allocator.free(equal_ratio);
    const weighted_ratio = try renderColumnProbe(allocator, &console, .{ .ratio = 3 }, true);
    defer allocator.free(weighted_ratio);
    try std.testing.expect(!std.mem.eql(u8, equal_ratio, weighted_ratio));

    const wrapping = try renderColumnProbe(allocator, &console, .{ .width = 8 }, false);
    defer allocator.free(wrapping);
    const no_wrapping = try renderColumnProbe(
        allocator,
        &console,
        .{ .width = 8, .no_wrap = true },
        false,
    );
    defer allocator.free(no_wrapping);
    try std.testing.expect(!std.mem.eql(u8, wrapping, no_wrapping));

    const red_style = try renderColumnProbe(
        allocator,
        &console,
        .{ .width = 18, .style = "red" },
        false,
    );
    defer allocator.free(red_style);
    const green_style = try renderColumnProbe(
        allocator,
        &console,
        .{ .width = 18, .style = "green" },
        false,
    );
    defer allocator.free(green_style);
    try std.testing.expect(!std.mem.eql(u8, red_style, green_style));

    const padded_edge = try renderPadEdgeProbe(allocator, &console, true);
    defer allocator.free(padded_edge);
    const unpadded_edge = try renderPadEdgeProbe(allocator, &console, false);
    defer allocator.free(unpadded_edge);
    try std.testing.expect(!std.mem.eql(u8, padded_edge, unpadded_edge));

    const square_box = try renderBoxProbe(allocator, &console, .square);
    defer allocator.free(square_box);
    const no_box = try renderBoxProbe(allocator, &console, .none);
    defer allocator.free(no_box);
    try std.testing.expect(!std.mem.eql(u8, square_box, no_box));

    const no_row_style = try renderRowStyleProbe(allocator, &console, .none);
    defer allocator.free(no_row_style);
    const red_row_style = try renderRowStyleProbe(allocator, &console, .red);
    defer allocator.free(red_row_style);
    const green_row_style = try renderRowStyleProbe(allocator, &console, .green);
    defer allocator.free(green_row_style);
    const replaced_row_style = try renderRowStyleProbe(allocator, &console, .replace);
    defer allocator.free(replaced_row_style);
    const cleared_row_style = try renderRowStyleProbe(allocator, &console, .clear);
    defer allocator.free(cleared_row_style);
    try std.testing.expect(!std.mem.eql(u8, red_row_style, green_row_style));
    try std.testing.expectEqualSlices(u8, green_row_style, replaced_row_style);
    try std.testing.expectEqualSlices(u8, no_row_style, cleared_row_style);
}

test "custom renderable scopes, errors, and native last-Arc drop are safe" {
    const allocator = std.testing.allocator;
    var console = try plainConsole();
    defer console.deinit();
    try console.setForceTerminal(true);
    try console.setSize(7, 24);

    var probe: CallbackProbe = .{ .height = 3, .console = &console };
    var custom = try rich.customRenderable(&probe, .{
        .render = probeRender,
        .measure = probeMeasure,
        .drop = probeDrop,
    });
    var table = try rich.Table.initGrid();
    defer table.deinit();
    try table.addColumnOptions(null, .{ .ratio = 1 });
    const cells = [_]*const rich.Renderable{&custom};
    try table.addRowRenderables(allocator, &cells);
    var panel = try rich.Panel.init(&custom);
    defer panel.deinit();
    custom.deinit();
    try std.testing.expectEqual(@as(usize, 0), probe.drop_count);

    var table_renderable = try table.finish();
    var panel_renderable = try panel.finish();
    const output = try console.render(allocator, &table_renderable);
    defer allocator.free(output);
    try std.testing.expectEqual(@as(usize, 1), probe.render_count);
    try std.testing.expect(probe.measure_count > 0);
    try std.testing.expectEqual(@as(u32, 7), probe.last_width);
    try std.testing.expect(probe.reentry_rejected);
    try std.testing.expectEqual(@as(usize, 21), std.mem.count(u8, output, "▄"));
    try std.testing.expect(probe.retained_sink != null);
    try std.testing.expectError(error.InvalidState, probe.retained_sink.?.line());

    const panel_output = try console.render(allocator, &panel_renderable);
    defer allocator.free(panel_output);
    try std.testing.expectEqual(@as(usize, 2), probe.render_count);
    try std.testing.expect(probe.prior_sink_rejected);
    try std.testing.expectError(error.InvalidState, probe.retained_sink.?.line());

    table_renderable.deinit();
    try std.testing.expectEqual(@as(usize, 0), probe.drop_count);
    panel_renderable.deinit();
    try std.testing.expectEqual(@as(usize, 1), probe.drop_count);

    var failure_probe: CallbackProbe = .{};
    var failing = try rich.customRenderable(&failure_probe, .{
        .render = failingRender,
        .drop = probeDrop,
    });
    defer failing.deinit();
    try std.testing.expectError(error.InvalidArgument, console.render(allocator, &failing));

    try std.testing.expectError(
        error.InvalidArgument,
        rich.customRenderable(&failure_probe, .{ .render = probeRender }),
    );

    var null_context = try rich.customRenderable(null, .{ .render = probeRender });
    null_context.deinit();
    null_context.deinit();
}

test "custom feature probes adapt height and width and validate measurement" {
    const allocator = std.testing.allocator;
    inline for (.{ 1, 3 }) |height| {
        inline for (.{ 1, 7 }) |width_value| {
            var console = try plainConsole();
            defer console.deinit();
            try console.setForceTerminal(true);
            try console.setSize(width_value, 24);
            var probe: CallbackProbe = .{ .height = height };
            var custom = try rich.customRenderable(&probe, .{
                .render = probeRender,
                .measure = probeMeasure,
                .drop = probeDrop,
            });
            const output = try console.render(allocator, &custom);
            defer allocator.free(output);
            try std.testing.expectEqual(
                @as(usize, height * width_value),
                std.mem.count(u8, output, "▄"),
            );
            try std.testing.expectEqual(@as(u32, width_value), probe.last_width);
            try std.testing.expect(std.mem.indexOf(u8, output, "38;2;255;0;0") != null);
            custom.deinit();
            try std.testing.expectEqual(@as(usize, 1), probe.drop_count);
        }
    }

    var console = try plainConsole();
    defer console.deinit();
    var default_callbacks: rich.CustomCallbacks = .{ .render = defaultRender };
    var defaults = try rich.customRenderable(null, default_callbacks);
    defer defaults.deinit();
    // The wrapper copies callback values. Mutating the caller's temporary
    // descriptor cannot redirect a live adapter.
    default_callbacks.render = failingRender;
    var default_table = try rich.Table.initGrid();
    defer default_table.deinit();
    try default_table.addColumnOptions(null, .{ .ratio = 1 });
    const default_cells = [_]*const rich.Renderable{&defaults};
    try default_table.addRowRenderables(allocator, &default_cells);
    var default_renderable = try default_table.finish();
    defer default_renderable.deinit();
    const default_output = try console.render(allocator, &default_renderable);
    defer allocator.free(default_output);
    try std.testing.expect(std.mem.indexOf(u8, default_output, "default measurement") != null);

    var malformed = try rich.customRenderable(null, .{
        .render = defaultRender,
        .measure = malformedMeasure,
    });
    defer malformed.deinit();
    var measurement_table = try rich.Table.initGrid();
    defer measurement_table.deinit();
    try measurement_table.addColumnOptions(null, .{ .ratio = 1 });
    const malformed_cells = [_]*const rich.Renderable{&malformed};
    try measurement_table.addRowRenderables(allocator, &malformed_cells);
    var measurement_renderable = try measurement_table.finish();
    defer measurement_renderable.deinit();
    if (console.render(allocator, &measurement_renderable)) |unexpected| {
        defer allocator.free(unexpected);
        return error.TestExpectedError;
    } else |err| {
        try std.testing.expectEqual(error.Render, err);
    }
}

test "native buffers are released when Zig output allocation fails" {
    var console = try plainConsole();
    defer console.deinit();
    var failing = std.testing.FailingAllocator.init(std.testing.allocator, .{
        .fail_index = 0,
    });
    try std.testing.expectError(
        error.OutOfMemory,
        console.renderMarkup(failing.allocator(), "allocator cleanup"),
    );
    const retry = try console.renderMarkup(std.testing.allocator, "still healthy");
    defer std.testing.allocator.free(retry);
    try std.testing.expectEqualStrings("still healthy", retry);
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
