using Microsoft.Win32.SafeHandles;

namespace RichRs;

internal abstract class NativeSafeHandle : SafeHandleZeroOrMinusOneIsInvalid
{
    protected NativeSafeHandle() : base(true) { }
    internal void Initialize(nint value) => SetHandle(value);
    internal void MarkConsumed() => SetHandleAsInvalid();
    internal void CleanupReplacement(nint value)
    {
        try { Free(value); } catch { }
    }
    protected sealed override bool ReleaseHandle()
    {
        try { Free(handle); return true; } catch { return false; }
    }
    protected abstract void Free(nint value);
}

internal sealed class ConsoleHandle : NativeSafeHandle { protected override void Free(nint value) => NativeMethods.ConsoleFree(value); }
internal sealed class RenderableHandle : NativeSafeHandle { protected override void Free(nint value) => NativeMethods.RenderableFree(value); }
internal sealed class TextHandle : NativeSafeHandle { protected override void Free(nint value) => NativeMethods.TextFree(value); }
internal sealed class StyleHandle : NativeSafeHandle { protected override void Free(nint value) => NativeMethods.StyleFree(value); }
internal sealed class SyntaxHandle : NativeSafeHandle { protected override void Free(nint value) => NativeMethods.SyntaxFree(value); }
internal sealed class PrettyHandle : NativeSafeHandle { protected override void Free(nint value) => NativeMethods.PrettyFree(value); }
internal sealed class MarkdownHandle : NativeSafeHandle { protected override void Free(nint value) => NativeMethods.MarkdownFree(value); }
internal sealed class JsonHandle : NativeSafeHandle { protected override void Free(nint value) => NativeMethods.JsonFree(value); }
internal sealed class RuleHandle : NativeSafeHandle { protected override void Free(nint value) => NativeMethods.RuleFree(value); }
internal sealed class ColumnsHandle : NativeSafeHandle { protected override void Free(nint value) => NativeMethods.ColumnsFree(value); }
internal sealed class AlignHandle : NativeSafeHandle { protected override void Free(nint value) => NativeMethods.AlignFree(value); }
internal sealed class PaddingHandle : NativeSafeHandle { protected override void Free(nint value) => NativeMethods.PaddingFree(value); }
internal sealed class ProgressHandle : NativeSafeHandle { protected override void Free(nint value) => NativeMethods.ProgressFree(value); }
internal sealed class StatusHandle : NativeSafeHandle { protected override void Free(nint value) => NativeMethods.StatusFree(value); }
internal sealed class SpinnerHandle : NativeSafeHandle { protected override void Free(nint value) => NativeMethods.SpinnerFree(value); }
internal sealed class PanelHandle : NativeSafeHandle { protected override void Free(nint value) => NativeMethods.PanelFree(value); }
internal sealed class TableHandle : NativeSafeHandle { protected override void Free(nint value) => NativeMethods.TableFree(value); }
internal sealed class TreeHandle : NativeSafeHandle { protected override void Free(nint value) => NativeMethods.TreeFree(value); }
