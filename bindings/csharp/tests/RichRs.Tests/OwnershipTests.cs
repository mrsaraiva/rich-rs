using System.Runtime.CompilerServices;
using System.Text;
using System.Collections;

namespace RichRs.Tests;

public sealed class OwnershipTests
{
    [Fact]
    public void PublicFinishSuccessInvalidatesBuilder()
    {
        Text text = new("finished");
        using Renderable renderable = text.Finish();
        Assert.Throws<ObjectDisposedException>(() => text.SetStyle("blue"));
        Assert.Throws<ObjectDisposedException>(() => text.Finish());
        text.Dispose();
        text.Dispose();
    }

    [Fact]
    public unsafe void NativeFactoryOwnsEveryOutputAcrossExceptions()
    {
        List<nint> freed = [];
        TrackingHandle thrown = new(freed.Add);
        Assert.Throws<OutOfMemoryException>(() => NativeFactory.Initialize(thrown, output =>
        {
            *output = Output;
            throw new OutOfMemoryException("injected after native output");
        }));
        Assert.Equal([Output], freed);

        freed.Clear();
        TrackingHandle failed = new(freed.Add);
        RichException error = Assert.Throws<RichException>(() => NativeFactory.Initialize(failed, output =>
        {
            *output = Output;
            return 3;
        }));
        Assert.Equal(RichErrorCode.InvalidArgument, error.Code);
        Assert.Equal([Output], freed);

        freed.Clear();
        TrackingHandle success = new(freed.Add);
        NativeFactory.Initialize(success, output =>
        {
            *output = Output;
            return 0;
        });
        Assert.Empty(freed);
        success.Dispose();
        Assert.Equal([Output], freed);
    }

    [Fact]
    public unsafe void ManagedFinishReconcilesEveryPointerShapeWithoutLeaksOrDoubleFree()
    {
        AssertTransfer("failure preserves", 3, Original, 0, RichErrorCode.InvalidArgument, [Original], []);
        AssertTransfer("consumed on error", 6, 0, 0, RichErrorCode.Render, [], []);
        AssertTransfer("valid success", 0, 0, Output, null, [], [Output]);

        AssertTransfer("success kept without output", 0, Original, 0, RichErrorCode.AbiMismatch, [Original], []);
        AssertTransfer("success consumed without output", 0, 0, 0, RichErrorCode.AbiMismatch, [], []);
        AssertTransfer("success kept plus output", 0, Original, Output, RichErrorCode.AbiMismatch, [Original], [Output]);
        AssertTransfer("success replacement without output", 0, Replacement, 0, RichErrorCode.AbiMismatch,
            [Replacement, Original], []);
        AssertTransfer("success replacement plus output", 0, Replacement, Output, RichErrorCode.AbiMismatch,
            [Replacement, Original], [Output]);
        AssertTransfer("success replacement aliases output", 0, Replacement, Replacement, RichErrorCode.AbiMismatch,
            [Replacement, Original], []);
        AssertTransfer("success original aliases output", 0, Original, Original, RichErrorCode.AbiMismatch, [Original], []);

        AssertTransfer("error replacement aliases output", 5, Replacement, Replacement, RichErrorCode.Io,
            [Replacement, Original], []);
        AssertTransfer("error consumed returns original as output", 4, 0, Original, RichErrorCode.InvalidState,
            [], [Original]);
    }

    [Fact]
    public void EveryOwnerDisposesTwiceAndRejectsUseAfterDispose()
    {
        using Renderable content = TestSupport.TextRenderable("content");

        RichConsole console = new(); DisposeTwice(console);
        Assert.Throws<ObjectDisposedException>(() => _ = console.Width);
        Text text = new("text"); DisposeTwice(text);
        Style style = Style.Parse("bold"); DisposeTwice(style);
        Table table = new(); DisposeTwice(table);
        Panel panel = new(content); DisposeTwice(panel);
        Tree tree = new("root"); DisposeTwice(tree);
        Tree renderableTree = new(content); DisposeTwice(renderableTree);
        Rule rule = new(); DisposeTwice(rule);
        Columns columns = new(); DisposeTwice(columns);
        Align align = new(content, HorizontalAlignment.Left); DisposeTwice(align);
        Padding padding = new(content, 0, 0, 0, 0); DisposeTwice(padding);
        Syntax syntax = new("x", "text"); DisposeTwice(syntax);
        Markdown markdown = new("text"); DisposeTwice(markdown);
        Json json = new("{}"); DisposeTwice(json);
        Progress progress = new(); DisposeTwice(progress);
        Status status = new("status"); DisposeTwice(status);
        Spinner spinner = new("dots"); DisposeTwice(spinner);
        Renderable temporary = TestSupport.TextRenderable("renderable"); DisposeTwice(temporary);
    }

    [Fact]
    public void OneRenderableIsReusableAcrossEveryCompositionFamily()
    {
        using RichConsole console = TestSupport.PlainConsole();
        using Renderable reusable = TestSupport.TextRenderable("reused", "bold cyan");

        using Table table = new();
        table.AddColumn(reusable);
        table.AddRow(new[] { reusable });
        using Renderable tableOutput = table.Finish();

        using Panel panel = new(reusable);
        using Renderable panelOutput = panel.Finish();

        using Tree tree = new(reusable);
        _ = tree.Add(NodeId.Root, reusable);
        using Renderable treeOutput = tree.Finish();

        using Columns columns = new();
        columns.Add(reusable);
        columns.Add(reusable);
        using Renderable columnsOutput = columns.Finish();

        using Align align = new(reusable, HorizontalAlignment.Center);
        using Renderable alignOutput = align.Finish();

        using Padding padding = new(reusable, 1, 1, 1, 1);
        using Renderable paddingOutput = padding.Finish();

        foreach (Renderable item in new[] { reusable, tableOutput, panelOutput, treeOutput, columnsOutput, alignOutput, paddingOutput })
        {
            string output = console.Render(item);
            Assert.Contains("reused", output, StringComparison.Ordinal);
            Assert.True(Utf8.Strict.GetByteCount(output) >= output.Length);
        }
    }

    [Fact]
    public void TemporaryArraysValidateAtomicallyBeforeNativeMutation()
    {
        using RichConsole console = TestSupport.PlainConsole();
        using Renderable valid = TestSupport.TextRenderable("valid");
        using Table table = new();
        table.AddColumn("only");

        Assert.Throws<EncoderFallbackException>(() => table.AddRow(new[] { "bad\ud800" }));
        Renderable[] withNull = [valid, null!];
        Assert.Throws<ArgumentNullException>(() => table.AddRow(withNull));
        table.AddRow(["after failures"]);
        using Renderable output = table.Finish();
        string rendered = console.Render(output);
        Assert.Contains("after failures", rendered, StringComparison.Ordinal);
    }

    [Fact]
    public void UnstableListsAreSnapshottedWithoutCountOrIndexerRaces()
    {
        using RichConsole console = TestSupport.PlainConsole();
        using Renderable first = TestSupport.TextRenderable("first");
        using Renderable second = TestSupport.TextRenderable("second");
        using Table table = new();
        table.AddColumn("left");
        table.AddColumn("right");

        UnstableReadOnlyList<string> strings = new("one", "two");
        table.AddRow(strings);
        Assert.Equal(0, strings.CountReads);
        Assert.Equal(0, strings.IndexReads);

        UnstableReadOnlyList<Renderable> renderables = new(first, second);
        table.AddRow(renderables);
        Assert.Equal(0, renderables.CountReads);
        Assert.Equal(0, renderables.IndexReads);

        UnstableReadOnlyList<Renderable> invalid = new(first, null!);
        Assert.Throws<ArgumentNullException>(() => table.AddRow(invalid));
        Assert.Equal(0, invalid.CountReads);
        Assert.Equal(0, invalid.IndexReads);

        table.AddRow(["after", "validation"]);
        using Renderable output = table.Finish();
        string rendered = console.Render(output);
        Assert.Contains("one", rendered, StringComparison.Ordinal);
        Assert.Contains("second", rendered, StringComparison.Ordinal);
        Assert.Contains("validation", rendered, StringComparison.Ordinal);
    }

    [Fact]
    public async Task ConcurrentDisposeCannotReleasePointerDuringNativeCall()
    {
        RichConsole console = TestSupport.PlainConsole();
        Exception? unexpected = null;
        Task caller = Task.Run(() =>
        {
            for (int i = 0; i < 2_000; i++)
            {
                try
                {
                    _ = console.RenderMarkup("race");
                }
                catch (ObjectDisposedException)
                {
                    return;
                }
                catch (Exception error)
                {
                    unexpected = error;
                    return;
                }
            }
        }, TestContext.Current.CancellationToken);
        Task disposer = Task.Run(() => console.Dispose(), TestContext.Current.CancellationToken);
        await Task.WhenAll(caller, disposer);
        Assert.Null(unexpected);
        console.Dispose();
    }

    [Fact]
    public void SafeHandleFinalizerIsAFallback()
    {
        WeakReference reference = CreateUndisposedOwner();
        for (int i = 0; i < 10 && reference.IsAlive; i++)
        {
            GC.Collect();
            GC.WaitForPendingFinalizers();
            GC.Collect();
        }
        Assert.False(reference.IsAlive);
    }

    [MethodImpl(MethodImplOptions.NoInlining)]
    private static WeakReference CreateUndisposedOwner()
    {
        Text owner = new("finalizer");
        return new WeakReference(owner);
    }

    private static void DisposeTwice(IDisposable value)
    {
        value.Dispose();
        value.Dispose();
    }

    private static unsafe void AssertTransfer(
        string name,
        uint result,
        nint inputAfter,
        nint outputAfter,
        RichErrorCode? expectedError,
        nint[] expectedBuilderFrees,
        nint[] expectedOutputFrees)
    {
        List<nint> builderFrees = [];
        List<nint> outputFrees = [];
        TrackingHandle builderHandle = new(builderFrees.Add, Original);
        TrackingHandle outputHandle = new(outputFrees.Add);
        using NativeOwner<TrackingHandle> owner = new(builderHandle);

        RichException? observed = null;
        try
        {
            owner.Finish((input, output) =>
            {
                *input = inputAfter;
                *output = outputAfter;
                return result;
            }, outputHandle);
        }
        catch (RichException error)
        {
            observed = error;
        }

        if (expectedError is null)
        {
            Assert.Null(observed);
        }
        else
        {
            Assert.NotNull(observed);
            Assert.Equal(expectedError.Value, observed.Code);
        }

        owner.Dispose();
        outputHandle.Dispose();
        Assert.Equal(expectedBuilderFrees, builderFrees.ToArray());
        Assert.Equal(expectedOutputFrees, outputFrees.ToArray());
        Assert.Equal(builderFrees.Count, builderFrees.Distinct().Count());
        Assert.Equal(outputFrees.Count, outputFrees.Distinct().Count());
        Assert.True(builderFrees.Intersect(outputFrees).Count() <= (name.Contains("original as output", StringComparison.Ordinal) ? 1 : 0), name);
    }

    private sealed class TrackingHandle : NativeSafeHandle
    {
        private readonly Action<nint> free;

        internal TrackingHandle(Action<nint> free, nint pointer = 0)
        {
            this.free = free;
            if (pointer != 0)
            {
                Initialize(pointer);
            }
        }

        protected override void Free(nint value) => free(value);
    }

    private sealed class UnstableReadOnlyList<T>(params T[] values) : IReadOnlyList<T>
    {
        public int Count
        {
            get
            {
                CountReads++;
                return CountReads == 1 ? values.Length : checked(values.Length + 1_000_000);
            }
        }

        public T this[int index]
        {
            get
            {
                IndexReads++;
                return IndexReads == 1 ? values[index] : throw new InvalidOperationException("unstable indexer was read repeatedly");
            }
        }

        internal int CountReads { get; private set; }
        internal int IndexReads { get; private set; }

        public IEnumerator<T> GetEnumerator() => ((IEnumerable<T>)values).GetEnumerator();
        IEnumerator IEnumerable.GetEnumerator() => values.GetEnumerator();
    }

    private static readonly nint Original = (nint)0x1000;
    private static readonly nint Replacement = (nint)0x2000;
    private static readonly nint Output = (nint)0x3000;
}
