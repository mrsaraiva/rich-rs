//go:build cgo

package rich

import (
	"runtime"
	"sort"
	"sync"
	"sync/atomic"
)

type handleKind uint8

const (
	hConsole handleKind = iota
	hRenderable
	hText
	hStyle
	hSyntax
	hMarkdown
	hJSON
	hRule
	hColumns
	hAlign
	hPadding
	hProgress
	hStatus
	hSpinner
	hPanel
	hTable
	hTree
)

type ownerState struct {
	mu   sync.Mutex
	id   uint64
	kind handleKind
	h    nativeHandle
}

func withOwnerSet(states []*ownerState, f func([]nativeHandle) error) error {
	if len(states) == 0 {
		return f(nil)
	}
	unique := make([]*ownerState, 0, len(states))
	seen := make(map[*ownerState]struct{}, len(states))
	for _, s := range states {
		if s == nil {
			return ErrInvalidState
		}
		if _, ok := seen[s]; !ok {
			seen[s] = struct{}{}
			unique = append(unique, s)
		}
	}
	sort.Slice(unique, func(i, j int) bool { return unique[i].id < unique[j].id })
	for _, s := range unique {
		s.mu.Lock()
	}
	defer func() {
		for i := len(unique) - 1; i >= 0; i-- {
			unique[i].mu.Unlock()
		}
	}()
	handles := make([]nativeHandle, len(states))
	for i, s := range states {
		if s.h.isNil() {
			return ErrInvalidState
		}
		handles[i] = s.h
	}
	err := f(handles)
	for _, s := range states {
		runtime.KeepAlive(s)
	}
	return err
}

var nextOwnerID atomic.Uint64

func newOwnerState(kind handleKind, h nativeHandle) *ownerState {
	s := &ownerState{id: nextOwnerID.Add(1), kind: kind, h: h}
	runtime.SetFinalizer(s, finalizeOwner)
	return s
}

func finalizeOwner(s *ownerState) { closeOwnerState(s) }

func closeOwnerState(s *ownerState) {
	if s == nil {
		return
	}
	s.mu.Lock()
	h := s.h
	s.h = nativeHandle{}
	s.mu.Unlock()
	if !h.isNil() {
		nativeFree(s.kind, h)
	}
}

func closeOwner(s *ownerState) error {
	if s == nil {
		return nil
	}
	runtime.SetFinalizer(s, nil)
	closeOwnerState(s)
	runtime.KeepAlive(s)
	return nil
}

func withOwner(s *ownerState, f func(nativeHandle) error) error {
	if s == nil {
		return ErrInvalidState
	}
	s.mu.Lock()
	defer s.mu.Unlock()
	if s.h.isNil() {
		return ErrInvalidState
	}
	err := f(s.h)
	runtime.KeepAlive(s)
	return err
}

func withOwners(a, b *ownerState, f func(nativeHandle, nativeHandle) error) error {
	if a == nil || b == nil {
		return ErrInvalidState
	}
	if a == b {
		a.mu.Lock()
		defer a.mu.Unlock()
		if a.h.isNil() {
			return ErrInvalidState
		}
		err := f(a.h, a.h)
		runtime.KeepAlive(a)
		return err
	}
	first, second := a, b
	reverse := false
	if first.id > second.id {
		first, second = second, first
		reverse = true
	}
	first.mu.Lock()
	second.mu.Lock()
	defer first.mu.Unlock()
	defer second.mu.Unlock()
	if a.h.isNil() || b.h.isNil() {
		return ErrInvalidState
	}
	var err error
	if reverse {
		err = f(second.h, first.h)
	} else {
		err = f(first.h, second.h)
	}
	runtime.KeepAlive(a)
	runtime.KeepAlive(b)
	return err
}

func finishOwner(s *ownerState) (*Renderable, error) {
	if s == nil {
		return nil, ErrInvalidState
	}
	s.mu.Lock()
	defer s.mu.Unlock()
	if s.h.isNil() {
		return nil, ErrInvalidState
	}
	original := s.h
	remaining, output, result := nativeFinish(s.kind, original)
	runtime.KeepAlive(s)
	remainingIsOriginal := !remaining.isNil() && remaining.equal(original)
	remainingIsReplacement := !remaining.isNil() && !remainingIsOriginal
	outputAliasesRemaining := !output.isNil() && !remaining.isNil() && output.equal(remaining)
	finishErr := checkResult(result)
	outputAliasesRetainedOriginal := finishErr != nil && remainingIsReplacement && !output.isNil() && output.equal(original)
	freeDistinctOutput := func() {
		if !output.isNil() && !outputAliasesRemaining && !outputAliasesRetainedOriginal {
			nativeFree(hRenderable, output)
		}
	}
	if finishErr != nil {
		malformedOutput := !output.isNil()
		freeDistinctOutput()
		if remaining.isNil() {
			s.h = nativeHandle{}
			runtime.SetFinalizer(s, nil)
			if malformedOutput {
				return nil, ErrABIMismatch
			}
			return nil, finishErr
		}
		if remainingIsReplacement {
			nativeFree(s.kind, remaining)
			return nil, ErrABIMismatch
		}
		if malformedOutput {
			return nil, ErrABIMismatch
		}
		return nil, finishErr
	}
	if !remaining.isNil() || output.isNil() {
		freeDistinctOutput()
		if !remaining.isNil() {
			nativeFree(s.kind, remaining)
		}
		s.h = nativeHandle{}
		runtime.SetFinalizer(s, nil)
		return nil, ErrABIMismatch
	}
	s.h = nativeHandle{}
	runtime.SetFinalizer(s, nil)
	return &Renderable{state: newOwnerState(hRenderable, output)}, nil
}
