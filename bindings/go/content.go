//go:build cgo

package rich

type Syntax struct{ state *ownerState }
type Markdown struct{ state *ownerState }
type JSON struct{ state *ownerState }

func NewSyntax(code, lexer string) (*Syntax, error) {
	h, err := nativeSyntaxNew(code, lexer)
	if err != nil {
		return nil, err
	}
	return &Syntax{state: newOwnerState(hSyntax, h)}, nil
}
func NewSyntaxFromPath(path string) (*Syntax, error) {
	h, err := nativeSyntaxPath(path)
	if err != nil {
		return nil, err
	}
	return &Syntax{state: newOwnerState(hSyntax, h)}, nil
}
func (s *Syntax) Close() error {
	if s == nil {
		return nil
	}
	return closeOwner(s.state)
}
func (s *Syntax) SetTheme(v string) error {
	if s == nil {
		return ErrInvalidState
	}
	return withOwner(s.state, func(h nativeHandle) error { return nativeSyntaxTheme(h, v) })
}
func (s *Syntax) SetLineNumbers(v bool) error {
	if s == nil {
		return ErrInvalidState
	}
	return withOwner(s.state, func(h nativeHandle) error { return nativeSyntaxBool(h, 0, v) })
}
func (s *Syntax) SetWordWrap(v bool) error {
	if s == nil {
		return ErrInvalidState
	}
	return withOwner(s.state, func(h nativeHandle) error { return nativeSyntaxBool(h, 1, v) })
}
func (s *Syntax) SetLineRange(v LineRange) error {
	if s == nil {
		return ErrInvalidState
	}
	return withOwner(s.state, func(h nativeHandle) error { return nativeSyntaxRange(h, v) })
}
func (s *Syntax) Finish() (*Renderable, error) {
	if s == nil {
		return nil, ErrInvalidState
	}
	return finishOwner(s.state)
}

func NewMarkdown(source string) (*Markdown, error) {
	h, err := nativeMarkdownNew(source)
	if err != nil {
		return nil, err
	}
	return &Markdown{state: newOwnerState(hMarkdown, h)}, nil
}
func (m *Markdown) Close() error {
	if m == nil {
		return nil
	}
	return closeOwner(m.state)
}
func (m *Markdown) SetCodeTheme(v string) error {
	if m == nil {
		return ErrInvalidState
	}
	return withOwner(m.state, func(h nativeHandle) error { return nativeMarkdownTheme(h, v) })
}
func (m *Markdown) SetHyperlinks(v bool) error {
	if m == nil {
		return ErrInvalidState
	}
	return withOwner(m.state, func(h nativeHandle) error { return nativeMarkdownHyperlinks(h, v) })
}
func (m *Markdown) SetJustify(v Justify) error {
	if m == nil {
		return ErrInvalidState
	}
	return withOwner(m.state, func(h nativeHandle) error { return nativeMarkdownJustify(h, v) })
}
func (m *Markdown) Finish() (*Renderable, error) {
	if m == nil {
		return nil, ErrInvalidState
	}
	return finishOwner(m.state)
}

func NewJSON(data string, options JSONOptions) (*JSON, error) {
	h, err := nativeJSONNew(data, options)
	if err != nil {
		return nil, err
	}
	return &JSON{state: newOwnerState(hJSON, h)}, nil
}
func (j *JSON) Close() error {
	if j == nil {
		return nil
	}
	return closeOwner(j.state)
}
func (j *JSON) Finish() (*Renderable, error) {
	if j == nil {
		return nil, ErrInvalidState
	}
	return finishOwner(j.state)
}
