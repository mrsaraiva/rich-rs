namespace RichRs;

/// <summary>A tree builder addressed by stable scalar node IDs.</summary>
public sealed unsafe class Tree : IDisposable
{
    /// <summary>The root node ID.</summary>
    public const ulong RootNodeId = 0;
    private readonly NativeOwner<TreeHandle> owner;

    /// <summary>Creates a tree with a plain root label.</summary>
    public unsafe Tree(string label)
    {
        byte[] encoded = Utf8.Encode(label);
        RichRuntime.EnsureAbi();
        owner = new(Utf8.Map(encoded, native => NativeFactory.Create(
            new TreeHandle(), output => NativeMethods.TreeNew(native, output))));
    }

    /// <summary>Creates a tree with a borrowed renderable root label.</summary>
    public unsafe Tree(Renderable label)
    {
        ArgumentNullException.ThrowIfNull(label);
        RichRuntime.EnsureAbi();
        owner = new(label.Borrow(renderable => NativeFactory.Create(
            new TreeHandle(), output => NativeMethods.TreeNewRenderable(renderable, output))));
    }

    /// <summary>Adds a plain child and returns its stable ID.</summary>
    public NodeId Add(NodeId parent, string label) => owner.Use(tree => new NodeId(
        Utf8.Map(label, native => NativeScalar.Ulong(output => NativeMethods.TreeAdd(tree, parent.Value, native, output)))));

    /// <summary>Adds a plain child beneath an unsigned ABI node ID.</summary>
    public NodeId Add(ulong parentId, string label) => Add(new NodeId(parentId), label);

    /// <summary>Adds a renderable child and returns its stable ID.</summary>
    public unsafe NodeId Add(NodeId parent, Renderable label)
    {
        ArgumentNullException.ThrowIfNull(label);
        return owner.Use(tree => label.Borrow(renderable => new NodeId(
            NativeScalar.Ulong(output => NativeMethods.TreeAddRenderable(tree, parent.Value, renderable, output)))));
    }

    /// <summary>Adds a renderable child beneath an unsigned ABI node ID.</summary>
    public NodeId Add(ulong parentId, Renderable label) => Add(new NodeId(parentId), label);

    /// <summary>Sets the tree style.</summary>
    public void SetStyle(string style) => WithString(style, NativeMethods.TreeSetStyle);
    /// <summary>Sets the guide style.</summary>
    public void SetGuideStyle(string style) => WithString(style, NativeMethods.TreeSetGuideStyle);
    /// <summary>Shows or hides the root.</summary>
    public bool HideRoot { set => owner.Use(pointer => NativeResult.Check(NativeMethods.TreeSetHideRoot(pointer, NativeFactory.Bool(value)))); }
    /// <summary>Consumes this tree and invalidates its node IDs.</summary>
    public unsafe Renderable Finish() => Renderable.Finish(owner, NativeMethods.TreeFinish);
    /// <summary>Releases an unfinished tree.</summary>
    public void Dispose() => owner.Dispose();

    private delegate uint StringSetter(nint tree, NativeRichStr value);
    private void WithString(string value, StringSetter setter) => owner.Use(pointer =>
        NativeResult.Check(Utf8.With(value, native => setter(pointer, native))));
}
