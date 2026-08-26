'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const test = require('node:test');

const rich = require('..');

function renderAndClose(console, builder) {
  const renderable = builder.finish();
  try {
    const output = console.render(renderable);
    assert.equal(typeof output, 'string');
    return output;
  } finally {
    renderable.close();
    builder.close();
  }
}

test('one renderable is reusable across every composition family and after insertion', () => {
  const console = new rich.Console().setForceTerminal(false).setSize(100, 30);
  const source = new rich.Text('shared');
  const shared = source.finish();
  const builders = [];
  const outputs = [];
  try {
    const table = new rich.Table()
      .addColumnRenderable(shared)
      .addRowRenderables([shared]);
    builders.push(table);
    const panel = new rich.Panel(shared);
    builders.push(panel);
    const tree = rich.Tree.fromRenderable(shared);
    tree.addRenderable(rich.ROOT_NODE_ID, shared);
    builders.push(tree);
    const columns = new rich.Columns().add(shared);
    builders.push(columns);
    builders.push(new rich.Align(shared, rich.HorizontalAlign.Center));
    builders.push(new rich.Padding(shared, 0, 1, 0, 1));

    assert.equal(console.render(shared), 'shared');
    shared.close();
    for (const builder of builders) outputs.push(builder.finish());
    for (const renderable of outputs) assert.match(console.render(renderable), /shared/);
  } finally {
    for (const renderable of outputs) renderable.close();
    for (const builder of builders) builder.close();
    shared.close();
    source.close();
    console.close();
  }
});

test('table and panel expose every option and row validation is atomic', () => {
  const console = new rich.Console().setForceTerminal(false).setSize(80, 24);
  const cellBuilder = new rich.Text('renderable cell');
  const cell = cellBuilder.finish();
  const table = new rich.Table()
    .setTitle('title')
    .setCaption('caption')
    .setTitleMarkup(console, '[bold]title[/bold]')
    .setCaptionMarkup(console, '[italic]caption[/italic]')
    .setBox(rich.Box.Ascii)
    .setShowHeader(true)
    .setShowLines(true)
    .setShowEdge(true)
    .setExpand(false)
    .setPadding(1, 1)
    .setStyle('blue')
    .addColumn('plain')
    .addColumnRenderable(cell);
  try {
    assert.throws(() => table.addRowStrings(['not inserted', 2]), TypeError);
    assert.throws(() => table.addRowRenderables([cell, {}]), TypeError);
    table.addRowStrings(['one', 'two']).addRowRenderables([cell, cell]);
    const tableRenderable = table.finish();
    const panel = new rich.Panel(tableRenderable)
      .setTitle('panel')
      .setSubtitle('subtitle')
      .setBox(rich.Box.Rounded)
      .setExpand(false)
      .setWidth(70)
      .setPadding(0, 1, 0, 1)
      .setStyle('white')
      .setBorderStyle('cyan');
    try {
      const output = renderAndClose(console, panel);
      assert.match(output, /one/);
      assert.doesNotMatch(output, /not inserted/);
    } finally {
      panel.close();
      tableRenderable.close();
    }
  } finally {
    table.close();
    cell.close();
    cellBuilder.close();
    console.close();
  }
});

test('tree IDs are BigInt, stable at breadth/depth, and invalid IDs do not mutate', () => {
  const console = new rich.Console().setForceTerminal(false).setSize(100, 50);
  const tree = new rich.Tree('root').setStyle('white').setGuideStyle('blue').setHideRoot(false);
  try {
    const siblings = [];
    for (let index = 0; index < 30; ++index) {
      siblings.push(tree.add(rich.ROOT_NODE_ID, `sibling-${index}`));
    }
    let parent = siblings[0];
    for (let depth = 0; depth < 30; ++depth) parent = tree.add(parent, `depth-${depth}`);
    assert.equal(typeof parent, 'bigint');
    assert.throws(
      () => tree.add(2n ** 63n, 'invalid'),
      (error) => error.code === rich.RichErrorCode.InvalidState,
    );
    assert.throws(() => tree.add(1, 'not-bigint'), TypeError);
    const output = renderAndClose(console, tree);
    assert.match(output, /sibling-29/);
    assert.match(output, /depth-29/);
    assert.doesNotMatch(output, /invalid/);
  } finally {
    tree.close();
    console.close();
  }
});

test('rule, columns, align, padding, syntax, markdown, and JSON render strings', () => {
  const console = new rich.Console().setForceTerminal(false).setSize(100, 30);
  const owners = [];
  try {
    const rule = new rich.Rule()
      .setTitle('rule')
      .setCharacters('-')
      .setStyle('cyan')
      .setAlign(rich.HorizontalAlign.Center);
    owners.push(rule);
    assert.match(renderAndClose(console, rule), /rule/);

    const columns = new rich.Columns()
      .addString('one')
      .addString('two')
      .setEqual(true)
      .setExpand(false)
      .setPadding(0, 2);
    owners.push(columns);
    assert.match(renderAndClose(console, columns), /one/);

    const baseBuilder = new rich.Text('layout');
    const base = baseBuilder.finish();
    owners.push(baseBuilder, base);
    const align = rich.Align.center(base)
      .setVertical(rich.VerticalAlign.Middle)
      .setWidth(40);
    owners.push(align);
    assert.match(renderAndClose(console, align), /layout/);
    const padding = new rich.Padding(base, 1, 2, 1, 2).setStyle('blue').setExpand(false);
    owners.push(padding);
    assert.match(renderAndClose(console, padding), /layout/);

    const syntax = new rich.Syntax('const x = 1;', 'javascript')
      .setTheme('monokai')
      .setLineNumbers(true)
      .setWordWrap(true)
      .setLineRange(null, 4);
    owners.push(syntax);
    assert.match(renderAndClose(console, syntax), /const/);
    const temporary = fs.mkdtempSync(path.join(os.tmpdir(), 'rich-node-syntax-'));
    const sourcePath = path.join(temporary, 'example.js');
    fs.writeFileSync(sourcePath, 'let fromPath = true;');
    try {
      const fromPath = rich.Syntax.fromPath(sourcePath);
      owners.push(fromPath);
      assert.match(renderAndClose(console, fromPath), /fromPath/);
    } finally {
      fs.rmSync(temporary, { recursive: true, force: true });
    }
    const markdown = new rich.Markdown('# heading')
      .setCodeTheme('monokai')
      .setHyperlinks(false)
      .setJustify(rich.Justify.Left);
    owners.push(markdown);
    assert.match(renderAndClose(console, markdown), /heading/);
    const json = new rich.Json('{"b":2,"a":1}', {
      indent: 4,
      highlight: true,
      sortKeys: true,
    });
    owners.push(json);
    assert.match(renderAndClose(console, json), /"a"/);
  } finally {
    for (const owner of owners.reverse()) owner.close();
    console.close();
  }
});

test('content and live native errors, BigInt precision, task zero, and finite rules', () => {
  const console = new rich.Console().setForceTerminal(false);
  assert.throws(
    () => rich.Syntax.fromPath('/definitely/not/a/rich-rs-file'),
    (error) => error.code === rich.RichErrorCode.Io,
  );
  assert.throws(
    () => new rich.Spinner('definitely-not-a-spinner'),
    (error) => error.code === rich.RichErrorCode.InvalidArgument,
  );
  const progress = new rich.Progress();
  const status = new rich.Status('working');
  const spinner = new rich.Spinner('dots');
  try {
    const task = progress.addTask('zero', 10);
    assert.equal(task, 0n);
    progress.update(task, 5);
    assert.equal(typeof progress.renderFrame(console), 'string');
    assert.throws(
      () => progress.update(BigInt(Number.MAX_SAFE_INTEGER) + 100n, 1),
      (error) => error.code === rich.RichErrorCode.InvalidState,
    );
    assert.throws(() => progress.update(0, 1), TypeError);
    assert.throws(() => progress.update(-1n, 1), RangeError);
    assert.equal(typeof status.renderFrame(console), 'string');
    assert.equal(typeof spinner.renderFrame(console, 0.25), 'string');
    assert.throws(
      () => spinner.renderFrame(console, Number.NaN),
      (error) => error.code === rich.RichErrorCode.InvalidArgument,
    );
    assert.throws(
      () => progress.addTask('bad', Number.POSITIVE_INFINITY),
      (error) => error.code === rich.RichErrorCode.InvalidArgument,
    );
  } finally {
    progress.close();
    status.close();
    spinner.close();
    console.close();
  }
});
