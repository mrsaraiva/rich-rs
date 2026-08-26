//go:build cgo

package rich

type Text struct{ state *ownerState }

func NewText(text string) (*Text, error) {
	h, err := nativeTextNew(text)
	if err != nil {
		return nil, err
	}
	return &Text{state: newOwnerState(hText, h)}, nil
}
func NewTextMarkup(console *Console, markup string) (*Text, error) {
	if console == nil {
		return nil, ErrInvalidState
	}
	var out nativeHandle
	err := withOwner(console.state, func(h nativeHandle) error { var err error; out, err = nativeTextNewMarkup(h, markup); return err })
	if err != nil {
		return nil, err
	}
	return &Text{state: newOwnerState(hText, out)}, nil
}
func (t *Text) Close() error {
	if t == nil {
		return nil
	}
	return closeOwner(t.state)
}
func (t *Text) SetStyle(style string) error {
	if t == nil {
		return ErrInvalidState
	}
	return withOwner(t.state, func(h nativeHandle) error { return nativeTextStyle(h, style) })
}
func (t *Text) Finish() (*Renderable, error) {
	if t == nil {
		return nil, ErrInvalidState
	}
	return finishOwner(t.state)
}
