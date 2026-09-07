using System.Text;

namespace RichRs.Tests;

public sealed class EncodingBufferTests : IDisposable
{
    public void Dispose()
    {
        NativeBuffer.BeforeDecodeForTest = null;
        NativeBuffer.AfterFreeForTest = null;
        NativeBuffer.ResultOverrideForTest = null;
    }

    [Fact]
    public void InputsAreStrictLengthDelimitedUtf8()
    {
        using RichConsole console = TestSupport.PlainConsole();
        Assert.Equal(string.Empty, console.RenderMarkup(string.Empty));
        Assert.Equal("Héllo 世界 🦀", console.RenderMarkup("Héllo 世界 🦀"));
        Assert.Equal("a\0b", console.RenderMarkup("a\0b"));
        Assert.Equal("abc", console.RenderMarkup("abc"));
        Assert.Equal(3, Encoding.UTF8.GetByteCount(console.RenderMarkup("abc")));
        Assert.Throws<EncoderFallbackException>(() => console.RenderMarkup("bad\ud800value"));
    }

    [Fact]
    public void NativeBufferIsFreedOnSuccess()
    {
        (nint Pointer, ulong Length) after = ((nint)1, 1);
        NativeBuffer.AfterFreeForTest = (pointer, length) => after = (pointer, length);
        Assert.Equal("1.3.0", RichRuntime.Version);
        Assert.Equal(nint.Zero, after.Pointer);
        Assert.Equal(0UL, after.Length);
    }

    [Fact]
    public unsafe void NativeBufferIsFreedOnNativeErrorAndUnknownResult()
    {
        (nint Pointer, ulong Length) after = ((nint)1, 1);
        NativeBuffer.AfterFreeForTest = (pointer, length) => after = (pointer, length);
        RichException nativeError = Assert.Throws<RichException>(() => NativeBuffer.Call(
            output => NativeMethods.ConsoleRenderMarkup(0, default, output)));
        Assert.Equal(RichErrorCode.NullArgument, nativeError.Code);
        Assert.Equal(nint.Zero, after.Pointer);
        Assert.Equal(0UL, after.Length);

        NativeBuffer.ResultOverrideForTest = _ => 0xffff_fffe;
        RichException unknown = Assert.Throws<RichException>(() => NativeBuffer.Call(NativeMethods.FfiVersion));
        Assert.Equal(RichErrorCode.AbiMismatch, unknown.Code);
        Assert.Equal(nint.Zero, after.Pointer);
        Assert.Equal(0UL, after.Length);
    }

    [Fact]
    public unsafe void NativeBufferIsFreedWhenDecoderOrAllocationFails()
    {
        (nint Pointer, ulong Length) after = ((nint)1, 1);
        NativeBuffer.AfterFreeForTest = (pointer, length) => after = (pointer, length);
        NativeBuffer.BeforeDecodeForTest = () => throw new DecoderFallbackException("injected");
        Assert.Throws<DecoderFallbackException>(() => NativeBuffer.Call(NativeMethods.FfiVersion));
        Assert.Equal(nint.Zero, after.Pointer);
        Assert.Equal(0UL, after.Length);

        NativeBuffer.BeforeDecodeForTest = () => throw new OutOfMemoryException("injected");
        Assert.Throws<OutOfMemoryException>(() => NativeBuffer.Call(NativeMethods.FfiVersion));
        Assert.Equal(nint.Zero, after.Pointer);
        Assert.Equal(0UL, after.Length);
    }
}
