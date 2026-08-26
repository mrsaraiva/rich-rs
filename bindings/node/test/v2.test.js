'use strict';

const assert = require('node:assert/strict');
const test = require('node:test');

process.env.RICH_RS_NATIVE_TEST = '1';
const rich = require('..');

function terminalConsole(width) {
  return new rich.Console()
    .setForceTerminal(true)
    .setColorSystem(rich.ColorSystem.TrueColor)
    .setSize(width, 40);
}

function customLines(height, observations = []) {
  return rich.customRenderable({
    measure(options) {
      observations.push({ kind: 'measure', options });
      return new rich.Measurement(1, options.maxWidth);
    },
    render(options, sink) {
      observations.push({ kind: 'render', options });
      for (let y = 0; y < height; ++y) {
        for (let x = 0; x < options.maxWidth; ++x) {
          sink.push('x', `rgb(${x % 256},${y},1) on rgb(0,0,0)`);
        }
        sink.line();
      }
    },
  });
}

test('ABI v2 values are immutable and guarded by exact receiver classes', () => {
  assert.equal(rich.Box.None, 14);
  assert.throws(() => new rich.RenderOptions(), TypeError);
  assert.throws(() => new rich.SegmentSink(), TypeError);
  const measurement = new rich.Measurement(1, 9);
  assert.ok(measurement instanceof rich.Measurement);
  assert.equal(Object.isFrozen(measurement), true);
  assert.throws(() => rich.SegmentSink.prototype.line.call({}), TypeError);
  assert.throws(() => new rich.Measurement(1), TypeError);
  assert.throws(() => new rich.Measurement(-1, 2), RangeError);
});

test('custom renderables receive adaptive immutable options and emit native styled segments', () => {
  for (const height of [1, 3, 5]) {
    for (const width of [1, 7, 120]) {
      const observations = [];
      const console = terminalConsole(width);
      const renderable = customLines(height, observations);
      try {
        const output = console.render(renderable);
        const render = observations.find((entry) => entry.kind === 'render');
        assert.ok(render);
        assert.ok(render.options instanceof rich.RenderOptions);
        assert.equal(Object.isFrozen(render.options), true);
        assert.equal(render.options.maxWidth, width);
        assert.equal(render.options.colorSystem, rich.ColorSystem.TrueColor);
        assert.equal(typeof render.options.isTerminal, 'boolean');
        assert.equal((output.match(/x/g) ?? []).length, width * height);
        assert.equal((output.match(/\n/g) ?? []).length, height);
        assert.match(output, /38;2;/);
        assert.match(output, /48;2;/);
      } finally {
        renderable.close();
        console.close();
      }
    }
  }
});

test('custom context lives through retained container Arcs and drops exactly once', () => {
  const before = rich._testing.customStats().drops;
  const custom = rich.customRenderable({ render(_options, sink) { sink.push('retained'); } });
  const table = rich.Table.grid().addRowRenderables([custom]);
  custom.close();
  assert.equal(rich._testing.customStats().drops, before);
  const retained = table.finish();
  table.close();
  assert.equal(rich._testing.customStats().drops, before);
  retained.close();
  assert.equal(rich._testing.customStats().drops, before + 1n);
  custom.close();
  retained.close();
});

test('custom measurement is invoked synchronously with the assigned native width', () => {
  const observations = [];
  const console = new rich.Console().setForceTerminal(false).setSize(37, 20);
  const custom = customLines(1, observations);
  const table = new rich.Table()
    .setShowHeader(false)
    .setShowEdge(false)
    .setPadEdge(false)
    .addColumnOptions({ ratio: 1 })
    .addRowRenderables([custom]);
  const renderable = table.finish();
  try {
    assert.match(console.render(renderable), /x/);
    const measurements = observations.filter((entry) => entry.kind === 'measure');
    assert.ok(measurements.length > 0);
    for (const entry of measurements) {
      assert.ok(entry.options instanceof rich.RenderOptions);
      assert.equal(entry.options.maxWidth, 37);
    }
  } finally {
    renderable.close();
    table.close();
    custom.close();
    console.close();
  }
});

test('callback exceptions preserve identity, discard staged output, and tombstone the sink', () => {
  const console = new rich.Console().setForceTerminal(false).setSize(40, 20);
  const sentinel = new Error('sentinel identity');
  let retainedSink;
  const broken = rich.customRenderable({
    render(_options, sink) {
      retainedSink = sink;
      sink.push('must not commit');
      throw sentinel;
    },
  });
  try {
    assert.throws(() => console.render(broken), (error) => error === sentinel);
    assert.throws(
      () => retainedSink.push('late'),
      (error) => error.code === rich.RichErrorCode.InvalidState,
    );
    assert.throws(
      () => retainedSink.line(),
      (error) => error.code === rich.RichErrorCode.InvalidState,
    );
  } finally {
    broken.close();
    console.close();
  }
});

test('callback scope rejects reentry and ordinary calls without closing owners', () => {
  const console = new rich.Console().setForceTerminal(false).setSize(40, 20);
  let custom;
  custom = rich.customRenderable({
    render(_options, sink) {
      assert.throws(
        () => console.width(),
        (error) => error.code === rich.RichErrorCode.InvalidState,
      );
      assert.throws(
        () => console.close(),
        (error) => error.code === rich.RichErrorCode.InvalidState,
      );
      assert.throws(
        () => console.render(custom),
        (error) => error.code === rich.RichErrorCode.InvalidState,
      );
      sink.push('safe');
    },
  });
  try {
    assert.equal(console.render(custom), 'safe');
    assert.equal(console.width(), 40);
  } finally {
    custom.close();
    console.close();
  }
});

test('measure failures and malformed measurements become outer render failures', () => {
  const console = new rich.Console().setForceTerminal(false).setSize(30, 20);
  const sentinel = new Error('measure identity');
  const throwing = rich.customRenderable({
    render(_options, sink) { sink.push('unused'); },
    measure() { throw sentinel; },
  });
  const malformed = rich.customRenderable({
    render(_options, sink) { sink.push('unused'); },
    measure() { return new rich.Measurement(10, 1); },
  });
  const makeTable = (value) => {
    const table = new rich.Table().setShowHeader(false).addRowRenderables([value]);
    return { table, renderable: table.finish() };
  };
  const throwingTable = makeTable(throwing);
  const malformedTable = makeTable(malformed);
  try {
    assert.throws(() => console.render(throwingTable.renderable), (error) => error === sentinel);
    assert.throws(
      () => console.render(malformedTable.renderable),
      (error) => error.code === rich.RichErrorCode.Render,
    );
  } finally {
    malformedTable.renderable.close();
    malformedTable.table.close();
    throwingTable.renderable.close();
    throwingTable.table.close();
    malformed.close();
    throwing.close();
    console.close();
  }
});

test('asynchronous or otherwise non-void render callbacks are rejected transactionally', () => {
  const console = new rich.Console().setForceTerminal(false);
  const asynchronous = rich.customRenderable({ async render() {} });
  const nonVoid = rich.customRenderable({ render(_options, sink) { return sink.push('staged'); } });
  try {
    for (const renderable of [asynchronous, nonVoid]) {
      assert.throws(
        () => console.render(renderable),
        (error) => error.code === rich.RichErrorCode.Render,
      );
    }
  } finally {
    nonVoid.close();
    asynchronous.close();
    console.close();
  }
});

test('custom construction validates shape and cleans post-native-success wrapper failures', () => {
  assert.throws(() => rich.customRenderable({}), TypeError);
  assert.throws(() => rich.customRenderable({ render: 1 }), TypeError);
  assert.throws(() => rich.customRenderable({ render() {}, surprise: true }), TypeError);
  assert.throws(() => rich.customRenderable({ render() {}, measure: false }), TypeError);
  const before = rich._testing.customStats().drops;
  rich._testing.handle('armAllocationThrow');
  assert.throws(
    () => rich.customRenderable({ render(_options, sink) { sink.push('never'); } }),
    /injected owner-wrapper/,
  );
  assert.equal(rich._testing.customStats().drops, before + 1n);
});

test('Pretty and Syntax project native guide behavior and preserve length-aware input', () => {
  const console = terminalConsole(24);
  const owners = [];
  const render = (builder) => {
    owners.push(builder);
    const value = builder.finish();
    owners.push(value);
    return console.render(value);
  };
  try {
    const prettySource = "{'世界': [1, {'nul': 'a\0b'}]}";
    const finishedPretty = new rich.Pretty(prettySource).setIndentGuides(false);
    const plainPretty = render(finishedPretty);
    assert.throws(
      () => finishedPretty.setIndentGuides(true),
      (error) => error.code === rich.RichErrorCode.InvalidState,
    );
    finishedPretty.close();
    finishedPretty.close();
    const guidedPretty = render(new rich.Pretty(prettySource).setIndentGuides(true));
    assert.match(plainPretty, /世界/);
    assert.match(plainPretty, /a\0b/);
    assert.notEqual(guidedPretty, plainPretty);

    const source = 'if ready:\n    if nested:\n        value = 1';
    const plainSyntax = render(new rich.Syntax(source, 'python3').setIndentGuides(false));
    const guidedSyntax = render(new rich.Syntax(source, 'python3').setIndentGuides(true));
    assert.notEqual(guidedSyntax, plainSyntax);
  } finally {
    for (const owner of owners.reverse()) owner.close();
    console.close();
  }
});

test('grid/table v2 options are strict, atomic, and retain configured headers', () => {
  const console = new rich.Console().setForceTerminal(false).setSize(36, 20);
  const headerBuilder = new rich.Text('header');
  const header = headerBuilder.finish();
  const table = rich.Table.grid()
    .setShowHeader(true)
    .setPadEdge(false)
    .setRowStyles(['', 'dim'])
    .setBox(rich.Box.None)
    .addColumnOptions(
      {
        justify: rich.Justify.Right,
        vertical: rich.VerticalAlign.Top,
        width: 12,
        minWidth: 3,
        ratio: 1,
        style: 'green',
        noWrap: true,
      },
      header,
    );
  header.close();
  try {
    assert.throws(() => table.addColumnOptions({ unknown: true }), TypeError);
    assert.throws(
      () => table.addColumnOptions({ width: 0 }),
      (error) => error.code === rich.RichErrorCode.InvalidArgument,
    );
    assert.throws(
      () => table.addColumnOptions({ justify: 999 }),
      (error) => error.code === rich.RichErrorCode.InvalidArgument,
    );
    assert.throws(() => table.setRowStyles(['valid', 1]), TypeError);
    table.addRowStrings(['right']);
    table.setRowStyles([]).setRowStyles(['', 'dim']);
    const renderable = table.finish();
    try {
      assert.throws(
        () => table.setPadEdge(true),
        (error) => error.code === rich.RichErrorCode.InvalidState,
      );
      const output = console.render(renderable);
      assert.match(output, /header/);
      assert.match(output, /right/);
      assert.doesNotMatch(output, /[┌┐└┘│]/u);
    } finally {
      renderable.close();
    }
  } finally {
    table.close();
    header.close();
    headerBuilder.close();
    console.close();
  }
});

test('custom renderable GC fallback reaches the native last-Arc drop', async () => {
  const before = rich._testing.customStats().drops;
  (() => {
    rich.customRenderable({ render(_options, sink) { sink.push('gc'); } });
  })();
  for (let attempt = 0; attempt < 100; ++attempt) {
    global.gc();
    await new Promise((resolve) => setImmediate(resolve));
    if (rich._testing.customStats().drops > before) break;
  }
  assert.equal(rich._testing.customStats().drops, before + 1n);
});
