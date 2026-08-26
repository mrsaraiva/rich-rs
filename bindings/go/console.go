//go:build cgo

package rich

type Console struct{ state *ownerState }
type Renderable struct{ state *ownerState }

func NewConsole() (*Console, error) {
	h, err := nativeNew(hConsole)
	if err != nil {
		return nil, err
	}
	return &Console{state: newOwnerState(hConsole, h)}, nil
}

func (c *Console) Close() error {
	if c == nil {
		return nil
	}
	return closeOwner(c.state)
}
func (r *Renderable) Close() error {
	if r == nil {
		return nil
	}
	return closeOwner(r.state)
}
func (c *Console) SetForceTerminal(v bool) error {
	if c == nil {
		return ErrInvalidState
	}
	return withOwner(c.state, func(h nativeHandle) error { return nativeConsoleBool(h, 0, v) })
}
func (c *Console) SetSize(width, height uint32) error {
	if c == nil {
		return ErrInvalidState
	}
	return withOwner(c.state, func(h nativeHandle) error { return nativeConsoleSize(h, width, height) })
}
func (c *Console) SetColorSystem(v ColorSystem) error {
	if c == nil {
		return ErrInvalidState
	}
	return withOwner(c.state, func(h nativeHandle) error { return nativeConsoleColor(h, v) })
}
func (c *Console) SetMarkupEnabled(v bool) error {
	if c == nil {
		return ErrInvalidState
	}
	return withOwner(c.state, func(h nativeHandle) error { return nativeConsoleBool(h, 1, v) })
}
func (c *Console) SetEmojiEnabled(v bool) error {
	if c == nil {
		return ErrInvalidState
	}
	return withOwner(c.state, func(h nativeHandle) error { return nativeConsoleBool(h, 2, v) })
}
func (c *Console) Width() (uint32, error) {
	if c == nil {
		return 0, ErrInvalidState
	}
	var out uint32
	err := withOwner(c.state, func(h nativeHandle) error { var err error; out, err = nativeConsoleWidth(h); return err })
	return out, err
}
func (c *Console) RenderMarkup(markup string) (string, error) {
	if c == nil {
		return "", ErrInvalidState
	}
	var out string
	err := withOwner(c.state, func(h nativeHandle) error { var err error; out, err = nativeConsoleRenderMarkup(h, markup); return err })
	return out, err
}
func (c *Console) Render(renderable *Renderable) (string, error) {
	if c == nil || renderable == nil {
		return "", ErrInvalidState
	}
	var out string
	err := withOwners(c.state, renderable.state, func(ch, rh nativeHandle) error { var err error; out, err = nativeConsoleRender(ch, rh); return err })
	return out, err
}
