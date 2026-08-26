namespace RichRs.Tests;

public sealed class SurfaceTests
{
    [Fact]
    public void PlainAndTerminalRenderingHonorAnsiAndOscPolicy()
    {
        using RichConsole console = TestSupport.PlainConsole();
        string plain = console.RenderMarkup("[bold link=https://example.com]hello[/]");
        Assert.DoesNotContain("\u001b[", plain, StringComparison.Ordinal);
        Assert.DoesNotContain("\u001b]", plain, StringComparison.Ordinal);

        console.ForceTerminal = true;
        string terminal = console.RenderMarkup("[bold]hello[/]");
        Assert.Contains("\u001b[", terminal, StringComparison.Ordinal);
        Assert.Contains("hello", terminal, StringComparison.Ordinal);
    }

    [Fact]
    public void TablePanelAndTreeExposeCompleteSurfaceAndStableIds()
    {
        using RichConsole console = TestSupport.PlainConsole();
        using Renderable header = TestSupport.TextRenderable("renderable header", "bold cyan");
        using Renderable cell = TestSupport.TextRenderable("renderable cell", "green");

        using Table table = new();
        table.SetTitle("title");
        table.SetCaption("caption");
        table.SetTitleMarkup(console, "[bold]markup title[/]");
        table.SetCaptionMarkup(console, "[italic]markup caption[/]");
        table.Box = Box.Square;
        table.ShowHeader = true;
        table.ShowLines = true;
        table.ShowEdge = true;
        table.Expand = false;
        table.SetPadding(1, 1);
        table.SetStyle("white");
        table.AddColumn("plain header");
        table.AddColumn(header);
        table.AddRow(["alpha", "βeta"]);
        table.AddRow(new[] { cell, cell });
        using Renderable tableRenderable = table.Finish();
        string tableOutput = console.Render(tableRenderable);
        Assert.Contains("alpha", tableOutput, StringComparison.Ordinal);

        using Panel panel = new(tableRenderable);
        panel.SetTitle("Panel");
        panel.SetSubtitle("subtitle");
        panel.Box = Box.Rounded;
        panel.Expand = false;
        panel.Width = 50;
        panel.SetPadding(0, 1, 0, 1);
        panel.SetStyle("white");
        panel.SetBorderStyle("blue");
        using Renderable panelRenderable = panel.Finish();
        Assert.Contains("Panel", console.Render(panelRenderable), StringComparison.Ordinal);

        using Tree tree = new("root");
        NodeId first = tree.Add(NodeId.Root, "first");
        for (int i = 0; i < 100; i++)
        {
            _ = tree.Add(NodeId.Root, $"sibling {i}");
        }
        NodeId deep = first;
        for (int i = 0; i < 40; i++)
        {
            deep = tree.Add(deep, $"deep {i}");
        }
        _ = tree.Add(first, cell);
        tree.SetStyle("white");
        tree.SetGuideStyle("blue");
        tree.HideRoot = false;
        RichException unknown = Assert.Throws<RichException>(() => tree.Add(new NodeId(999_999), "unknown"));
        Assert.Equal(RichErrorCode.InvalidState, unknown.Code);
        _ = tree.Add(first, "still valid");
        using Renderable treeRenderable = tree.Finish();
        string treeOutput = console.Render(treeRenderable);
        Assert.Contains("deep 39", treeOutput, StringComparison.Ordinal);
        Assert.Throws<ObjectDisposedException>(() => tree.Add(NodeId.Root, "finished"));
    }

    [Fact]
    public void LayoutFamiliesExposeEveryOperation()
    {
        using RichConsole console = TestSupport.PlainConsole();
        using Renderable content = TestSupport.TextRenderable("layout content", "bold magenta");

        using Rule rule = new();
        rule.SetTitle("Rule");
        rule.SetCharacters("-");
        rule.SetStyle("blue");
        rule.Alignment = HorizontalAlignment.Center;
        using Renderable ruleRenderable = rule.Finish();

        using Columns columns = new();
        columns.Add(content);
        columns.Add("plain");
        columns.Equal = true;
        columns.Expand = false;
        columns.SetPadding(0, 1);
        using Renderable columnsRenderable = columns.Finish();

        using Align align = new(content, HorizontalAlignment.Right);
        align.Vertical = VerticalAlignment.Middle;
        align.Width = 30;
        using Renderable aligned = align.Finish();

        using Align centeredBuilder = Align.Center(content);
        using Renderable centered = centeredBuilder.Finish();

        using Padding padding = new(content, 1, 2, 1, 2);
        padding.SetStyle("on blue");
        padding.Expand = false;
        using Renderable padded = padding.Finish();

        foreach (Renderable item in new[] { ruleRenderable, columnsRenderable, aligned, centered, padded })
        {
            string output = console.Render(item);
            Assert.NotEmpty(output);
            _ = Utf8.Strict.GetBytes(output);
        }
    }

    [Fact]
    public void ContentFamiliesExposeEveryOperationAndErrors()
    {
        using RichConsole console = TestSupport.PlainConsole();
        using Syntax syntax = new("const answer: u32 = 42;", "rust");
        syntax.SetTheme("monokai");
        syntax.LineNumbers = true;
        syntax.WordWrap = true;
        syntax.SetLineRange(1, null);
        using Renderable syntaxRenderable = syntax.Finish();

        string temporary = Path.GetTempFileName();
        File.WriteAllText(temporary, "fn main() {}\n");
        try
        {
            using Syntax fromPath = Syntax.FromPath(temporary);
            fromPath.SetLineRange(null, 1);
            using Renderable pathRenderable = fromPath.Finish();
            Assert.NotEmpty(console.Render(pathRenderable));
        }
        finally
        {
            File.Delete(temporary);
        }
        Assert.Equal(RichErrorCode.Io,
            Assert.Throws<RichException>(() => Syntax.FromPath("/definitely/not/a/rich-rs-source-file")).Code);

        using Markdown markdown = new("# Header\n\n`code`");
        markdown.SetCodeTheme("monokai");
        markdown.Hyperlinks = false;
        markdown.Justification = Justify.Left;
        using Renderable markdownRenderable = markdown.Finish();

        using Json json = new("{\"b\":1,\"a\":2}", new JsonOptions(SortKeys: true));
        using Renderable jsonRenderable = json.Finish();
        Assert.Equal(RichErrorCode.InvalidArgument, Assert.Throws<RichException>(() => new Json("{broken")).Code);

        foreach (Renderable item in new[] { syntaxRenderable, markdownRenderable, jsonRenderable })
        {
            string output = console.Render(item);
            Assert.NotEmpty(output);
            _ = Utf8.Strict.GetBytes(output);
        }
    }

    [Fact]
    public void LiveFamiliesAreCallerDrivenAndValidateNativeDomains()
    {
        using RichConsole console = TestSupport.PlainConsole();
        using Progress progress = new();
        ulong taskId = progress.AddTask("work", 100);
        Assert.Equal(0UL, taskId);
        progress.Update(taskId, 50);
        Assert.Equal(RichErrorCode.InvalidState,
            Assert.Throws<RichException>(() => progress.Update(999_999, 1)).Code);
        Assert.NotEmpty(progress.RenderFrame(console));

        using Status status = new("working");
        Assert.NotEmpty(status.RenderFrame(console));

        using Spinner spinner = new("dots");
        Assert.NotEmpty(spinner.RenderFrame(console, 0.25));
        Assert.Equal(RichErrorCode.InvalidArgument,
            Assert.Throws<RichException>(() => spinner.RenderFrame(console, double.NaN)).Code);
        Assert.Equal(RichErrorCode.InvalidArgument,
            Assert.Throws<RichException>(() => new Spinner("not-a-spinner")).Code);
        Assert.Equal(RichErrorCode.InvalidArgument,
            Assert.Throws<RichException>(() => progress.AddTask("bad", double.PositiveInfinity)).Code);
    }

    [Fact]
    public void StyleAndNativeEnumValidationRemainNativeAuthoritative()
    {
        using Style style = Style.Parse("bold red");
        Assert.Equal(RichErrorCode.InvalidArgument,
            Assert.Throws<RichException>(() => Style.Parse("not-a-real-style-token")).Code);
        using RichConsole console = TestSupport.PlainConsole();
        Assert.Equal(RichErrorCode.InvalidArgument,
            Assert.Throws<RichException>(() => console.ColorSystem = (ColorSystem)999).Code);
    }
}
