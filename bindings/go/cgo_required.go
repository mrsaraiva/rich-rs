//go:build !cgo

package rich

// This deliberately fails compilation with a concise requirement instead of
// exposing an incomplete fallback implementation.
const richRequiresCGOEnabled1AndTargetMatchingNativeRichFFI = 1

var _ [0 - richRequiresCGOEnabled1AndTargetMatchingNativeRichFFI]byte
