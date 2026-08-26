//go:build cgo

package rich

// ABIVersion is the ABI expected by this Go module.
const ABIVersion uint32 = 1

type ColorSystem uint32

const (
	ColorNone ColorSystem = iota
	ColorStandard
	ColorEightBit
	ColorTruecolor
	ColorWindows
)

type HorizontalAlign uint32

const (
	AlignLeft HorizontalAlign = iota
	AlignCenter
	AlignRight
)

type VerticalAlign uint32

const (
	VerticalTop VerticalAlign = iota
	VerticalMiddle
	VerticalBottom
)

type Justify uint32

const (
	JustifyDefault Justify = iota
	JustifyLeft
	JustifyCenter
	JustifyRight
	JustifyFull
)

type BoxStyle uint32

const (
	BoxRounded BoxStyle = iota
	BoxHeavy
	BoxDouble
	BoxASCII
	BoxMinimal
	BoxSquare
	BoxSimple
	BoxHeavyHead
	BoxHeavyEdge
	BoxDoubleEdge
	BoxHorizontals
	BoxMinimalHeavyHead
	BoxSimpleHeavy
	BoxMarkdown
)

type NodeID uint64

const RootNodeID NodeID = 0

type TaskID uint64

// LineRange independently represents optional start and end lines.
type LineRange struct {
	Start *uint32
	End   *uint32
}

// JSONOptions configures JSON rendering. Use NewJSONOptions for native defaults.
type JSONOptions struct {
	Indent    uint32
	Highlight bool
	SortKeys  bool
}

func NewJSONOptions() JSONOptions {
	return JSONOptions{Indent: 2, Highlight: true, SortKeys: false}
}

type PaddingDimensions struct {
	Top, Right, Bottom, Left uint32
}
