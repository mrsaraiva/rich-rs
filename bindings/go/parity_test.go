//go:build cgo

package rich

import (
	"errors"
	"fmt"
	"runtime/cgo"
	"strings"
	"sync"
	"sync/atomic"
	"testing"
	"time"
)

type probeRenderer struct {
	mu          sync.Mutex
	height      uint32
	options     []RenderOptions
	retained    *SegmentSink
	reenter     func() error
	callbackErr error
	panicRender bool
	measurement *Measurement
	active      atomic.Int32
	maxActive   atomic.Int32
}

func (r *probeRenderer) Render(options RenderOptions, sink *SegmentSink) error {
	active := r.active.Add(1)
	defer r.active.Add(-1)
	for {
		old := r.maxActive.Load()
		if active <= old || r.maxActive.CompareAndSwap(old, active) {
			break
		}
	}
	if r.panicRender {
		panic("contained callback panic")
	}
	if r.reenter != nil {
		if err := r.reenter(); !errors.Is(err, ErrInvalidState) {
			return fmt.Errorf("reentry error = %v", err)
		}
	}
	r.mu.Lock()
	r.options = append(r.options, options)
	r.retained = sink
	r.mu.Unlock()
	if r.callbackErr != nil {
		return r.callbackErr
	}
	for y := uint32(0); y < r.height; y++ {
		for x := uint32(0); x < options.MaxWidth; x++ {
			if err := sink.Push("▄", "rgb(255,0,0) on rgb(0,0,0)"); err != nil {
				return err
			}
		}
		if err := sink.Line(); err != nil {
			return err
		}
	}
	return nil
}

func (r *probeRenderer) Measure(options RenderOptions) (Measurement, error) {
	if r.measurement != nil {
		return *r.measurement, r.callbackErr
	}
	return Measurement{Minimum: 1, Maximum: options.MaxWidth}, r.callbackErr
}

type defaultMeasureRenderer struct{ probeRenderer }

func (r *defaultMeasureRenderer) Measure(RenderOptions) (Measurement, error) {
	panic("the embedded method is deliberately hidden through the wrapper below")
}

type renderOnly struct{ renderer *probeRenderer }

func (r renderOnly) Render(options RenderOptions, sink *SegmentSink) error {
	return r.renderer.Render(options, sink)
}

type partialFailureRenderer struct{}

func (partialFailureRenderer) Render(_ RenderOptions, sink *SegmentSink) error {
	if err := sink.Push("must-not-commit", "bold red"); err != nil {
		return err
	}
	return ErrIO
}

type poisonedSinkRenderer struct{}

func (poisonedSinkRenderer) Render(_ RenderOptions, sink *SegmentSink) error {
	_ = sink.Push("must-not-commit", "definitely-not-a-style")
	return nil
}

type closeAttemptRenderer struct {
	target *Renderable
	err    error
}

type blockingRenderer struct {
	entered   chan uintptr
	release   chan struct{}
	active    atomic.Int32
	maxActive atomic.Int32
}

func (r *blockingRenderer) Render(_ RenderOptions, sink *SegmentSink) error {
	active := r.active.Add(1)
	defer r.active.Add(-1)
	for {
		old := r.maxActive.Load()
		if active <= old || r.maxActive.CompareAndSwap(old, active) {
			break
		}
	}
	r.entered <- nativeThreadID()
	<-r.release
	return sink.Push("serialized", "")
}

func (r *closeAttemptRenderer) Render(_ RenderOptions, sink *SegmentSink) error {
	r.err = r.target.Close()
	return sink.Push("still-live", "")
}

func TestABIV2LayoutsConstantsAndInventory(t *testing.T) {
	requireNoError(t, EnsureABI())
	if ABIVersion != 2 || BoxNone != 14 || len(nativeSymbolInventory) != 125 {
		t.Fatalf("ABI=%d BoxNone=%d symbols=%d", ABIVersion, BoxNone, len(nativeSymbolInventory))
	}
	layout := nativeLayoutInfo()
	if layout.RenderOptionsSize != 24 || layout.MeasurementSize != 8 || layout.ColumnOptionsSize != 40 || layout.ColumnStyleOffset != 24 {
		t.Fatalf("v2 layouts: %+v", layout)
	}
}

func TestConfiguredTablePrettyAndSyntaxSurface(t *testing.T) {
	console := makePlainConsole(t)
	requireNoError(t, console.SetSize(44, 20))

	header := makeTextRenderable(t, "Header")
	table, err := NewGridTable()
	requireNoError(t, err)
	defer table.Close()
	requireNoError(t, table.SetBox(BoxNone))
	requireNoError(t, table.SetPadEdge(false))
	requireNoError(t, table.SetRowStyles("", "dim"))
	if err := table.SetRowStyles("dim", "definitely-not-a-style"); !errors.Is(err, ErrInvalidArgument) {
		t.Fatalf("invalid row style = %v", err)
	}
	requireNoError(t, table.SetRowStyles())
	requireNoError(t, table.SetRowStyles("", "dim"))
	width, minWidth, ratio := uint32(12), uint32(3), uint32(1)
	justify, vertical := JustifyRight, VerticalBottom
	requireNoError(t, table.AddColumnOptions(header, ColumnOptions{
		Justify: &justify, Vertical: &vertical, Width: &width, MinWidth: &minWidth,
		Ratio: &ratio, Style: "cyan", NoWrap: true,
	}))
	// The borrowed header remains independently usable.
	headerOut, err := console.Render(header)
	requireNoError(t, err)
	if headerOut != "Header" {
		t.Fatalf("header output = %q", headerOut)
	}
	zero := uint32(0)
	if err := table.AddColumnOptions(nil, ColumnOptions{Width: &zero}); !errors.Is(err, ErrInvalidArgument) {
		t.Fatalf("zero selected width = %v", err)
	}
	requireNoError(t, table.AddRow("value"))
	tableRenderable, err := table.Finish()
	requireNoError(t, err)
	defer tableRenderable.Close()
	if output, err := console.Render(tableRenderable); err != nil || !strings.Contains(output, "value") {
		t.Fatalf("configured table output=%q err=%v", output, err)
	}
	requireNoError(t, tableRenderable.Close())
	headerOut, err = console.Render(header)
	requireNoError(t, err)
	if headerOut != "Header" {
		t.Fatalf("header after table destruction = %q", headerOut)
	}
	panel, err := NewPanel(header)
	requireNoError(t, err)
	defer panel.Close()
	if err := panel.SetBox(BoxNone); !errors.Is(err, ErrInvalidArgument) {
		t.Fatalf("Panel accepted Table-only BoxNone: %v", err)
	}

	pretty, err := NewPretty("{'nul': 'a\x00b', 'items': [1, 2]}")
	requireNoError(t, err)
	prettyCopy := *pretty
	requireNoError(t, pretty.SetIndentGuides(true))
	prettyRenderable, err := pretty.Finish()
	requireNoError(t, err)
	defer prettyRenderable.Close()
	if _, err := prettyCopy.Finish(); !errors.Is(err, ErrInvalidState) {
		t.Fatalf("pretty copy remained live: %v", err)
	}
	requireNoError(t, pretty.Close())
	prettyOutput, err := console.Render(prettyRenderable)
	requireNoError(t, err)
	if !strings.Contains(prettyOutput, "items") {
		t.Fatalf("pretty output = %q", prettyOutput)
	}

	source := "if ready:\n    call()"
	without, err := NewSyntax(source, "python")
	requireNoError(t, err)
	defer without.Close()
	withoutRenderable, err := without.Finish()
	requireNoError(t, err)
	defer withoutRenderable.Close()
	with, err := NewSyntax(source, "python")
	requireNoError(t, err)
	defer with.Close()
	requireNoError(t, with.SetIndentGuides(true))
	withRenderable, err := with.Finish()
	requireNoError(t, err)
	defer withRenderable.Close()
	a, err := console.Render(withoutRenderable)
	requireNoError(t, err)
	b, err := console.Render(withRenderable)
	requireNoError(t, err)
	if a == b || !strings.Contains(b, "│") {
		t.Fatalf("syntax guide probe did not change output\nwithout=%q\nwith=%q", a, b)
	}
}

func TestCustomRenderableScopedSinkErrorsAndPanic(t *testing.T) {
	console := makePlainConsole(t)
	requireNoError(t, console.SetSize(9, 10))
	renderer := &probeRenderer{height: 1}
	renderer.reenter = func() error {
		_, err := console.Width()
		return err
	}
	custom, err := NewCustomRenderable(renderer)
	requireNoError(t, err)
	output, err := console.Render(custom)
	requireNoError(t, err)
	if strings.Count(output, "▄") != 9 {
		t.Fatalf("glyph count = %d output=%q", strings.Count(output, "▄"), output)
	}
	renderer.mu.Lock()
	retained := renderer.retained
	options := append([]RenderOptions(nil), renderer.options...)
	renderer.mu.Unlock()
	if len(options) == 0 || options[len(options)-1].MaxWidth != 9 || options[len(options)-1].IsTerminal {
		t.Fatalf("callback options = %+v", options)
	}
	if err := retained.Push("late", ""); !errors.Is(err, ErrInvalidState) {
		t.Fatalf("retained sink remained live: %v", err)
	}
	requireNoError(t, custom.Close())

	closeAttempt := &closeAttemptRenderer{}
	closeTarget, err := NewCustomRenderable(closeAttempt)
	requireNoError(t, err)
	closeAttempt.target = closeTarget
	closeOutput, err := console.Render(closeTarget)
	requireNoError(t, err)
	if closeOutput != "still-live" || !errors.Is(closeAttempt.err, ErrInvalidState) {
		t.Fatalf("callback Close output=%q error=%v", closeOutput, closeAttempt.err)
	}
	// The rejected destructor did not mutate the shared owner state.
	closeOutput, err = console.Render(closeTarget)
	requireNoError(t, err)
	if closeOutput != "still-live" {
		t.Fatalf("owner closed by guarded no-op: %q", closeOutput)
	}
	requireNoError(t, closeTarget.Close())

	for _, test := range []struct {
		name     string
		renderer *probeRenderer
		want     error
	}{
		{"error", &probeRenderer{height: 1, callbackErr: ErrInvalidArgument}, ErrInvalidArgument},
		{"panic", &probeRenderer{height: 1, panicRender: true}, ErrNativePanic},
		{"measurement", &probeRenderer{height: 1, measurement: &Measurement{Minimum: 5, Maximum: 2}}, ErrRender},
	} {
		t.Run(test.name, func(t *testing.T) {
			renderable, err := NewCustomRenderable(test.renderer)
			requireNoError(t, err)
			defer renderable.Close()
			target := renderable
			if test.name == "measurement" {
				table, tableErr := NewGridTable()
				requireNoError(t, tableErr)
				defer table.Close()
				requireNoError(t, table.AddRenderableRow(renderable))
				target, tableErr = table.Finish()
				requireNoError(t, tableErr)
				defer target.Close()
			}
			got, err := console.Render(target)
			if !errors.Is(err, test.want) || got != "" {
				t.Fatalf("output=%q error=%v want=%v", got, err, test.want)
			}
		})
	}

	defaultRenderer := &probeRenderer{height: 1}
	renderable, err := NewCustomRenderable(renderOnly{renderer: defaultRenderer})
	requireNoError(t, err)
	defer renderable.Close()
	_, err = console.Render(renderable)
	requireNoError(t, err)

	for _, transactional := range []struct {
		name     string
		renderer CustomRenderer
		want     error
	}{
		{"partial callback error", partialFailureRenderer{}, ErrIO},
		{"poisoned sink", poisonedSinkRenderer{}, ErrInvalidArgument},
	} {
		t.Run(transactional.name, func(t *testing.T) {
			renderable, err := NewCustomRenderable(transactional.renderer)
			requireNoError(t, err)
			defer renderable.Close()
			output, err := console.Render(renderable)
			if output != "" || !errors.Is(err, transactional.want) {
				t.Fatalf("transaction committed output=%q err=%v", output, err)
			}
		})
	}
}

func TestCustomRenderableLastArcDropAndConcurrentSerialization(t *testing.T) {
	var drops atomic.Int32
	customDropHook.Lock()
	previous := customDropHook.fn
	customDropHook.fn = func() { drops.Add(1) }
	customDropHook.Unlock()
	t.Cleanup(func() {
		customDropHook.Lock()
		customDropHook.fn = previous
		customDropHook.Unlock()
	})

	renderer := &probeRenderer{height: 1}
	custom, err := NewCustomRenderable(renderer)
	requireNoError(t, err)
	table, err := NewGridTable()
	requireNoError(t, err)
	requireNoError(t, table.AddRenderableRow(custom))
	requireNoError(t, custom.Close())
	if drops.Load() != 0 {
		t.Fatalf("drop ran while table retained custom: %d", drops.Load())
	}
	tableRenderable, err := table.Finish()
	requireNoError(t, err)
	requireNoError(t, table.Close())
	console := makePlainConsole(t)
	requireNoError(t, console.SetSize(13, 10))

	var wait sync.WaitGroup
	for i := 0; i < 8; i++ {
		wait.Add(1)
		go func() {
			defer wait.Done()
			if _, renderErr := console.Render(tableRenderable); renderErr != nil {
				t.Errorf("concurrent render: %v", renderErr)
			}
		}()
	}
	wait.Wait()
	if renderer.maxActive.Load() != 1 {
		t.Fatalf("callbacks overlapped: max=%d", renderer.maxActive.Load())
	}
	requireNoError(t, tableRenderable.Close())
	if drops.Load() != 1 {
		t.Fatalf("last-Arc drop count = %d", drops.Load())
	}
	requireNoError(t, tableRenderable.Close())
	if drops.Load() != 1 {
		t.Fatalf("idempotent close drop count = %d", drops.Load())
	}
}

func TestCustomRenderableNativeSerializationAcrossRetainedArcs(t *testing.T) {
	var drops atomic.Int32
	customDropHook.Lock()
	previousDrop := customDropHook.fn
	customDropHook.fn = func() { drops.Add(1) }
	customDropHook.Unlock()
	t.Cleanup(func() {
		customDropHook.Lock()
		customDropHook.fn = previousDrop
		customDropHook.Unlock()
	})

	renderer := &blockingRenderer{
		entered: make(chan uintptr, 2),
		release: make(chan struct{}),
	}
	custom, err := NewCustomRenderable(renderer)
	requireNoError(t, err)
	retainInContainer := func() *Renderable {
		t.Helper()
		table, tableErr := NewGridTable()
		requireNoError(t, tableErr)
		requireNoError(t, table.AddRenderableRow(custom))
		container, finishErr := table.Finish()
		requireNoError(t, finishErr)
		requireNoError(t, table.Close())
		return container
	}
	firstContainer := retainInContainer()
	secondContainer := retainInContainer()
	requireNoError(t, custom.Close())
	if drops.Load() != 0 {
		t.Fatalf("adapter dropped while containers retain it: %d", drops.Load())
	}

	firstConsole := makePlainConsole(t)
	secondConsole := makePlainConsole(t)
	requireNoError(t, firstConsole.SetSize(24, 10))
	requireNoError(t, secondConsole.SetSize(24, 10))
	type nativeAttempt struct{ console, renderable nativeHandle }
	attempted := make(chan nativeAttempt, 2)
	consoleRenderBeforeHook = func(console, renderable nativeHandle) {
		attempted <- nativeAttempt{console: console, renderable: renderable}
	}
	t.Cleanup(func() { consoleRenderBeforeHook = nil })

	type renderResult struct {
		output string
		err    error
	}
	firstResult := make(chan renderResult, 1)
	secondResult := make(chan renderResult, 1)
	go func() {
		output, renderErr := firstConsole.Render(firstContainer)
		firstResult <- renderResult{output: output, err: renderErr}
	}()
	firstAttempt := <-attempted
	firstThread := <-renderer.entered
	go func() {
		output, renderErr := secondConsole.Render(secondContainer)
		secondResult <- renderResult{output: output, err: renderErr}
	}()
	secondAttempt := <-attempted
	if firstAttempt.console.equal(secondAttempt.console) || firstAttempt.renderable.equal(secondAttempt.renderable) {
		t.Fatal("probe did not use independent console and retained-container owners")
	}
	select {
	case thread := <-renderer.entered:
		t.Fatalf("same adapter callbacks overlapped on native threads %#x and %#x", firstThread, thread)
	case <-time.After(200 * time.Millisecond):
	}

	renderer.release <- struct{}{}
	secondThread := <-renderer.entered
	if secondThread == firstThread {
		t.Fatalf("both native calls used thread %#x; foreign-thread coverage was not established", firstThread)
	}
	if renderer.maxActive.Load() != 1 {
		t.Fatalf("callbacks overlapped: max=%d", renderer.maxActive.Load())
	}

	closeResults := make(chan error, 2)
	go func() { closeResults <- firstContainer.Close() }()
	go func() { closeResults <- secondContainer.Close() }()
	closedBeforeRelease := 0
	select {
	case <-time.After(100 * time.Millisecond):
	case err := <-closeResults:
		requireNoError(t, err)
		closedBeforeRelease = 1
	}
	if drops.Load() != 0 {
		t.Fatalf("last Arc dropped during an active callback: %d", drops.Load())
	}
	renderer.release <- struct{}{}

	for index, resultChannel := range []<-chan renderResult{firstResult, secondResult} {
		result := <-resultChannel
		if result.err != nil || !strings.Contains(result.output, "serialized") {
			t.Fatalf("render %d output=%q err=%v", index, result.output, result.err)
		}
	}
	for i := closedBeforeRelease; i < 2; i++ {
		requireNoError(t, <-closeResults)
	}
	if drops.Load() != 1 {
		t.Fatalf("last-Arc drop count after callback quiescence = %d", drops.Load())
	}
	requireNoError(t, firstContainer.Close())
	requireNoError(t, secondContainer.Close())
	if drops.Load() != 1 {
		t.Fatalf("idempotent closes changed drop count: %d", drops.Load())
	}
}

func TestCustomConstructorReceiptReconciliation(t *testing.T) {
	tests := []struct {
		name           string
		result         uint32
		createOutput   bool
		suppress       bool
		want           error
		wantSource     string
		wantOutputFree int
		wantSuccess    bool
	}{
		{name: "known error without output", result: uint32(ErrorInvalidArgument), want: ErrInvalidArgument, wantSource: "wrapper"},
		{name: "error with output", result: uint32(ErrorInvalidArgument), createOutput: true, want: ErrABIMismatch, wantSource: "native-drop", wantOutputFree: 1},
		{name: "success without output", result: 0, createOutput: true, suppress: true, want: ErrABIMismatch, wantSource: "native-drop", wantOutputFree: 1},
		{name: "success with output", result: 0, createOutput: true, wantSource: "native-drop", wantOutputFree: 1, wantSuccess: true},
	}
	for _, test := range tests {
		t.Run(test.name, func(t *testing.T) {
			var mu sync.Mutex
			var token uintptr
			var output nativeHandle
			var deleteSources []string
			var invalidatedTokens int
			var outputFrees int
			customNewBeforeHook = func(gotToken uintptr, _ bool) *customNewTestOverride {
				mu.Lock()
				token = gotToken
				mu.Unlock()
				return &customNewTestOverride{
					result: test.result, createOutput: test.createOutput, suppressOutput: test.suppress,
					observeOutput: func(handle nativeHandle) {
						mu.Lock()
						output = handle
						mu.Unlock()
					},
				}
			}
			customHandleDeleteHook.Lock()
			previousDelete := customHandleDeleteHook.fn
			customHandleDeleteHook.fn = func(gotToken uintptr, source string) {
				deleted := false
				func() {
					defer func() { deleted = recover() != nil }()
					_ = cgo.Handle(gotToken).Value()
				}()
				mu.Lock()
				defer mu.Unlock()
				if gotToken == token {
					deleteSources = append(deleteSources, source)
					if deleted {
						invalidatedTokens++
					}
				}
			}
			customHandleDeleteHook.Unlock()
			previousFree := replaceNativeHandleFreedHook(nil)
			replaceNativeHandleFreedHook(func(kind handleKind, handle nativeHandle) {
				mu.Lock()
				defer mu.Unlock()
				if kind == hRenderable && !output.isNil() && handle.equal(output) {
					outputFrees++
				}
			})
			t.Cleanup(func() {
				customNewBeforeHook = nil
				customHandleDeleteHook.Lock()
				customHandleDeleteHook.fn = previousDelete
				customHandleDeleteHook.Unlock()
				replaceNativeHandleFreedHook(previousFree)
			})

			renderable, err := NewCustomRenderable(renderOnly{renderer: &probeRenderer{height: 1}})
			if test.wantSuccess {
				requireNoError(t, err)
				if renderable == nil {
					t.Fatal("successful receipt returned no owner")
				}
				mu.Lock()
				prematureDeletes := append([]string(nil), deleteSources...)
				prematureFrees := outputFrees
				mu.Unlock()
				if len(prematureDeletes) != 0 || prematureFrees != 0 {
					t.Fatalf("ownership released before Close: deletes=%v frees=%d", prematureDeletes, prematureFrees)
				}
				requireNoError(t, renderable.Close())
				requireNoError(t, renderable.Close())
			} else if renderable != nil || !errors.Is(err, test.want) {
				t.Fatalf("renderable=%v err=%v want=%v", renderable, err, test.want)
			}

			mu.Lock()
			defer mu.Unlock()
			if token == 0 {
				t.Fatal("constructor did not receive numeric cgo.Handle token")
			}
			if len(deleteSources) != 1 || deleteSources[0] != test.wantSource {
				t.Fatalf("handle delete attempts=%v want=[%s]", deleteSources, test.wantSource)
			}
			if invalidatedTokens != 1 {
				t.Fatalf("invalid cgo.Handle receipts=%d want=1", invalidatedTokens)
			}
			if outputFrees != test.wantOutputFree {
				t.Fatalf("native output frees=%d want=%d output=%v", outputFrees, test.wantOutputFree, output)
			}
			if test.createOutput && output.isNil() {
				t.Fatal("fault seam did not create a real native output")
			}
		})
	}
}

func TestAntiFakeAdaptiveCustomPrettyAndTableProbes(t *testing.T) {
	for _, height := range []uint32{1, 3} {
		for _, width := range []uint32{7, 11} {
			console := makePlainConsole(t)
			requireNoError(t, console.SetForceTerminal(true))
			requireNoError(t, console.SetColorSystem(ColorTruecolor))
			requireNoError(t, console.SetSize(width, 10))
			renderer := &probeRenderer{height: height}
			custom, err := NewCustomRenderable(renderer)
			requireNoError(t, err)
			output, err := console.Render(custom)
			requireNoError(t, err)
			if strings.Count(output, "▄") != int(height*width) || !strings.Contains(output, "\x1b[38;2;") {
				t.Fatalf("height=%d width=%d output=%q", height, width, output)
			}
			requireNoError(t, custom.Close())
			requireNoError(t, console.Close())
		}
	}
	for _, probe := range []struct {
		repr, key string
	}{
		{repr: "{'alpha': [1, 2, 3, 4, 5, 6]}", key: "alpha"},
		{repr: "{'café': {'値': ['雪\x00山', True, None]}}", key: "café"},
	} {
		outputs := make(map[bool]string, 2)
		for _, guides := range []bool{false, true} {
			pretty, err := NewPretty(probe.repr)
			requireNoError(t, err)
			requireNoError(t, pretty.SetIndentGuides(guides))
			renderable, err := pretty.Finish()
			requireNoError(t, err)
			console := makePlainConsole(t)
			requireNoError(t, console.SetSize(20, 20))
			output, err := console.Render(renderable)
			requireNoError(t, err)
			if !strings.Contains(output, probe.key) {
				t.Fatalf("pretty probe %q => %q", probe.repr, output)
			}
			outputs[guides] = output
			requireNoError(t, renderable.Close())
			requireNoError(t, pretty.Close())
			requireNoError(t, console.Close())
		}
		if outputs[false] == outputs[true] {
			t.Fatalf("indent guides did not affect Pretty output for %q: %q", probe.repr, outputs[true])
		}
		if strings.Contains(probe.repr, "\x00") && (!strings.Contains(outputs[false], "\x00") || !strings.Contains(outputs[true], "\x00")) {
			t.Fatalf("embedded NUL was not preserved for %q: off=%q on=%q", probe.repr, outputs[false], outputs[true])
		}
	}
}

func TestPrettyMalformedFinishReceiptRetainsOriginal(t *testing.T) {
	pretty, err := NewPretty("{'original': true}")
	requireNoError(t, err)
	copyView := *pretty
	original := pretty.state.h
	replacement, err := nativePrettyNew("{'replacement': true}")
	requireNoError(t, err)
	var freed []nativeHandle
	oldFreeHook := replaceNativeHandleFreedHook(nil)
	replaceNativeHandleFreedHook(func(kind handleKind, handle nativeHandle) {
		if kind == hPretty {
			freed = append(freed, handle)
		}
	})
	finishBeforeHook = func(kind handleKind) *finishTestOverride {
		if kind != hPretty {
			t.Fatalf("finish kind = %v", kind)
		}
		return &finishTestOverride{
			result: uint32(ErrorInvalidArgument), replaceRemaining: true, remaining: replacement,
		}
	}
	t.Cleanup(func() {
		finishBeforeHook = nil
		replaceNativeHandleFreedHook(oldFreeHook)
	})
	renderable, err := copyView.Finish()
	finishBeforeHook = nil
	if renderable != nil || !errors.Is(err, ErrABIMismatch) {
		t.Fatalf("renderable=%v err=%v", renderable, err)
	}
	if len(freed) != 1 || !freed[0].equal(replacement) {
		t.Fatalf("replacement frees = %v", freed)
	}
	if !pretty.state.h.equal(original) {
		t.Fatal("original Pretty owner was not retained")
	}
	requireNoError(t, pretty.SetIndentGuides(true))
	requireNoError(t, pretty.Close())
	if len(freed) != 2 || !freed[1].equal(original) {
		t.Fatalf("final frees = %v", freed)
	}
	if err := copyView.SetIndentGuides(false); !errors.Is(err, ErrInvalidState) {
		t.Fatalf("copy remained live: %v", err)
	}
}

func TestPrettyAndCustomCloseCallFinishRaces(t *testing.T) {
	for iteration := 0; iteration < 20; iteration++ {
		pretty, err := NewPretty("{'race': [1, 2, 3]}")
		requireNoError(t, err)
		copyView := *pretty
		finished := make(chan *Renderable, 1)
		finishError := make(chan error, 1)
		go func() {
			renderable, finishErr := copyView.Finish()
			finished <- renderable
			finishError <- finishErr
		}()
		go func() { _ = pretty.Close() }()
		renderable := <-finished
		err = <-finishError
		if err != nil && !errors.Is(err, ErrInvalidState) {
			t.Fatalf("Pretty close/finish: %v", err)
		}
		if renderable != nil {
			requireNoError(t, renderable.Close())
		}
		requireNoError(t, pretty.Close())

		console := makePlainConsole(t)
		custom, err := NewCustomRenderable(renderOnly{renderer: &probeRenderer{height: 1}})
		requireNoError(t, err)
		customCopy := *custom
		var wait sync.WaitGroup
		for i := 0; i < 4; i++ {
			wait.Add(1)
			go func() {
				defer wait.Done()
				for j := 0; j < 10; j++ {
					_, renderErr := console.Render(&customCopy)
					if renderErr != nil && !errors.Is(renderErr, ErrInvalidState) {
						t.Errorf("custom render/close: %v", renderErr)
						return
					}
				}
			}()
		}
		wait.Add(1)
		go func() { defer wait.Done(); _ = custom.Close() }()
		wait.Wait()
		requireNoError(t, customCopy.Close())
	}
}

func TestTableOptionsChangeLayoutIndependently(t *testing.T) {
	console, err := NewConsole()
	requireNoError(t, err)
	defer console.Close()
	requireNoError(t, console.SetForceTerminal(true))
	requireNoError(t, console.SetColorSystem(ColorTruecolor))
	requireNoError(t, console.SetSize(32, 20))

	render := func(configure func(*Table) error, columns []ColumnOptions, rows ...[]string) string {
		t.Helper()
		table, err := NewTable()
		requireNoError(t, err)
		defer table.Close()
		requireNoError(t, table.SetShowHeader(false))
		requireNoError(t, table.SetExpand(true))
		if configure != nil {
			requireNoError(t, configure(table))
		}
		for _, column := range columns {
			requireNoError(t, table.AddColumnOptions(nil, column))
		}
		for _, row := range rows {
			requireNoError(t, table.AddRow(row...))
		}
		renderable, err := table.Finish()
		requireNoError(t, err)
		defer renderable.Close()
		output, err := console.Render(renderable)
		requireNoError(t, err)
		return output
	}

	left, right := JustifyLeft, JustifyRight
	width8, width12, min4, min10, ratio1, ratio3 := uint32(8), uint32(12), uint32(4), uint32(10), uint32(1), uint32(3)
	probes := []struct {
		name string
		a, b string
	}{
		{"none box", render(func(table *Table) error { return table.SetBox(BoxNone) }, []ColumnOptions{{}}, []string{"x"}), render(func(table *Table) error { return table.SetBox(BoxSimple) }, []ColumnOptions{{}}, []string{"x"})},
		{"pad edge", render(func(table *Table) error {
			if err := table.SetBox(BoxNone); err != nil {
				return err
			}
			return table.SetPadEdge(false)
		}, []ColumnOptions{{}}, []string{"x"}), render(func(table *Table) error {
			if err := table.SetBox(BoxNone); err != nil {
				return err
			}
			return table.SetPadEdge(true)
		}, []ColumnOptions{{}}, []string{"x"})},
		{"row styles", render(func(table *Table) error {
			if err := table.SetBox(BoxNone); err != nil {
				return err
			}
			return table.SetRowStyles("red", "blue")
		}, []ColumnOptions{{}}, []string{"first"}, []string{"second"}), render(func(table *Table) error {
			if err := table.SetBox(BoxNone); err != nil {
				return err
			}
			return table.SetRowStyles("green", "blue")
		}, []ColumnOptions{{}}, []string{"first"}, []string{"second"})},
		{"justify", render(func(table *Table) error { return table.SetBox(BoxNone) }, []ColumnOptions{{Width: &width8, Justify: &left}}, []string{"x"}), render(func(table *Table) error { return table.SetBox(BoxNone) }, []ColumnOptions{{Width: &width8, Justify: &right}}, []string{"x"})},
		{"fixed width", render(func(table *Table) error {
			if err := table.SetBox(BoxNone); err != nil {
				return err
			}
			return table.SetExpand(false)
		}, []ColumnOptions{{Width: &width8}}, []string{"fixed"}), render(func(table *Table) error {
			if err := table.SetBox(BoxNone); err != nil {
				return err
			}
			return table.SetExpand(false)
		}, []ColumnOptions{{Width: &width12}}, []string{"fixed"})},
		{"minimum width", render(func(table *Table) error {
			if err := table.SetBox(BoxNone); err != nil {
				return err
			}
			return table.SetExpand(false)
		}, []ColumnOptions{{MinWidth: &min4}}, []string{"x"}), render(func(table *Table) error {
			if err := table.SetBox(BoxNone); err != nil {
				return err
			}
			return table.SetExpand(false)
		}, []ColumnOptions{{MinWidth: &min10}}, []string{"x"})},
		{"ratio", render(func(table *Table) error { return table.SetBox(BoxNone) }, []ColumnOptions{{Ratio: &ratio1}, {Ratio: &ratio1}}, []string{"left", "right"}), render(func(table *Table) error { return table.SetBox(BoxNone) }, []ColumnOptions{{Ratio: &ratio1}, {Ratio: &ratio3}}, []string{"left", "right"})},
		{"no wrap", render(func(table *Table) error {
			if err := table.SetBox(BoxNone); err != nil {
				return err
			}
			return table.SetExpand(false)
		}, []ColumnOptions{{Width: &width8}}, []string{"a long cell value"}), render(func(table *Table) error {
			if err := table.SetBox(BoxNone); err != nil {
				return err
			}
			return table.SetExpand(false)
		}, []ColumnOptions{{Width: &width8, NoWrap: true}}, []string{"a long cell value"})},
	}
	for _, probe := range probes {
		if probe.a == probe.b {
			t.Errorf("%s did not change rendered bytes: %q", probe.name, probe.a)
		}
	}
}
