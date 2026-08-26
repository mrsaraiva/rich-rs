export const RichErrorCode: Readonly<{
  Ok: 0;
  NullArgument: 1;
  InvalidUtf8: 2;
  InvalidArgument: 3;
  InvalidState: 4;
  Io: 5;
  Render: 6;
  Panic: 7;
  AbiMismatch: 8;
}>;
export type RichErrorCode = (typeof RichErrorCode)[keyof typeof RichErrorCode];

export class RichError extends Error {
  readonly code: RichErrorCode;
  constructor(message: string, code: RichErrorCode);
}

export const ColorSystem: Readonly<{
  None: 0;
  Standard: 1;
  EightBit: 2;
  TrueColor: 3;
  Windows: 4;
}>;
export type ColorSystemValue = (typeof ColorSystem)[keyof typeof ColorSystem];

export const HorizontalAlign: Readonly<{ Left: 0; Center: 1; Right: 2 }>;
export type HorizontalAlignValue =
  (typeof HorizontalAlign)[keyof typeof HorizontalAlign];

export const VerticalAlign: Readonly<{ Top: 0; Middle: 1; Bottom: 2 }>;
export type VerticalAlignValue = (typeof VerticalAlign)[keyof typeof VerticalAlign];

export const Justify: Readonly<{
  Default: 0;
  Left: 1;
  Center: 2;
  Right: 3;
  Full: 4;
}>;
export type JustifyValue = (typeof Justify)[keyof typeof Justify];

export const Box: Readonly<{
  Rounded: 0;
  Heavy: 1;
  Double: 2;
  Ascii: 3;
  Minimal: 4;
  Square: 5;
  Simple: 6;
  HeavyHead: 7;
  HeavyEdge: 8;
  DoubleEdge: 9;
  Horizontals: 10;
  MinimalHeavyHead: 11;
  SimpleHeavy: 12;
  Markdown: 13;
  None: 14;
}>;
export type BoxValue = (typeof Box)[keyof typeof Box];

export type NodeId = bigint;
export type TaskId = bigint;
export const ROOT_NODE_ID: 0n;

export function ensureAbi(): void;
export function version(): string;

interface Owner {
  /** Idempotently releases this wrapper's native owner. */
  close(): void;
}

export class Renderable implements Owner {
  private constructor();
  close(): void;
}

export class Console implements Owner {
  constructor();
  setForceTerminal(force: boolean): this;
  setSize(width: number, height: number): this;
  setColorSystem(system: ColorSystemValue): this;
  setMarkupEnabled(enabled: boolean): this;
  setEmojiEnabled(enabled: boolean): this;
  width(): number;
  renderMarkup(markup: string): string;
  render(renderable: Renderable): string;
  close(): void;
}

export class Text implements Owner {
  constructor(text: string);
  static fromMarkup(console: Console, markup: string): Text;
  setStyle(style: string): this;
  finish(): Renderable;
  close(): void;
}

export class Style implements Owner {
  constructor(style: string);
  close(): void;
}

export class Table implements Owner {
  constructor();
  static grid(): Table;
  setTitle(title: string): this;
  setCaption(caption: string): this;
  setTitleMarkup(console: Console, markup: string): this;
  setCaptionMarkup(console: Console, markup: string): this;
  setBox(box: BoxValue): this;
  setPadEdge(pad: boolean): this;
  setRowStyles(styles: readonly string[]): this;
  setShowHeader(show: boolean): this;
  setShowLines(show: boolean): this;
  setShowEdge(show: boolean): this;
  setExpand(expand: boolean): this;
  setPadding(left: number, right: number): this;
  setStyle(style: string): this;
  addColumn(header: string): this;
  addColumnRenderable(header: Renderable): this;
  addColumnOptions(options: TableColumnOptions, header?: Renderable | null): this;
  addRowStrings(cells: readonly string[]): this;
  addRowRenderables(cells: readonly Renderable[]): this;
  finish(): Renderable;
  close(): void;
}

export interface TableColumnOptions {
  justify?: JustifyValue;
  vertical?: VerticalAlignValue;
  width?: number;
  minWidth?: number;
  ratio?: number;
  style?: string;
  noWrap?: boolean;
}

export class Panel implements Owner {
  constructor(content: Renderable);
  setTitle(title: string): this;
  setSubtitle(subtitle: string): this;
  setBox(box: BoxValue): this;
  setExpand(expand: boolean): this;
  setWidth(width: number): this;
  setPadding(top: number, right: number, bottom: number, left: number): this;
  setStyle(style: string): this;
  setBorderStyle(style: string): this;
  finish(): Renderable;
  close(): void;
}

export class Tree implements Owner {
  constructor(label: string);
  static fromRenderable(label: Renderable): Tree;
  add(parentId: NodeId, label: string): NodeId;
  addRenderable(parentId: NodeId, label: Renderable): NodeId;
  setStyle(style: string): this;
  setGuideStyle(style: string): this;
  setHideRoot(hide: boolean): this;
  finish(): Renderable;
  close(): void;
}

export class Rule implements Owner {
  constructor();
  setTitle(title: string): this;
  setCharacters(characters: string): this;
  setStyle(style: string): this;
  setAlign(align: HorizontalAlignValue): this;
  finish(): Renderable;
  close(): void;
}

export class Columns implements Owner {
  constructor();
  add(renderable: Renderable): this;
  addString(text: string): this;
  setEqual(equal: boolean): this;
  setExpand(expand: boolean): this;
  setPadding(vertical: number, horizontal: number): this;
  finish(): Renderable;
  close(): void;
}

export class Align implements Owner {
  constructor(content: Renderable, horizontal: HorizontalAlignValue);
  static center(content: Renderable): Align;
  setVertical(vertical: VerticalAlignValue): this;
  setWidth(width: number): this;
  finish(): Renderable;
  close(): void;
}

export class Padding implements Owner {
  constructor(
    content: Renderable,
    top: number,
    right: number,
    bottom: number,
    left: number,
  );
  setStyle(style: string): this;
  setExpand(expand: boolean): this;
  finish(): Renderable;
  close(): void;
}

export class Syntax implements Owner {
  constructor(code: string, lexer: string);
  static fromPath(path: string): Syntax;
  setTheme(theme: string): this;
  setLineNumbers(enabled: boolean): this;
  setWordWrap(enabled: boolean): this;
  setIndentGuides(enabled: boolean): this;
  setLineRange(start: number | null, end: number | null): this;
  finish(): Renderable;
  close(): void;
}

export class Pretty implements Owner {
  constructor(debugRepr: string);
  setIndentGuides(enabled: boolean): this;
  finish(): Renderable;
  close(): void;
}

export class RenderOptions {
  private constructor();
  readonly minWidth: number;
  readonly maxWidth: number;
  readonly maxHeight: number;
  readonly colorSystem: ColorSystemValue;
  readonly isTerminal: boolean;
}

export class Measurement {
  constructor(minimum: number, maximum: number);
  readonly minimum: number;
  readonly maximum: number;
}

export class SegmentSink {
  private constructor();
  push(text: string, style?: string): this;
  line(): this;
}

export interface CustomRenderer {
  render(options: RenderOptions, sink: SegmentSink): void;
  measure?(options: RenderOptions): Measurement;
}

export function customRenderable(renderer: CustomRenderer): Renderable;

export class Markdown implements Owner {
  constructor(source: string);
  setCodeTheme(theme: string): this;
  setHyperlinks(enabled: boolean): this;
  setJustify(justify: JustifyValue): this;
  finish(): Renderable;
  close(): void;
}

export interface JsonOptions {
  indent?: number;
  highlight?: boolean;
  sortKeys?: boolean;
}

export class Json implements Owner {
  constructor(data: string, options?: JsonOptions);
  finish(): Renderable;
  close(): void;
}

export class Progress implements Owner {
  constructor();
  addTask(description: string, total: number): TaskId;
  update(taskId: TaskId, completed: number): this;
  renderFrame(console: Console): string;
  close(): void;
}

export class Status implements Owner {
  constructor(message: string);
  renderFrame(console: Console): string;
  close(): void;
}

export class Spinner implements Owner {
  constructor(name: string);
  renderFrame(console: Console, timeSeconds: number): string;
  close(): void;
}
