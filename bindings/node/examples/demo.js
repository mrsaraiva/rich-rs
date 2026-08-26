'use strict';

const rich = require('..');
const { performance } = require('node:perf_hooks');

const owners = [];
const own = (owner) => {
  owners.push(owner);
  return owner;
};

function finish(builder) {
  return own(builder.finish());
}

function plain(text) {
  return finish(own(new rich.Text(text)));
}

function markup(console, text) {
  return finish(own(rich.Text.fromMarkup(console, text)));
}

function hlsToRgb(hue, lightness, saturation) {
  if (saturation === 0) {
    const value = Math.trunc(lightness * 255);
    return [value, value, value];
  }
  const q =
    lightness < 0.5
      ? lightness * (1 + saturation)
      : lightness + saturation - lightness * saturation;
  const p = 2 * lightness - q;
  const component = (offset) => {
    let value = offset;
    if (value < 0) value += 1;
    if (value > 1) value -= 1;
    if (value < 1 / 6) return p + (q - p) * 6 * value;
    if (value < 1 / 2) return q;
    if (value < 2 / 3) return p + (q - p) * (2 / 3 - value) * 6;
    return p;
  };
  return [
    Math.trunc(component(hue + 1 / 3) * 255),
    Math.trunc(component(hue) * 255),
    Math.trunc(component(hue - 1 / 3) * 255),
  ];
}

function colorBox() {
  return own(
    rich.customRenderable({
      measure(options) {
        return new rich.Measurement(1, options.maxWidth);
      },
      render(options, sink) {
        for (let y = 0; y < 5; ++y) {
          for (let x = 0; x < options.maxWidth; ++x) {
            const hue = x / options.maxWidth;
            const lower = hlsToRgb(hue, 0.1 + (y / 5) * 0.7, 1);
            const upper = hlsToRgb(hue, 0.1 + (y / 5) * 0.7 + 0.7 / 10, 1);
            sink.push(
              '▄',
              `rgb(${upper[0]},${upper[1]},${upper[2]}) on rgb(${lower[0]},${lower[1]},${lower[2]})`,
            );
          }
          sink.line();
        }
      },
    }),
  );
}

function makeTestCard(console) {
  const table = own(
    rich.Table.grid()
      .setPadding(1, 1)
      .setPadEdge(true)
      .setTitleMarkup(console, '[italic]Rich features'),
  );
  table.addColumnOptions({
    justify: rich.Justify.Center,
    vertical: rich.VerticalAlign.Top,
    width: 12,
    style: 'bold red',
    noWrap: true,
  });
  table.addColumnOptions({});

  const colors = own(
    new rich.Table()
      .setBox(rich.Box.None)
      .setExpand(false)
      .setShowHeader(false)
      .setShowEdge(false)
      .setPadEdge(false),
  );
  colors.addRowRenderables([
    markup(
      console,
      '✓ [bold green]4-bit color[/]\n✓ [bold blue]8-bit color[/]\n✓ [bold magenta]Truecolor (16.7 million)[/]\n✓ [bold yellow]Dumb terminals[/]\n✓ [bold cyan]Automatic color conversion',
    ),
    colorBox(),
  ]);
  table.addRowRenderables([plain('Colors'), finish(colors)]);
  table.addRowRenderables([plain(''), plain('')]);

  table.addRowRenderables([
    plain('Styles'),
    markup(
      console,
      'All ansi styles: [bold]bold[/], [dim]dim[/], [italic]italic[/italic], [underline]underline[/], [strike]strikethrough[/], [reverse]reverse[/], and even [blink]blink[/].',
    ),
  ]);
  table.addRowRenderables([plain(''), plain('')]);

  const lorem =
    'Lorem ipsum dolor sit amet, consectetur adipiscing elit. Quisque in metus sed sapien ultricies pretium a at justo. Maecenas luctus velit et auctor maximus.';
  const loremTable = own(rich.Table.grid().setPadding(1, 1).setPadEdge(false));
  loremTable.addColumnOptions({ ratio: 1, justify: rich.Justify.Left });
  loremTable.addColumnOptions({ ratio: 1, justify: rich.Justify.Center });
  loremTable.addColumnOptions({ ratio: 1, justify: rich.Justify.Right });
  loremTable.addColumnOptions({ ratio: 1, justify: rich.Justify.Full });
  loremTable.addRowRenderables([
    finish(own(new rich.Text(lorem).setStyle('green'))),
    finish(own(new rich.Text(lorem).setStyle('yellow'))),
    finish(own(new rich.Text(lorem).setStyle('blue'))),
    finish(own(new rich.Text(lorem).setStyle('red'))),
  ]);
  const textSection = own(rich.Table.grid().setPadding(0, 0));
  textSection.addColumnOptions({});
  textSection.addRowRenderables([
    markup(
      console,
      'Word wrap text. Justify [green]left[/], [yellow]center[/], [blue]right[/] or [red]full[/].\n',
    ),
  ]);
  textSection.addRowRenderables([plain('')]);
  textSection.addRowRenderables([finish(loremTable)]);
  table.addRowRenderables([plain('Text'), finish(textSection)]);
  table.addRowRenderables([plain(''), plain('')]);

  table.addRowRenderables([
    plain('Asian\nlanguage\nsupport'),
    markup(
      console,
      ':flag_for_china:  该库支持中文，日文和韩文文本！\n:flag_for_japan:  ライブラリは中国語、日本語、韓国語のテキストをサポートしています\n:flag_for_south_korea:  이 라이브러리는 중국어, 일본어 및 한국어 텍스트를 지원합니다',
    ),
  ]);
  table.addRowRenderables([plain(''), plain('')]);

  table.addRowRenderables([
    plain('Markup'),
    markup(
      console,
      '[bold magenta]Rich[/] supports a simple [i]bbcode[/i]-like [b]markup[/b] for [yellow]color[/], [underline]style[/], and emoji! :+1: :apple: :ant: :bear: :baguette_bread: :bus: ',
    ),
  ]);
  table.addRowRenderables([plain(''), plain('')]);

  const movies = own(
    new rich.Table()
      .setShowEdge(false)
      .setShowHeader(true)
      .setExpand(false)
      .setBox(rich.Box.Simple)
      .setRowStyles(['', 'dim']),
  );
  movies.addColumnOptions(
    { style: 'green', noWrap: true, minWidth: 12 },
    markup(console, '[green]Date'),
  );
  movies.addColumnOptions(
    { style: 'blue', ratio: 1 },
    markup(console, '[blue]Title'),
  );
  movies.addColumnOptions(
    { style: 'cyan', justify: rich.Justify.Right, noWrap: true, minWidth: 17 },
    markup(console, '[cyan]Production Budget'),
  );
  movies.addColumnOptions(
    { style: 'magenta', justify: rich.Justify.Right, noWrap: true, minWidth: 14 },
    markup(console, '[magenta]Box Office'),
  );
  const movieRows = [
    ['Dec 20, 2019', 'Star Wars: The Rise of Skywalker', '$275,000,000', '$375,126,118'],
    ['May 25, 2018', '[b]Solo[/]: A Star Wars Story', '$275,000,000', '$393,151,347'],
    [
      'Dec 15, 2017',
      'Star Wars Ep. VIII: The Last Jedi',
      '$262,000,000',
      '[bold]$1,332,539,889[/bold]',
    ],
    [
      'May 19, 1999',
      'Star Wars Ep. [b]I[/b]: [i]The phantom Menace',
      '$115,000,000',
      '$1,027,044,677',
    ],
  ];
  for (const row of movieRows) movies.addRowRenderables(row.map((cell) => markup(console, cell)));
  table.addRowRenderables([plain('Tables'), finish(movies)]);
  table.addRowRenderables([plain(''), plain('')]);

  const code = `def iter_last(values: Iterable[T]) -> Iterable[Tuple[bool, T]]:
    """Iterate and generate a tuple with a flag for last value."""
    iter_values = iter(values)
    try:
        previous_value = next(iter_values)
    except StopIteration:
        return
    for value in iter_values:
        yield False, previous_value
        previous_value = value
    yield True, previous_value`;
  const syntax = finish(
    own(new rich.Syntax(code, 'python3').setLineNumbers(true).setIndentGuides(true)),
  );
  const pretty = finish(
    own(
      new rich.Pretty(
        "{'foo': [3.1427, ('Paul Atreides', 'Vladimir Harkonnen', 'Thufir Hawat')], 'atomic': (False, True, None)}",
      ).setIndentGuides(true),
    ),
  );
  const comparison = own(
    new rich.Table()
      .setShowHeader(false)
      .setPadEdge(false)
      .setBox(rich.Box.None)
      .setPadding(1, 1)
      .setExpand(true),
  );
  comparison.addColumnOptions({ ratio: 1 });
  comparison.addColumnOptions({ ratio: 1 });
  comparison.addRowRenderables([syntax, pretty]);
  table.addRowRenderables([
    plain('Syntax\nhighlighting\n&\npretty\nprinting'),
    finish(comparison),
  ]);
  table.addRowRenderables([plain(''), plain('')]);

  const markdownSource = `# Markdown

Supports much of the *markdown* __syntax__!

- Headers
- Basic formatting: **bold**, *italic*, \`code\`
- Block quotes
- Lists, and more...
`;
  const markdownComparison = own(
    new rich.Table()
      .setShowHeader(false)
      .setPadEdge(false)
      .setBox(rich.Box.None)
      .setExpand(true),
  );
  markdownComparison.addColumnOptions({ ratio: 1 });
  markdownComparison.addColumnOptions({ ratio: 1 });
  markdownComparison.addRowRenderables([
    markup(console, `[cyan]${markdownSource}`),
    finish(own(new rich.Markdown(markdownSource))),
  ]);
  table.addRowRenderables([plain('Markdown'), finish(markdownComparison)]);
  table.addRowRenderables([plain(''), plain('')]);
  table.addRowRenderables([plain(''), plain('')]);
  table.addRowRenderables([
    plain('+more!'),
    plain('Progress bars, columns, styled logging handler, tracebacks, etc...'),
  ]);
  return finish(table);
}

function parityConsole() {
  return own(
    new rich.Console()
      .setSize(120, 72)
      .setForceTerminal(true)
      .setColorSystem(rich.ColorSystem.TrueColor)
      .setMarkupEnabled(true)
      .setEmojiEnabled(true),
  );
}

function renderTiming(console, milliseconds, label) {
  const text = markup(
    console,
    `[dim]rendered in [not dim][cyan]${milliseconds.toFixed(1)}[/cyan]ms[/] (${label} cache)`,
  );
  process.stdout.write(console.render(text));
  process.stdout.write('\n');
}

function main() {
  try {
    const measureConsole = parityConsole();
    const outputConsole = parityConsole();
    const card = makeTestCard(measureConsole);
    let started = performance.now();
    measureConsole.render(card);
    const cold = performance.now() - started;
    started = performance.now();
    measureConsole.render(card);
    const warm = performance.now() - started;

    process.stdout.write(outputConsole.render(card));
    process.stdout.write('\n');
    renderTiming(outputConsole, cold, 'cold');
    renderTiming(outputConsole, warm, 'warm');
    process.stdout.write('\n');

    const sponsorBody = markup(
      outputConsole,
      "[b magenta]Hope you enjoy using Rich-rs![/]\n\nConsider sponsoring the upstream port to ensure it's maintained.\n\n[cyan]https://github.com/sponsors/willmcgugan[/cyan] | [b]Rust port: [/b][green]Marcos Saraiva - https://github.com/mrsaraiva/rich-rs[/green]",
    );
    const sponsor = finish(
      own(
        new rich.Panel(sponsorBody)
          .setTitle('Help ensure Rich is maintained')
          .setBorderStyle('green')
          .setPadding(1, 2, 1, 2),
      ),
    );
    process.stdout.write(outputConsole.render(sponsor));
    process.stdout.write('\n');
  } finally {
    for (const owner of owners.reverse()) owner.close();
  }
}

main();
