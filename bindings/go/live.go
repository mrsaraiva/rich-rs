//go:build cgo

package rich

type Progress struct{ state *ownerState }
type Status struct{ state *ownerState }
type Spinner struct{ state *ownerState }

func NewProgress() (*Progress, error) {
	h, e := nativeNew(hProgress)
	if e != nil {
		return nil, e
	}
	return &Progress{state: newOwnerState(hProgress, h)}, nil
}
func (p *Progress) Close() error {
	if p == nil {
		return nil
	}
	return closeOwner(p.state)
}
func (p *Progress) AddTask(description string, total float64) (TaskID, error) {
	if p == nil {
		return 0, ErrInvalidState
	}
	var id TaskID
	err := withOwner(p.state, func(h nativeHandle) error { var e error; id, e = nativeProgressAdd(h, description, total); return e })
	return id, err
}
func (p *Progress) Update(id TaskID, completed float64) error {
	if p == nil {
		return ErrInvalidState
	}
	return withOwner(p.state, func(h nativeHandle) error { return nativeProgressUpdate(h, id, completed) })
}
func (p *Progress) RenderFrame(console *Console) (string, error) {
	if p == nil || console == nil {
		return "", ErrInvalidState
	}
	var out string
	err := withOwners(p.state, console.state, func(ph, ch nativeHandle) error { var e error; out, e = nativeProgressFrame(ph, ch); return e })
	return out, err
}

func NewStatus(message string) (*Status, error) {
	h, e := nativeStatusNew(message)
	if e != nil {
		return nil, e
	}
	return &Status{state: newOwnerState(hStatus, h)}, nil
}
func (s *Status) Close() error {
	if s == nil {
		return nil
	}
	return closeOwner(s.state)
}
func (s *Status) RenderFrame(console *Console) (string, error) {
	if s == nil || console == nil {
		return "", ErrInvalidState
	}
	var out string
	err := withOwners(s.state, console.state, func(sh, ch nativeHandle) error { var e error; out, e = nativeStatusFrame(sh, ch); return e })
	return out, err
}

func NewSpinner(name string) (*Spinner, error) {
	h, e := nativeSpinnerNew(name)
	if e != nil {
		return nil, e
	}
	return &Spinner{state: newOwnerState(hSpinner, h)}, nil
}
func (s *Spinner) Close() error {
	if s == nil {
		return nil
	}
	return closeOwner(s.state)
}
func (s *Spinner) RenderFrame(console *Console, timeSeconds float64) (string, error) {
	if s == nil || console == nil {
		return "", ErrInvalidState
	}
	var out string
	err := withOwners(s.state, console.state, func(sh, ch nativeHandle) error {
		var e error
		out, e = nativeSpinnerFrame(sh, ch, timeSeconds)
		return e
	})
	return out, err
}
