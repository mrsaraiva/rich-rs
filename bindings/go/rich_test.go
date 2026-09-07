//go:build cgo

package rich

import (
	"errors"
	"math"
	"runtime"
	"strings"
	"sync"
	"testing"
	"time"
	"unicode/utf8"
)

func requireNoError(t *testing.T, err error) {
	t.Helper()
	if err != nil {
		t.Fatal(err)
	}
}

func makeTextRenderable(t *testing.T, value string) *Renderable {
	t.Helper()
	b, err := NewText(value)
	requireNoError(t, err)
	r, err := b.Finish()
	requireNoError(t, err)
	requireNoError(t, b.Close())
	t.Cleanup(func() { _ = r.Close() })
	return r
}

func makePlainConsole(t *testing.T) *Console {
	t.Helper()
	c, err := NewConsole()
	requireNoError(t, err)
	requireNoError(t, c.SetForceTerminal(false))
	t.Cleanup(func() { _ = c.Close() })
	return c
}

func TestABIErrorAndLayoutContract(t *testing.T) {
	requireNoError(t, EnsureABI())
	version, err := Version()
	requireNoError(t, err)
	if version != "1.3.0" {
		t.Fatalf("version = %q", version)
	}
	l := nativeLayoutInfo()
	if l.ResultSize != 4 || l.StrSize != 16 || l.StrAlign != 8 || l.StrDataOffset != 0 || l.StrLenOffset != 8 || l.BufferSize != 16 || l.BufferAlign != 8 || l.BufferDataOffset != 0 || l.BufferLenOffset != 8 {
		t.Fatalf("layout: %+v", l)
	}
	known := []ErrorCode{ErrorNullArgument, ErrorInvalidUTF8, ErrorInvalidArgument, ErrorInvalidState, ErrorIO, ErrorRender, ErrorNativePanic}
	for _, code := range known {
		err := checkResult(uint32(code))
		var native *Error
		if !errors.As(err, &native) || native.Code != code {
			t.Fatalf("code %d: %v", code, err)
		}
	}
	if !errors.Is(checkResult(999), ErrABIMismatch) {
		t.Fatal("unknown result was not ABI mismatch")
	}
	if checkResult(0) != nil {
		t.Fatal("success mapped to error")
	}
}

func TestExactStringInputAndOutput(t *testing.T) {
	c := makePlainConsole(t)
	for _, value := range []string{"", "héllo 世界 🦀", "before\x00after"} {
		r := makeTextRenderable(t, value)
		out, err := c.Render(r)
		requireNoError(t, err)
		if out != value {
			t.Fatalf("roundtrip %q = %q", value, out)
		}
		if !utf8.ValidString(out) {
			t.Fatalf("invalid output: %q", out)
		}
	}
	_, err := NewText(string([]byte{0xff, 0xfe}))
	if !errors.Is(err, ErrInvalidUTF8) {
		t.Fatalf("invalid UTF-8: %v", err)
	}
	_, err = c.RenderMarkup(string([]byte{0xff}))
	if !errors.Is(err, ErrInvalidUTF8) {
		t.Fatalf("markup invalid UTF-8: %v", err)
	}
}

func TestConsoleTextAndStyleSurface(t *testing.T) {
	c := makePlainConsole(t)
	requireNoError(t, c.SetSize(72, 20))
	requireNoError(t, c.SetColorSystem(ColorTruecolor))
	requireNoError(t, c.SetMarkupEnabled(true))
	requireNoError(t, c.SetEmojiEnabled(true))
	width, err := c.Width()
	requireNoError(t, err)
	if width != 72 {
		t.Fatalf("width = %d", width)
	}
	if err := c.SetColorSystem(ColorSystem(999)); !errors.Is(err, ErrInvalidArgument) {
		t.Fatalf("color enum: %v", err)
	}
	style, err := ParseStyle("bold magenta")
	requireNoError(t, err)
	requireNoError(t, style.Close())
	if _, err := ParseStyle("definitely-not-a-style"); !errors.Is(err, ErrInvalidArgument) {
		t.Fatalf("style: %v", err)
	}
	text, err := NewTextMarkup(c, "[bold]markup[/bold]")
	requireNoError(t, err)
	defer text.Close()
	renderable, err := text.Finish()
	requireNoError(t, err)
	defer renderable.Close()
	out, err := c.Render(renderable)
	requireNoError(t, err)
	if !strings.Contains(out, "markup") {
		t.Fatalf("markup text: %q", out)
	}
}

func TestNativeBufferCleanupAllPaths(t *testing.T) {
	originalCopy, originalFree, originalResult, originalLayout := copyBufferBytes, bufferFreedHook, bufferResultHook, bufferLayoutHook
	t.Cleanup(func() {
		copyBufferBytes, bufferFreedHook, bufferResultHook, bufferLayoutHook = originalCopy, originalFree, originalResult, originalLayout
	})
	var freed int
	bufferFreedHook = func(dataNil bool, length uint64) {
		if !dataNil || length != 0 {
			t.Errorf("buffer not cleared: nil=%v len=%d", dataNil, length)
		}
		freed++
	}
	_, err := Version()
	requireNoError(t, err)
	copyBufferBytes = func([]byte) (string, error) { return "", errors.New("injected copy failure") }
	_, err = Version()
	if err == nil || err.Error() != "injected copy failure" {
		t.Fatalf("copy seam: %v", err)
	}
	copyBufferBytes = originalCopy
	bufferResultHook = func(uint32) uint32 { return 999 }
	_, err = Version()
	if !errors.Is(err, ErrABIMismatch) {
		t.Fatalf("unknown: %v", err)
	}
	bufferResultHook = nil
	bufferLayoutHook = func(bool, uint64) (bool, uint64) { return true, 1 }
	_, err = Version()
	if !errors.Is(err, ErrABIMismatch) {
		t.Fatalf("layout: %v", err)
	}
	bufferLayoutHook = nil
	bufferLayoutHook = func(bool, uint64) (bool, uint64) { return false, math.MaxUint64 }
	_, err = Version()
	if !errors.Is(err, ErrABIMismatch) {
		t.Fatalf("oversized layout: %v", err)
	}
	bufferLayoutHook = nil
	c := makePlainConsole(t)
	_, err = c.RenderMarkup(string([]byte{0xff}))
	if !errors.Is(err, ErrInvalidUTF8) {
		t.Fatalf("native failure: %v", err)
	}
	_, err = c.RenderMarkup("")
	requireNoError(t, err)
	if freed != 7 {
		t.Fatalf("free count = %d", freed)
	}
}

func TestCopySafeCloseFinishAndFallback(t *testing.T) {
	b, err := NewText("copy")
	requireNoError(t, err)
	copyView := *b
	requireNoError(t, b.Close())
	requireNoError(t, b.Close())
	if err := copyView.SetStyle("bold"); !errors.Is(err, ErrInvalidState) {
		t.Fatalf("copied close: %v", err)
	}

	b, err = NewText("finish")
	requireNoError(t, err)
	copyView = *b
	r, err := copyView.Finish()
	requireNoError(t, err)
	defer r.Close()
	if _, err := b.Finish(); !errors.Is(err, ErrInvalidState) {
		t.Fatalf("copied finish: %v", err)
	}
	requireNoError(t, b.Close())

	b, err = NewText("failed")
	requireNoError(t, err)
	forced := finishTestOverride{result: uint32(ErrorInvalidArgument)}
	finishBeforeHook = func(handleKind) *finishTestOverride { return &forced }
	_, err = b.Finish()
	finishBeforeHook = nil
	if !errors.Is(err, ErrInvalidArgument) {
		t.Fatalf("finish failure: %v", err)
	}
	requireNoError(t, b.SetStyle("bold"))
	requireNoError(t, b.Close())

	h, err := nativeTextNew("fallback")
	requireNoError(t, err)
	s := newOwnerState(hText, h)
	count := 0
	originalHook := replaceNativeFreedHook(nil)
	replaceNativeFreedHook(func(kind handleKind) {
		if kind == hText {
			count++
		}
	})
	finalizeOwner(s)
	replaceNativeFreedHook(originalHook)
	runtime.SetFinalizer(s, nil)
	if count != 1 || !s.h.isNil() {
		t.Fatalf("fallback free count=%d nil=%v", count, s.h.isNil())
	}

	freed := make(chan struct{}, 1)
	replaceNativeFreedHook(func(kind handleKind) {
		if kind == hStyle {
			select {
			case freed <- struct{}{}:
			default:
			}
		}
	})
	func() {
		abandoned, e := ParseStyle("bold")
		requireNoError(t, e)
		runtime.KeepAlive(abandoned)
	}()
	deadline := time.After(2 * time.Second)
	for {
		runtime.GC()
		select {
		case <-freed:
			replaceNativeFreedHook(originalHook)
			return
		case <-deadline:
			replaceNativeFreedHook(originalHook)
			t.Fatal("runtime finalizer did not release abandoned owner")
		default:
			runtime.Gosched()
		}
	}
}

func TestConsumedPanicFinishPreservesErrorAndInvalidatesCopies(t *testing.T) {
	builder, err := NewText("consumed panic")
	requireNoError(t, err)
	copyView := *builder

	textFrees, renderableFrees := 0, 0
	originalFreeHook := replaceNativeFreedHook(nil)
	replaceNativeFreedHook(func(kind handleKind) {
		switch kind {
		case hText:
			textFrees++
		case hRenderable:
			renderableFrees++
		}
	})
	override := finishTestOverride{result: uint32(ErrorNativePanic), consume: true}
	finishBeforeHook = func(kind handleKind) *finishTestOverride {
		if kind != hText {
			t.Fatalf("unexpected finish kind: %v", kind)
		}
		return &override
	}
	t.Cleanup(func() {
		finishBeforeHook = nil
		replaceNativeFreedHook(originalFreeHook)
	})

	output, err := copyView.Finish()
	finishBeforeHook = nil
	if output != nil {
		t.Fatal("panic finish returned a renderable")
	}
	if !errors.Is(err, ErrNativePanic) {
		t.Fatalf("panic finish error = %v", err)
	}
	if err := builder.SetStyle("bold"); !errors.Is(err, ErrInvalidState) {
		t.Fatalf("original builder remained live: %v", err)
	}
	if _, err := copyView.Finish(); !errors.Is(err, ErrInvalidState) {
		t.Fatalf("copied builder remained live: %v", err)
	}
	requireNoError(t, builder.Close())
	requireNoError(t, copyView.Close())
	if textFrees != 1 {
		t.Fatalf("consumed builder free count = %d, want 1", textFrees)
	}
	if renderableFrees != 0 {
		t.Fatalf("panic path produced/freed %d unexpected outputs", renderableFrees)
	}
}

func TestMalformedReplacementFinishRetainsOriginalAndFreesReplacement(t *testing.T) {
	builder, err := NewText("original")
	requireNoError(t, err)
	copyView := *builder
	original := builder.state.h
	replacement, err := nativeTextNew("replacement")
	requireNoError(t, err)

	var freed []nativeHandle
	originalHandleHook := replaceNativeHandleFreedHook(nil)
	replaceNativeHandleFreedHook(func(kind handleKind, handle nativeHandle) {
		if kind == hText {
			freed = append(freed, handle)
		}
	})
	override := finishTestOverride{
		result:           uint32(ErrorInvalidArgument),
		replaceRemaining: true,
		remaining:        replacement,
	}
	finishBeforeHook = func(kind handleKind) *finishTestOverride {
		if kind != hText {
			t.Fatalf("unexpected finish kind: %v", kind)
		}
		return &override
	}
	t.Cleanup(func() {
		finishBeforeHook = nil
		replaceNativeHandleFreedHook(originalHandleHook)
	})

	output, err := copyView.Finish()
	finishBeforeHook = nil
	if output != nil {
		t.Fatal("malformed finish returned a renderable")
	}
	if !errors.Is(err, ErrABIMismatch) {
		t.Fatalf("malformed replacement error = %v", err)
	}
	if len(freed) != 1 || !freed[0].equal(replacement) {
		t.Fatalf("replacement frees = %v, want exactly replacement", freed)
	}
	if !builder.state.h.equal(original) || copyView.state != builder.state {
		t.Fatal("original shared owner state was replaced or invalidated")
	}
	requireNoError(t, builder.SetStyle("bold"))
	requireNoError(t, builder.Close())
	if len(freed) != 2 || !freed[1].equal(original) {
		t.Fatalf("final frees = %v, want replacement then original", freed)
	}
	if err := copyView.SetStyle("italic"); !errors.Is(err, ErrInvalidState) {
		t.Fatalf("copied view remained live after close: %v", err)
	}
	requireNoError(t, copyView.Close())
	if len(freed) != 2 {
		t.Fatalf("double free after copied close: %d", len(freed))
	}
}

func TestMalformedSuccessReplacementDisposesReturnedOwnership(t *testing.T) {
	builder, err := NewText("consumed original")
	requireNoError(t, err)
	copyView := *builder
	original := builder.state.h
	replacement, err := nativeTextNew("unexpected replacement")
	requireNoError(t, err)

	var freed []nativeHandle
	originalHandleHook := replaceNativeHandleFreedHook(nil)
	replaceNativeHandleFreedHook(func(kind handleKind, handle nativeHandle) {
		if kind == hText {
			freed = append(freed, handle)
		}
	})
	override := finishTestOverride{
		result:           0,
		consume:          true,
		replaceRemaining: true,
		remaining:        replacement,
	}
	finishBeforeHook = func(handleKind) *finishTestOverride { return &override }
	t.Cleanup(func() {
		finishBeforeHook = nil
		replaceNativeHandleFreedHook(originalHandleHook)
	})

	output, err := builder.Finish()
	finishBeforeHook = nil
	if output != nil || !errors.Is(err, ErrABIMismatch) {
		t.Fatalf("malformed success result = (%v, %v)", output, err)
	}
	if len(freed) != 2 || !freed[0].equal(original) || !freed[1].equal(replacement) {
		t.Fatalf("frees = %v, want consumed original then returned replacement", freed)
	}
	if !builder.state.h.isNil() {
		t.Fatal("malformed success left owner state live")
	}
	if _, err := copyView.Finish(); !errors.Is(err, ErrInvalidState) {
		t.Fatalf("copied view remained live: %v", err)
	}
	requireNoError(t, builder.Close())
	requireNoError(t, copyView.Close())
	if len(freed) != 2 {
		t.Fatalf("double free after close: %d", len(freed))
	}
}

func TestFinishRejectsRetainedInputOutputAliasWithoutWrongFamilyFree(t *testing.T) {
	for _, result := range []uint32{uint32(ErrorInvalidArgument), 0} {
		name := "error"
		if result == 0 {
			name = "ok"
		}
		t.Run(name, func(t *testing.T) {
			builder, err := NewText("retained alias")
			requireNoError(t, err)
			copyView := *builder
			original := builder.state.h

			var freedKinds []handleKind
			var freedHandles []nativeHandle
			originalHandleHook := replaceNativeHandleFreedHook(nil)
			replaceNativeHandleFreedHook(func(kind handleKind, handle nativeHandle) {
				freedKinds = append(freedKinds, kind)
				freedHandles = append(freedHandles, handle)
			})
			override := finishTestOverride{result: result, setOutput: true, output: original}
			finishBeforeHook = func(handleKind) *finishTestOverride { return &override }
			t.Cleanup(func() {
				finishBeforeHook = nil
				replaceNativeHandleFreedHook(originalHandleHook)
			})

			output, err := copyView.Finish()
			finishBeforeHook = nil
			if output != nil || !errors.Is(err, ErrABIMismatch) {
				t.Fatalf("retained alias result = (%v, %v)", output, err)
			}
			if result != 0 {
				if len(freedKinds) != 0 {
					t.Fatalf("error alias finish frees = %v, want none", freedKinds)
				}
				if !builder.state.h.equal(original) {
					t.Fatal("error alias invalidated the retained input")
				}
				requireNoError(t, builder.Close())
			} else if !builder.state.h.isNil() {
				t.Fatal("successful malformed finish left input state live")
			}
			if len(freedKinds) != 1 || freedKinds[0] != hText || !freedHandles[0].equal(original) {
				t.Fatalf("alias frees = (%v, %v), want input exactly once as text", freedKinds, freedHandles)
			}
			if _, err := copyView.Finish(); !errors.Is(err, ErrInvalidState) {
				t.Fatalf("copied state remained live: %v", err)
			}
			requireNoError(t, copyView.Close())
			if len(freedKinds) != 1 {
				t.Fatalf("copied close double-freed input: %v", freedKinds)
			}
		})
	}
}

func TestFinishRejectsReplacementInputOutputAliasWithoutWrongFamilyFree(t *testing.T) {
	for _, result := range []uint32{uint32(ErrorInvalidArgument), 0} {
		name := "error"
		if result == 0 {
			name = "ok"
		}
		t.Run(name, func(t *testing.T) {
			builder, err := NewText("original")
			requireNoError(t, err)
			copyView := *builder
			original := builder.state.h
			replacement, err := nativeTextNew("replacement alias")
			requireNoError(t, err)

			var freedKinds []handleKind
			var freedHandles []nativeHandle
			originalHandleHook := replaceNativeHandleFreedHook(nil)
			replaceNativeHandleFreedHook(func(kind handleKind, handle nativeHandle) {
				freedKinds = append(freedKinds, kind)
				freedHandles = append(freedHandles, handle)
			})
			override := finishTestOverride{
				result:           result,
				consume:          result == 0,
				replaceRemaining: true,
				remaining:        replacement,
				setOutput:        true,
				output:           replacement,
			}
			finishBeforeHook = func(handleKind) *finishTestOverride { return &override }
			t.Cleanup(func() {
				finishBeforeHook = nil
				replaceNativeHandleFreedHook(originalHandleHook)
			})

			output, err := copyView.Finish()
			finishBeforeHook = nil
			if output != nil || !errors.Is(err, ErrABIMismatch) {
				t.Fatalf("replacement alias result = (%v, %v)", output, err)
			}
			if result != 0 {
				if len(freedKinds) != 1 || freedKinds[0] != hText || !freedHandles[0].equal(replacement) {
					t.Fatalf("error alias frees = (%v, %v), want replacement once as text", freedKinds, freedHandles)
				}
				if !builder.state.h.equal(original) {
					t.Fatal("error alias invalidated original input")
				}
				requireNoError(t, builder.Close())
			} else {
				if len(freedKinds) != 2 || freedKinds[0] != hText || freedKinds[1] != hText ||
					!freedHandles[0].equal(original) || !freedHandles[1].equal(replacement) {
					t.Fatalf("success alias frees = (%v, %v), want original then replacement as text", freedKinds, freedHandles)
				}
				if !builder.state.h.isNil() {
					t.Fatal("successful malformed finish left state live")
				}
			}
			if result != 0 {
				if len(freedKinds) != 2 || freedKinds[1] != hText || !freedHandles[1].equal(original) {
					t.Fatalf("final frees = (%v, %v), want replacement then original", freedKinds, freedHandles)
				}
			}
			requireNoError(t, copyView.Close())
			if len(freedKinds) != 2 {
				t.Fatalf("copied close changed free count: %v", freedKinds)
			}
		})
	}
}

func makeFinishTestRenderable(t *testing.T) nativeHandle {
	t.Helper()
	input, err := nativeTextNew("real fault-seam output")
	requireNoError(t, err)
	remaining, output, result := nativeFinish(hText, input)
	if err := checkResult(result); err != nil {
		t.Fatalf("create real fault-seam output: %v", err)
	}
	if !remaining.isNil() || output.isNil() {
		t.Fatalf("create real fault-seam output receipt = (%v, %v)", remaining, output)
	}
	return output
}

func TestFinishTreatsConsumedOriginalAddressAsOutputSlotOwnership(t *testing.T) {
	t.Run("ok address reuse is accepted", func(t *testing.T) {
		builder, err := NewText("address reuse success")
		requireNoError(t, err)
		copyView := *builder
		original := builder.state.h
		realOutput := makeFinishTestRenderable(t)
		receiptOutput := realOutput
		receiptOutput.testAddress = original.p

		var freedKinds []handleKind
		var freedHandles []nativeHandle
		originalHandleHook := replaceNativeHandleFreedHook(nil)
		replaceNativeHandleFreedHook(func(kind handleKind, handle nativeHandle) {
			freedKinds = append(freedKinds, kind)
			freedHandles = append(freedHandles, handle)
		})
		override := finishTestOverride{result: 0, consume: true, setOutput: true, output: receiptOutput}
		finishBeforeHook = func(handleKind) *finishTestOverride { return &override }
		t.Cleanup(func() {
			finishBeforeHook = nil
			replaceNativeHandleFreedHook(originalHandleHook)
		})

		output, err := copyView.Finish()
		finishBeforeHook = nil
		requireNoError(t, err)
		if output == nil || output.state == nil || output.state.kind != hRenderable || !output.state.h.equal(original) {
			t.Fatal("consumed-address output was not adopted as renderable ownership")
		}
		if len(freedKinds) != 1 || freedKinds[0] != hText || freedHandles[0].p != original.p {
			t.Fatalf("finish frees = (%v, %v), want only consumed text input", freedKinds, freedHandles)
		}
		if !builder.state.h.isNil() {
			t.Fatal("successful finish left builder state live")
		}
		requireNoError(t, output.Close())
		requireNoError(t, builder.Close())
		requireNoError(t, copyView.Close())
		if len(freedKinds) != 2 || freedKinds[1] != hRenderable || freedHandles[1].p != realOutput.p {
			t.Fatalf("final frees = (%v, %v), want text then real renderable", freedKinds, freedHandles)
		}
	})

	t.Run("error output is cleaned by output slot kind", func(t *testing.T) {
		builder, err := NewText("address reuse error")
		requireNoError(t, err)
		copyView := *builder
		original := builder.state.h
		realOutput := makeFinishTestRenderable(t)
		receiptOutput := realOutput
		receiptOutput.testAddress = original.p

		var actualFrees []handleKind
		var actualHandles []nativeHandle
		originalHandleHook := replaceNativeHandleFreedHook(nil)
		replaceNativeHandleFreedHook(func(kind handleKind, handle nativeHandle) {
			actualFrees = append(actualFrees, kind)
			actualHandles = append(actualHandles, handle)
		})
		override := finishTestOverride{
			result:    uint32(ErrorNativePanic),
			consume:   true,
			setOutput: true,
			output:    receiptOutput,
		}
		finishBeforeHook = func(handleKind) *finishTestOverride { return &override }
		t.Cleanup(func() {
			finishBeforeHook = nil
			replaceNativeHandleFreedHook(originalHandleHook)
		})

		output, err := builder.Finish()
		finishBeforeHook = nil
		if output != nil || !errors.Is(err, ErrABIMismatch) {
			t.Fatalf("malformed error result = (%v, %v)", output, err)
		}
		if len(actualFrees) != 2 || actualFrees[0] != hText || actualFrees[1] != hRenderable ||
			actualHandles[0].p != original.p || actualHandles[1].p != realOutput.p {
			t.Fatalf("actual frees = (%v, %v), want text then real renderable", actualFrees, actualHandles)
		}
		if !builder.state.h.isNil() {
			t.Fatal("consumed error left builder state live")
		}
		if _, err := copyView.Finish(); !errors.Is(err, ErrInvalidState) {
			t.Fatalf("copy remained live: %v", err)
		}
		requireNoError(t, builder.Close())
		requireNoError(t, copyView.Close())
		if len(actualFrees) != 2 {
			t.Fatalf("close changed cleanup counts: %v", actualFrees)
		}
	})
}

func TestFinishTreatsReplacementOriginalAddressByRetainedOwnership(t *testing.T) {
	for _, result := range []uint32{uint32(ErrorInvalidArgument), 0} {
		name := "error"
		if result == 0 {
			name = "ok"
		}
		t.Run(name, func(t *testing.T) {
			builder, err := NewText("replacement original-address output")
			requireNoError(t, err)
			copyView := *builder
			original := builder.state.h
			replacement, err := nativeTextNew("replacement")
			requireNoError(t, err)
			receiptOutput := original
			var realOutput nativeHandle
			if result == 0 {
				realOutput = makeFinishTestRenderable(t)
				receiptOutput = realOutput
				receiptOutput.testAddress = original.p
			}

			var actualKinds []handleKind
			var actualHandles []nativeHandle
			originalHandleHook := replaceNativeHandleFreedHook(nil)
			replaceNativeHandleFreedHook(func(kind handleKind, handle nativeHandle) {
				actualKinds = append(actualKinds, kind)
				actualHandles = append(actualHandles, handle)
			})
			override := finishTestOverride{
				result:           result,
				consume:          result == 0,
				replaceRemaining: true,
				remaining:        replacement,
				setOutput:        true,
				output:           receiptOutput,
			}
			finishBeforeHook = func(handleKind) *finishTestOverride { return &override }
			t.Cleanup(func() {
				finishBeforeHook = nil
				replaceNativeHandleFreedHook(originalHandleHook)
			})

			output, err := copyView.Finish()
			finishBeforeHook = nil
			if output != nil || !errors.Is(err, ErrABIMismatch) {
				t.Fatalf("replacement/original-address result = (%v, %v)", output, err)
			}
			if result != 0 {
				if len(actualKinds) != 1 || actualKinds[0] != hText || !actualHandles[0].equal(replacement) {
					t.Fatalf("error actual frees = (%v, %v), want replacement text", actualKinds, actualHandles)
				}
				if !builder.state.h.equal(original) {
					t.Fatal("error replacement invalidated original shared state")
				}
				requireNoError(t, builder.SetStyle("bold"))
				requireNoError(t, builder.Close())
				if len(actualKinds) != 2 || actualKinds[1] != hText || !actualHandles[1].equal(original) {
					t.Fatalf("error final frees = (%v, %v), want replacement then original", actualKinds, actualHandles)
				}
			} else {
				if len(actualKinds) != 3 || actualKinds[0] != hText || actualKinds[1] != hRenderable || actualKinds[2] != hText ||
					actualHandles[0].p != original.p || actualHandles[1].p != realOutput.p || actualHandles[2].p != replacement.p {
					t.Fatalf("success actual frees = (%v, %v), want original text, real renderable, replacement text", actualKinds, actualHandles)
				}
				if !builder.state.h.isNil() {
					t.Fatal("successful malformed finish left state live")
				}
			}
			requireNoError(t, copyView.Close())
			wantFrees := 2
			if result == 0 {
				wantFrees = 3
			}
			if len(actualKinds) != wantFrees {
				t.Fatalf("copied close changed cleanup counts: %v", actualKinds)
			}
		})
	}
}

func TestConcurrentCallsCloseAndFinish(t *testing.T) {
	for iteration := 0; iteration < 30; iteration++ {
		c, err := NewConsole()
		requireNoError(t, err)
		copyConsole := *c
		var wg sync.WaitGroup
		for i := 0; i < 4; i++ {
			wg.Add(1)
			go func() {
				defer wg.Done()
				for j := 0; j < 20; j++ {
					_, e := copyConsole.RenderMarkup("hello")
					if e != nil && !errors.Is(e, ErrInvalidState) {
						t.Errorf("render race: %v", e)
						return
					}
				}
			}()
		}
		wg.Add(1)
		go func() { defer wg.Done(); _ = c.Close() }()
		wg.Wait()
		requireNoError(t, copyConsole.Close())

		b, err := NewText("finish race")
		requireNoError(t, err)
		copyBuilder := *b
		results := make(chan error, 2)
		var renderablesMu sync.Mutex
		var renderables []*Renderable
		finish := func(builder *Text) {
			r, e := builder.Finish()
			if r != nil {
				renderablesMu.Lock()
				renderables = append(renderables, r)
				renderablesMu.Unlock()
			}
			results <- e
		}
		go finish(b)
		go finish(&copyBuilder)
		e1, e2 := <-results, <-results
		if (e1 == nil) == (e2 == nil) {
			t.Fatalf("finish outcomes: %v, %v", e1, e2)
		}
		if e1 != nil && !errors.Is(e1, ErrInvalidState) {
			t.Fatal(e1)
		}
		if e2 != nil && !errors.Is(e2, ErrInvalidState) {
			t.Fatal(e2)
		}
		for _, r := range renderables {
			requireNoError(t, r.Close())
		}
		requireNoError(t, b.Close())

		b, err = NewText("close finish race")
		requireNoError(t, err)
		copyBuilder = *b
		finished := make(chan *Renderable, 1)
		finishErr := make(chan error, 1)
		go func() { r, e := copyBuilder.Finish(); finished <- r; finishErr <- e }()
		go func() { _ = b.Close() }()
		rendered := <-finished
		e := <-finishErr
		if e != nil && !errors.Is(e, ErrInvalidState) {
			t.Fatalf("close/finish: %v", e)
		}
		if rendered != nil {
			requireNoError(t, rendered.Close())
		}
		requireNoError(t, b.Close())
	}
}

func TestPlainAndTerminalOutput(t *testing.T) {
	plain := makePlainConsole(t)
	out, err := plain.RenderMarkup("[bold link=https://example.com]hello[/bold link]")
	requireNoError(t, err)
	if strings.Contains(out, "\x1b[") || strings.Contains(out, "\x1b]") {
		t.Fatalf("plain escapes: %q", out)
	}
	terminal, err := NewConsole()
	requireNoError(t, err)
	defer terminal.Close()
	requireNoError(t, terminal.SetForceTerminal(true))
	out, err = terminal.RenderMarkup("[bold]hello[/bold]")
	requireNoError(t, err)
	if !strings.Contains(out, "\x1b[") {
		t.Fatalf("missing ANSI: %q", out)
	}
}

func TestAllFeatureFamiliesAndReusableComposition(t *testing.T) {
	c := makePlainConsole(t)
	reused := makeTextRenderable(t, "reused")

	table, err := NewTable()
	requireNoError(t, err)
	defer table.Close()
	requireNoError(t, table.SetTitle("Title"))
	requireNoError(t, table.SetCaption("Caption"))
	requireNoError(t, table.SetTitleMarkup(c, "[bold]Markup[/bold]"))
	requireNoError(t, table.SetCaptionMarkup(c, "[italic]Cap[/italic]"))
	requireNoError(t, table.SetBox(BoxRounded))
	requireNoError(t, table.SetShowHeader(true))
	requireNoError(t, table.SetShowLines(true))
	requireNoError(t, table.SetShowEdge(true))
	requireNoError(t, table.SetExpand(false))
	requireNoError(t, table.SetPadding(1, 1))
	requireNoError(t, table.SetStyle("blue"))
	requireNoError(t, table.AddColumn("plain"))
	requireNoError(t, table.AddColumnRenderable(reused))
	requireNoError(t, table.AddRow("one", "two"))
	requireNoError(t, table.AddRenderableRow(reused, reused))
	tableR, err := table.Finish()
	requireNoError(t, err)
	defer tableR.Close()

	panel, err := NewPanel(reused)
	requireNoError(t, err)
	defer panel.Close()
	requireNoError(t, panel.SetTitle("Panel"))
	requireNoError(t, panel.SetSubtitle("Sub"))
	requireNoError(t, panel.SetBox(BoxSquare))
	requireNoError(t, panel.SetExpand(false))
	requireNoError(t, panel.SetWidth(30))
	requireNoError(t, panel.SetPadding(PaddingDimensions{Top: 1, Right: 1, Bottom: 1, Left: 1}))
	requireNoError(t, panel.SetStyle("white"))
	requireNoError(t, panel.SetBorderStyle("green"))
	panelR, err := panel.Finish()
	requireNoError(t, err)
	defer panelR.Close()

	columns, err := NewColumns()
	requireNoError(t, err)
	defer columns.Close()
	requireNoError(t, columns.Add(reused))
	requireNoError(t, columns.Add(tableR))
	requireNoError(t, columns.AddString("plain"))
	requireNoError(t, columns.SetEqual(true))
	requireNoError(t, columns.SetExpand(false))
	requireNoError(t, columns.SetPadding(0, 1))
	columnsR, err := columns.Finish()
	requireNoError(t, err)
	defer columnsR.Close()

	align, err := NewAlign(reused, AlignRight)
	requireNoError(t, err)
	defer align.Close()
	requireNoError(t, align.SetVertical(VerticalMiddle))
	requireNoError(t, align.SetWidth(20))
	alignR, err := align.Finish()
	requireNoError(t, err)
	defer alignR.Close()
	center, err := NewCenteredAlign(reused)
	requireNoError(t, err)
	defer center.Close()
	centerR, err := center.Finish()
	requireNoError(t, err)
	defer centerR.Close()
	padding, err := NewPadding(reused, PaddingDimensions{Top: 1, Right: 2, Bottom: 1, Left: 2})
	requireNoError(t, err)
	defer padding.Close()
	requireNoError(t, padding.SetStyle("red"))
	requireNoError(t, padding.SetExpand(false))
	paddingR, err := padding.Finish()
	requireNoError(t, err)
	defer paddingR.Close()
	rule, err := NewRule()
	requireNoError(t, err)
	defer rule.Close()
	requireNoError(t, rule.SetTitle("Rule"))
	requireNoError(t, rule.SetCharacters("-"))
	requireNoError(t, rule.SetStyle("cyan"))
	requireNoError(t, rule.SetAlign(AlignCenter))
	ruleR, err := rule.Finish()
	requireNoError(t, err)
	defer ruleR.Close()

	tree, err := NewTreeRenderable(reused)
	requireNoError(t, err)
	defer tree.Close()
	parent := RootNodeID
	for i := 0; i < 12; i++ {
		parent, err = tree.Add(parent, "deep")
		requireNoError(t, err)
	}
	for i := 0; i < 12; i++ {
		_, err = tree.Add(RootNodeID, "wide")
		requireNoError(t, err)
	}
	_, err = tree.AddRenderable(parent, reused)
	requireNoError(t, err)
	if _, err = tree.Add(NodeID(math.MaxUint64), "bad"); !errors.Is(err, ErrInvalidState) {
		t.Fatalf("unknown parent: %v", err)
	}
	requireNoError(t, tree.SetStyle("white"))
	requireNoError(t, tree.SetGuideStyle("dim"))
	requireNoError(t, tree.SetHideRoot(false))
	treeR, err := tree.Finish()
	requireNoError(t, err)
	defer treeR.Close()
	plainTree, err := NewTree("root")
	requireNoError(t, err)
	defer plainTree.Close()
	plainTreeR, err := plainTree.Finish()
	requireNoError(t, err)
	defer plainTreeR.Close()

	out, err := c.Render(reused)
	requireNoError(t, err)
	if out != "reused" {
		t.Fatalf("independent reused output: %q", out)
	}
	requireNoError(t, reused.Close())
	for name, r := range map[string]*Renderable{"table": tableR, "panel": panelR, "columns": columnsR, "align": alignR, "center": centerR, "padding": paddingR, "rule": ruleR, "tree": treeR, "plain-tree": plainTreeR} {
		out, e := c.Render(r)
		requireNoError(t, e)
		if !utf8.ValidString(out) {
			t.Fatalf("%s invalid UTF-8", name)
		}
	}
}

func TestContentAndLiveFamilies(t *testing.T) {
	c := makePlainConsole(t)
	start, end := uint32(1), uint32(2)
	syntax, err := NewSyntax("fn main() {}", "rust")
	requireNoError(t, err)
	defer syntax.Close()
	requireNoError(t, syntax.SetTheme("monokai"))
	requireNoError(t, syntax.SetLineNumbers(true))
	requireNoError(t, syntax.SetWordWrap(true))
	requireNoError(t, syntax.SetLineRange(LineRange{Start: &start, End: &end}))
	syntaxR, err := syntax.Finish()
	requireNoError(t, err)
	defer syntaxR.Close()
	if _, err = NewSyntaxFromPath("/definitely/not/a/rich-file"); !errors.Is(err, ErrIO) {
		t.Fatalf("path: %v", err)
	}
	if _, err = NewSyntaxFromPath("bad\x00path"); !errors.Is(err, ErrInvalidArgument) {
		t.Fatalf("NUL path: %v", err)
	}
	markdown, err := NewMarkdown("# Header")
	requireNoError(t, err)
	defer markdown.Close()
	requireNoError(t, markdown.SetCodeTheme("monokai"))
	requireNoError(t, markdown.SetHyperlinks(false))
	requireNoError(t, markdown.SetJustify(JustifyLeft))
	markdownR, err := markdown.Finish()
	requireNoError(t, err)
	defer markdownR.Close()
	options := NewJSONOptions()
	options.SortKeys = true
	jsonValue, err := NewJSON(`{"b":2,"a":1}`, options)
	requireNoError(t, err)
	defer jsonValue.Close()
	jsonR, err := jsonValue.Finish()
	requireNoError(t, err)
	defer jsonR.Close()
	if _, err = NewJSON("{", NewJSONOptions()); !errors.Is(err, ErrInvalidArgument) {
		t.Fatalf("JSON: %v", err)
	}
	for _, r := range []*Renderable{syntaxR, markdownR, jsonR} {
		out, e := c.Render(r)
		requireNoError(t, e)
		if !utf8.ValidString(out) {
			t.Fatal("invalid content UTF-8")
		}
	}

	progress, err := NewProgress()
	requireNoError(t, err)
	defer progress.Close()
	id, err := progress.AddTask("work", 100)
	requireNoError(t, err)
	if id != 0 {
		t.Fatalf("first task = %d", id)
	}
	requireNoError(t, progress.Update(id, 50))
	frame, err := progress.RenderFrame(c)
	requireNoError(t, err)
	if !utf8.ValidString(frame) {
		t.Fatal("progress UTF-8")
	}
	if err = progress.Update(TaskID(math.MaxUint64), 1); !errors.Is(err, ErrInvalidState) {
		t.Fatalf("task: %v", err)
	}
	if _, err = progress.AddTask("bad", math.NaN()); !errors.Is(err, ErrInvalidArgument) {
		t.Fatalf("NaN: %v", err)
	}
	if err = progress.Update(id, math.Inf(1)); !errors.Is(err, ErrInvalidArgument) {
		t.Fatalf("infinite update: %v", err)
	}
	status, err := NewStatus("working")
	requireNoError(t, err)
	defer status.Close()
	frame, err = status.RenderFrame(c)
	requireNoError(t, err)
	if !utf8.ValidString(frame) {
		t.Fatal("status UTF-8")
	}
	spinner, err := NewSpinner("dots")
	requireNoError(t, err)
	defer spinner.Close()
	frame, err = spinner.RenderFrame(c, 0.25)
	requireNoError(t, err)
	if !utf8.ValidString(frame) {
		t.Fatal("spinner UTF-8")
	}
	if _, err = spinner.RenderFrame(c, math.Inf(1)); !errors.Is(err, ErrInvalidArgument) {
		t.Fatalf("infinite: %v", err)
	}
}

func TestTemporaryArraysAreFailureAtomic(t *testing.T) {
	t.Cleanup(func() {
		nativeStringAllocationHook = nil
		nativeStringLifetimeHook = nil
		nativeArrayLifetimeHook = nil
	})
	table, err := NewTable()
	requireNoError(t, err)
	defer table.Close()
	requireNoError(t, table.AddColumn("one"))
	requireNoError(t, table.AddColumn("two"))
	injected := errors.New("injected native string allocation")
	stringAllocations, stringFrees := 0, 0
	arrayAllocations, arrayFrees := 0, 0
	nativeStringLifetimeHook = func(allocated bool) {
		if allocated {
			stringAllocations++
		} else {
			stringFrees++
		}
	}
	nativeArrayLifetimeHook = func(_ string, allocated bool) {
		if allocated {
			arrayAllocations++
		} else {
			arrayFrees++
		}
	}
	nativeStringAllocationHook = func(s string) error {
		if s == "fail" {
			return injected
		}
		return nil
	}
	err = table.AddRow("partial", "fail")
	nativeStringAllocationHook = nil
	if !errors.Is(err, injected) {
		t.Fatalf("allocation: %v", err)
	}
	if stringAllocations != 1 || stringFrees != 1 || arrayAllocations != 1 || arrayFrees != 1 {
		t.Fatalf("partial string array lifetime: strings %d/%d array %d/%d", stringAllocations, stringFrees, arrayAllocations, arrayFrees)
	}
	nativeStringLifetimeHook = nil
	if err = table.AddRenderableRow(makeTextRenderable(t, "ok"), nil); !errors.Is(err, ErrInvalidState) {
		t.Fatalf("nil renderable: %v", err)
	}
	if arrayAllocations != 1 || arrayFrees != 1 {
		t.Fatalf("renderable array allocated before validation: %d/%d", arrayAllocations, arrayFrees)
	}
	validRenderable := makeTextRenderable(t, "valid")
	requireNoError(t, table.AddRenderableRow(validRenderable, validRenderable))
	if arrayAllocations != 2 || arrayFrees != 2 {
		t.Fatalf("renderable array lifetime: %d/%d", arrayAllocations, arrayFrees)
	}
	nativeArrayLifetimeHook = nil
	requireNoError(t, table.AddRow("final", "row"))
	r, err := table.Finish()
	requireNoError(t, err)
	defer r.Close()
	out, err := makePlainConsole(t).Render(r)
	requireNoError(t, err)
	if strings.Contains(out, "partial") {
		t.Fatalf("partial row mutation: %q", out)
	}
}

func TestEveryOwnerCloseIsIdempotent(t *testing.T) {
	c, _ := NewConsole()
	style, _ := ParseStyle("bold")
	text, _ := NewText("x")
	syntax, _ := NewSyntax("x", "text")
	pretty, _ := NewPretty("{'x': 1}")
	markdown, _ := NewMarkdown("x")
	jsonValue, _ := NewJSON("{}", NewJSONOptions())
	rule, _ := NewRule()
	columns, _ := NewColumns()
	progress, _ := NewProgress()
	status, _ := NewStatus("x")
	spinner, _ := NewSpinner("dots")
	table, _ := NewTable()
	tree, _ := NewTree("x")
	r := makeTextRenderable(t, "x")
	align, _ := NewAlign(r, AlignLeft)
	padding, _ := NewPadding(r, PaddingDimensions{})
	panel, _ := NewPanel(r)
	closers := []interface{ Close() error }{c, style, text, syntax, pretty, markdown, jsonValue, rule, columns, align, padding, progress, status, spinner, panel, table, tree, r}
	for _, closer := range closers {
		requireNoError(t, closer.Close())
		requireNoError(t, closer.Close())
	}
}
