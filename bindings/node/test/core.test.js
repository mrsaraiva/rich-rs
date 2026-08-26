'use strict';

const assert = require('node:assert/strict');
const test = require('node:test');

process.env.RICH_RS_NATIVE_TEST = '1';
const rich = require('..');
const foreignAddon = require('./foreign-addon/build/Release/foreign_owner.node');

function plainConsole() {
  return new rich.Console()
    .setForceTerminal(false)
    .setSize(80, 24)
    .setMarkupEnabled(true)
    .setEmojiEnabled(true);
}

test('length-aware UTF-8 preserves empty, Unicode, embedded NUL, and surrogate policy', () => {
  const console = plainConsole();
  try {
    assert.equal(console.renderMarkup(''), '');
    assert.equal(console.renderMarkup('héllo 世界 🦀'), 'héllo 世界 🦀');
    assert.equal(console.renderMarkup('a\0b'), 'a\0b');
    assert.equal(console.renderMarkup('a\0b').length, 3);
    assert.equal(console.renderMarkup('\ud800'), '\ufffd');
    assert.equal(console.width(), 80);
    const markupBuilder = rich.Text.fromMarkup(console, '[bold]markup[/bold]');
    const markup = markupBuilder.finish();
    try {
      assert.equal(console.render(markup), 'markup');
    } finally {
      markup.close();
      markupBuilder.close();
    }
  } finally {
    console.close();
  }
});

test('plain and terminal consoles preserve the ANSI/OSC contract', () => {
  const plain = plainConsole();
  const terminal = new rich.Console()
    .setForceTerminal(true)
    .setColorSystem(rich.ColorSystem.TrueColor);
  try {
    const plainOutput = plain.renderMarkup('[bold red]styled[/bold red]');
    const terminalOutput = terminal.renderMarkup('[bold red]styled[/bold red]');
    assert.doesNotMatch(plainOutput, /\x1b(?:\[|\])/);
    assert.match(terminalOutput, /\x1b\[/);
    assert.match(terminalOutput, /styled/);
  } finally {
    plain.close();
    terminal.close();
  }
});

test('finish invalidates on success while native pre-transfer failure retains the builder', () => {
  const console = plainConsole();
  const text = new rich.Text('retained');
  try {
    assert.deepEqual(rich._testing.finishFailure(text), {
      code: rich.RichErrorCode.NullArgument,
      retained: true,
    });
    text.setStyle('bold');
    const renderable = text.finish();
    try {
      assert.equal(console.render(renderable), 'retained');
      assert.throws(
        () => text.setStyle('red'),
        (error) => error.code === rich.RichErrorCode.InvalidState,
      );
      assert.throws(
        () => text.finish(),
        (error) => error.code === rich.RichErrorCode.InvalidState,
      );
    } finally {
      renderable.close();
    }
  } finally {
    text.close();
    text.close();
    console.close();
  }
});

test('all owners close idempotently and reject later access locally', () => {
  const owners = [];
  const own = (owner) => {
    owners.push(owner);
    return owner;
  };
  const console = own(plainConsole());
  const text = own(new rich.Text('base'));
  const renderable = own(text.finish());
  own(new rich.Style('bold'));
  own(new rich.Table());
  own(new rich.Panel(renderable));
  own(new rich.Tree('root'));
  own(new rich.Rule());
  own(new rich.Columns());
  own(new rich.Align(renderable, rich.HorizontalAlign.Left));
  own(new rich.Padding(renderable, 0, 0, 0, 0));
  own(new rich.Syntax('x', 'text'));
  own(new rich.Markdown('x'));
  own(new rich.Json('{}'));
  own(new rich.Progress());
  own(new rich.Status('x'));
  own(new rich.Spinner('dots'));

  for (const owner of owners.reverse()) {
    owner.close();
    owner.close();
  }
  assert.throws(
    () => console.width(),
    (error) => error instanceof rich.RichError && error.code === rich.RichErrorCode.InvalidState,
  );
});

test('wrong receiver, wrapper kind, JavaScript range, and native domain errors stay distinct', () => {
  assert.throws(() => rich.Console.prototype.width.call({}), TypeError);
  assert.throws(() => rich.Console.prototype.close.call({}), TypeError);
  assert.throws(() => new rich.Renderable(), TypeError);
  const console = plainConsole();
  const text = new rich.Text('x');
  try {
    assert.throws(() => rich.Console.prototype.close.call(text), TypeError);
    text.setStyle('bold');
    assert.throws(() => console.render(text), TypeError);
    assert.throws(() => console.setSize(-1, 1), RangeError);
    assert.throws(() => console.setSize(1.5, 1), RangeError);
    assert.throws(() => console.setForceTerminal(1), TypeError);
    assert.throws(
      () => console.setColorSystem(999),
      (error) => error.code === rich.RichErrorCode.InvalidArgument,
    );
    assert.throws(
      () => new rich.Json('{'),
      (error) => error instanceof rich.RichError && error.code === rich.RichErrorCode.InvalidArgument,
    );
    assert.throws(
      () => new rich.Json('{}', { surprise: true }),
      TypeError,
    );
  } finally {
    text.close();
    console.close();
  }
});

test('foreign native wrappers are rejected before private owner data is dereferenced', () => {
  const foreignConsole = foreignAddon.makeForeign();
  Object.setPrototypeOf(foreignConsole, rich.Console.prototype);
  assert.equal(foreignConsole instanceof rich.Console, true);
  assert.throws(() => rich.Console.prototype.width.call(foreignConsole), TypeError);
  assert.throws(() => rich.Console.prototype.close.call(foreignConsole), TypeError);

  const foreignRenderable = foreignAddon.makeForeign();
  Object.setPrototypeOf(foreignRenderable, rich.Renderable.prototype);
  assert.equal(foreignRenderable instanceof rich.Renderable, true);
  assert.throws(() => new rich.Panel(foreignRenderable), TypeError);
});

test('GC fallback is reachable in a bounded collection cycle', async () => {
  let finalized = false;
  const registry = new FinalizationRegistry(() => {
    finalized = true;
  });
  (() => {
    const style = new rich.Style('bold');
    registry.register(style, 'style');
  })();
  for (let attempt = 0; attempt < 100 && !finalized; ++attempt) {
    global.gc();
    await new Promise((resolve) => setImmediate(resolve));
  }
  assert.equal(finalized, true);
});
