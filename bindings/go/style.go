//go:build cgo

package rich

type Style struct{ state *ownerState }

func ParseStyle(style string) (*Style, error) {
	h, err := nativeStyleParse(style)
	if err != nil {
		return nil, err
	}
	return &Style{state: newOwnerState(hStyle, h)}, nil
}
func (s *Style) Close() error {
	if s == nil {
		return nil
	}
	return closeOwner(s.state)
}
