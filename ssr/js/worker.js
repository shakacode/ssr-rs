const net = require("net");

const WORKER_ID = process.pid;
const ENCODING = "utf8";
const MESSAGE_LENGTH_BUFFER_SIZE = 4; // 32-bit

const env = {
  port: process.env["PORT"],
  globalRenderer: process.env["GLOBAL_RENDERER"],
  log: process.env["LOG"],
};

const log = {
  trace:
    function (msg, reqId) {
      if (this.__minimal) return;
      this.__dispatch(msg, reqId);
    },
  always:
    function (msg, reqId) {
      log.__dispatch(msg, reqId)
    },
  __minimal: env.log === "minimal",
  __dispatch:
    function (msg, reqId) {
      const worker =
        !!reqId
        ? `[JS] Worker [id: ${WORKER_ID} port: ${env.port} request: ${reqId}]`
        : `[JS] Worker [id: ${WORKER_ID} port: ${env.port}]`;
      const message = msg.charAt(msg.length - 1) === "\n" ? msg : `${msg}\n`;
      process.stderr.write(`${worker}: ${message}`);
    },
}

log.always(`Node.js version: ${process.version}`);

process.on("uncaughtException", (err, origin) => {
  log.always(`Uncaught Exception: ${err}`);
  process.exit(1);
});

const server = net.createServer();

const port = (() => {
  if (env.port === undefined) {
    log.always("Port is not provided");
    return process.exit(1);
  }
  const port = parseInt(env.port, 10);
  if (!Number.isInteger(port)) {
    log.always(`Port is invalid: ${env.port}`);
    return process.exit(1);
  }
  return port;
})();

const globalRenderer = env.globalRenderer ? require(env.globalRenderer) : null;

server.on("connection", connection => {
  log.trace("New connection");

  let metaLength = null;
  let dataLength = null;
  let bytesRead = 0;
  let contents = null;

  connection.on("data", bytes => {
    log.trace(`New data chunk`);

    try {
      let chunk = Buffer.from(bytes, ENCODING);

      if (metaLength === null && dataLength === null) {
        log.trace(`Received initial chunk: ${chunk}`);
        const metaLengthStartIdx = 0;
        const dataLengthStartIdx = MESSAGE_LENGTH_BUFFER_SIZE;
        metaLength = chunk.readUIntBE(metaLengthStartIdx, MESSAGE_LENGTH_BUFFER_SIZE);
        dataLength = chunk.readUIntBE(dataLengthStartIdx, MESSAGE_LENGTH_BUFFER_SIZE);
        contents = chunk.slice(dataLengthStartIdx + MESSAGE_LENGTH_BUFFER_SIZE);
        bytesRead = contents.length;
      } else {
        log.trace(`Received subsequent chunk: ${chunk}`);
        const totalLength = contents.length + chunk.length;
        contents = Buffer.concat([contents, chunk], totalLength);
        bytesRead = totalLength;
      }

      log.trace(`Meta length: ${metaLength}`);
      log.trace(`Data length: ${dataLength}`);
      log.trace(`Contents: ${contents}`);
      log.trace(`Bytes read: ${bytesRead}`);

      if (metaLength + dataLength > bytesRead) {
        log.trace("Waiting for the next chunk");
      } else {
        log.trace("Finished reading data");
        // We can safely parse meta b/c this is what we get from Rust
        const meta = JSON.parse(contents.slice(0, metaLength).toString(ENCODING));

        log.trace(`Parsed meta: ${JSON.stringify(meta)}`, meta.requestId);

        // However, applying JSON.parse on data is not safe since it might contain
        // malicious contents, which has been escaped by the renderer
        // and it would undo the escaping
        const hydrationData = contents.slice(metaLength, metaLength + dataLength).toString(ENCODING);

        log.trace(`Hydration data: ${hydrationData}`, meta.requestId);

        const jsonData = JSON.parse(JSON.parse(hydrationData));

        log.trace(`JSON data: ${JSON.stringify(jsonData)}`, meta.requestId);

        const renderer = meta.requestRenderer ? require(meta.requestRenderer) : globalRenderer;

        if (!renderer) {
          throw new Error(`Renderer is not provided for request ${meta.requestId}`);
        } else if (!renderer.render) {
          throw new Error(`Renderer.render function is not defined for request ${meta.requestId}`);
        }

        const output = renderer.render({url: meta.url, jsonData, hydrationData});

        log.trace(`Rendered output: ${output}`, meta.requestId);

        connection.end(Buffer.from(output, "utf8"));
      }
    } catch (err) {
      log.always(err.stack);
      connection.end(Buffer.from(`ERROR:${err.stack}`, "utf8"));
    }
  });

  connection.once("close", () => {
    log.trace("Connection closed");
  });

  connection.on("error", err => {
    log.always(`Connection error: ${err.message}`);
  });
});

server.listen(port, () => {
  log.always(`Ready on port ${server.address().port}`);
});
