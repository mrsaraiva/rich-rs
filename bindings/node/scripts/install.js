'use strict';

// Deliberately suppress npm's implicit node-gyp build. Consumers must provide a
// target-matching rich-ffi archive and invoke `npm run build` explicitly.
process.stdout.write(
  '@rich-rs/native: source installed; run npm run build with rich-ffi paths when ready\n',
);
