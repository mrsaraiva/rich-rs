using System.Runtime.CompilerServices;
using System.Text;

namespace RichRs.Tests;

public sealed class AbiV2SurfaceTests
{
    [Fact]
    public void PrettyAndSyntaxGuidesAreNativeAndReusable()
    {
        using RichConsole console = TestSupport.PlainConsole();
        console.SetSize(18, 24);
        const string representation = "{'ключ': [\n    123456789,\n    {'nul': '\0', 'more': 987654321}\n]}";

        using Pretty plain = new(representation);
        plain.SetIndentGuides(false);
        using Renderable plainOutput = plain.Finish();
        string withoutGuides = console.Render(plainOutput);

        using Pretty guided = new(representation);
        guided.SetIndentGuides(true);
        using Renderable guidedOutput = guided.Finish();
        string withGuides = console.Render(guidedOutput);
        Assert.Contains("ключ", withoutGuides, StringComparison.Ordinal);
        Assert.Contains("ключ", withGuides, StringComparison.Ordinal);
        Assert.NotEqual(withoutGuides, withGuides);
        Assert.Throws<ObjectDisposedException>(() => guided.SetIndentGuides(false));

        const string secondRepresentation = "[('second', {'deep': [123456, 789012, 345678]})]";
        string secondWithout = RenderPretty(secondRepresentation, false);
        string secondWith = RenderPretty(secondRepresentation, true);
        Assert.Contains("second", secondWithout, StringComparison.Ordinal);
        Assert.NotEqual(secondWithout, secondWith);

        const string code = "if True:\n    if False:\n        pass\n";
        using Syntax syntaxPlain = new(code, "python");
        syntaxPlain.SetIndentGuides(false);
        using Renderable syntaxPlainOutput = syntaxPlain.Finish();
        using Syntax syntaxGuided = new(code, "python");
        syntaxGuided.SetIndentGuides(true);
        using Renderable syntaxGuidedOutput = syntaxGuided.Finish();
        Assert.NotEqual(console.Render(syntaxPlainOutput), console.Render(syntaxGuidedOutput));

        string RenderPretty(string value, bool indentGuides)
        {
            using Pretty pretty = new(value);
            pretty.SetIndentGuides(indentGuides);
            using Renderable output = pretty.Finish();
            return console.Render(output);
        }
    }

    [Fact]
    public void ConfiguredTableSurfaceExercisesEveryPropertyAndBorrow()
    {
        using RichConsole console = TestSupport.PlainConsole();
        console.SetSize(54, 24);
        using Renderable header = TestSupport.TextRenderable("borrowed header", "bold red");
        using Table table = Table.CreateGrid();
        table.ShowHeader = true;
        table.Box = Box.None;
        table.SetPadEdge(false);
        table.SetRowStyles(["", "dim"]);
        table.AddColumnOptions(new ColumnOptions(
            Justification: Justify.Right,
            Vertical: VerticalAlignment.Middle,
            Width: 18,
            MinimumWidth: 4,
            Ratio: 2,
            Style: "green",
            NoWrap: true), header);
        table.AddColumnOptions(new ColumnOptions(Ratio: 1, Style: "cyan"));
        table.AddRow(["a long value that must not wrap", "second"]);
        table.AddRow(["dim row", "tail"]);
        using Renderable output = table.Finish();

        string rendered = console.Render(output);
        Assert.Contains("borrowed header", rendered, StringComparison.Ordinal);
        Assert.Contains("second", rendered, StringComparison.Ordinal);
        Assert.Contains("borrowed header", console.Render(header), StringComparison.Ordinal);
        Assert.Throws<ObjectDisposedException>(() => table.SetPadEdge(true));
    }

    [Fact]
    public void ColumnValidationIsNativeAuthoritativeAndAtomic()
    {
        using RichConsole console = TestSupport.PlainConsole();
        using Table table = new();
        AssertInvalid(new ColumnOptions(Width: 0));
        AssertInvalid(new ColumnOptions(MinimumWidth: 0));
        AssertInvalid(new ColumnOptions(Ratio: 0));
        AssertInvalid(new ColumnOptions(Justification: (Justify)999));
        AssertInvalid(new ColumnOptions(Vertical: (VerticalAlignment)999));
        AssertInvalid(new ColumnOptions(Style: "not-a-style"));
        Assert.Throws<ArgumentNullException>(() => table.AddColumnOptions(new ColumnOptions(Style: null!)));
        Assert.Throws<EncoderFallbackException>(() => table.AddColumnOptions(new ColumnOptions(Style: "\ud800")));

        table.SetRowStyles(["", "dim"]);
        Assert.Throws<RichException>(() => table.SetRowStyles(["dim", "not-a-style"]));
        table.SetRowStyles([]);
        table.AddColumn("only");
        table.AddRow(["unchanged"]);
        using Renderable output = table.Finish();
        Assert.Contains("unchanged", console.Render(output), StringComparison.Ordinal);

        void AssertInvalid(ColumnOptions options)
        {
            RichException error = Assert.Throws<RichException>(() => table.AddColumnOptions(options));
            Assert.Equal(RichErrorCode.InvalidArgument, error.Code);
        }
    }

    [Fact]
    public void NoneBoxIsTableOnly()
    {
        using Table table = new();
        table.Box = Box.None;
        using Renderable content = TestSupport.TextRenderable("panel");
        using Panel panel = new(content);
        RichException error = Assert.Throws<RichException>(() => panel.Box = Box.None);
        Assert.Equal(RichErrorCode.InvalidArgument, error.Code);
    }

    [Fact]
    public void EveryConfiguredTableFeatureChangesNativeOutputIndependently()
    {
        using RichConsole console = TestSupport.PlainConsole();
        console.ForceTerminal = true;
        console.SetSize(42, 24);
        Assert.NotEqual(RenderTable(console), RenderTable(console, ratio: 3));
        Assert.NotEqual(RenderTable(console), RenderTable(console, ratio: null, width: 9));
        Assert.NotEqual(RenderTable(console), RenderTable(console, ratio: null, minimumWidth: 25));
        Assert.NotEqual(RenderTable(console), RenderTable(console, justification: Justify.Right));
        Assert.NotEqual(RenderTable(console, ratio: null, width: 9),
            RenderTable(console, ratio: null, width: 9, noWrap: true));
        Assert.NotEqual(RenderTable(console), RenderTable(console, padEdge: false));
        Assert.NotEqual(RenderTable(console), RenderTable(console, rowStyles: true));
        Assert.NotEqual(RenderTable(console), RenderTable(console, noBox: true));
    }

    [Fact]
    public void CustomRendererReceivesOptionsAndTombstonesSink()
    {
        using RichConsole console = TestSupport.PlainConsole();
        console.ForceTerminal = true;
        console.ColorSystem = ColorSystem.TrueColor;
        console.SetSize(11, 20);
        RecordingRenderer renderer = new(3);
        using Renderable custom = CustomRenderable.Create(renderer);
        int callerThread = Environment.CurrentManagedThreadId;
        string output = console.Render(custom);

        Assert.Equal(callerThread, renderer.RenderThread);
        Assert.Equal(0, renderer.MeasureThread);
        Assert.Equal(11U, renderer.LastOptions.MaxWidth);
        Assert.Equal(20U, renderer.LastOptions.MaxHeight);
        Assert.Equal(ColorSystem.TrueColor, renderer.LastOptions.ColorSystem);
        Assert.False(renderer.LastOptions.IsTerminal);
        Assert.Equal(33, Count(output, "▄"));
        Assert.Contains("38;2;", output, StringComparison.Ordinal);
        Assert.NotNull(renderer.RetainedSink);
        Assert.Throws<InvalidOperationException>(() => renderer.RetainedSink.Line());

        using Table table = Table.CreateGrid();
        table.AddColumn("custom");
        table.AddRow(new[] { custom });
        using Renderable composed = table.Finish();
        _ = console.Render(composed);
        Assert.Equal(callerThread, renderer.MeasureThread);
    }

    [Theory]
    [InlineData(7, 1)]
    [InlineData(19, 3)]
    public void CustomRendererAdaptsToTwoWidthsAndHeights(int width, int height)
    {
        using RichConsole console = TestSupport.PlainConsole();
        console.ForceTerminal = true;
        console.ColorSystem = ColorSystem.TrueColor;
        console.SetSize((uint)width, 20);
        RecordingRenderer renderer = new(height);
        using Renderable custom = CustomRenderable.Create(renderer);
        string output = console.Render(custom);
        Assert.Equal(width * height, Count(output, "▄"));
        Assert.Equal((uint)width, renderer.LastOptions.MaxWidth);
    }

    [Fact]
    public void CallbackExceptionsAreContainedAndRethrownAfterNativeRender()
    {
        using RichConsole console = TestSupport.PlainConsole();
        using Renderable custom = CustomRenderable.Create(new ThrowingRenderer());
        CallbackTestException error = Assert.Throws<CallbackTestException>(() => console.Render(custom));
        Assert.Equal("contained", error.Message);
        Assert.Equal("after", console.RenderMarkup("after"));
    }

    [Fact]
    public unsafe void FailedCallbackPublishesNoPartialNativeBuffer()
    {
        using RichConsole console = TestSupport.PlainConsole();
        using Renderable custom = CustomRenderable.Create(new PartialThenThrowRenderer());
        NativeRichBuffer buffer = default;
        nint bufferPointer = (nint)(&buffer);
        CallbackGuard.BeginOuterRender();
        try
        {
            uint result = console.Borrow(consolePointer => custom.Borrow(renderablePointer =>
                NativeMethods.ConsoleRender(consolePointer, renderablePointer, (NativeRichBuffer*)bufferPointer)));
            Assert.Equal((uint)RichErrorCode.Render, result);
            Assert.Equal(0, (nint)buffer.Data);
            Assert.Equal(0UL, buffer.Length);
        }
        finally
        {
            NativeMethods.BufferFree(&buffer);
            CallbackGuard.EndOuterRender();
        }
    }

    [Fact]
    public void MalformedMeasurementBecomesNativeRenderError()
    {
        using RichConsole console = TestSupport.PlainConsole();
        using Renderable custom = CustomRenderable.Create(new MalformedMeasurementRenderer());
        using Table table = Table.CreateGrid();
        table.AddColumn("custom");
        table.AddRow(new[] { custom });
        using Renderable composed = table.Finish();
        RichException error = Assert.Throws<RichException>(() => console.Render(composed));
        Assert.Equal(RichErrorCode.Render, error.Code);
    }

    [Fact]
    public void SinkRejectsWrongThreadAndStrictEncodingWithoutNativeUse()
    {
        using RichConsole console = TestSupport.PlainConsole();
        WrongThreadRenderer renderer = new();
        using Renderable custom = CustomRenderable.Create(renderer);
        Assert.Contains("valid", console.Render(custom), StringComparison.Ordinal);
        Assert.NotEqual(renderer.CallbackThread, renderer.WrongThreadThread);
        Assert.IsType<InvalidOperationException>(renderer.WrongThreadError);
        Assert.True(renderer.EncodingRejected);
    }

    [Fact]
    public void CallbackReentryAndDisposeAreRejectedLocallyWithoutClosingOwners()
    {
        using RichConsole console = TestSupport.PlainConsole();
        Text text = new("still owned");
        ReentryRenderer renderer = new(console, text);
        using Renderable custom = CustomRenderable.Create(renderer);
        Assert.Contains("ok", console.Render(custom), StringComparison.Ordinal);
        Assert.True(renderer.RenderRejected);
        Assert.True(renderer.DisposeRejected);
        text.SetStyle("blue");
        text.Dispose();
    }

    [Fact]
    public async Task ConcurrentRenderAndDisposeAreSynchronized()
    {
        using RichConsole console = TestSupport.PlainConsole();
        BlockingRenderer renderer = new();
        Renderable custom = CustomRenderable.Create(renderer);
        Task<string> render = Task.Run(() => console.Render(custom), TestContext.Current.CancellationToken);
        Assert.True(renderer.Entered.Wait(TimeSpan.FromSeconds(5), TestContext.Current.CancellationToken));
        Task dispose = Task.Run(custom.Dispose, TestContext.Current.CancellationToken);
        Assert.False(dispose.IsCompleted);
        renderer.Release.Set();
        Assert.Contains("done", await render);
        await dispose;
        Assert.Throws<ObjectDisposedException>(() => console.Render(custom));
    }

    [Fact]
    public async Task NativeSerializesConcurrentCallbacksForOneAdapter()
    {
        using RichConsole first = TestSupport.PlainConsole();
        using RichConsole second = TestSupport.PlainConsole();
        SerializedRenderer renderer = new();
        using Renderable custom = CustomRenderable.Create(renderer);
        Task<string> one = Task.Run(() => first.Render(custom), TestContext.Current.CancellationToken);
        Task<string> two = Task.Run(() => second.Render(custom), TestContext.Current.CancellationToken);
        await Task.WhenAll(one, two);
        Assert.Equal(1, renderer.MaximumActive);
        Assert.Equal(2, renderer.RenderCount);
    }

    [Fact]
    public void RendererContextSurvivesRetainedArcAndDiesAfterLastArc()
    {
        WeakReference renderer = CreateRetainedRenderer(out Renderable retained);
        Collect();
        Assert.True(renderer.IsAlive);
        using RichConsole console = TestSupport.PlainConsole();
        Assert.Contains("retained", console.Render(retained), StringComparison.Ordinal);
        retained.Dispose();
        Collect();
        Assert.False(renderer.IsAlive);
    }

    [Fact]
    public void ManagedFailureBeforeNativeConstructionReleasesGcHandle()
    {
        WeakReference renderer = CreateFailedRenderer();
        Collect();
        Assert.False(renderer.IsAlive);
    }

    [Fact]
    public void ManagedFailureAfterNativeSuccessReleasesRenderableAndGcHandle()
    {
        WeakReference renderer = CreatePostSuccessFailedRenderer();
        Collect();
        Assert.False(renderer.IsAlive);
    }

    private static int Count(string source, string value)
    {
        int count = 0;
        int offset = 0;
        while ((offset = source.IndexOf(value, offset, StringComparison.Ordinal)) >= 0)
        {
            count++;
            offset += value.Length;
        }
        return count;
    }

    private static string RenderTable(
        RichConsole console,
        uint? ratio = 1,
        uint? width = null,
        uint? minimumWidth = null,
        Justify justification = Justify.Left,
        bool noWrap = false,
        bool padEdge = true,
        bool rowStyles = false,
        bool noBox = false)
    {
        using Table table = new();
        table.Box = noBox ? Box.None : Box.Simple;
        table.Expand = true;
        table.SetPadEdge(padEdge);
        if (rowStyles)
        {
            table.SetRowStyles(["", "dim"]);
        }
        table.AddColumnOptions(new ColumnOptions(Justification: justification, Width: width,
            MinimumWidth: minimumWidth, Ratio: ratio, NoWrap: noWrap));
        table.AddColumnOptions(new ColumnOptions(Ratio: 1));
        table.AddRow(["alpha beta gamma delta", "right"]);
        table.AddRow(["second row", "tail"]);
        using Renderable output = table.Finish();
        return console.Render(output);
    }

    [MethodImpl(MethodImplOptions.NoInlining)]
    private static WeakReference CreateRetainedRenderer(out Renderable retained)
    {
        RecordingRenderer renderer = new(1, "retained");
        WeakReference weak = new(renderer);
        using Renderable custom = CustomRenderable.Create(renderer);
        using Columns columns = new();
        columns.Add(custom);
        retained = columns.Finish();
        return weak;
    }

    [MethodImpl(MethodImplOptions.NoInlining)]
    private static WeakReference CreateFailedRenderer()
    {
        RecordingRenderer renderer = new(1);
        WeakReference weak = new(renderer);
        CustomRenderable.BeforeNativeCreateForTest = () => throw new OutOfMemoryException("injected");
        try
        {
            Assert.Throws<OutOfMemoryException>(() => CustomRenderable.Create(renderer));
        }
        finally
        {
            CustomRenderable.BeforeNativeCreateForTest = null;
        }
        return weak;
    }

    [MethodImpl(MethodImplOptions.NoInlining)]
    private static WeakReference CreatePostSuccessFailedRenderer()
    {
        RecordingRenderer renderer = new(1);
        WeakReference weak = new(renderer);
        Renderable.AfterNativeCreateForTest = () => throw new OutOfMemoryException("injected after success");
        try
        {
            Assert.Throws<OutOfMemoryException>(() => CustomRenderable.Create(renderer));
        }
        finally
        {
            Renderable.AfterNativeCreateForTest = null;
        }
        return weak;
    }

    private static void Collect()
    {
        for (int i = 0; i < 10; i++)
        {
            GC.Collect();
            GC.WaitForPendingFinalizers();
            GC.Collect();
        }
    }

    private sealed class RecordingRenderer(int height, string text = "▄") : ICustomRenderer
    {
        internal SegmentSink? RetainedSink { get; private set; }
        internal RenderOptions LastOptions { get; private set; }
        internal int RenderThread { get; private set; }
        internal int MeasureThread { get; private set; }

        public void Render(RenderOptions options, SegmentSink sink)
        {
            LastOptions = options;
            RenderThread = Environment.CurrentManagedThreadId;
            RetainedSink = sink;
            for (int y = 0; y < height; y++)
            {
                for (uint x = 0; x < options.MaxWidth; x++)
                {
                    sink.Push(text, "rgb(1,2,3) on rgb(4,5,6)");
                }
                sink.Line();
            }
        }

        public Measurement Measure(RenderOptions options)
        {
            MeasureThread = Environment.CurrentManagedThreadId;
            return new(1, options.MaxWidth);
        }
    }

    private sealed class ThrowingRenderer : ICustomRenderer
    {
        public void Render(RenderOptions options, SegmentSink sink) => throw new CallbackTestException("contained");
    }

    private sealed class PartialThenThrowRenderer : ICustomRenderer
    {
        public void Render(RenderOptions options, SegmentSink sink)
        {
            sink.Push("must not escape", "red");
            throw new CallbackTestException("after staging");
        }
    }

    private sealed class CallbackTestException(string message) : Exception(message);

    private sealed class MalformedMeasurementRenderer : ICustomRenderer
    {
        public void Render(RenderOptions options, SegmentSink sink) => sink.Push("not rendered");
        public Measurement Measure(RenderOptions options) => new(2, 1);
    }

    private sealed class WrongThreadRenderer : ICustomRenderer
    {
        internal Exception? WrongThreadError { get; private set; }
        internal bool EncodingRejected { get; private set; }
        internal int CallbackThread { get; private set; }
        internal int WrongThreadThread { get; private set; }

        public void Render(RenderOptions options, SegmentSink sink)
        {
            CallbackThread = Environment.CurrentManagedThreadId;
            Thread wrongThread = new(() =>
            {
                WrongThreadThread = Environment.CurrentManagedThreadId;
                try
                {
                    sink.Line();
                }
                catch (Exception error)
                {
                    WrongThreadError = error;
                }
            });
            wrongThread.Start();
            wrongThread.Join();
            EncodingRejected = Assert.Throws<EncoderFallbackException>(() => sink.Push("\ud800")).Message.Length > 0;
            sink.Push("valid");
            sink.Line();
        }
    }

    private sealed class ReentryRenderer(RichConsole console, Text text) : ICustomRenderer
    {
        internal bool RenderRejected { get; private set; }
        internal bool DisposeRejected { get; private set; }

        public void Render(RenderOptions options, SegmentSink sink)
        {
            RenderRejected = Assert.Throws<InvalidOperationException>(() => console.RenderMarkup("no")).Message.Length > 0;
            DisposeRejected = Assert.Throws<InvalidOperationException>(text.Dispose).Message.Length > 0;
            sink.Push("ok");
            sink.Line();
        }
    }

    private sealed class BlockingRenderer : ICustomRenderer
    {
        internal ManualResetEventSlim Entered { get; } = new();
        internal ManualResetEventSlim Release { get; } = new();

        public void Render(RenderOptions options, SegmentSink sink)
        {
            Entered.Set();
            Assert.True(Release.Wait(TimeSpan.FromSeconds(5)));
            sink.Push("done");
            sink.Line();
        }
    }

    private sealed class SerializedRenderer : ICustomRenderer
    {
        private int active;
        private int renderCount;
        internal int MaximumActive { get; private set; }
        internal int RenderCount => renderCount;

        public void Render(RenderOptions options, SegmentSink sink)
        {
            int now = Interlocked.Increment(ref active);
            MaximumActive = Math.Max(MaximumActive, now);
            Interlocked.Increment(ref renderCount);
            Thread.SpinWait(200_000);
            sink.Push("serialized");
            sink.Line();
            Interlocked.Decrement(ref active);
        }
    }
}
