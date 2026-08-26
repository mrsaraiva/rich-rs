//go:build cgo

package rich

type Tree struct{ state *ownerState }

func NewTree(label string) (*Tree, error) {
	h, e := nativeTreeNew(label)
	if e != nil {
		return nil, e
	}
	return &Tree{state: newOwnerState(hTree, h)}, nil
}
func NewTreeRenderable(label *Renderable) (*Tree, error) {
	if label == nil {
		return nil, ErrInvalidState
	}
	var out nativeHandle
	err := withOwner(label.state, func(h nativeHandle) error { var e error; out, e = nativeTreeNewRenderable(h); return e })
	if err != nil {
		return nil, err
	}
	return &Tree{state: newOwnerState(hTree, out)}, nil
}
func (t *Tree) Close() error {
	if t == nil {
		return nil
	}
	return closeOwner(t.state)
}
func (t *Tree) Add(parent NodeID, label string) (NodeID, error) {
	if t == nil {
		return 0, ErrInvalidState
	}
	var out NodeID
	err := withOwner(t.state, func(h nativeHandle) error { var e error; out, e = nativeTreeAdd(h, parent, label); return e })
	return out, err
}
func (t *Tree) AddRenderable(parent NodeID, label *Renderable) (NodeID, error) {
	if t == nil || label == nil {
		return 0, ErrInvalidState
	}
	var out NodeID
	err := withOwners(t.state, label.state, func(th, lh nativeHandle) error {
		var e error
		out, e = nativeTreeAddRenderable(th, parent, lh)
		return e
	})
	return out, err
}
func (t *Tree) SetStyle(v string) error {
	if t == nil {
		return ErrInvalidState
	}
	return withOwner(t.state, func(h nativeHandle) error { return nativeStringSetter(hTree, 0, h, v) })
}
func (t *Tree) SetGuideStyle(v string) error {
	if t == nil {
		return ErrInvalidState
	}
	return withOwner(t.state, func(h nativeHandle) error { return nativeStringSetter(hTree, 1, h, v) })
}
func (t *Tree) SetHideRoot(v bool) error {
	if t == nil {
		return ErrInvalidState
	}
	return withOwner(t.state, func(h nativeHandle) error { return nativeTreeHideRoot(h, v) })
}
func (t *Tree) Finish() (*Renderable, error) {
	if t == nil {
		return nil, ErrInvalidState
	}
	return finishOwner(t.state)
}
