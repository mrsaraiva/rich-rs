//go:build cgo

package rich

type Rule struct{ state *ownerState }
type Columns struct{ state *ownerState }
type Align struct{ state *ownerState }
type Padding struct{ state *ownerState }

func NewRule() (*Rule, error) {
	h, err := nativeNew(hRule)
	if err != nil {
		return nil, err
	}
	return &Rule{state: newOwnerState(hRule, h)}, nil
}
func (r *Rule) Close() error {
	if r == nil {
		return nil
	}
	return closeOwner(r.state)
}
func (r *Rule) SetTitle(v string) error {
	if r == nil {
		return ErrInvalidState
	}
	return withOwner(r.state, func(h nativeHandle) error { return nativeStringSetter(hRule, 0, h, v) })
}
func (r *Rule) SetCharacters(v string) error {
	if r == nil {
		return ErrInvalidState
	}
	return withOwner(r.state, func(h nativeHandle) error { return nativeStringSetter(hRule, 1, h, v) })
}
func (r *Rule) SetStyle(v string) error {
	if r == nil {
		return ErrInvalidState
	}
	return withOwner(r.state, func(h nativeHandle) error { return nativeStringSetter(hRule, 2, h, v) })
}
func (r *Rule) SetAlign(v HorizontalAlign) error {
	if r == nil {
		return ErrInvalidState
	}
	return withOwner(r.state, func(h nativeHandle) error { return nativeRuleAlign(h, v) })
}
func (r *Rule) Finish() (*Renderable, error) {
	if r == nil {
		return nil, ErrInvalidState
	}
	return finishOwner(r.state)
}

func NewColumns() (*Columns, error) {
	h, err := nativeNew(hColumns)
	if err != nil {
		return nil, err
	}
	return &Columns{state: newOwnerState(hColumns, h)}, nil
}
func (c *Columns) Close() error {
	if c == nil {
		return nil
	}
	return closeOwner(c.state)
}
func (c *Columns) Add(renderable *Renderable) error {
	if c == nil || renderable == nil {
		return ErrInvalidState
	}
	return withOwners(c.state, renderable.state, nativeColumnsAdd)
}
func (c *Columns) AddString(value string) error {
	if c == nil {
		return ErrInvalidState
	}
	return withOwner(c.state, func(h nativeHandle) error { return nativeColumnsString(h, value) })
}
func (c *Columns) SetEqual(v bool) error {
	if c == nil {
		return ErrInvalidState
	}
	return withOwner(c.state, func(h nativeHandle) error { return nativeColumnsBool(h, 0, v) })
}
func (c *Columns) SetExpand(v bool) error {
	if c == nil {
		return ErrInvalidState
	}
	return withOwner(c.state, func(h nativeHandle) error { return nativeColumnsBool(h, 1, v) })
}
func (c *Columns) SetPadding(vertical, horizontal uint32) error {
	if c == nil {
		return ErrInvalidState
	}
	return withOwner(c.state, func(h nativeHandle) error { return nativeColumnsPadding(h, vertical, horizontal) })
}
func (c *Columns) Finish() (*Renderable, error) {
	if c == nil {
		return nil, ErrInvalidState
	}
	return finishOwner(c.state)
}

func NewAlign(content *Renderable, horizontal HorizontalAlign) (*Align, error) {
	if content == nil {
		return nil, ErrInvalidState
	}
	var out nativeHandle
	err := withOwner(content.state, func(h nativeHandle) error { var err error; out, err = nativeAlignNew(h, horizontal, false); return err })
	if err != nil {
		return nil, err
	}
	return &Align{state: newOwnerState(hAlign, out)}, nil
}
func NewCenteredAlign(content *Renderable) (*Align, error) {
	if content == nil {
		return nil, ErrInvalidState
	}
	var out nativeHandle
	err := withOwner(content.state, func(h nativeHandle) error { var err error; out, err = nativeAlignNew(h, AlignCenter, true); return err })
	if err != nil {
		return nil, err
	}
	return &Align{state: newOwnerState(hAlign, out)}, nil
}
func (a *Align) Close() error {
	if a == nil {
		return nil
	}
	return closeOwner(a.state)
}
func (a *Align) SetVertical(v VerticalAlign) error {
	if a == nil {
		return ErrInvalidState
	}
	return withOwner(a.state, func(h nativeHandle) error { return nativeAlignVertical(h, v) })
}
func (a *Align) SetWidth(v uint32) error {
	if a == nil {
		return ErrInvalidState
	}
	return withOwner(a.state, func(h nativeHandle) error { return nativeAlignWidth(h, v) })
}
func (a *Align) Finish() (*Renderable, error) {
	if a == nil {
		return nil, ErrInvalidState
	}
	return finishOwner(a.state)
}

func NewPadding(content *Renderable, dimensions PaddingDimensions) (*Padding, error) {
	if content == nil {
		return nil, ErrInvalidState
	}
	var out nativeHandle
	err := withOwner(content.state, func(h nativeHandle) error { var err error; out, err = nativePaddingNew(h, dimensions); return err })
	if err != nil {
		return nil, err
	}
	return &Padding{state: newOwnerState(hPadding, out)}, nil
}
func (p *Padding) Close() error {
	if p == nil {
		return nil
	}
	return closeOwner(p.state)
}
func (p *Padding) SetStyle(v string) error {
	if p == nil {
		return ErrInvalidState
	}
	return withOwner(p.state, func(h nativeHandle) error { return nativePaddingStyle(h, v) })
}
func (p *Padding) SetExpand(v bool) error {
	if p == nil {
		return ErrInvalidState
	}
	return withOwner(p.state, func(h nativeHandle) error { return nativePaddingExpand(h, v) })
}
func (p *Padding) Finish() (*Renderable, error) {
	if p == nil {
		return nil, ErrInvalidState
	}
	return finishOwner(p.state)
}
