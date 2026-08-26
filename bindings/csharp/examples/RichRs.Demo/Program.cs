using RichRs;
using System;
using System.Diagnostics;
using System.Globalization;
using System.Text;

internal static class Program
{
    private const string Code = """"
        def iter_last(values: Iterable[T]) -> Iterable[Tuple[bool, T]]:
            """Iterate and generate a tuple with a flag for last value."""
            iter_values = iter(values)
            try:
                previous_value = next(iter_values)
            except StopIteration:
                return
            for value in iter_values:
                yield False, previous_value
                previous_value = value
            yield True, previous_value
        """";

    private const string MarkdownSource = """
        # Markdown

        Supports much of the *markdown* __syntax__!

        - Headers
        - Basic formatting: **bold**, *italic*, `code`
        - Block quotes
        - Lists, and more...

        """;

    public static int Main(string[] args)
    {
        try
        {
            if (args is ["--invalid-string-probe"])
            {
                RunInvalidStringProbe();
                return 0;
            }
            if (args is ["--version-only"])
            {
                Console.Out.WriteLine(RichRuntime.Version);
                return 0;
            }
            if (args.Length > 1 || (args.Length == 1 && args[0] != "--parity"))
            {
                Console.Error.WriteLine("usage: RichRs.Demo [--parity]");
                return 1;
            }

            using RichConsole constructionConsole = NewConsole();
            using Renderable card = MakeTestCard(constructionConsole);
            using RichConsole measureConsole = NewConsole();
            long start = Stopwatch.GetTimestamp();
            _ = measureConsole.Render(card);
            double coldMilliseconds = Stopwatch.GetElapsedTime(start).TotalMilliseconds;
            start = Stopwatch.GetTimestamp();
            _ = measureConsole.Render(card);
            double warmMilliseconds = Stopwatch.GetElapsedTime(start).TotalMilliseconds;

            using RichConsole outputConsole = NewConsole();
            using Renderable sponsor = MakeSponsorPanel(constructionConsole);
            StringBuilder output = new();
            output.Append(outputConsole.Render(card));
            output.Append('\n');
            output.Append(outputConsole.RenderMarkup(Timing(coldMilliseconds, "cold")));
            output.Append('\n');
            output.Append(outputConsole.RenderMarkup(Timing(warmMilliseconds, "warm")));
            output.Append("\n\n");
            output.Append(outputConsole.Render(sponsor));
            output.Append('\n');
            Console.Out.Write(output.ToString());
            return 0;
        }
        catch (Exception error)
        {
            Console.Error.WriteLine(error.Message);
            return 1;
        }
    }

    private static RichConsole NewConsole()
    {
        RichConsole console = new();
        console.ForceTerminal = true;
        console.ColorSystem = ColorSystem.TrueColor;
        console.MarkupEnabled = true;
        console.EmojiEnabled = true;
        console.SetSize(120, 72);
        return console;
    }

    private static Renderable MakeTestCard(RichConsole console)
    {
        using Table card = Table.CreateGrid();
        card.SetTitleMarkup(console, "[italic]Rich features[/]");
        card.SetPadding(1, 1);
        card.SetPadEdge(true);
        card.AddColumnOptions(new ColumnOptions(Justification: Justify.Center,
            Vertical: VerticalAlignment.Top, Width: 12, Style: "bold red", NoWrap: true));
        card.AddColumnOptions(new ColumnOptions());
        AddFeature(card, "Colors", MakeColorDemo(console));
        AddBlank(card);
        AddFeature(card, "Styles", Markup(console,
            "All ansi styles: [bold]bold[/], [dim]dim[/], [italic]italic[/italic], " +
            "[underline]underline[/], [strike]strikethrough[/], [reverse]reverse[/], " +
            "and even [blink]blink[/]."));
        AddBlank(card);
        AddFeature(card, "Text", MakeTextExamples(console));
        AddBlank(card);
        AddFeature(card, "Asian\nlanguage\nsupport", Markup(console,
            "🇨🇳  该库支持中文，日文和韩文文本！\n" +
            "🇯🇵  ライブラリは中国語、日本語、韓国語のテキストをサポートしています\n" +
            "🇰🇷  이 라이브러리는 중국어, 일본어 및 한국어 텍스트를 지원합니다"));
        AddBlank(card);
        AddFeature(card, "Markup", Markup(console,
            "[bold magenta]Rich[/] supports a simple [i]bbcode[/i]-like [b]markup[/b] for " +
            "[yellow]color[/], [underline]style[/], and emoji! " +
            ":+1: :apple: :ant: :bear: :baguette_bread: :bus:"));
        AddBlank(card);
        AddFeature(card, "Tables", MakeMovieTable(console));
        AddBlank(card);
        AddFeature(card, "Syntax\nhighlighting\n&\npretty\nprinting", MakeSyntaxPrettyComparison());
        AddBlank(card);
        AddFeature(card, "Markdown", MakeMarkdownComparison());
        AddBlank(card);
        AddBlank(card);
        AddFeature(card, "+more!", Styled(
            "Progress bars, columns, styled logging handler, tracebacks, etc...", ""));
        return card.Finish();
    }

    private static Renderable MakeColorDemo(RichConsole console)
    {
        using Renderable checklist = Markup(console,
            "✓ [bold green]4-bit color[/]\n" +
            "✓ [bold blue]8-bit color[/]\n" +
            "✓ [bold magenta]Truecolor (16.7 million)[/]\n" +
            "✓ [bold yellow]Dumb terminals[/]\n" +
            "✓ [bold cyan]Automatic color conversion[/]");
        using Renderable swatches = CustomRenderable.Create(new ColorBox());
        using Table table = new();
        table.Box = Box.None;
        table.ShowHeader = false;
        table.ShowEdge = false;
        table.Expand = false;
        table.SetPadEdge(false);
        table.AddRow(new[] { checklist, swatches });
        return table.Finish();
    }

    private static Renderable MakeTextExamples(RichConsole console)
    {
        const string loremText = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. " +
            "Quisque in metus sed sapien ultricies pretium a at justo. " +
            "Maecenas luctus velit et auctor maximus.";
        using Renderable left = Styled(loremText, "green");
        using Renderable center = Styled(loremText, "yellow");
        using Renderable right = Styled(loremText, "blue");
        using Renderable full = Styled(loremText, "red");
        using Table loremTable = Table.CreateGrid();
        loremTable.SetPadding(1, 1);
        loremTable.SetPadEdge(false);
        loremTable.AddColumnOptions(new ColumnOptions(Justification: Justify.Left, Ratio: 1));
        loremTable.AddColumnOptions(new ColumnOptions(Justification: Justify.Center, Ratio: 1));
        loremTable.AddColumnOptions(new ColumnOptions(Justification: Justify.Right, Ratio: 1));
        loremTable.AddColumnOptions(new ColumnOptions(Justification: Justify.Full, Ratio: 1));
        loremTable.AddRow(new[] { left, center, right, full });
        using Renderable lorem = loremTable.Finish();
        using Renderable intro = Markup(console,
            "Word wrap text. Justify [green]left[/], [yellow]center[/], " +
            "[blue]right[/] or [red]full[/].\n");
        using Renderable blank = Styled("", "");
        using Table section = Table.CreateGrid();
        section.SetPadding(0, 0);
        section.AddColumnOptions(new ColumnOptions());
        section.AddRow(new[] { intro });
        section.AddRow(new[] { blank });
        section.AddRow(new[] { lorem });
        return section.Finish();
    }

    private static Renderable MakeMovieTable(RichConsole console)
    {
        using Table table = new();
        table.Box = Box.Simple;
        table.ShowEdge = false;
        table.Expand = false;
        table.SetRowStyles(["", "dim"]);
        using Renderable date = Markup(console, "[green]Date");
        using Renderable title = Markup(console, "[blue]Title");
        using Renderable budget = Markup(console, "[cyan]Production Budget");
        using Renderable office = Markup(console, "[magenta]Box Office");
        table.AddColumnOptions(new ColumnOptions(MinimumWidth: 12, Style: "green", NoWrap: true), date);
        table.AddColumnOptions(new ColumnOptions(Ratio: 1, Style: "blue"), title);
        table.AddColumnOptions(new ColumnOptions(Justification: Justify.Right,
            MinimumWidth: 17, Style: "cyan", NoWrap: true), budget);
        table.AddColumnOptions(new ColumnOptions(Justification: Justify.Right,
            MinimumWidth: 14, Style: "magenta", NoWrap: true), office);
        string[][] movies =
        [
            ["Dec 20, 2019", "Star Wars: The Rise of Skywalker", "$275,000,000", "$375,126,118"],
            ["May 25, 2018", "[b]Solo[/]: A Star Wars Story", "$275,000,000", "$393,151,347"],
            ["Dec 15, 2017", "Star Wars Ep. VIII: The Last Jedi", "$262,000,000", "[bold]$1,332,539,889[/]"],
            ["May 19, 1999", "Star Wars Ep. [b]I[/b]: [i]The phantom Menace[/i]", "$115,000,000", "$1,027,044,677"],
        ];
        foreach (string[] movie in movies)
        {
            Renderable[] cells = new Renderable[movie.Length];
            try
            {
                for (int index = 0; index < movie.Length; index++)
                {
                    cells[index] = Markup(console, movie[index]);
                }
                table.AddRow(cells);
            }
            finally
            {
                DisposeAll(cells);
            }
        }
        return table.Finish();
    }

    private static Renderable MakeSyntaxPrettyComparison()
    {
        const string prettyData = "{'foo': [3.1427, ('Paul Atreides', 'Vladimir Harkonnen', " +
            "'Thufir Hawat')], 'atomic': (False, True, None)}";
        using Syntax syntax = new(Code, "python3");
        syntax.LineNumbers = true;
        syntax.SetIndentGuides(true);
        using Renderable syntaxRenderable = syntax.Finish();
        using Pretty pretty = new(prettyData);
        pretty.SetIndentGuides(true);
        using Renderable prettyRenderable = pretty.Finish();
        using Table comparison = new();
        comparison.Box = Box.None;
        comparison.ShowHeader = false;
        comparison.Expand = true;
        comparison.SetPadEdge(false);
        comparison.SetPadding(1, 1);
        comparison.AddColumnOptions(new ColumnOptions(Ratio: 1));
        comparison.AddColumnOptions(new ColumnOptions(Ratio: 1));
        comparison.AddRow(new[] { syntaxRenderable, prettyRenderable });
        return comparison.Finish();
    }

    private static Renderable MakeMarkdownComparison()
    {
        using Renderable raw = Styled(MarkdownSource, "cyan");
        using Markdown markdown = new(MarkdownSource);
        using Renderable rendered = markdown.Finish();
        using Table comparison = new();
        comparison.Box = Box.None;
        comparison.ShowHeader = false;
        comparison.Expand = true;
        comparison.SetPadEdge(false);
        comparison.AddColumnOptions(new ColumnOptions(Ratio: 1));
        comparison.AddColumnOptions(new ColumnOptions(Ratio: 1));
        comparison.AddRow(new[] { raw, rendered });
        return comparison.Finish();
    }

    private static Renderable MakeSponsorPanel(RichConsole console)
    {
        using Renderable body = Markup(console,
            "[b magenta]Hope you enjoy using Rich-rs![/]\n\n" +
            "Consider sponsoring the upstream port to ensure it's maintained.\n\n" +
            "[cyan]https://github.com/sponsors/willmcgugan[/cyan] | [b]Rust port: [/b]" +
            "[green]Marcos Saraiva - https://github.com/mrsaraiva/rich-rs[/green]");
        using Panel panel = new(body);
        panel.SetTitle("Help ensure Rich is maintained");
        panel.SetBorderStyle("green");
        panel.SetPadding(1, 2, 1, 2);
        return panel.Finish();
    }

    private static void AddFeature(Table card, string name, Renderable demo)
    {
        using (demo)
        using (Renderable label = Styled(name, ""))
        {
            card.AddRow(new[] { label, demo });
        }
    }

    private static void AddBlank(Table card)
    {
        using Renderable label = Styled("", "");
        using Renderable empty = Styled("", "");
        card.AddRow(new[] { label, empty });
    }

    private static Renderable Markup(RichConsole console, string value)
    {
        using Text text = Text.FromMarkup(console, value);
        return text.Finish();
    }

    private static Renderable Styled(string value, string style)
    {
        using Text text = new(value);
        if (style.Length != 0)
        {
            text.SetStyle(style);
        }
        return text.Finish();
    }

    private static void DisposeAll(Renderable[] values)
    {
        foreach (Renderable? value in values)
        {
            value?.Dispose();
        }
    }

    private static string Timing(double milliseconds, string cache) =>
        "[dim]rendered in [not dim][cyan]" + milliseconds.ToString("F1", CultureInfo.InvariantCulture) +
        "[/cyan]ms[/] (" + cache + " cache)";

    private static void RunInvalidStringProbe()
    {
        const string invalid = "\ud800";
        MustReject(() => _ = new Text(invalid));
        MustReject(() => _ = Style.Parse(invalid));
        MustReject(() => _ = new Syntax(invalid, "text"));
        MustReject(() => _ = new Syntax("x", invalid));
        MustReject(() => _ = Syntax.FromPath(invalid));
        MustReject(() => _ = new Pretty(invalid));
        MustReject(() => _ = new Markdown(invalid));
        MustReject(() => _ = new Json(invalid));
        MustReject(() => _ = new Tree(invalid));
        MustReject(() => _ = new Status(invalid));
        MustReject(() => _ = new Spinner(invalid));
        Console.Out.WriteLine("strict-before-native");
    }

    private static void MustReject(Action action)
    {
        try
        {
            action();
            throw new InvalidOperationException("Invalid UTF-16 was accepted.");
        }
        catch (EncoderFallbackException)
        {
        }
    }

    private sealed class ColorBox : ICustomRenderer
    {
        public void Render(RenderOptions options, SegmentSink sink)
        {
            for (uint y = 0; y < 5; y++)
            {
                for (uint x = 0; x < options.MaxWidth; x++)
                {
                    double hue = (double)x / options.MaxWidth;
                    double lowerLightness = 0.1 + ((double)y / 5.0 * 0.7);
                    Rgb background = HlsToRgb(hue, lowerLightness);
                    Rgb foreground = HlsToRgb(hue, lowerLightness + 0.7 / 10.0);
                    string style = string.Create(CultureInfo.InvariantCulture,
                        $"rgb({foreground.Red},{foreground.Green},{foreground.Blue}) on " +
                        $"rgb({background.Red},{background.Green},{background.Blue})");
                    sink.Push("▄", style);
                }
                sink.Line();
            }
        }

        public Measurement Measure(RenderOptions options) => new(1, options.MaxWidth);

        private static Rgb HlsToRgb(double hue, double lightness)
        {
            double q = lightness < 0.5 ? lightness * 2.0 : lightness + 1.0 - lightness;
            double p = 2.0 * lightness - q;
            return new((uint)(HueToRgb(p, q, hue + 1.0 / 3.0) * 255.0),
                (uint)(HueToRgb(p, q, hue) * 255.0),
                (uint)(HueToRgb(p, q, hue - 1.0 / 3.0) * 255.0));
        }

        private static double HueToRgb(double p, double q, double value)
        {
            if (value < 0.0) value += 1.0;
            if (value > 1.0) value -= 1.0;
            if (value < 1.0 / 6.0) return p + (q - p) * 6.0 * value;
            if (value < 0.5) return q;
            if (value < 2.0 / 3.0) return p + (q - p) * (2.0 / 3.0 - value) * 6.0;
            return p;
        }

        private readonly record struct Rgb(uint Red, uint Green, uint Blue);
    }
}
