'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const test = require('node:test');

process.env.RICH_RS_NATIVE_TEST = '1';
const rich = require('..');

test('ABI, package version, layouts, and stable result mapping', () => {
  assert.doesNotThrow(() => rich.ensureAbi());
  assert.equal(rich.version(), '1.3.0');
  assert.equal(rich._testing.constants.ABI_VERSION, 2);
  assert.equal(rich._testing.constants.RESULT_SIZE, 4);
  assert.ok(rich._testing.constants.STR_SIZE >= 16);
  assert.equal(rich._testing.constants.STR_SIZE, rich._testing.constants.BUFFER_SIZE);

  const codes = [
    rich.RichErrorCode.NullArgument,
    rich.RichErrorCode.InvalidUtf8,
    rich.RichErrorCode.InvalidArgument,
    rich.RichErrorCode.InvalidState,
    rich.RichErrorCode.Io,
    rich.RichErrorCode.Render,
    rich.RichErrorCode.Panic,
  ];
  for (const code of codes) {
    assert.throws(
      () => rich._testing.result(code),
      (error) => error instanceof rich.RichError && error.code === code,
    );
  }
  assert.doesNotThrow(() => rich._testing.result(rich.RichErrorCode.Ok));
  assert.throws(
    () => rich._testing.result(12345),
    (error) =>
      error instanceof rich.RichError && error.code === rich.RichErrorCode.AbiMismatch,
  );
});

test('native buffer RAII covers success and every exception path', () => {
  const before = rich._testing.buffer('stats');
  assert.equal(rich._testing.buffer('success'), '1.3.0');
  assert.equal(rich._testing.buffer('stats'), before + 1n);
  assert.throws(() => rich._testing.buffer('throw'), /injected/);
  assert.equal(rich._testing.buffer('stats'), before + 2n);
  assert.throws(
    () => rich._testing.buffer('unknownResult'),
    (error) => error.code === rich.RichErrorCode.AbiMismatch,
  );
  assert.equal(rich._testing.buffer('stats'), before + 3n);
  assert.throws(
    () => rich._testing.buffer('invalidLayout'),
    (error) => error.code === rich.RichErrorCode.AbiMismatch,
  );
  assert.equal(rich._testing.buffer('stats'), before + 4n);
  assert.throws(
    () => rich._testing.buffer('nativeError'),
    (error) => error.code === rich.RichErrorCode.InvalidArgument,
  );
  assert.equal(rich._testing.buffer('stats'), before + 5n);
});

test('checked handle adoption rejects malformed outputs and frees every pending owner', () => {
  const before = rich._testing.handle('stats');
  assert.throws(
    () => rich._testing.handle('nullSuccess'),
    (error) => error.code === rich.RichErrorCode.AbiMismatch,
  );
  assert.equal(rich._testing.handle('stats'), before);
  assert.throws(
    () => rich._testing.handle('nonNullError'),
    (error) => error.code === rich.RichErrorCode.InvalidArgument,
  );
  assert.equal(rich._testing.handle('stats'), before + 1n);
  assert.throws(() => rich._testing.handle('allocationThrow'), /injected owner-wrapper/);
  assert.equal(rich._testing.handle('stats'), before + 2n);

  const text = new rich.Text('finish allocation');
  rich._testing.handle('armAllocationThrow');
  assert.throws(() => text.finish(), /injected owner-wrapper/);
  assert.equal(rich._testing.handle('stats'), before + 3n);
  assert.throws(
    () => text.finish(),
    (error) => error.code === rich.RichErrorCode.InvalidState,
  );
  text.close();
});

test('finish reconciles the complete input/output/result ownership matrix', () => {
  const inputModes = ['original', 'null', 'replacement'];
  for (const returnOk of [true, false]) {
    for (const inputMode of inputModes) {
      for (const withOutput of [false, true]) {
        const text = new rich.Text(`${inputMode}:${withOutput}:${returnOk}`);
        const before = rich._testing.handle('stats');
        const validSuccess = returnOk && inputMode === 'null' && withOutput;
        const validError = !returnOk && !withOutput && inputMode !== 'replacement';
        const malformed = !validSuccess && !validError;

        if (validSuccess) {
          const renderable = rich._testing.finishMatrix(
            text,
            inputMode,
            withOutput,
            returnOk,
          );
          renderable.close();
        } else {
          assert.throws(
            () => rich._testing.finishMatrix(text, inputMode, withOutput, returnOk),
            (error) =>
              error.code ===
              (malformed ? rich.RichErrorCode.AbiMismatch : rich.RichErrorCode.InvalidArgument),
          );
        }

        const expectedCleanup = malformed
          ? BigInt(Number(inputMode !== 'null') + Number(withOutput))
          : 0n;
        assert.equal(
          rich._testing.handle('stats'),
          before + expectedCleanup,
          `${inputMode}:${withOutput}:${returnOk}`,
        );

        const retained = validError && inputMode === 'original';
        if (retained) {
          assert.doesNotThrow(() => text.setStyle('bold'));
        } else {
          assert.throws(
            () => text.setStyle('bold'),
            (error) => error.code === rich.RichErrorCode.InvalidState,
          );
        }
        text.close();
      }
    }
  }

  for (const inputMode of ['original', 'replacement']) {
    const aliased = new rich.Text(`aliased ${inputMode}/output`);
    const beforeAlias = rich._testing.handle('stats');
    assert.throws(
      () => rich._testing.finishMatrix(aliased, inputMode, true, false, 'input'),
      (error) => error.code === rich.RichErrorCode.AbiMismatch,
    );
    assert.equal(rich._testing.handle('stats'), beforeAlias + 1n);
    assert.throws(
      () => aliased.setStyle('bold'),
      (error) => error.code === rich.RichErrorCode.InvalidState,
    );
    aliased.close();
  }

  const beforeReuse = rich._testing.handle('stats');
  const reused = rich._testing.finishAddressReuse();
  assert.ok(reused instanceof rich.Renderable);
  reused.close();
  assert.equal(rich._testing.handle('stats'), beforeReuse);
});

test('compiled inventory exactly accounts for all 125 canonical functions', () => {
  const header = fs.readFileSync(path.resolve(__dirname, '../../../ffi/include/rich.h'), 'utf8');
  const inventory = fs.readFileSync(path.resolve(__dirname, '../src/native_symbols.inc'), 'utf8');
  const declarations = [
    ...header.matchAll(/^(?:RichResult|void|uint32_t) (rich_[a-z0-9_]+)\s*\(/gm),
  ].map((match) => match[1]);
  const projected = [...inventory.matchAll(/RICH_NATIVE_SYMBOL\((rich_[a-z0-9_]+)\)/g)].map(
    (match) => match[1],
  );
  assert.equal(declarations.length, 125);
  assert.deepEqual(projected.sort(), declarations.sort());
});
