package main

import (
	"fmt"
	"os"

	rich "github.com/mrsaraiva/rich-rs/bindings/go"
)

type closer interface{ Close() error }

func check(err error) {
	if err != nil {
		panic(err)
	}
}

func later(value closer) { check(value.Close()) }

func main() {
	console, err := rich.NewConsole()
	check(err)
	defer later(console)
	check(console.SetForceTerminal(false))
	check(console.SetSize(60, 24))

	text, err := rich.NewText("reusable rich-rs")
	check(err)
	defer later(text)
	check(text.SetStyle("bold cyan"))
	reusable, err := text.Finish()
	check(err)
	defer later(reusable)

	table, err := rich.NewTable()
	check(err)
	defer later(table)
	check(table.SetTitle("Go binding"))
	check(table.AddColumn("first"))
	check(table.AddColumn("second"))
	check(table.AddRenderableRow(reusable, reusable))
	tableRenderable, err := table.Finish()
	check(err)
	defer later(tableRenderable)

	panel, err := rich.NewPanel(tableRenderable)
	check(err)
	defer later(panel)
	check(panel.SetTitle("rich-rs"))
	check(panel.SetExpand(false))
	panelRenderable, err := panel.Finish()
	check(err)
	defer later(panelRenderable)

	columns, err := rich.NewColumns()
	check(err)
	defer later(columns)
	check(columns.Add(panelRenderable))
	check(columns.Add(reusable))
	check(columns.Add(reusable))
	columnsRenderable, err := columns.Finish()
	check(err)
	defer later(columnsRenderable)

	output, err := console.Render(columnsRenderable)
	check(err)
	_, err = fmt.Fprint(os.Stdout, output)
	check(err)
}
