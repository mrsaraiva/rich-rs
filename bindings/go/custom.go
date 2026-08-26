//go:build cgo

package rich

/*
#include <rich.h>
*/
import "C"

import (
	"errors"
	"runtime/cgo"
	"sync"
	"unsafe"
)

// CustomRenderer supplies segments synchronously during a native render.
// Implementations must not call other rich APIs from Render.
type CustomRenderer interface {
	Render(RenderOptions, *SegmentSink) error
}

// CustomMeasurer may be implemented by a CustomRenderer to override native's
// default measurement of {1, MaxWidth}.
type CustomMeasurer interface {
	Measure(RenderOptions) (Measurement, error)
}

type customRendererContext struct {
	renderer CustomRenderer
}

var customDropHook struct {
	sync.RWMutex
	fn func()
}

var customHandleDeleteHook struct {
	sync.RWMutex
	fn func(token uintptr, source string)
}

func deleteCustomHandle(handle cgo.Handle, source string) {
	handle.Delete()
	customHandleDeleteHook.RLock()
	hook := customHandleDeleteHook.fn
	customHandleDeleteHook.RUnlock()
	if hook != nil {
		hook(uintptr(handle), source)
	}
}

// SegmentSink is valid only for the dynamic extent of one Render callback.
// Its methods copy their inputs into native staging storage.
type SegmentSink struct {
	mu     sync.Mutex
	native *C.RichSegmentSink
	active bool
}

func (s *SegmentSink) invalidate() {
	if s == nil {
		return
	}
	s.mu.Lock()
	s.native = nil
	s.active = false
	s.mu.Unlock()
}

// Push stages a text segment with a native style expression. An empty style is
// unstyled. A failed push poisons the native transaction.
func (s *SegmentSink) Push(text, style string) error {
	if s == nil {
		return ErrInvalidState
	}
	s.mu.Lock()
	defer s.mu.Unlock()
	if !s.active || s.native == nil {
		return ErrInvalidState
	}
	nativeText, err := allocNativeString(text)
	if err != nil {
		return err
	}
	defer nativeText.free()
	nativeStyle, err := allocNativeString(style)
	if err != nil {
		return err
	}
	defer nativeStyle.free()
	return checkResult(uint32(C.rich_segment_sink_push(s.native, nativeText.v, nativeStyle.v)))
}

// Line stages one native newline segment.
func (s *SegmentSink) Line() error {
	if s == nil {
		return ErrInvalidState
	}
	s.mu.Lock()
	defer s.mu.Unlock()
	if !s.active || s.native == nil {
		return ErrInvalidState
	}
	return checkResult(uint32(C.rich_segment_sink_line(s.native)))
}

// NewCustomRenderable registers a synchronous renderer. Native code retains
// only a numeric runtime/cgo.Handle token and deletes it on the last native Arc
// drop; no Go pointer crosses into retained native memory.
func NewCustomRenderable(renderer CustomRenderer) (*Renderable, error) {
	if renderer == nil {
		return nil, ErrInvalidArgument
	}
	context := &customRendererContext{renderer: renderer}
	handle := cgo.NewHandle(context)
	_, hasMeasure := renderer.(CustomMeasurer)
	native, transferred, err := nativeCustomRenderableNew(uintptr(handle), hasMeasure)
	if err != nil {
		if !transferred {
			deleteCustomHandle(handle, "wrapper")
		}
		return nil, err
	}
	return &Renderable{state: newOwnerState(hRenderable, native)}, nil
}

func renderOptionsFromNative(options *C.RichRenderOptions) (RenderOptions, error) {
	if options == nil {
		return RenderOptions{}, ErrNullArgument
	}
	for _, reserved := range options.reserved {
		if reserved != 0 {
			return RenderOptions{}, ErrABIMismatch
		}
	}
	color := ColorSystem(options.color_system)
	if color > ColorWindows {
		return RenderOptions{}, ErrABIMismatch
	}
	if options.is_terminal > 1 {
		return RenderOptions{}, ErrABIMismatch
	}
	return RenderOptions{
		MinWidth: uint32(options.min_width), MaxWidth: uint32(options.max_width),
		MaxHeight: uint32(options.max_height), ColorSystem: color,
		IsTerminal: options.is_terminal == 1,
	}, nil
}

func callbackResult(err error) C.RichResult {
	if err == nil {
		return C.RichResult(C.RICH_OK)
	}
	for _, pair := range []struct {
		target error
		code   ErrorCode
	}{
		{ErrNullArgument, ErrorNullArgument}, {ErrInvalidUTF8, ErrorInvalidUTF8},
		{ErrInvalidArgument, ErrorInvalidArgument}, {ErrInvalidState, ErrorInvalidState},
		{ErrIO, ErrorIO}, {ErrRender, ErrorRender}, {ErrNativePanic, ErrorNativePanic},
	} {
		if errors.Is(err, pair.target) {
			return C.RichResult(pair.code)
		}
	}
	return C.RichResult(ErrorRender)
}

func customContext(context unsafe.Pointer) (*customRendererContext, error) {
	if context == nil {
		return nil, ErrInvalidState
	}
	handle := cgo.Handle(uintptr(context))
	value := handle.Value()
	state, ok := value.(*customRendererContext)
	if !ok || state == nil || state.renderer == nil {
		return nil, ErrInvalidState
	}
	return state, nil
}

//export goRichCustomRender
func goRichCustomRender(context unsafe.Pointer, nativeOptions *C.RichRenderOptions, nativeSink *C.RichSegmentSink) (result C.RichResult) {
	result = C.RichResult(C.RICH_ERROR_PANIC)
	defer func() {
		if recover() != nil {
			result = C.RichResult(C.RICH_ERROR_PANIC)
		}
	}()
	state, err := customContext(context)
	if err != nil {
		return callbackResult(err)
	}
	options, err := renderOptionsFromNative(nativeOptions)
	if err != nil {
		return callbackResult(err)
	}
	if nativeSink == nil {
		return C.RichResult(C.RICH_ERROR_NULL_ARGUMENT)
	}
	sink := &SegmentSink{native: nativeSink, active: true}
	defer sink.invalidate()
	return callbackResult(state.renderer.Render(options, sink))
}

//export goRichCustomMeasure
func goRichCustomMeasure(context unsafe.Pointer, nativeOptions *C.RichRenderOptions, out *C.RichMeasurement) (result C.RichResult) {
	result = C.RichResult(C.RICH_ERROR_PANIC)
	defer func() {
		if recover() != nil {
			result = C.RichResult(C.RICH_ERROR_PANIC)
		}
	}()
	if out == nil {
		return C.RichResult(C.RICH_ERROR_NULL_ARGUMENT)
	}
	out.minimum = 0
	out.maximum = 0
	state, err := customContext(context)
	if err != nil {
		return callbackResult(err)
	}
	options, err := renderOptionsFromNative(nativeOptions)
	if err != nil {
		return callbackResult(err)
	}
	measurer, ok := state.renderer.(CustomMeasurer)
	if !ok {
		return C.RichResult(C.RICH_ERROR_INVALID_STATE)
	}
	measurement, err := measurer.Measure(options)
	if err != nil {
		return callbackResult(err)
	}
	out.minimum = C.uint32_t(measurement.Minimum)
	out.maximum = C.uint32_t(measurement.Maximum)
	return C.RichResult(C.RICH_OK)
}

//export goRichCustomDrop
func goRichCustomDrop(context unsafe.Pointer) {
	defer func() { _ = recover() }()
	if context != nil {
		deleteCustomHandle(cgo.Handle(uintptr(context)), "native-drop")
	}
	customDropHook.RLock()
	hook := customDropHook.fn
	customDropHook.RUnlock()
	if hook != nil {
		hook()
	}
}
