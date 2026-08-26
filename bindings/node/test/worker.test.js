'use strict';

const assert = require('node:assert/strict');
const path = require('node:path');
const test = require('node:test');
const { Worker } = require('node:worker_threads');

test('multiple Worker environments have isolated constructor and error state', async () => {
  const packagePath = path.resolve(__dirname, '..');
  const source = `
    const { parentPort, workerData } = require('node:worker_threads');
    const rich = require(workerData);
    const console = new rich.Console().setForceTerminal(false);
    const text = new rich.Text('worker');
    const renderable = text.finish();
    const output = console.render(renderable);
    renderable.close(); text.close(); console.close();
    parentPort.postMessage({ version: rich.version(), output });
  `;
  const run = () =>
    new Promise((resolve, reject) => {
      const worker = new Worker(source, { eval: true, workerData: packagePath });
      worker.once('message', resolve);
      worker.once('error', reject);
      worker.once('exit', (code) => {
        if (code !== 0) reject(new Error(`Worker exited with ${code}`));
      });
    });
  const results = await Promise.all([run(), run(), run()]);
  assert.deepEqual(results, [
    { version: '1.2.2', output: 'worker' },
    { version: '1.2.2', output: 'worker' },
    { version: '1.2.2', output: 'worker' },
  ]);
});
