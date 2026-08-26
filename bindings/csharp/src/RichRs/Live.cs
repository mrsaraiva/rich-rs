namespace RichRs;

/// <summary>Caller-driven native progress state.</summary>
public sealed unsafe class Progress : IDisposable
{
    private readonly NativeOwner<ProgressHandle> owner;

    /// <summary>Creates progress state without a background refresh thread.</summary>
    public unsafe Progress()
    {
        RichRuntime.EnsureAbi();
        owner = new(NativeFactory.Create(new ProgressHandle(), NativeMethods.ProgressNew));
    }

    /// <summary>Adds a task. Task ID zero is valid.</summary>
    public ulong AddTask(string description, double total) => owner.Use(progress =>
        Utf8.Map(description, native => NativeScalar.Ulong(output =>
            NativeMethods.ProgressAddTask(progress, native, total, output))));

    /// <summary>Updates task completion.</summary>
    public void Update(ulong taskId, double completed) => owner.Use(progress =>
        NativeResult.Check(NativeMethods.ProgressUpdate(progress, taskId, completed)));

    /// <summary>Renders one progress frame.</summary>
    public string RenderFrame(RichConsole console)
    {
        ArgumentNullException.ThrowIfNull(console);
        return owner.Use(progress => console.Borrow(consolePointer =>
            NativeBuffer.Call(output => NativeMethods.ProgressRenderFrame(progress, consolePointer, output))));
    }

    /// <summary>Releases progress state.</summary>
    public void Dispose() => owner.Dispose();
}

/// <summary>Caller-driven status state.</summary>
public sealed unsafe class Status : IDisposable
{
    private readonly NativeOwner<StatusHandle> owner;

    /// <summary>Creates status state without starting a background thread.</summary>
    public unsafe Status(string message)
    {
        byte[] encoded = Utf8.Encode(message);
        RichRuntime.EnsureAbi();
        owner = new(Utf8.Map(encoded, native => NativeFactory.Create(
            new StatusHandle(), output => NativeMethods.StatusNew(native, output))));
    }

    /// <summary>Renders one status frame.</summary>
    public string RenderFrame(RichConsole console)
    {
        ArgumentNullException.ThrowIfNull(console);
        return owner.Use(status => console.Borrow(consolePointer =>
            NativeBuffer.Call(output => NativeMethods.StatusRenderFrame(status, consolePointer, output))));
    }

    /// <summary>Releases status state.</summary>
    public void Dispose() => owner.Dispose();
}

/// <summary>Caller-driven spinner state.</summary>
public sealed unsafe class Spinner : IDisposable
{
    private readonly NativeOwner<SpinnerHandle> owner;

    /// <summary>Creates a spinner by native name.</summary>
    public unsafe Spinner(string name)
    {
        byte[] encoded = Utf8.Encode(name);
        RichRuntime.EnsureAbi();
        owner = new(Utf8.Map(encoded, native => NativeFactory.Create(
            new SpinnerHandle(), output => NativeMethods.SpinnerNew(native, output))));
    }

    /// <summary>Renders one spinner frame for caller-supplied time.</summary>
    public string RenderFrame(RichConsole console, double timeSeconds)
    {
        ArgumentNullException.ThrowIfNull(console);
        return owner.Use(spinner => console.Borrow(consolePointer =>
            NativeBuffer.Call(output => NativeMethods.SpinnerRenderFrame(spinner, consolePointer, timeSeconds, output))));
    }

    /// <summary>Releases spinner state.</summary>
    public void Dispose() => owner.Dispose();
}
