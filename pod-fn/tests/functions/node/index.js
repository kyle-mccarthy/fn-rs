const net = require('net');
const fs = require('fs');

function index(req, res) {
  res.body = 'hello';
  return res;
}

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
      let data = buf.toString();
      let json = JSON.parse(data);

      let res = onRequest(json.req, json.res);

      client.write(JSON.stringify(res));
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