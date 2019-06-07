const readline = require('readline');

async function fn() {
  const rl = readline.createInterface(process.stdin);

  // read until EOL
  const input = await new Promise((res) => {
    rl.on('line', line => {
      res(line);
    });
  });

  const { req, res } = JSON.parse(input);

  res.body = "hello";

  process.stdout.write(JSON.stringify(res));

  rl.close();
}

fn();