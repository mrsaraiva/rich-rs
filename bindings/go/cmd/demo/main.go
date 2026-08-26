package main

import (
	"bytes"
	"fmt"
	"os"
	"time"

	rich "github.com/mrsaraiva/rich-rs/bindings/go"
)

type closer interface{ Close() error }
type owners struct{ values []closer }

func (o *owners) add(value closer) { o.values = append(o.values, value) }
func (o *owners) closeAll() error {
	var first error
	for i := len(o.values) - 1; i >= 0; i-- {
		if err := o.values[i].Close(); err != nil && first == nil {
			first = err
		}
	}
	return first
}

func finishText(o *owners, text *rich.Text) (*rich.Renderable, error) {
	renderable, err := text.Finish()
	if err != nil {
		return nil, err
	}
	o.add(renderable)
	return renderable, nil
}
func plain(o *owners, value string) (*rich.Renderable, error) {
	text, err := rich.NewText(value)
	if err != nil {
		return nil, err
	}
	o.add(text)
	return finishText(o, text)
}
func styled(o *owners, value, style string) (*rich.Renderable, error) {
	text, err := rich.NewText(value)
	if err != nil {
		return nil, err
	}
	o.add(text)
	if err := text.SetStyle(style); err != nil {
		return nil, err
	}
	return finishText(o, text)
}
func markup(o *owners, console *rich.Console, value string) (*rich.Renderable, error) {
	text, err := rich.NewTextMarkup(console, value)
	if err != nil {
		return nil, err
	}
	o.add(text)
	return finishText(o, text)
}
func finishTable(o *owners, table *rich.Table) (*rich.Renderable, error) {
	renderable, err := table.Finish()
	if err != nil {
		return nil, err
	}
	o.add(renderable)
	return renderable, nil
}
func addFeature(o *owners, card *rich.Table, name string, demo *rich.Renderable) error {
	label, err := plain(o, name)
	if err != nil {
		return err
	}
	return card.AddRenderableRow(label, demo)
}
func addBlank(o *owners, card *rich.Table) error {
	label, err := plain(o, "")
	if err != nil {
		return err
	}
	empty, err := plain(o, "")
	if err != nil {
		return err
	}
	return card.AddRenderableRow(label, empty)
}

func hueToRGB(p, q, value float64) float64 {
	if value < 0 {
		value++
	}
	if value > 1 {
		value--
	}
	switch {
	case value < 1.0/6.0:
		return p + (q-p)*6*value
	case value < 0.5:
		return q
	case value < 2.0/3.0:
		return p + (q-p)*(2.0/3.0-value)*6
	default:
		return p
	}
}

//go:noinline
func roundedFloat(value float64) float64 { return value }

//go:noinline
func hlsToRGB(h, l, s float64) (uint8, uint8, uint8) {
	if s == 0 {
		value := uint8(l * 255)
		return value, value, value
	}
	q := l * (1.0 + s)
	if l >= 0.5 {
		q = roundedFloat(l+s) - roundedFloat(l*s)
	}
	p := 2*l - q
	return uint8(hueToRGB(p, q, h+1.0/3.0) * 255), uint8(hueToRGB(p, q, h) * 255), uint8(hueToRGB(p, q, h-1.0/3.0) * 255)
}

type colorBox struct{ height uint32 }

func (box colorBox) Render(options rich.RenderOptions, sink *rich.SegmentSink) error {
	for y := uint32(0); y < box.height; y++ {
		for x := uint32(0); x < options.MaxWidth; x++ {
			h := float64(x) / float64(options.MaxWidth)
			l1 := 0.1 + (float64(y)/float64(box.height))*0.7
			l2 := l1 + 0.7/float64(2*box.height)
			br, bg, bb := hlsToRGB(h, l1, 1.0)
			fr, fg, fb := hlsToRGB(h, l2, 1.0)
			style := fmt.Sprintf("rgb(%d,%d,%d) on rgb(%d,%d,%d)", fr, fg, fb, br, bg, bb)
			if err := sink.Push("▄", style); err != nil {
				return err
			}
		}
		if err := sink.Line(); err != nil {
			return err
		}
	}
	return nil
}
func (colorBox) Measure(options rich.RenderOptions) (rich.Measurement, error) {
	return rich.Measurement{Minimum: 1, Maximum: options.MaxWidth}, nil
}

func makeColorDemo(o *owners, console *rich.Console) (*rich.Renderable, error) {
	checklist, err := markup(o, console, "✓ [bold green]4-bit color[/]\n✓ [bold blue]8-bit color[/]\n✓ [bold magenta]Truecolor (16.7 million)[/]\n✓ [bold yellow]Dumb terminals[/]\n✓ [bold cyan]Automatic color conversion[/]")
	if err != nil {
		return nil, err
	}
	swatches, err := rich.NewCustomRenderable(colorBox{height: 5})
	if err != nil {
		return nil, err
	}
	o.add(swatches)
	table, err := rich.NewTable()
	if err != nil {
		return nil, err
	}
	o.add(table)
	for _, operation := range []func() error{
		func() error { return table.SetBox(rich.BoxNone) },
		func() error { return table.SetShowHeader(false) },
		func() error { return table.SetShowEdge(false) },
		func() error { return table.SetExpand(false) },
		func() error { return table.SetPadEdge(false) },
		func() error { return table.AddRenderableRow(checklist, swatches) },
	} {
		if err := operation(); err != nil {
			return nil, err
		}
	}
	return finishTable(o, table)
}

func makeTextExamples(o *owners, console *rich.Console) (*rich.Renderable, error) {
	lorem := "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Quisque in metus sed sapien ultricies pretium a at justo. Maecenas luctus velit et auctor maximus."
	values := make([]*rich.Renderable, 0, 4)
	for _, style := range []string{"green", "yellow", "blue", "red"} {
		value, err := styled(o, lorem, style)
		if err != nil {
			return nil, err
		}
		values = append(values, value)
	}
	table, err := rich.NewGridTable()
	if err != nil {
		return nil, err
	}
	o.add(table)
	if err := table.SetPadding(1, 1); err != nil {
		return nil, err
	}
	if err := table.SetPadEdge(false); err != nil {
		return nil, err
	}
	for _, justify := range []rich.Justify{rich.JustifyLeft, rich.JustifyCenter, rich.JustifyRight, rich.JustifyFull} {
		ratio, value := uint32(1), justify
		if err := table.AddColumnOptions(nil, rich.ColumnOptions{Ratio: &ratio, Justify: &value}); err != nil {
			return nil, err
		}
	}
	if err := table.AddRenderableRow(values...); err != nil {
		return nil, err
	}
	loremTable, err := finishTable(o, table)
	if err != nil {
		return nil, err
	}
	intro, err := markup(o, console, "Word wrap text. Justify [green]left[/], [yellow]center[/], [blue]right[/] or [red]full[/].\n")
	if err != nil {
		return nil, err
	}
	blank, err := plain(o, "")
	if err != nil {
		return nil, err
	}
	section, err := rich.NewGridTable()
	if err != nil {
		return nil, err
	}
	o.add(section)
	if err := section.SetPadding(0, 0); err != nil {
		return nil, err
	}
	if err := section.AddColumnOptions(nil, rich.ColumnOptions{}); err != nil {
		return nil, err
	}
	for _, row := range []*rich.Renderable{intro, blank, loremTable} {
		if err := section.AddRenderableRow(row); err != nil {
			return nil, err
		}
	}
	return finishTable(o, section)
}

func makeMovieTable(o *owners, console *rich.Console) (*rich.Renderable, error) {
	table, err := rich.NewTable()
	if err != nil {
		return nil, err
	}
	o.add(table)
	for _, operation := range []func() error{
		func() error { return table.SetBox(rich.BoxSimple) },
		func() error { return table.SetShowEdge(false) },
		func() error { return table.SetExpand(false) },
		func() error { return table.SetRowStyles("", "dim") },
	} {
		if err := operation(); err != nil {
			return nil, err
		}
	}
	headers := make([]*rich.Renderable, 0, 4)
	for _, value := range []string{"[green]Date", "[blue]Title", "[cyan]Production Budget", "[magenta]Box Office"} {
		header, err := markup(o, console, value)
		if err != nil {
			return nil, err
		}
		headers = append(headers, header)
	}
	minDate, minBudget, minOffice, ratio := uint32(12), uint32(17), uint32(14), uint32(1)
	right := rich.JustifyRight
	columns := []rich.ColumnOptions{
		{Style: "green", MinWidth: &minDate, NoWrap: true},
		{Style: "blue", Ratio: &ratio},
		{Style: "cyan", Justify: &right, MinWidth: &minBudget, NoWrap: true},
		{Style: "magenta", Justify: &right, MinWidth: &minOffice, NoWrap: true},
	}
	for i := range columns {
		if err := table.AddColumnOptions(headers[i], columns[i]); err != nil {
			return nil, err
		}
	}
	movies := [][]string{
		{"Dec 20, 2019", "Star Wars: The Rise of Skywalker", "$275,000,000", "$375,126,118"},
		{"May 25, 2018", "[b]Solo[/]: A Star Wars Story", "$275,000,000", "$393,151,347"},
		{"Dec 15, 2017", "Star Wars Ep. VIII: The Last Jedi", "$262,000,000", "[bold]$1,332,539,889[/]"},
		{"May 19, 1999", "Star Wars Ep. [b]I[/b]: [i]The phantom Menace[/i]", "$115,000,000", "$1,027,044,677"},
	}
	for _, movie := range movies {
		cells := make([]*rich.Renderable, 0, len(movie))
		for _, value := range movie {
			cell, err := markup(o, console, value)
			if err != nil {
				return nil, err
			}
			cells = append(cells, cell)
		}
		if err := table.AddRenderableRow(cells...); err != nil {
			return nil, err
		}
	}
	return finishTable(o, table)
}

func makeSyntaxPrettyComparison(o *owners) (*rich.Renderable, error) {
	code := "def iter_last(values: Iterable[T]) -> Iterable[Tuple[bool, T]]:\n    \"\"\"Iterate and generate a tuple with a flag for last value.\"\"\"\n    iter_values = iter(values)\n    try:\n        previous_value = next(iter_values)\n    except StopIteration:\n        return\n    for value in iter_values:\n        yield False, previous_value\n        previous_value = value\n    yield True, previous_value"
	prettyData := "{'foo': [3.1427, ('Paul Atreides', 'Vladimir Harkonnen', 'Thufir Hawat')], 'atomic': (False, True, None)}"
	syntax, err := rich.NewSyntax(code, "python3")
	if err != nil {
		return nil, err
	}
	o.add(syntax)
	if err := syntax.SetLineNumbers(true); err != nil {
		return nil, err
	}
	if err := syntax.SetIndentGuides(true); err != nil {
		return nil, err
	}
	syntaxRenderable, err := syntax.Finish()
	if err != nil {
		return nil, err
	}
	o.add(syntaxRenderable)
	pretty, err := rich.NewPretty(prettyData)
	if err != nil {
		return nil, err
	}
	o.add(pretty)
	if err := pretty.SetIndentGuides(true); err != nil {
		return nil, err
	}
	prettyRenderable, err := pretty.Finish()
	if err != nil {
		return nil, err
	}
	o.add(prettyRenderable)
	comparison, err := rich.NewTable()
	if err != nil {
		return nil, err
	}
	o.add(comparison)
	for _, operation := range []func() error{
		func() error { return comparison.SetBox(rich.BoxNone) },
		func() error { return comparison.SetShowHeader(false) },
		func() error { return comparison.SetExpand(true) },
		func() error { return comparison.SetPadEdge(false) },
		func() error { return comparison.SetPadding(1, 1) },
	} {
		if err := operation(); err != nil {
			return nil, err
		}
	}
	for i := 0; i < 2; i++ {
		ratio := uint32(1)
		if err := comparison.AddColumnOptions(nil, rich.ColumnOptions{Ratio: &ratio}); err != nil {
			return nil, err
		}
	}
	if err := comparison.AddRenderableRow(syntaxRenderable, prettyRenderable); err != nil {
		return nil, err
	}
	return finishTable(o, comparison)
}

func makeMarkdownComparison(o *owners) (*rich.Renderable, error) {
	source := "# Markdown\n\nSupports much of the *markdown* __syntax__!\n\n- Headers\n- Basic formatting: **bold**, *italic*, `code`\n- Block quotes\n- Lists, and more...\n"
	raw, err := styled(o, source, "cyan")
	if err != nil {
		return nil, err
	}
	markdown, err := rich.NewMarkdown(source)
	if err != nil {
		return nil, err
	}
	o.add(markdown)
	rendered, err := markdown.Finish()
	if err != nil {
		return nil, err
	}
	o.add(rendered)
	comparison, err := rich.NewTable()
	if err != nil {
		return nil, err
	}
	o.add(comparison)
	for _, operation := range []func() error{
		func() error { return comparison.SetBox(rich.BoxNone) },
		func() error { return comparison.SetShowHeader(false) },
		func() error { return comparison.SetExpand(true) },
		func() error { return comparison.SetPadEdge(false) },
	} {
		if err := operation(); err != nil {
			return nil, err
		}
	}
	for i := 0; i < 2; i++ {
		ratio := uint32(1)
		if err := comparison.AddColumnOptions(nil, rich.ColumnOptions{Ratio: &ratio}); err != nil {
			return nil, err
		}
	}
	if err := comparison.AddRenderableRow(raw, rendered); err != nil {
		return nil, err
	}
	return finishTable(o, comparison)
}

func makeTestCard(o *owners, console *rich.Console) (*rich.Renderable, error) {
	card, err := rich.NewGridTable()
	if err != nil {
		return nil, err
	}
	o.add(card)
	if err := card.SetTitleMarkup(console, "[italic]Rich features[/]"); err != nil {
		return nil, err
	}
	if err := card.SetPadding(1, 1); err != nil {
		return nil, err
	}
	if err := card.SetPadEdge(true); err != nil {
		return nil, err
	}
	width, center, top := uint32(12), rich.JustifyCenter, rich.VerticalTop
	if err := card.AddColumnOptions(nil, rich.ColumnOptions{Width: &width, Justify: &center, Vertical: &top, Style: "bold red", NoWrap: true}); err != nil {
		return nil, err
	}
	if err := card.AddColumnOptions(nil, rich.ColumnOptions{}); err != nil {
		return nil, err
	}
	colors, err := makeColorDemo(o, console)
	if err != nil {
		return nil, err
	}
	if err := addFeature(o, card, "Colors", colors); err != nil {
		return nil, err
	}
	if err := addBlank(o, card); err != nil {
		return nil, err
	}
	styles, err := markup(o, console, "All ansi styles: [bold]bold[/], [dim]dim[/], [italic]italic[/italic], [underline]underline[/], [strike]strikethrough[/], [reverse]reverse[/], and even [blink]blink[/].")
	if err != nil {
		return nil, err
	}
	if err := addFeature(o, card, "Styles", styles); err != nil {
		return nil, err
	}
	if err := addBlank(o, card); err != nil {
		return nil, err
	}
	textExamples, err := makeTextExamples(o, console)
	if err != nil {
		return nil, err
	}
	if err := addFeature(o, card, "Text", textExamples); err != nil {
		return nil, err
	}
	if err := addBlank(o, card); err != nil {
		return nil, err
	}
	asian, err := markup(o, console, "🇨🇳  该库支持中文，日文和韩文文本！\n🇯🇵  ライブラリは中国語、日本語、韓国語のテキストをサポートしています\n🇰🇷  이 라이브러리는 중국어, 일본어 및 한국어 텍스트를 지원합니다")
	if err != nil {
		return nil, err
	}
	if err := addFeature(o, card, "Asian\nlanguage\nsupport", asian); err != nil {
		return nil, err
	}
	if err := addBlank(o, card); err != nil {
		return nil, err
	}
	markupDemo, err := markup(o, console, "[bold magenta]Rich[/] supports a simple [i]bbcode[/i]-like [b]markup[/b] for [yellow]color[/], [underline]style[/], and emoji! :+1: :apple: :ant: :bear: :baguette_bread: :bus:")
	if err != nil {
		return nil, err
	}
	if err := addFeature(o, card, "Markup", markupDemo); err != nil {
		return nil, err
	}
	if err := addBlank(o, card); err != nil {
		return nil, err
	}
	movies, err := makeMovieTable(o, console)
	if err != nil {
		return nil, err
	}
	if err := addFeature(o, card, "Tables", movies); err != nil {
		return nil, err
	}
	if err := addBlank(o, card); err != nil {
		return nil, err
	}
	syntaxPretty, err := makeSyntaxPrettyComparison(o)
	if err != nil {
		return nil, err
	}
	if err := addFeature(o, card, "Syntax\nhighlighting\n&\npretty\nprinting", syntaxPretty); err != nil {
		return nil, err
	}
	if err := addBlank(o, card); err != nil {
		return nil, err
	}
	markdownDemo, err := makeMarkdownComparison(o)
	if err != nil {
		return nil, err
	}
	if err := addFeature(o, card, "Markdown", markdownDemo); err != nil {
		return nil, err
	}
	if err := addBlank(o, card); err != nil {
		return nil, err
	}
	if err := addBlank(o, card); err != nil {
		return nil, err
	}
	more, err := plain(o, "Progress bars, columns, styled logging handler, tracebacks, etc...")
	if err != nil {
		return nil, err
	}
	if err := addFeature(o, card, "+more!", more); err != nil {
		return nil, err
	}
	return finishTable(o, card)
}

func makeSponsorPanel(o *owners, console *rich.Console) (*rich.Renderable, error) {
	body, err := markup(o, console, "[b magenta]Hope you enjoy using Rich-rs![/]\n\nConsider sponsoring the upstream port to ensure it's maintained.\n\n[cyan]https://github.com/sponsors/willmcgugan[/cyan] | [b]Rust port: [/b][green]Marcos Saraiva - https://github.com/mrsaraiva/rich-rs[/green]")
	if err != nil {
		return nil, err
	}
	panel, err := rich.NewPanel(body)
	if err != nil {
		return nil, err
	}
	o.add(panel)
	if err := panel.SetTitle("Help ensure Rich is maintained"); err != nil {
		return nil, err
	}
	if err := panel.SetBorderStyle("green"); err != nil {
		return nil, err
	}
	if err := panel.SetPadding(rich.PaddingDimensions{Top: 1, Right: 2, Bottom: 1, Left: 2}); err != nil {
		return nil, err
	}
	renderable, err := panel.Finish()
	if err != nil {
		return nil, err
	}
	o.add(renderable)
	return renderable, nil
}

func renderDemo() (output string, err error) {
	var native owners
	defer func() {
		if closeErr := native.closeAll(); err == nil && closeErr != nil {
			output, err = "", closeErr
		}
	}()
	console, err := rich.NewConsole()
	if err != nil {
		return "", err
	}
	native.add(console)
	for _, operation := range []func() error{
		func() error { return console.SetForceTerminal(true) },
		func() error { return console.SetColorSystem(rich.ColorTruecolor) },
		func() error { return console.SetSize(120, 72) },
		func() error { return console.SetMarkupEnabled(true) },
		func() error { return console.SetEmojiEnabled(true) },
	} {
		if err := operation(); err != nil {
			return "", err
		}
	}
	card, err := makeTestCard(&native, console)
	if err != nil {
		return "", err
	}
	coldStart := time.Now()
	if _, err := console.Render(card); err != nil {
		return "", err
	}
	warmStart := time.Now()
	if _, err := console.Render(card); err != nil {
		return "", err
	}
	finish := time.Now()
	coldMS := float64(warmStart.Sub(coldStart).Nanoseconds()) / 1e6
	warmMS := float64(finish.Sub(warmStart).Nanoseconds()) / 1e6
	cardOutput, err := console.Render(card)
	if err != nil {
		return "", err
	}
	coldTiming, err := console.RenderMarkup(fmt.Sprintf("[dim]rendered in [not dim][cyan]%.1f[/cyan]ms[/] (cold cache)", coldMS))
	if err != nil {
		return "", err
	}
	warmTiming, err := console.RenderMarkup(fmt.Sprintf("[dim]rendered in [not dim][cyan]%.1f[/cyan]ms[/] (warm cache)", warmMS))
	if err != nil {
		return "", err
	}
	sponsor, err := makeSponsorPanel(&native, console)
	if err != nil {
		return "", err
	}
	sponsorOutput, err := console.Render(sponsor)
	if err != nil {
		return "", err
	}
	var result bytes.Buffer
	result.WriteString(cardOutput)
	result.WriteByte('\n')
	result.WriteString(coldTiming)
	result.WriteByte('\n')
	result.WriteString(warmTiming)
	result.WriteString("\n\n")
	result.WriteString(sponsorOutput)
	result.WriteByte('\n')
	return result.String(), nil
}

func main() {
	if len(os.Args) > 2 || (len(os.Args) == 2 && os.Args[1] != "--parity") {
		fmt.Fprintln(os.Stderr, "usage: demo [--parity]")
		os.Exit(2)
	}
	output, err := renderDemo()
	if err != nil {
		fmt.Fprintln(os.Stderr, "rich-rs Go demo:", err)
		os.Exit(1)
	}
	if _, err := os.Stdout.Write([]byte(output)); err != nil {
		fmt.Fprintln(os.Stderr, "rich-rs Go demo:", err)
		os.Exit(1)
	}
}
