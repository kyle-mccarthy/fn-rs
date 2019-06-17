const net = require('net');
const fastStringify = require('fast-json-stringify');
const fastParse = require('turbo-json-parse');


// path: String,
//     method: String,
//     headers: HashMap<String, String>,
//     query_string: String,
//
//     body: Option<String>,

const parse = fastParse({
  type: 'object',
  properties: {
    req: {
      type: 'object',
      properties: {
        path: { type: 'string' },
        method: { type: 'string' },
        query_string: { type: 'string' },
        body: { type: 'string', default: '' },
        headers: {
          type: 'object',
          properties: {}
        },
      }
    },
    res: {
      type: 'object',
      properties: {
        script: {
          type: 'string',
        },
        body: {
          type: 'string',
        },
        status_code: {
          type: 'number'
        },
        headers: {
          type: 'object',
          properties: {}
        },
      },
    }
  }
}, {
  buffer: true,
  ordered: true,
  required: true,
  fullMatch: false
});

const stringify = fastStringify({
  type: 'object',
  properties: {
    script: {
      type: 'string',
    },
    body: {
      type: 'string',
    },
    status_code: {
      type: 'number'
    },
    headers: {
      type: 'object',
      properties: {}
    },
  }
});

function index(req, res) {
  res.body = 'hello';
  return res;
}

// script: String,
//     body: String,
//     headers: HashMap<String, String>,
//
//     #[serde(default = FunctionResponse::default_status_code())]
// status_code: u16,

function handler() {
  const server = setup(index);
  tearDown(server);
}

function setup(onRequest) {
  const socket = process.argv[2];

  const server = net.createServer((client) => {
    // console.log('client connected to socket');

    client.on('end', () => {
      // console.log('client disconnected');
    });

    client.on('data', (buf) => {
      let json = parse(buf);

      let res = onRequest(json.req, json.res);

      client.write(stringify(res));
    });

    client.on('error', (err) => {
      console.log('client encountered error');
      console.log(err);
    });
  });

  server.listen(socket, () => {
    console.log('server bound to socket');
  });


  server.on('error', (err) => {
    console.log('server encountered error');
    console.log(err);
  });

  server.on('close', (hadErr) => {
    if (hadErr) {
      console.log('server gracefully closing');
    } else {
      console.log('server closing because of error');
    }
  });
  
  return server;
}

function tearDown(server) {

  process.on('exit', () => {
    console.log('on exit');
  });

  process.on('SIGTERM', () => {
    server.close(() => {
      console.log('gracefully shitting down');
    });

    process.exit();
  });
}

handler();