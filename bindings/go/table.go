//go:build cgo

package rich

type Table struct{ state *ownerState }

func NewTable() (*Table, error) {
	h, e := nativeTableNew(false)
	if e != nil {
		return nil, e
	}
	return &Table{state: newOwnerState(hTable, h)}, nil
}

// NewGridTable creates the exact native Table::grid defaults.
func NewGridTable() (*Table, error) {
	h, err := nativeTableNew(true)
	if err != nil {
		return nil, err
	}
	return &Table{state: newOwnerState(hTable, h)}, nil
}
func (t *Table) Close() error {
	if t == nil {
		return nil
	}
	return closeOwner(t.state)
}
func (t *Table) Finish() (*Renderable, error) {
	if t == nil {
		return nil, ErrInvalidState
	}
	return finishOwner(t.state)
}
func (t *Table) SetTitle(v string) error {
	if t == nil {
		return ErrInvalidState
	}
	return withOwner(t.state, func(h nativeHandle) error { return nativeStringSetter(hTable, 0, h, v) })
}
func (t *Table) SetCaption(v string) error {
	if t == nil {
		return ErrInvalidState
	}
	return withOwner(t.state, func(h nativeHandle) error { return nativeStringSetter(hTable, 1, h, v) })
}
func (t *Table) SetTitleMarkup(console *Console, v string) error {
	if t == nil || console == nil {
		return ErrInvalidState
	}
	return withOwners(t.state, console.state, func(th, ch nativeHandle) error { return nativeTableMarkup(th, ch, 0, v) })
}
func (t *Table) SetCaptionMarkup(console *Console, v string) error {
	if t == nil || console == nil {
		return ErrInvalidState
	}
	return withOwners(t.state, console.state, func(th, ch nativeHandle) error { return nativeTableMarkup(th, ch, 1, v) })
}
func (t *Table) SetBox(v BoxStyle) error {
	if t == nil {
		return ErrInvalidState
	}
	return withOwner(t.state, func(h nativeHandle) error { return nativeTableBox(h, v) })
}
func (t *Table) SetPadEdge(v bool) error {
	if t == nil {
		return ErrInvalidState
	}
	return withOwner(t.state, func(h nativeHandle) error { return nativeTablePadEdge(h, v) })
}
func (t *Table) SetRowStyles(styles ...string) error {
	if t == nil {
		return ErrInvalidState
	}
	return withOwner(t.state, func(h nativeHandle) error { return nativeTableRowStyles(h, styles) })
}
func (t *Table) SetShowHeader(v bool) error {
	if t == nil {
		return ErrInvalidState
	}
	return withOwner(t.state, func(h nativeHandle) error { return nativeTableBool(h, 0, v) })
}
func (t *Table) SetShowLines(v bool) error {
	if t == nil {
		return ErrInvalidState
	}
	return withOwner(t.state, func(h nativeHandle) error { return nativeTableBool(h, 1, v) })
}
func (t *Table) SetShowEdge(v bool) error {
	if t == nil {
		return ErrInvalidState
	}
	return withOwner(t.state, func(h nativeHandle) error { return nativeTableBool(h, 2, v) })
}
func (t *Table) SetExpand(v bool) error {
	if t == nil {
		return ErrInvalidState
	}
	return withOwner(t.state, func(h nativeHandle) error { return nativeTableBool(h, 3, v) })
}
func (t *Table) SetPadding(left, right uint32) error {
	if t == nil {
		return ErrInvalidState
	}
	return withOwner(t.state, func(h nativeHandle) error { return nativeTablePadding(h, left, right) })
}
func (t *Table) SetStyle(v string) error {
	if t == nil {
		return ErrInvalidState
	}
	return withOwner(t.state, func(h nativeHandle) error { return nativeStringSetter(hTable, 2, h, v) })
}
func (t *Table) AddColumn(header string) error {
	if t == nil {
		return ErrInvalidState
	}
	return withOwner(t.state, func(h nativeHandle) error { return nativeStringSetter(hTable, 3, h, header) })
}
func (t *Table) AddColumnRenderable(header *Renderable) error {
	if t == nil || header == nil {
		return ErrInvalidState
	}
	return withOwners(t.state, header.state, nativeTableColumnRenderable)
}

// AddColumnOptions validates and adds one configured column atomically. Header
// may be nil and remains independently reusable when non-nil.
func (t *Table) AddColumnOptions(header *Renderable, options ColumnOptions) error {
	if t == nil {
		return ErrInvalidState
	}
	if header == nil {
		return withOwner(t.state, func(h nativeHandle) error {
			return nativeTableColumnOptions(h, nativeHandle{}, options)
		})
	}
	return withOwners(t.state, header.state, func(table, renderable nativeHandle) error {
		return nativeTableColumnOptions(table, renderable, options)
	})
}
func (t *Table) AddRow(cells ...string) error {
	if t == nil {
		return ErrInvalidState
	}
	return withOwner(t.state, func(h nativeHandle) error { return nativeTableRowStrings(h, cells) })
}
func (t *Table) AddRenderableRow(cells ...*Renderable) error {
	if t == nil {
		return ErrInvalidState
	}
	states := make([]*ownerState, 1, len(cells)+1)
	states[0] = t.state
	for _, cell := range cells {
		if cell == nil {
			return ErrInvalidState
		}
		states = append(states, cell.state)
	}
	return withOwnerSet(states, func(handles []nativeHandle) error { return nativeTableRowRenderables(handles[0], handles[1:]) })
}
