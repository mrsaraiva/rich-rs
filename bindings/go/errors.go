//go:build cgo

package rich

import "fmt"

// ErrorCode is a stable rich-ffi result category.
type ErrorCode uint32

const (
	ErrorNullArgument    ErrorCode = 1
	ErrorInvalidUTF8     ErrorCode = 2
	ErrorInvalidArgument ErrorCode = 3
	ErrorInvalidState    ErrorCode = 4
	ErrorIO              ErrorCode = 5
	ErrorRender          ErrorCode = 6
	ErrorNativePanic     ErrorCode = 7
	// ErrorABIMismatch represents an unknown native result or malformed ABI output.
	ErrorABIMismatch ErrorCode = ^ErrorCode(0)
)

// Error preserves the exact stable native error category.
type Error struct{ Code ErrorCode }

func (e *Error) Error() string {
	if e == nil {
		return "<nil>"
	}
	switch e.Code {
	case ErrorNullArgument:
		return "rich: native null argument"
	case ErrorInvalidUTF8:
		return "rich: invalid UTF-8"
	case ErrorInvalidArgument:
		return "rich: invalid argument"
	case ErrorInvalidState:
		return "rich: invalid or closed state"
	case ErrorIO:
		return "rich: native I/O error"
	case ErrorRender:
		return "rich: native render error"
	case ErrorNativePanic:
		return "rich: native panic was contained"
	case ErrorABIMismatch:
		return "rich: ABI mismatch"
	default:
		return fmt.Sprintf("rich: native error code %d", uint32(e.Code))
	}
}

func (e *Error) Is(target error) bool {
	t, ok := target.(*Error)
	return ok && e != nil && t != nil && e.Code == t.Code
}

var (
	ErrNullArgument    = &Error{Code: ErrorNullArgument}
	ErrInvalidUTF8     = &Error{Code: ErrorInvalidUTF8}
	ErrInvalidArgument = &Error{Code: ErrorInvalidArgument}
	ErrInvalidState    = &Error{Code: ErrorInvalidState}
	ErrIO              = &Error{Code: ErrorIO}
	ErrRender          = &Error{Code: ErrorRender}
	ErrNativePanic     = &Error{Code: ErrorNativePanic}
	ErrABIMismatch     = &Error{Code: ErrorABIMismatch}
)

func checkResult(result uint32) error {
	if result == 0 {
		return nil
	}
	switch ErrorCode(result) {
	case ErrorNullArgument, ErrorInvalidUTF8, ErrorInvalidArgument, ErrorInvalidState,
		ErrorIO, ErrorRender, ErrorNativePanic:
		return &Error{Code: ErrorCode(result)}
	default:
		return &Error{Code: ErrorABIMismatch}
	}
}
