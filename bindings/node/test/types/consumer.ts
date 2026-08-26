import {
  Align,
  Box,
  ColorSystem,
  Console,
  HorizontalAlign,
  Json,
  Panel,
  Progress,
  Renderable,
  ROOT_NODE_ID,
  RichError,
  RichErrorCode,
  Table,
  Text,
  Tree,
  type NodeId,
  type TaskId,
} from '../../index.js';

const richConsole = new Console()
  .setForceTerminal(false)
  .setColorSystem(ColorSystem.TrueColor);
const builder = new Text('typed');
const reusable = builder.finish();
const table = new Table().setBox(Box.Rounded).addColumnRenderable(reusable);
table.addRowRenderables([reusable]);
const tableRenderable = table.finish();
const panel = new Panel(tableRenderable).finish();
const aligned = new Align(panel, HorizontalAlign.Center).finish();
const output: string = richConsole.render(aligned);

const tree = Tree.fromRenderable(reusable);
const id: NodeId = tree.add(ROOT_NODE_ID, 'child');
tree.addRenderable(id, reusable);
const progress = new Progress();
const task: TaskId = progress.addTask('typed', 1);
progress.update(task, 1);
new Json('{"typed":true}', { indent: 2, highlight: true, sortKeys: false });

const error: RichError = new RichError('typed', RichErrorCode.InvalidArgument);
const evidence: readonly [string, RichErrorCode] = [output, error.code];
void evidence;

// @ts-expect-error The runtime is a one-way frozen object, not a numeric enum.
RichErrorCode[RichErrorCode.InvalidArgument];

// @ts-expect-error Only values represented by the runtime object are valid codes.
const unknownCode: RichErrorCode = 99;
void unknownCode;

// @ts-expect-error Renderable handles are created only by finish().
new Renderable();
