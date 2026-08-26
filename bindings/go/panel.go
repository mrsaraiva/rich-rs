//go:build cgo

package rich

type Panel struct{ state *ownerState }

func NewPanel(content *Renderable) (*Panel, error) {
	if content == nil {
		return nil, ErrInvalidState
	}
	var out nativeHandle
	err := withOwner(content.state, func(h nativeHandle) error { var e error; out, e = nativePanelNew(h); return e })
	if err != nil {
		return nil, err
	}
	return &Panel{state: newOwnerState(hPanel, out)}, nil
}
func (p *Panel) Close() error {
	if p == nil {
		return nil
	}
	return closeOwner(p.state)
}
func (p *Panel) SetTitle(v string) error {
	if p == nil {
		return ErrInvalidState
	}
	return withOwner(p.state, func(h nativeHandle) error { return nativeStringSetter(hPanel, 0, h, v) })
}
func (p *Panel) SetSubtitle(v string) error {
	if p == nil {
		return ErrInvalidState
	}
	return withOwner(p.state, func(h nativeHandle) error { return nativeStringSetter(hPanel, 1, h, v) })
}
func (p *Panel) SetBox(v BoxStyle) error {
	if p == nil {
		return ErrInvalidState
	}
	return withOwner(p.state, func(h nativeHandle) error { return nativePanelBox(h, v) })
}
func (p *Panel) SetExpand(v bool) error {
	if p == nil {
		return ErrInvalidState
	}
	return withOwner(p.state, func(h nativeHandle) error { return nativePanelExpand(h, v) })
}
func (p *Panel) SetWidth(v uint32) error {
	if p == nil {
		return ErrInvalidState
	}
	return withOwner(p.state, func(h nativeHandle) error { return nativePanelWidth(h, v) })
}
func (p *Panel) SetPadding(v PaddingDimensions) error {
	if p == nil {
		return ErrInvalidState
	}
	return withOwner(p.state, func(h nativeHandle) error { return nativePanelPadding(h, v) })
}
func (p *Panel) SetStyle(v string) error {
	if p == nil {
		return ErrInvalidState
	}
	return withOwner(p.state, func(h nativeHandle) error { return nativeStringSetter(hPanel, 2, h, v) })
}
func (p *Panel) SetBorderStyle(v string) error {
	if p == nil {
		return ErrInvalidState
	}
	return withOwner(p.state, func(h nativeHandle) error { return nativeStringSetter(hPanel, 3, h, v) })
}
func (p *Panel) Finish() (*Renderable, error) {
	if p == nil {
		return nil, ErrInvalidState
	}
	return finishOwner(p.state)
}
