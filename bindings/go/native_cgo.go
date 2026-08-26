//go:build cgo

package rich

/*
#cgo linux LDFLAGS: -lgcc_s -lutil -lrt -lpthread -lm -ldl -lc
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>
#include <rich.h>

static size_t go_rich_result_size(void) { return sizeof(RichResult); }
static size_t go_rich_str_size(void) { return sizeof(RichStr); }
static size_t go_rich_str_align(void) { return _Alignof(RichStr); }
static size_t go_rich_str_data_offset(void) { return offsetof(RichStr, data); }
static size_t go_rich_str_len_offset(void) { return offsetof(RichStr, len); }
static size_t go_rich_buffer_size(void) { return sizeof(RichBuffer); }
static size_t go_rich_buffer_align(void) { return _Alignof(RichBuffer); }
static size_t go_rich_buffer_data_offset(void) { return offsetof(RichBuffer, data); }
static size_t go_rich_buffer_len_offset(void) { return offsetof(RichBuffer, len); }
*/
import "C"

import (
	"errors"
	"math"
	"runtime"
	"sync"
	"unicode/utf8"
	"unsafe"
)

type nativeHandle struct {
	p           unsafe.Pointer
	testAddress unsafe.Pointer
}

func (h nativeHandle) isNil() bool { return h.p == nil }
func (h nativeHandle) address() unsafe.Pointer {
	if h.testAddress != nil {
		return h.testAddress
	}
	return h.p
}
func (h nativeHandle) equal(o nativeHandle) bool { return h.address() == o.address() }

type nativeLayout struct {
	ResultSize                                                 uintptr
	StrSize, StrAlign, StrDataOffset, StrLenOffset             uintptr
	BufferSize, BufferAlign, BufferDataOffset, BufferLenOffset uintptr
}

func nativeLayoutInfo() nativeLayout {
	return nativeLayout{
		ResultSize: uintptr(C.go_rich_result_size()),
		StrSize:    uintptr(C.go_rich_str_size()), StrAlign: uintptr(C.go_rich_str_align()),
		StrDataOffset: uintptr(C.go_rich_str_data_offset()), StrLenOffset: uintptr(C.go_rich_str_len_offset()),
		BufferSize: uintptr(C.go_rich_buffer_size()), BufferAlign: uintptr(C.go_rich_buffer_align()),
		BufferDataOffset: uintptr(C.go_rich_buffer_data_offset()), BufferLenOffset: uintptr(C.go_rich_buffer_len_offset()),
	}
}

func EnsureABI() error {
	if uint32(C.RICH_FFI_ABI_VERSION) != ABIVersion || uint32(C.rich_ffi_abi_version()) != ABIVersion {
		return ErrABIMismatch
	}
	l := nativeLayoutInfo()
	if l.ResultSize != 4 || l.StrSize != 16 || l.StrAlign != 8 || l.StrDataOffset != 0 || l.StrLenOffset != 8 ||
		l.BufferSize != 16 || l.BufferAlign != 8 || l.BufferDataOffset != 0 || l.BufferLenOffset != 8 {
		return ErrABIMismatch
	}
	if uint32(C.RICH_OK) != 0 || uint32(C.RICH_ERROR_NULL_ARGUMENT) != uint32(ErrorNullArgument) ||
		uint32(C.RICH_ERROR_INVALID_UTF8) != uint32(ErrorInvalidUTF8) ||
		uint32(C.RICH_ERROR_INVALID_ARGUMENT) != uint32(ErrorInvalidArgument) ||
		uint32(C.RICH_ERROR_INVALID_STATE) != uint32(ErrorInvalidState) ||
		uint32(C.RICH_ERROR_IO) != uint32(ErrorIO) || uint32(C.RICH_ERROR_RENDER) != uint32(ErrorRender) ||
		uint32(C.RICH_ERROR_PANIC) != uint32(ErrorNativePanic) ||
		uint32(C.RICH_COLOR_NONE) != uint32(ColorNone) || uint32(C.RICH_COLOR_STANDARD) != uint32(ColorStandard) ||
		uint32(C.RICH_COLOR_EIGHT_BIT) != uint32(ColorEightBit) || uint32(C.RICH_COLOR_TRUECOLOR) != uint32(ColorTruecolor) ||
		uint32(C.RICH_COLOR_WINDOWS) != uint32(ColorWindows) || uint32(C.RICH_ALIGN_LEFT) != uint32(AlignLeft) ||
		uint32(C.RICH_ALIGN_CENTER) != uint32(AlignCenter) || uint32(C.RICH_ALIGN_RIGHT) != uint32(AlignRight) ||
		uint32(C.RICH_VERTICAL_TOP) != uint32(VerticalTop) || uint32(C.RICH_VERTICAL_MIDDLE) != uint32(VerticalMiddle) ||
		uint32(C.RICH_VERTICAL_BOTTOM) != uint32(VerticalBottom) || uint64(C.RICH_TREE_ROOT_ID) != uint64(RootNodeID) {
		return ErrABIMismatch
	}
	return nil
}

type nativeString struct {
	v   C.RichStr
	mem unsafe.Pointer
}

func allocNativeString(s string) (nativeString, error) {
	if nativeStringAllocationHook != nil {
		if err := nativeStringAllocationHook(s); err != nil {
			return nativeString{}, err
		}
	}
	if len(s) == 0 {
		return nativeString{}, nil
	}
	mem := C.CBytes([]byte(s))
	if mem == nil {
		return nativeString{}, errors.New("rich: native input allocation failed")
	}
	if nativeStringLifetimeHook != nil {
		nativeStringLifetimeHook(true)
	}
	return nativeString{v: C.RichStr{data: (*C.uint8_t)(mem), len: C.uint64_t(len(s))}, mem: mem}, nil
}

func (s *nativeString) free() {
	if s.mem != nil {
		C.free(s.mem)
		if nativeStringLifetimeHook != nil {
			nativeStringLifetimeHook(false)
		}
		s.mem = nil
	}
}

var copyBufferBytes = func(data []byte) (string, error) { return string(data), nil }
var bufferFreedHook func(dataNil bool, length uint64)
var bufferResultHook func(uint32) uint32
var bufferLayoutHook func(dataNil bool, length uint64) (bool, uint64)
var nativeStringAllocationHook func(string) error
var nativeStringLifetimeHook func(allocated bool)
var nativeArrayLifetimeHook func(kind string, allocated bool)
var nativeFreedHook func(handleKind)
var nativeHandleFreedHook func(handleKind, nativeHandle)
var nativeFreeHooksMu sync.RWMutex

func replaceNativeFreedHook(hook func(handleKind)) func(handleKind) {
	nativeFreeHooksMu.Lock()
	previous := nativeFreedHook
	nativeFreedHook = hook
	nativeFreeHooksMu.Unlock()
	return previous
}

func replaceNativeHandleFreedHook(hook func(handleKind, nativeHandle)) func(handleKind, nativeHandle) {
	nativeFreeHooksMu.Lock()
	previous := nativeHandleFreedHook
	nativeHandleFreedHook = hook
	nativeFreeHooksMu.Unlock()
	return previous
}

type finishTestOverride struct {
	result           uint32
	consume          bool
	replaceRemaining bool
	remaining        nativeHandle
	setOutput        bool
	output           nativeHandle
}

var finishBeforeHook func(handleKind) *finishTestOverride

func nativeBufferResult(call func(*C.RichBuffer) C.RichResult) (out string, err error) {
	var buffer C.RichBuffer
	result := call(&buffer)
	defer func() {
		C.rich_buffer_free(&buffer)
		if bufferFreedHook != nil {
			bufferFreedHook(buffer.data == nil, uint64(buffer.len))
		}
	}()
	resultCode := uint32(result)
	if bufferResultHook != nil {
		resultCode = bufferResultHook(resultCode)
	}
	if e := checkResult(resultCode); e != nil {
		return "", e
	}
	dataNil, length := buffer.data == nil, uint64(buffer.len)
	if bufferLayoutHook != nil {
		dataNil, length = bufferLayoutHook(dataNil, length)
	}
	if length == 0 {
		if !dataNil {
			return "", ErrABIMismatch
		}
		return copyBufferBytes(nil)
	}
	if dataNil || length > uint64(maxInt()) {
		return "", ErrABIMismatch
	}
	data := unsafe.Slice((*byte)(unsafe.Pointer(buffer.data)), int(length))
	if !utf8.Valid(data) {
		return "", ErrABIMismatch
	}
	out, err = copyBufferBytes(data)
	runtime.KeepAlive(buffer)
	return out, err
}

func maxInt() int { return int(^uint(0) >> 1) }

func Version() (string, error) {
	if err := EnsureABI(); err != nil {
		return "", err
	}
	return nativeBufferResult(func(out *C.RichBuffer) C.RichResult { return C.rich_ffi_version(out) })
}

func nativeFree(kind handleKind, h nativeHandle) {
	if h.isNil() {
		return
	}
	switch kind {
	case hConsole:
		C.rich_console_free((*C.RichConsole)(h.p))
	case hRenderable:
		C.rich_renderable_free((*C.RichRenderable)(h.p))
	case hText:
		C.rich_text_free((*C.RichText)(h.p))
	case hStyle:
		C.rich_style_free((*C.RichStyle)(h.p))
	case hSyntax:
		C.rich_syntax_free((*C.RichSyntax)(h.p))
	case hMarkdown:
		C.rich_markdown_free((*C.RichMarkdown)(h.p))
	case hJSON:
		C.rich_json_free((*C.RichJson)(h.p))
	case hRule:
		C.rich_rule_free((*C.RichRule)(h.p))
	case hColumns:
		C.rich_columns_free((*C.RichColumns)(h.p))
	case hAlign:
		C.rich_align_free((*C.RichAlign)(h.p))
	case hPadding:
		C.rich_padding_free((*C.RichPadding)(h.p))
	case hProgress:
		C.rich_progress_free((*C.RichProgress)(h.p))
	case hStatus:
		C.rich_status_free((*C.RichStatus)(h.p))
	case hSpinner:
		C.rich_spinner_free((*C.RichSpinner)(h.p))
	case hPanel:
		C.rich_panel_free((*C.RichPanel)(h.p))
	case hTable:
		C.rich_table_free((*C.RichTable)(h.p))
	case hTree:
		C.rich_tree_free((*C.RichTree)(h.p))
	}
	nativeFreeHooksMu.RLock()
	if nativeFreedHook != nil {
		nativeFreedHook(kind)
	}
	if nativeHandleFreedHook != nil {
		nativeHandleFreedHook(kind, h)
	}
	nativeFreeHooksMu.RUnlock()
}

func nativeNew(kind handleKind) (nativeHandle, error) {
	if err := EnsureABI(); err != nil {
		return nativeHandle{}, err
	}
	var p unsafe.Pointer
	var r C.RichResult
	switch kind {
	case hConsole:
		var v *C.RichConsole
		r = C.rich_console_new(&v)
		p = unsafe.Pointer(v)
	case hRule:
		var v *C.RichRule
		r = C.rich_rule_new(&v)
		p = unsafe.Pointer(v)
	case hColumns:
		var v *C.RichColumns
		r = C.rich_columns_new(&v)
		p = unsafe.Pointer(v)
	case hProgress:
		var v *C.RichProgress
		r = C.rich_progress_new(&v)
		p = unsafe.Pointer(v)
	case hTable:
		var v *C.RichTable
		r = C.rich_table_new(&v)
		p = unsafe.Pointer(v)
	default:
		return nativeHandle{}, ErrABIMismatch
	}
	if err := checkResult(uint32(r)); err != nil {
		if p != nil {
			nativeFree(kind, nativeHandle{p: p})
		}
		return nativeHandle{}, err
	}
	if p == nil {
		return nativeHandle{}, ErrABIMismatch
	}
	return nativeHandle{p: p}, nil
}

func nativeFinish(kind handleKind, h nativeHandle) (nativeHandle, nativeHandle, uint32) {
	if finishBeforeHook != nil {
		if override := finishBeforeHook(kind); override != nil {
			if override.consume {
				nativeFree(kind, h)
			}
			if override.replaceRemaining {
				if override.setOutput {
					return override.remaining, override.output, override.result
				}
				return override.remaining, nativeHandle{}, override.result
			}
			if override.consume {
				if override.setOutput {
					return nativeHandle{}, override.output, override.result
				}
				return nativeHandle{}, nativeHandle{}, override.result
			}
			if override.setOutput {
				return h, override.output, override.result
			}
			return h, nativeHandle{}, override.result
		}
	}
	var out *C.RichRenderable
	remaining := h.p
	var r C.RichResult
	switch kind {
	case hText:
		v := (*C.RichText)(h.p)
		r = C.rich_text_finish(&v, &out)
		remaining = unsafe.Pointer(v)
	case hSyntax:
		v := (*C.RichSyntax)(h.p)
		r = C.rich_syntax_finish(&v, &out)
		remaining = unsafe.Pointer(v)
	case hMarkdown:
		v := (*C.RichMarkdown)(h.p)
		r = C.rich_markdown_finish(&v, &out)
		remaining = unsafe.Pointer(v)
	case hJSON:
		v := (*C.RichJson)(h.p)
		r = C.rich_json_finish(&v, &out)
		remaining = unsafe.Pointer(v)
	case hRule:
		v := (*C.RichRule)(h.p)
		r = C.rich_rule_finish(&v, &out)
		remaining = unsafe.Pointer(v)
	case hColumns:
		v := (*C.RichColumns)(h.p)
		r = C.rich_columns_finish(&v, &out)
		remaining = unsafe.Pointer(v)
	case hAlign:
		v := (*C.RichAlign)(h.p)
		r = C.rich_align_finish(&v, &out)
		remaining = unsafe.Pointer(v)
	case hPadding:
		v := (*C.RichPadding)(h.p)
		r = C.rich_padding_finish(&v, &out)
		remaining = unsafe.Pointer(v)
	case hPanel:
		v := (*C.RichPanel)(h.p)
		r = C.rich_panel_finish(&v, &out)
		remaining = unsafe.Pointer(v)
	case hTable:
		v := (*C.RichTable)(h.p)
		r = C.rich_table_finish(&v, &out)
		remaining = unsafe.Pointer(v)
	case hTree:
		v := (*C.RichTree)(h.p)
		r = C.rich_tree_finish(&v, &out)
		remaining = unsafe.Pointer(v)
	default:
		return nativeHandle{p: remaining}, nativeHandle{}, uint32(ErrorABIMismatch)
	}
	return nativeHandle{p: remaining}, nativeHandle{p: unsafe.Pointer(out)}, uint32(r)
}

func nativeBool(v bool) C.uint8_t {
	if v {
		return 1
	}
	return 0
}

func nativeConsoleBool(h nativeHandle, which uint8, value bool) error {
	var r C.RichResult
	switch which {
	case 0:
		r = C.rich_console_set_force_terminal((*C.RichConsole)(h.p), nativeBool(value))
	case 1:
		r = C.rich_console_set_markup_enabled((*C.RichConsole)(h.p), nativeBool(value))
	case 2:
		r = C.rich_console_set_emoji_enabled((*C.RichConsole)(h.p), nativeBool(value))
	default:
		return ErrABIMismatch
	}
	return checkResult(uint32(r))
}

func nativeConsoleSize(h nativeHandle, width, height uint32) error {
	return checkResult(uint32(C.rich_console_set_size((*C.RichConsole)(h.p), C.uint32_t(width), C.uint32_t(height))))
}
func nativeConsoleColor(h nativeHandle, system ColorSystem) error {
	return checkResult(uint32(C.rich_console_set_color_system((*C.RichConsole)(h.p), C.uint32_t(system))))
}
func nativeConsoleWidth(h nativeHandle) (uint32, error) {
	var out C.uint32_t
	err := checkResult(uint32(C.rich_console_width((*C.RichConsole)(h.p), &out)))
	return uint32(out), err
}
func nativeConsoleRenderMarkup(h nativeHandle, markup string) (string, error) {
	s, err := allocNativeString(markup)
	if err != nil {
		return "", err
	}
	defer s.free()
	return nativeBufferResult(func(out *C.RichBuffer) C.RichResult {
		return C.rich_console_render_markup((*C.RichConsole)(h.p), s.v, out)
	})
}
func nativeConsoleRender(h, renderable nativeHandle) (string, error) {
	return nativeBufferResult(func(out *C.RichBuffer) C.RichResult {
		return C.rich_console_render((*C.RichConsole)(h.p), (*C.RichRenderable)(renderable.p), out)
	})
}

func nativeTextNew(text string) (nativeHandle, error) {
	if err := EnsureABI(); err != nil {
		return nativeHandle{}, err
	}
	s, err := allocNativeString(text)
	if err != nil {
		return nativeHandle{}, err
	}
	defer s.free()
	var out *C.RichText
	r := C.rich_text_new(s.v, &out)
	if err := checkResult(uint32(r)); err != nil {
		if out != nil {
			C.rich_text_free(out)
		}
		return nativeHandle{}, err
	}
	if out == nil {
		return nativeHandle{}, ErrABIMismatch
	}
	return nativeHandle{p: unsafe.Pointer(out)}, nil
}
func nativeTextNewMarkup(console nativeHandle, markup string) (nativeHandle, error) {
	if err := EnsureABI(); err != nil {
		return nativeHandle{}, err
	}
	s, err := allocNativeString(markup)
	if err != nil {
		return nativeHandle{}, err
	}
	defer s.free()
	var out *C.RichText
	r := C.rich_text_new_markup((*C.RichConsole)(console.p), s.v, &out)
	if err := checkResult(uint32(r)); err != nil {
		if out != nil {
			C.rich_text_free(out)
		}
		return nativeHandle{}, err
	}
	if out == nil {
		return nativeHandle{}, ErrABIMismatch
	}
	return nativeHandle{p: unsafe.Pointer(out)}, nil
}
func nativeTextStyle(h nativeHandle, style string) error {
	s, err := allocNativeString(style)
	if err != nil {
		return err
	}
	defer s.free()
	return checkResult(uint32(C.rich_text_set_style((*C.RichText)(h.p), s.v)))
}
func nativeStyleParse(style string) (nativeHandle, error) {
	if err := EnsureABI(); err != nil {
		return nativeHandle{}, err
	}
	s, err := allocNativeString(style)
	if err != nil {
		return nativeHandle{}, err
	}
	defer s.free()
	var out *C.RichStyle
	r := C.rich_style_parse(s.v, &out)
	if err := checkResult(uint32(r)); err != nil {
		if out != nil {
			C.rich_style_free(out)
		}
		return nativeHandle{}, err
	}
	if out == nil {
		return nativeHandle{}, ErrABIMismatch
	}
	return nativeHandle{p: unsafe.Pointer(out)}, nil
}

func nativeSyntaxNew(code, lexer string) (nativeHandle, error) {
	if err := EnsureABI(); err != nil {
		return nativeHandle{}, err
	}
	a, err := allocNativeString(code)
	if err != nil {
		return nativeHandle{}, err
	}
	defer a.free()
	b, err := allocNativeString(lexer)
	if err != nil {
		return nativeHandle{}, err
	}
	defer b.free()
	var out *C.RichSyntax
	r := C.rich_syntax_new(a.v, b.v, &out)
	if err := checkResult(uint32(r)); err != nil {
		if out != nil {
			C.rich_syntax_free(out)
		}
		return nativeHandle{}, err
	}
	if out == nil {
		return nativeHandle{}, ErrABIMismatch
	}
	return nativeHandle{p: unsafe.Pointer(out)}, nil
}
func nativeSyntaxPath(path string) (nativeHandle, error) {
	if err := EnsureABI(); err != nil {
		return nativeHandle{}, err
	}
	s, err := allocNativeString(path)
	if err != nil {
		return nativeHandle{}, err
	}
	defer s.free()
	var out *C.RichSyntax
	r := C.rich_syntax_from_path(s.v, &out)
	if err := checkResult(uint32(r)); err != nil {
		if out != nil {
			C.rich_syntax_free(out)
		}
		return nativeHandle{}, err
	}
	if out == nil {
		return nativeHandle{}, ErrABIMismatch
	}
	return nativeHandle{p: unsafe.Pointer(out)}, nil
}
func nativeSyntaxTheme(h nativeHandle, theme string) error {
	s, err := allocNativeString(theme)
	if err != nil {
		return err
	}
	defer s.free()
	return checkResult(uint32(C.rich_syntax_set_theme((*C.RichSyntax)(h.p), s.v)))
}
func nativeSyntaxBool(h nativeHandle, which uint8, value bool) error {
	var r C.RichResult
	if which == 0 {
		r = C.rich_syntax_set_line_numbers((*C.RichSyntax)(h.p), nativeBool(value))
	} else {
		r = C.rich_syntax_set_word_wrap((*C.RichSyntax)(h.p), nativeBool(value))
	}
	return checkResult(uint32(r))
}
func nativeSyntaxRange(h nativeHandle, lr LineRange) error {
	var hs, he C.uint8_t
	var start, end C.uint32_t
	if lr.Start != nil {
		hs = 1
		start = C.uint32_t(*lr.Start)
	}
	if lr.End != nil {
		he = 1
		end = C.uint32_t(*lr.End)
	}
	return checkResult(uint32(C.rich_syntax_set_line_range((*C.RichSyntax)(h.p), hs, start, he, end)))
}

func nativeMarkdownNew(source string) (nativeHandle, error) {
	if err := EnsureABI(); err != nil {
		return nativeHandle{}, err
	}
	s, err := allocNativeString(source)
	if err != nil {
		return nativeHandle{}, err
	}
	defer s.free()
	var out *C.RichMarkdown
	r := C.rich_markdown_new(s.v, &out)
	if err := checkResult(uint32(r)); err != nil {
		if out != nil {
			C.rich_markdown_free(out)
		}
		return nativeHandle{}, err
	}
	if out == nil {
		return nativeHandle{}, ErrABIMismatch
	}
	return nativeHandle{p: unsafe.Pointer(out)}, nil
}
func nativeMarkdownTheme(h nativeHandle, theme string) error {
	s, e := allocNativeString(theme)
	if e != nil {
		return e
	}
	defer s.free()
	return checkResult(uint32(C.rich_markdown_set_code_theme((*C.RichMarkdown)(h.p), s.v)))
}
func nativeMarkdownHyperlinks(h nativeHandle, v bool) error {
	return checkResult(uint32(C.rich_markdown_set_hyperlinks((*C.RichMarkdown)(h.p), nativeBool(v))))
}
func nativeMarkdownJustify(h nativeHandle, v Justify) error {
	return checkResult(uint32(C.rich_markdown_set_justify((*C.RichMarkdown)(h.p), C.uint32_t(v))))
}

func nativeJSONNew(data string, options JSONOptions) (nativeHandle, error) {
	if err := EnsureABI(); err != nil {
		return nativeHandle{}, err
	}
	s, e := allocNativeString(data)
	if e != nil {
		return nativeHandle{}, e
	}
	defer s.free()
	var out *C.RichJson
	r := C.rich_json_new(s.v, C.uint32_t(options.Indent), nativeBool(options.Highlight), nativeBool(options.SortKeys), &out)
	if err := checkResult(uint32(r)); err != nil {
		if out != nil {
			C.rich_json_free(out)
		}
		return nativeHandle{}, err
	}
	if out == nil {
		return nativeHandle{}, ErrABIMismatch
	}
	return nativeHandle{p: unsafe.Pointer(out)}, nil
}

func nativeStringSetter(kind handleKind, which uint8, h nativeHandle, value string) error {
	s, e := allocNativeString(value)
	if e != nil {
		return e
	}
	defer s.free()
	var r C.RichResult
	switch kind {
	case hRule:
		if which == 0 {
			r = C.rich_rule_set_title((*C.RichRule)(h.p), s.v)
		} else if which == 1 {
			r = C.rich_rule_set_characters((*C.RichRule)(h.p), s.v)
		} else {
			r = C.rich_rule_set_style((*C.RichRule)(h.p), s.v)
		}
	case hPanel:
		if which == 0 {
			r = C.rich_panel_set_title((*C.RichPanel)(h.p), s.v)
		} else if which == 1 {
			r = C.rich_panel_set_subtitle((*C.RichPanel)(h.p), s.v)
		} else if which == 2 {
			r = C.rich_panel_set_style((*C.RichPanel)(h.p), s.v)
		} else {
			r = C.rich_panel_set_border_style((*C.RichPanel)(h.p), s.v)
		}
	case hTable:
		if which == 0 {
			r = C.rich_table_set_title((*C.RichTable)(h.p), s.v)
		} else if which == 1 {
			r = C.rich_table_set_caption((*C.RichTable)(h.p), s.v)
		} else if which == 2 {
			r = C.rich_table_set_style((*C.RichTable)(h.p), s.v)
		} else {
			r = C.rich_table_add_column((*C.RichTable)(h.p), s.v)
		}
	case hTree:
		if which == 0 {
			r = C.rich_tree_set_style((*C.RichTree)(h.p), s.v)
		} else {
			r = C.rich_tree_set_guide_style((*C.RichTree)(h.p), s.v)
		}
	default:
		return ErrABIMismatch
	}
	return checkResult(uint32(r))
}

func nativeRuleAlign(h nativeHandle, a HorizontalAlign) error {
	return checkResult(uint32(C.rich_rule_set_align((*C.RichRule)(h.p), C.uint32_t(a))))
}
func nativeColumnsString(h nativeHandle, s string) error {
	v, e := allocNativeString(s)
	if e != nil {
		return e
	}
	defer v.free()
	return checkResult(uint32(C.rich_columns_add_str((*C.RichColumns)(h.p), v.v)))
}
func nativeColumnsAdd(h, r nativeHandle) error {
	return checkResult(uint32(C.rich_columns_add((*C.RichColumns)(h.p), (*C.RichRenderable)(r.p))))
}
func nativeColumnsBool(h nativeHandle, which uint8, v bool) error {
	if which == 0 {
		return checkResult(uint32(C.rich_columns_set_equal((*C.RichColumns)(h.p), nativeBool(v))))
	}
	return checkResult(uint32(C.rich_columns_set_expand((*C.RichColumns)(h.p), nativeBool(v))))
}
func nativeColumnsPadding(h nativeHandle, v, x uint32) error {
	return checkResult(uint32(C.rich_columns_set_padding((*C.RichColumns)(h.p), C.uint32_t(v), C.uint32_t(x))))
}

func nativeAlignNew(content nativeHandle, a HorizontalAlign, center bool) (nativeHandle, error) {
	if err := EnsureABI(); err != nil {
		return nativeHandle{}, err
	}
	var out *C.RichAlign
	var r C.RichResult
	if center {
		r = C.rich_align_center((*C.RichRenderable)(content.p), &out)
	} else {
		r = C.rich_align_new((*C.RichRenderable)(content.p), C.uint32_t(a), &out)
	}
	if e := checkResult(uint32(r)); e != nil {
		if out != nil {
			C.rich_align_free(out)
		}
		return nativeHandle{}, e
	}
	if out == nil {
		return nativeHandle{}, ErrABIMismatch
	}
	return nativeHandle{p: unsafe.Pointer(out)}, nil
}
func nativeAlignVertical(h nativeHandle, v VerticalAlign) error {
	return checkResult(uint32(C.rich_align_set_vertical((*C.RichAlign)(h.p), C.uint32_t(v))))
}
func nativeAlignWidth(h nativeHandle, v uint32) error {
	return checkResult(uint32(C.rich_align_set_width((*C.RichAlign)(h.p), C.uint32_t(v))))
}

func nativePaddingNew(content nativeHandle, p PaddingDimensions) (nativeHandle, error) {
	if err := EnsureABI(); err != nil {
		return nativeHandle{}, err
	}
	var out *C.RichPadding
	r := C.rich_padding_new((*C.RichRenderable)(content.p), C.uint32_t(p.Top), C.uint32_t(p.Right), C.uint32_t(p.Bottom), C.uint32_t(p.Left), &out)
	if e := checkResult(uint32(r)); e != nil {
		if out != nil {
			C.rich_padding_free(out)
		}
		return nativeHandle{}, e
	}
	if out == nil {
		return nativeHandle{}, ErrABIMismatch
	}
	return nativeHandle{p: unsafe.Pointer(out)}, nil
}
func nativePaddingStyle(h nativeHandle, v string) error {
	s, e := allocNativeString(v)
	if e != nil {
		return e
	}
	defer s.free()
	return checkResult(uint32(C.rich_padding_set_style((*C.RichPadding)(h.p), s.v)))
}
func nativePaddingExpand(h nativeHandle, v bool) error {
	return checkResult(uint32(C.rich_padding_set_expand((*C.RichPadding)(h.p), nativeBool(v))))
}

func nativeProgressAdd(h nativeHandle, description string, total float64) (TaskID, error) {
	s, e := allocNativeString(description)
	if e != nil {
		return 0, e
	}
	defer s.free()
	var out C.uint64_t
	r := C.rich_progress_add_task((*C.RichProgress)(h.p), s.v, C.double(total), &out)
	return TaskID(out), checkResult(uint32(r))
}
func nativeProgressUpdate(h nativeHandle, id TaskID, completed float64) error {
	return checkResult(uint32(C.rich_progress_update((*C.RichProgress)(h.p), C.uint64_t(id), C.double(completed))))
}
func nativeProgressFrame(h, c nativeHandle) (string, error) {
	return nativeBufferResult(func(out *C.RichBuffer) C.RichResult {
		return C.rich_progress_render_frame((*C.RichProgress)(h.p), (*C.RichConsole)(c.p), out)
	})
}

func nativeStatusNew(message string) (nativeHandle, error) {
	if err := EnsureABI(); err != nil {
		return nativeHandle{}, err
	}
	s, e := allocNativeString(message)
	if e != nil {
		return nativeHandle{}, e
	}
	defer s.free()
	var out *C.RichStatus
	r := C.rich_status_new(s.v, &out)
	if e := checkResult(uint32(r)); e != nil {
		if out != nil {
			C.rich_status_free(out)
		}
		return nativeHandle{}, e
	}
	if out == nil {
		return nativeHandle{}, ErrABIMismatch
	}
	return nativeHandle{p: unsafe.Pointer(out)}, nil
}
func nativeStatusFrame(h, c nativeHandle) (string, error) {
	return nativeBufferResult(func(out *C.RichBuffer) C.RichResult {
		return C.rich_status_render_frame((*C.RichStatus)(h.p), (*C.RichConsole)(c.p), out)
	})
}
func nativeSpinnerNew(name string) (nativeHandle, error) {
	if err := EnsureABI(); err != nil {
		return nativeHandle{}, err
	}
	s, e := allocNativeString(name)
	if e != nil {
		return nativeHandle{}, e
	}
	defer s.free()
	var out *C.RichSpinner
	r := C.rich_spinner_new(s.v, &out)
	if e := checkResult(uint32(r)); e != nil {
		if out != nil {
			C.rich_spinner_free(out)
		}
		return nativeHandle{}, e
	}
	if out == nil {
		return nativeHandle{}, ErrABIMismatch
	}
	return nativeHandle{p: unsafe.Pointer(out)}, nil
}
func nativeSpinnerFrame(h, c nativeHandle, t float64) (string, error) {
	return nativeBufferResult(func(out *C.RichBuffer) C.RichResult {
		return C.rich_spinner_render_frame((*C.RichSpinner)(h.p), (*C.RichConsole)(c.p), C.double(t), out)
	})
}

func nativePanelNew(content nativeHandle) (nativeHandle, error) {
	if err := EnsureABI(); err != nil {
		return nativeHandle{}, err
	}
	var out *C.RichPanel
	r := C.rich_panel_new((*C.RichRenderable)(content.p), &out)
	if e := checkResult(uint32(r)); e != nil {
		if out != nil {
			C.rich_panel_free(out)
		}
		return nativeHandle{}, e
	}
	if out == nil {
		return nativeHandle{}, ErrABIMismatch
	}
	return nativeHandle{p: unsafe.Pointer(out)}, nil
}
func nativePanelBox(h nativeHandle, v BoxStyle) error {
	return checkResult(uint32(C.rich_panel_set_box((*C.RichPanel)(h.p), C.uint32_t(v))))
}
func nativePanelExpand(h nativeHandle, v bool) error {
	return checkResult(uint32(C.rich_panel_set_expand((*C.RichPanel)(h.p), nativeBool(v))))
}
func nativePanelWidth(h nativeHandle, v uint32) error {
	return checkResult(uint32(C.rich_panel_set_width((*C.RichPanel)(h.p), C.uint32_t(v))))
}
func nativePanelPadding(h nativeHandle, p PaddingDimensions) error {
	return checkResult(uint32(C.rich_panel_set_padding((*C.RichPanel)(h.p), C.uint32_t(p.Top), C.uint32_t(p.Right), C.uint32_t(p.Bottom), C.uint32_t(p.Left))))
}

func nativeTableMarkup(table, console nativeHandle, which uint8, value string) error {
	s, e := allocNativeString(value)
	if e != nil {
		return e
	}
	defer s.free()
	if which == 0 {
		return checkResult(uint32(C.rich_table_set_title_markup((*C.RichTable)(table.p), (*C.RichConsole)(console.p), s.v)))
	}
	return checkResult(uint32(C.rich_table_set_caption_markup((*C.RichTable)(table.p), (*C.RichConsole)(console.p), s.v)))
}
func nativeTableBox(h nativeHandle, v BoxStyle) error {
	return checkResult(uint32(C.rich_table_set_box((*C.RichTable)(h.p), C.uint32_t(v))))
}
func nativeTableBool(h nativeHandle, which uint8, v bool) error {
	var r C.RichResult
	switch which {
	case 0:
		r = C.rich_table_set_show_header((*C.RichTable)(h.p), nativeBool(v))
	case 1:
		r = C.rich_table_set_show_lines((*C.RichTable)(h.p), nativeBool(v))
	case 2:
		r = C.rich_table_set_show_edge((*C.RichTable)(h.p), nativeBool(v))
	default:
		r = C.rich_table_set_expand((*C.RichTable)(h.p), nativeBool(v))
	}
	return checkResult(uint32(r))
}
func nativeTablePadding(h nativeHandle, l, r uint32) error {
	return checkResult(uint32(C.rich_table_set_padding((*C.RichTable)(h.p), C.uint32_t(l), C.uint32_t(r))))
}
func nativeTableColumnRenderable(h, r nativeHandle) error {
	return checkResult(uint32(C.rich_table_add_column_renderable((*C.RichTable)(h.p), (*C.RichRenderable)(r.p))))
}

type nativeStringArray struct {
	mem     unsafe.Pointer
	strings []nativeString
	count   C.uint32_t
}

func allocNativeStringArray(values []string) (nativeStringArray, error) {
	if uint64(len(values)) > math.MaxUint32 {
		return nativeStringArray{}, ErrInvalidArgument
	}
	if len(values) == 0 {
		return nativeStringArray{}, nil
	}
	elementSize := unsafe.Sizeof(C.RichStr{})
	if uintptr(len(values)) > ^uintptr(0)/elementSize {
		return nativeStringArray{}, ErrInvalidArgument
	}
	size := C.size_t(uintptr(len(values)) * elementSize)
	mem := C.malloc(size)
	if mem == nil {
		return nativeStringArray{}, errors.New("rich: native array allocation failed")
	}
	if nativeArrayLifetimeHook != nil {
		nativeArrayLifetimeHook("strings", true)
	}
	a := nativeStringArray{mem: mem, strings: make([]nativeString, 0, len(values)), count: C.uint32_t(len(values))}
	dst := unsafe.Slice((*C.RichStr)(mem), len(values))
	for i, v := range values {
		s, e := allocNativeString(v)
		if e != nil {
			a.free()
			return nativeStringArray{}, e
		}
		a.strings = append(a.strings, s)
		dst[i] = s.v
	}
	return a, nil
}
func (a *nativeStringArray) free() {
	for i := range a.strings {
		a.strings[i].free()
	}
	if a.mem != nil {
		C.free(a.mem)
		if nativeArrayLifetimeHook != nil {
			nativeArrayLifetimeHook("strings", false)
		}
		a.mem = nil
	}
}
func nativeTableRowStrings(h nativeHandle, values []string) error {
	a, e := allocNativeStringArray(values)
	if e != nil {
		return e
	}
	defer a.free()
	return checkResult(uint32(C.rich_table_add_row_strs((*C.RichTable)(h.p), (*C.RichStr)(a.mem), a.count)))
}
func nativeTableRowRenderables(h nativeHandle, values []nativeHandle) error {
	if uint64(len(values)) > math.MaxUint32 {
		return ErrInvalidArgument
	}
	if len(values) == 0 {
		return checkResult(uint32(C.rich_table_add_row_renderables((*C.RichTable)(h.p), nil, 0)))
	}
	elementSize := unsafe.Sizeof(uintptr(0))
	if uintptr(len(values)) > ^uintptr(0)/elementSize {
		return ErrInvalidArgument
	}
	size := C.size_t(uintptr(len(values)) * elementSize)
	mem := C.malloc(size)
	if mem == nil {
		return errors.New("rich: native pointer-array allocation failed")
	}
	if nativeArrayLifetimeHook != nil {
		nativeArrayLifetimeHook("renderables", true)
	}
	defer func() {
		C.free(mem)
		if nativeArrayLifetimeHook != nil {
			nativeArrayLifetimeHook("renderables", false)
		}
	}()
	dst := unsafe.Slice((**C.RichRenderable)(mem), len(values))
	for i, v := range values {
		dst[i] = (*C.RichRenderable)(v.p)
	}
	return checkResult(uint32(C.rich_table_add_row_renderables(
		(*C.RichTable)(h.p),
		(**C.RichRenderable)(mem),
		C.uint32_t(len(values)),
	)))
}

func nativeTreeNew(label string) (nativeHandle, error) {
	if err := EnsureABI(); err != nil {
		return nativeHandle{}, err
	}
	s, e := allocNativeString(label)
	if e != nil {
		return nativeHandle{}, e
	}
	defer s.free()
	var out *C.RichTree
	r := C.rich_tree_new(s.v, &out)
	if e := checkResult(uint32(r)); e != nil {
		if out != nil {
			C.rich_tree_free(out)
		}
		return nativeHandle{}, e
	}
	if out == nil {
		return nativeHandle{}, ErrABIMismatch
	}
	return nativeHandle{p: unsafe.Pointer(out)}, nil
}
func nativeTreeNewRenderable(label nativeHandle) (nativeHandle, error) {
	if err := EnsureABI(); err != nil {
		return nativeHandle{}, err
	}
	var out *C.RichTree
	r := C.rich_tree_new_renderable((*C.RichRenderable)(label.p), &out)
	if e := checkResult(uint32(r)); e != nil {
		if out != nil {
			C.rich_tree_free(out)
		}
		return nativeHandle{}, e
	}
	if out == nil {
		return nativeHandle{}, ErrABIMismatch
	}
	return nativeHandle{p: unsafe.Pointer(out)}, nil
}
func nativeTreeAdd(h nativeHandle, parent NodeID, label string) (NodeID, error) {
	s, e := allocNativeString(label)
	if e != nil {
		return 0, e
	}
	defer s.free()
	var out C.uint64_t
	r := C.rich_tree_add((*C.RichTree)(h.p), C.uint64_t(parent), s.v, &out)
	return NodeID(out), checkResult(uint32(r))
}
func nativeTreeAddRenderable(h nativeHandle, parent NodeID, label nativeHandle) (NodeID, error) {
	var out C.uint64_t
	r := C.rich_tree_add_renderable((*C.RichTree)(h.p), C.uint64_t(parent), (*C.RichRenderable)(label.p), &out)
	return NodeID(out), checkResult(uint32(r))
}
func nativeTreeHideRoot(h nativeHandle, v bool) error {
	return checkResult(uint32(C.rich_tree_set_hide_root((*C.RichTree)(h.p), nativeBool(v))))
}

// This exhaustive inventory is compared with the canonical header by tests.
var nativeSymbolInventory = []string{
	"rich_ffi_abi_version", "rich_ffi_version", "rich_buffer_free", "rich_console_new", "rich_console_free", "rich_console_set_force_terminal", "rich_console_set_size", "rich_console_set_color_system", "rich_console_set_markup_enabled", "rich_console_set_emoji_enabled", "rich_console_width", "rich_console_render_markup", "rich_console_render", "rich_renderable_free", "rich_text_new", "rich_text_new_markup", "rich_text_set_style", "rich_text_finish", "rich_text_free", "rich_style_parse", "rich_style_free", "rich_syntax_new", "rich_syntax_from_path", "rich_syntax_set_theme", "rich_syntax_set_line_numbers", "rich_syntax_set_word_wrap", "rich_syntax_set_line_range", "rich_syntax_finish", "rich_syntax_free", "rich_markdown_new", "rich_markdown_set_code_theme", "rich_markdown_set_hyperlinks", "rich_markdown_set_justify", "rich_markdown_finish", "rich_markdown_free", "rich_json_new", "rich_json_finish", "rich_json_free", "rich_rule_new", "rich_rule_set_title", "rich_rule_set_characters", "rich_rule_set_style", "rich_rule_set_align", "rich_rule_finish", "rich_rule_free", "rich_columns_new", "rich_columns_add", "rich_columns_add_str", "rich_columns_set_equal", "rich_columns_set_expand", "rich_columns_set_padding", "rich_columns_finish", "rich_columns_free", "rich_align_new", "rich_align_center", "rich_align_set_vertical", "rich_align_set_width", "rich_align_finish", "rich_align_free", "rich_padding_new", "rich_padding_set_style", "rich_padding_set_expand", "rich_padding_finish", "rich_padding_free", "rich_progress_new", "rich_progress_add_task", "rich_progress_update", "rich_progress_render_frame", "rich_progress_free", "rich_status_new", "rich_status_render_frame", "rich_status_free", "rich_spinner_new", "rich_spinner_render_frame", "rich_spinner_free", "rich_panel_new", "rich_panel_set_title", "rich_panel_set_subtitle", "rich_panel_set_box", "rich_panel_set_expand", "rich_panel_set_width", "rich_panel_set_padding", "rich_panel_set_style", "rich_panel_set_border_style", "rich_panel_finish", "rich_panel_free", "rich_table_new", "rich_table_finish", "rich_table_free", "rich_table_set_title", "rich_table_set_caption", "rich_table_set_title_markup", "rich_table_set_caption_markup", "rich_table_set_box", "rich_table_set_show_header", "rich_table_set_show_lines", "rich_table_set_show_edge", "rich_table_set_expand", "rich_table_set_padding", "rich_table_set_style", "rich_table_add_column", "rich_table_add_column_renderable", "rich_table_add_row_strs", "rich_table_add_row_renderables", "rich_tree_new", "rich_tree_new_renderable", "rich_tree_add", "rich_tree_add_renderable", "rich_tree_set_style", "rich_tree_set_guide_style", "rich_tree_set_hide_root", "rich_tree_finish", "rich_tree_free",
}
