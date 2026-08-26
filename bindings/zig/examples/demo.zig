const std = @import("std");
const rich = @import("rich");

const Allocator = std.mem.Allocator;

fn markup(console: *rich.Console, value: []const u8) !rich.Renderable {
    var text = try rich.Text.fromMarkup(console, value);
    defer text.deinit();
    return text.finish();
}

fn styled(value: []const u8, style: []const u8) !rich.Renderable {
    var text = try rich.Text.init(value);
    defer text.deinit();
    try text.setStyle(style);
    return text.finish();
}

fn textRenderable(value: []const u8) !rich.Renderable {
    var text = try rich.Text.init(value);
    defer text.deinit();
    return text.finish();
}

fn addFeature(card: *rich.Table, allocator: Allocator, name: []const u8, demo: *const rich.Renderable) !void {
    var label = try textRenderable(name);
    defer label.deinit();
    const cells = [_]*const rich.Renderable{ &label, demo };
    try card.addRowRenderables(allocator, &cells);
}

fn addBlank(card: *rich.Table, allocator: Allocator) !void {
    const cells = [_][]const u8{ "", "" };
    try card.addRowStrings(allocator, &cells);
}

fn hueToRgb(p: f64, q: f64, initial: f64) f64 {
    @setFloatMode(.strict);
    var value = initial;
    if (value < 0.0) value += 1.0;
    if (value > 1.0) value -= 1.0;
    if (value < 1.0 / 6.0) return p + (q - p) * 6.0 * value;
    if (value < 0.5) return q;
    if (value < 2.0 / 3.0) return p + (q - p) * (2.0 / 3.0 - value) * 6.0;
    return p;
}

const Rgb = struct { red: u8, green: u8, blue: u8 };

fn hlsToRgb(h: f64, l: f64) Rgb {
    @setFloatMode(.strict);
    const q = if (l < 0.5) l * (1.0 + 1.0) else l + 1.0 - l * 1.0;
    const p = 2.0 * l - q;
    return .{
        .red = @intFromFloat(hueToRgb(p, q, h + 1.0 / 3.0) * 255.0),
        .green = @intFromFloat(hueToRgb(p, q, h) * 255.0),
        .blue = @intFromFloat(hueToRgb(p, q, h - 1.0 / 3.0) * 255.0),
    };
}

fn colorRender(context: ?*anyopaque, options: rich.RenderOptions, sink: *rich.SegmentSink) !void {
    _ = context;
    const height: u32 = 5;
    for (0..height) |y| {
        for (0..options.max_width) |x| {
            const h = @as(f64, @floatFromInt(x)) / @as(f64, @floatFromInt(options.max_width));
            const l1 = 0.1 + (@as(f64, @floatFromInt(y)) / @as(f64, @floatFromInt(height))) * 0.7;
            const background = hlsToRgb(h, l1);
            const foreground = hlsToRgb(h, l1 + 0.7 / (2.0 * @as(f64, @floatFromInt(height))));
            var style_buffer: [96]u8 = undefined;
            const style = try std.fmt.bufPrint(
                &style_buffer,
                "rgb({d},{d},{d}) on rgb({d},{d},{d})",
                .{ foreground.red, foreground.green, foreground.blue, background.red, background.green, background.blue },
            );
            try sink.push("▄", style);
        }
        try sink.line();
    }
}

fn colorMeasure(context: ?*anyopaque, options: rich.RenderOptions) !rich.Measurement {
    _ = context;
    return .{ .minimum = 1, .maximum = options.max_width };
}

fn makeColorDemo(console: *rich.Console, allocator: Allocator) !rich.Renderable {
    var checklist = try markup(
        console,
        "✓ [bold green]4-bit color[/]\n" ++
            "✓ [bold blue]8-bit color[/]\n" ++
            "✓ [bold magenta]Truecolor (16.7 million)[/]\n" ++
            "✓ [bold yellow]Dumb terminals[/]\n" ++
            "✓ [bold cyan]Automatic color conversion[/]",
    );
    defer checklist.deinit();
    var swatches = try rich.customRenderable(null, .{ .render = colorRender, .measure = colorMeasure });
    defer swatches.deinit();
    var table = try rich.Table.init();
    defer table.deinit();
    try table.setBox(.none);
    try table.setShowHeader(false);
    try table.setShowEdge(false);
    try table.setExpand(false);
    try table.setPadEdge(false);
    const cells = [_]*const rich.Renderable{ &checklist, &swatches };
    try table.addRowRenderables(allocator, &cells);
    return table.finish();
}

fn makeTextExamples(console: *rich.Console, allocator: Allocator) !rich.Renderable {
    const lorem = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. " ++
        "Quisque in metus sed sapien ultricies pretium a at justo. " ++
        "Maecenas luctus velit et auctor maximus.";
    var left = try styled(lorem, "green");
    defer left.deinit();
    var center = try styled(lorem, "yellow");
    defer center.deinit();
    var right = try styled(lorem, "blue");
    defer right.deinit();
    var full = try styled(lorem, "red");
    defer full.deinit();

    var table = try rich.Table.initGrid();
    defer table.deinit();
    try table.setPadding(1, 1);
    try table.setPadEdge(false);
    inline for (.{ rich.Justify.left, .center, .right, .full }) |justify| {
        try table.addColumnOptions(null, .{ .ratio = 1, .justify = justify });
    }
    const examples = [_]*const rich.Renderable{ &left, &center, &right, &full };
    try table.addRowRenderables(allocator, &examples);
    var examples_renderable = try table.finish();
    defer examples_renderable.deinit();

    var intro = try markup(console, "Word wrap text. Justify [green]left[/], [yellow]center[/], [blue]right[/] or [red]full[/].\n");
    defer intro.deinit();
    var blank = try textRenderable("");
    defer blank.deinit();
    var section = try rich.Table.initGrid();
    defer section.deinit();
    try section.setPadding(0, 0);
    try section.addColumnOptions(null, .{});
    for ([_]*const rich.Renderable{ &intro, &blank, &examples_renderable }) |item| {
        const row = [_]*const rich.Renderable{item};
        try section.addRowRenderables(allocator, &row);
    }
    return section.finish();
}

fn makeMovieTable(console: *rich.Console, allocator: Allocator) !rich.Renderable {
    var table = try rich.Table.init();
    defer table.deinit();
    try table.setBox(.simple);
    try table.setShowEdge(false);
    try table.setExpand(false);
    try table.setRowStyles(allocator, &.{ "", "dim" });

    var date_header = try markup(console, "[green]Date");
    defer date_header.deinit();
    var title_header = try markup(console, "[blue]Title");
    defer title_header.deinit();
    var budget_header = try markup(console, "[cyan]Production Budget");
    defer budget_header.deinit();
    var office_header = try markup(console, "[magenta]Box Office");
    defer office_header.deinit();
    try table.addColumnOptions(&date_header, .{ .style = "green", .min_width = 12, .no_wrap = true });
    try table.addColumnOptions(&title_header, .{ .style = "blue", .ratio = 1 });
    try table.addColumnOptions(&budget_header, .{ .style = "cyan", .justify = .right, .min_width = 17, .no_wrap = true });
    try table.addColumnOptions(&office_header, .{ .style = "magenta", .justify = .right, .min_width = 14, .no_wrap = true });

    const movies = [_][4][]const u8{
        .{ "Dec 20, 2019", "Star Wars: The Rise of Skywalker", "$275,000,000", "$375,126,118" },
        .{ "May 25, 2018", "[b]Solo[/]: A Star Wars Story", "$275,000,000", "$393,151,347" },
        .{ "Dec 15, 2017", "Star Wars Ep. VIII: The Last Jedi", "$262,000,000", "[bold]$1,332,539,889[/]" },
        .{ "May 19, 1999", "Star Wars Ep. [b]I[/b]: [i]The phantom Menace[/i]", "$115,000,000", "$1,027,044,677" },
    };
    for (movies) |movie| {
        var cells: [4]rich.Renderable = undefined;
        var initialized: usize = 0;
        defer for (cells[0..initialized]) |*cell| cell.deinit();
        for (movie, 0..) |value, index| {
            cells[index] = try markup(console, value);
            initialized += 1;
        }
        const borrowed = [_]*const rich.Renderable{ &cells[0], &cells[1], &cells[2], &cells[3] };
        try table.addRowRenderables(allocator, &borrowed);
    }
    return table.finish();
}

fn makeSyntaxPrettyComparison(allocator: Allocator) !rich.Renderable {
    const code =
        "def iter_last(values: Iterable[T]) -> Iterable[Tuple[bool, T]]:\n" ++
        "    \"\"\"Iterate and generate a tuple with a flag for last value.\"\"\"\n" ++
        "    iter_values = iter(values)\n" ++
        "    try:\n" ++
        "        previous_value = next(iter_values)\n" ++
        "    except StopIteration:\n" ++
        "        return\n" ++
        "    for value in iter_values:\n" ++
        "        yield False, previous_value\n" ++
        "        previous_value = value\n" ++
        "    yield True, previous_value";
    const pretty_data = "{'foo': [3.1427, ('Paul Atreides', 'Vladimir Harkonnen', 'Thufir Hawat')], " ++
        "'atomic': (False, True, None)}";
    var syntax = try rich.Syntax.init(code, "python3");
    defer syntax.deinit();
    try syntax.setLineNumbers(true);
    try syntax.setIndentGuides(true);
    var syntax_renderable = try syntax.finish();
    defer syntax_renderable.deinit();
    var pretty = try rich.Pretty.init(pretty_data);
    defer pretty.deinit();
    try pretty.setIndentGuides(true);
    var pretty_renderable = try pretty.finish();
    defer pretty_renderable.deinit();
    var comparison = try rich.Table.init();
    defer comparison.deinit();
    try comparison.setBox(.none);
    try comparison.setShowHeader(false);
    try comparison.setExpand(true);
    try comparison.setPadEdge(false);
    try comparison.setPadding(1, 1);
    try comparison.addColumnOptions(null, .{ .ratio = 1 });
    try comparison.addColumnOptions(null, .{ .ratio = 1 });
    const cells = [_]*const rich.Renderable{ &syntax_renderable, &pretty_renderable };
    try comparison.addRowRenderables(allocator, &cells);
    return comparison.finish();
}

fn makeMarkdownComparison(allocator: Allocator) !rich.Renderable {
    const source = "# Markdown\n\n" ++
        "Supports much of the *markdown* __syntax__!\n\n" ++
        "- Headers\n" ++
        "- Basic formatting: **bold**, *italic*, `code`\n" ++
        "- Block quotes\n" ++
        "- Lists, and more...\n";
    var raw_source = try styled(source, "cyan");
    defer raw_source.deinit();
    var markdown = try rich.Markdown.init(source);
    defer markdown.deinit();
    var rendered = try markdown.finish();
    defer rendered.deinit();
    var comparison = try rich.Table.init();
    defer comparison.deinit();
    try comparison.setBox(.none);
    try comparison.setShowHeader(false);
    try comparison.setExpand(true);
    try comparison.setPadEdge(false);
    try comparison.addColumnOptions(null, .{ .ratio = 1 });
    try comparison.addColumnOptions(null, .{ .ratio = 1 });
    const cells = [_]*const rich.Renderable{ &raw_source, &rendered };
    try comparison.addRowRenderables(allocator, &cells);
    return comparison.finish();
}

fn makeTestCard(console: *rich.Console, allocator: Allocator) !rich.Renderable {
    var card = try rich.Table.initGrid();
    defer card.deinit();
    try card.setTitleMarkup(console, "[italic]Rich features[/]");
    try card.setPadding(1, 1);
    try card.setPadEdge(true);
    try card.addColumnOptions(null, .{ .width = 12, .justify = .center, .vertical = .top, .style = "bold red", .no_wrap = true });
    try card.addColumnOptions(null, .{});

    var colors = try makeColorDemo(console, allocator);
    defer colors.deinit();
    try addFeature(&card, allocator, "Colors", &colors);
    try addBlank(&card, allocator);
    var styles = try markup(console, "All ansi styles: [bold]bold[/], [dim]dim[/], [italic]italic[/italic], " ++
        "[underline]underline[/], [strike]strikethrough[/], [reverse]reverse[/], and even [blink]blink[/].");
    defer styles.deinit();
    try addFeature(&card, allocator, "Styles", &styles);
    try addBlank(&card, allocator);
    var text_examples = try makeTextExamples(console, allocator);
    defer text_examples.deinit();
    try addFeature(&card, allocator, "Text", &text_examples);
    try addBlank(&card, allocator);
    var asian = try markup(console, "🇨🇳  该库支持中文，日文和韩文文本！\n" ++
        "🇯🇵  ライブラリは中国語、日本語、韓国語のテキストをサポートしています\n" ++
        "🇰🇷  이 라이브러리는 중국어, 일본어 및 한국어 텍스트를 지원합니다");
    defer asian.deinit();
    try addFeature(&card, allocator, "Asian\nlanguage\nsupport", &asian);
    try addBlank(&card, allocator);
    var markup_demo = try markup(console, "[bold magenta]Rich[/] supports a simple [i]bbcode[/i]-like [b]markup[/b] for " ++
        "[yellow]color[/], [underline]style[/], and emoji! :+1: :apple: :ant: :bear: :baguette_bread: :bus:");
    defer markup_demo.deinit();
    try addFeature(&card, allocator, "Markup", &markup_demo);
    try addBlank(&card, allocator);
    var movies = try makeMovieTable(console, allocator);
    defer movies.deinit();
    try addFeature(&card, allocator, "Tables", &movies);
    try addBlank(&card, allocator);
    var syntax_pretty = try makeSyntaxPrettyComparison(allocator);
    defer syntax_pretty.deinit();
    try addFeature(&card, allocator, "Syntax\nhighlighting\n&\npretty\nprinting", &syntax_pretty);
    try addBlank(&card, allocator);
    var markdown = try makeMarkdownComparison(allocator);
    defer markdown.deinit();
    try addFeature(&card, allocator, "Markdown", &markdown);
    try addBlank(&card, allocator);
    try addBlank(&card, allocator);
    var more = try textRenderable("Progress bars, columns, styled logging handler, tracebacks, etc...");
    defer more.deinit();
    try addFeature(&card, allocator, "+more!", &more);
    return card.finish();
}

fn makeSponsorPanel(console: *rich.Console) !rich.Renderable {
    var body = try markup(console, "[b magenta]Hope you enjoy using Rich-rs![/]\n\n" ++
        "Consider sponsoring the upstream port to ensure it's maintained.\n\n" ++
        "[cyan]https://github.com/sponsors/willmcgugan[/cyan] | [b]Rust port: [/b]" ++
        "[green]Marcos Saraiva - https://github.com/mrsaraiva/rich-rs[/green]");
    defer body.deinit();
    var panel = try rich.Panel.init(&body);
    defer panel.deinit();
    try panel.setTitle("Help ensure Rich is maintained");
    try panel.setBorderStyle("green");
    try panel.setPadding(1, 2, 1, 2);
    return panel.finish();
}

pub fn main(init: std.process.Init) !void {
    const args = try init.minimal.args.toSlice(init.arena.allocator());
    if (args.len > 2 or (args.len == 2 and !std.mem.eql(u8, args[1], "--parity"))) {
        var stderr_buffer: [256]u8 = undefined;
        var stderr_writer = std.Io.File.stderr().writer(init.io, &stderr_buffer);
        try stderr_writer.interface.writeAll("usage: rich-zig-demo [--parity]\n");
        try stderr_writer.interface.flush();
        std.process.exit(1);
    }

    var gpa: std.heap.DebugAllocator(.{}) = .init;
    defer if (gpa.deinit() == .leak) @panic("rich Zig demo leaked memory");
    const allocator = gpa.allocator();
    var console = try rich.Console.init();
    defer console.deinit();
    try console.setForceTerminal(true);
    try console.setColorSystem(.truecolor);
    try console.setMarkupEnabled(true);
    try console.setEmojiEnabled(true);
    try console.setSize(120, 72);
    var card = try makeTestCard(&console, allocator);
    defer card.deinit();

    const cold_start = std.Io.Clock.awake.now(init.io);
    const cold_output = try console.render(allocator, &card);
    const warm_start = std.Io.Clock.awake.now(init.io);
    allocator.free(cold_output);
    const warm_output = try console.render(allocator, &card);
    const finish = std.Io.Clock.awake.now(init.io);
    allocator.free(warm_output);
    const cold_ms = @as(f64, @floatFromInt(cold_start.durationTo(warm_start).toNanoseconds())) / 1_000_000.0;
    const warm_ms = @as(f64, @floatFromInt(warm_start.durationTo(finish).toNanoseconds())) / 1_000_000.0;

    const rendered = try console.render(allocator, &card);
    defer allocator.free(rendered);
    const cold_markup = try std.fmt.allocPrint(allocator, "[dim]rendered in [not dim][cyan]{d:.1}[/cyan]ms[/] (cold cache)", .{cold_ms});
    defer allocator.free(cold_markup);
    const cold_timing = try console.renderMarkup(allocator, cold_markup);
    defer allocator.free(cold_timing);
    const warm_markup = try std.fmt.allocPrint(allocator, "[dim]rendered in [not dim][cyan]{d:.1}[/cyan]ms[/] (warm cache)", .{warm_ms});
    defer allocator.free(warm_markup);
    const warm_timing = try console.renderMarkup(allocator, warm_markup);
    defer allocator.free(warm_timing);
    var sponsor = try makeSponsorPanel(&console);
    defer sponsor.deinit();
    const sponsor_output = try console.render(allocator, &sponsor);
    defer allocator.free(sponsor_output);

    var stdout_buffer: [4096]u8 = undefined;
    var stdout_writer = std.Io.File.stdout().writer(init.io, &stdout_buffer);
    const stdout = &stdout_writer.interface;
    try stdout.writeAll(rendered);
    try stdout.writeByte('\n');
    try stdout.writeAll(cold_timing);
    try stdout.writeByte('\n');
    try stdout.writeAll(warm_timing);
    try stdout.writeAll("\n\n");
    try stdout.writeAll(sponsor_output);
    try stdout.writeByte('\n');
    try stdout.flush();
}
