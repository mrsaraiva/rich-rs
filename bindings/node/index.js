'use strict';

const path = require('node:path');

const configuration = process.env.RICH_RS_NATIVE_DEBUG === '1' ? 'Debug' : 'Release';
const addonPath = path.join(__dirname, 'build', configuration, 'rich_native.node');

let native;
try {
  native = require(addonPath);
} catch (error) {
  const detail = error instanceof Error ? error.message : String(error);
  throw new Error(
    `Unable to load @rich-rs/native (${configuration}) from ${addonPath}. ` +
      'Build a target-matching rich-ffi static library, then run "npm run build -- ' +
      '--rich_ffi_include_dir=/path/to/include --rich_ffi_static=/path/to/library". ' +
      `Original error: ${detail}`,
    { cause: error },
  );
}

class RichError extends Error {
  constructor(message, code) {
    super(message);
    this.name = 'RichError';
    this.code = code;
  }
}

native._configure(RichError);

const c = native._constants;
const frozen = (value) => Object.freeze(value);

const RichErrorCode = frozen({
  Ok: c.OK,
  NullArgument: c.ERROR_NULL_ARGUMENT,
  InvalidUtf8: c.ERROR_INVALID_UTF8,
  InvalidArgument: c.ERROR_INVALID_ARGUMENT,
  InvalidState: c.ERROR_INVALID_STATE,
  Io: c.ERROR_IO,
  Render: c.ERROR_RENDER,
  Panic: c.ERROR_PANIC,
  AbiMismatch: c.ABI_MISMATCH,
});

const ColorSystem = frozen({
  None: c.COLOR_NONE,
  Standard: c.COLOR_STANDARD,
  EightBit: c.COLOR_EIGHT_BIT,
  TrueColor: c.COLOR_TRUECOLOR,
  Windows: c.COLOR_WINDOWS,
});

const HorizontalAlign = frozen({
  Left: c.ALIGN_LEFT,
  Center: c.ALIGN_CENTER,
  Right: c.ALIGN_RIGHT,
});

const VerticalAlign = frozen({
  Top: c.VERTICAL_TOP,
  Middle: c.VERTICAL_MIDDLE,
  Bottom: c.VERTICAL_BOTTOM,
});

const Justify = frozen({
  Default: c.JUSTIFY_DEFAULT,
  Left: c.JUSTIFY_LEFT,
  Center: c.JUSTIFY_CENTER,
  Right: c.JUSTIFY_RIGHT,
  Full: c.JUSTIFY_FULL,
});

const Box = frozen({
  Rounded: c.BOX_ROUNDED,
  Heavy: c.BOX_HEAVY,
  Double: c.BOX_DOUBLE,
  Ascii: c.BOX_ASCII,
  Minimal: c.BOX_MINIMAL,
  Square: c.BOX_SQUARE,
  Simple: c.BOX_SIMPLE,
  HeavyHead: c.BOX_HEAVY_HEAD,
  HeavyEdge: c.BOX_HEAVY_EDGE,
  DoubleEdge: c.BOX_DOUBLE_EDGE,
  Horizontals: c.BOX_HORIZONTALS,
  MinimalHeavyHead: c.BOX_MINIMAL_HEAVY_HEAD,
  SimpleHeavy: c.BOX_SIMPLE_HEAVY,
  Markdown: c.BOX_MARKDOWN,
  None: c.BOX_NONE,
});

const api = {
  RichError,
  RichErrorCode,
  ColorSystem,
  HorizontalAlign,
  VerticalAlign,
  Justify,
  Box,
  ROOT_NODE_ID: BigInt(c.TREE_ROOT_ID),
  ensureAbi: native.ensureAbi,
  version: native.version,
  Console: native.Console,
  Renderable: native.Renderable,
  Text: native.Text,
  Style: native.Style,
  Table: native.Table,
  Panel: native.Panel,
  Tree: native.Tree,
  Rule: native.Rule,
  Columns: native.Columns,
  Align: native.Align,
  Padding: native.Padding,
  Syntax: native.Syntax,
  Pretty: native.Pretty,
  RenderOptions: native.RenderOptions,
  Measurement: native.Measurement,
  SegmentSink: native.SegmentSink,
  customRenderable: native.customRenderable,
  Markdown: native.Markdown,
  Json: native.Json,
  Progress: native.Progress,
  Status: native.Status,
  Spinner: native.Spinner,
};

if (process.env.RICH_RS_NATIVE_TEST === '1') {
  Object.defineProperty(api, '_testing', {
    value: frozen({
      constants: frozen({ ...c }),
      result: native._testResult,
      buffer: native._testBuffer,
      handle: native._testHandle,
      finishFailure: native._testFinishFailure,
      finishMatrix: native._testFinishMatrix,
      finishAddressReuse: native._testFinishAddressReuse,
      customStats: native._testCustomStats,
    }),
    enumerable: false,
  });
}

module.exports = Object.freeze(api);
