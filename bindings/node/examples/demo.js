'use strict';

const rich = require('..');

const owners = [];
const own = (owner) => {
  owners.push(owner);
  return owner;
};

try {
  rich.ensureAbi();
  const console = own(new rich.Console())
    .setForceTerminal(false)
    .setSize(72, 24)
    .setColorSystem(rich.ColorSystem.TrueColor);
  own(new rich.Style('bold cyan'));

  const text = own(new rich.Text('Reusable rich-rs Node renderable').setStyle('bold cyan'));
  const reusable = own(text.finish());

  const table = own(new rich.Table())
    .setTitle('Node-API 10')
    .setBox(rich.Box.Rounded)
    .addColumn('Binding')
    .addColumnRenderable(reusable)
    .addRowStrings(['Node 24 / 22', 'plain output'])
    .addRowRenderables([reusable, reusable]);
  const tableRenderable = own(table.finish());

  const panel = own(new rich.Panel(tableRenderable))
    .setTitle('rich-rs')
    .setBorderStyle('blue')
    .setPadding(0, 1, 0, 1);
  const panelRenderable = own(panel.finish());

  const columns = own(new rich.Columns())
    .add(panelRenderable)
    .add(reusable)
    .setPadding(0, 2);
  const composition = own(columns.finish());

  // Exercise the remaining synchronous owners; none starts a timer or thread.
  own(new rich.Rule()).setTitle('demo').finish().close();
  own(new rich.Align(reusable, rich.HorizontalAlign.Center)).finish().close();
  own(new rich.Padding(reusable, 0, 1, 0, 1)).finish().close();
  own(new rich.Syntax('const value = 42;', 'javascript')).finish().close();
  own(new rich.Markdown('# rich-rs')).finish().close();
  own(new rich.Json('{"ready":true}')).finish().close();
  own(new rich.Tree('root')).add(rich.ROOT_NODE_ID, 'child');
  const progress = own(new rich.Progress());
  const task = progress.addTask('demo', 1);
  progress.update(task, 1);
  progress.renderFrame(console);
  own(new rich.Status('ready')).renderFrame(console);
  own(new rich.Spinner('dots')).renderFrame(console, 0);

  process.stdout.write(`${console.render(composition)}\n`);
} finally {
  for (const owner of owners.reverse()) owner.close();
}
