const { createServer } = require("http");
const { Server } = require("socket.io");
const Client = require("socket.io-client");
const {web3client, kafkaConsumer} = require('./server.ts')



jest.mock('socket.io-client', ()=>{
  const mSocket = {
    emit: jest.fn(),
  };
  return jest.fn(() => mSocket);
})

describe("my awesome project", () => {
  let io, serverSocket, clientSocket;

  beforeAll((done) => {
    const httpServer = createServer();
    io = new Server(httpServer);
    httpServer.listen(() => {
      const port = httpServer.address().port;
      clientSocket = new Client(`http://localhost:${port}`);
      io.on("connection", (socket) => {
        serverSocket = socket;
      });
      clientSocket.on("connect", done);
    });
  });

  afterAll(() => {
    io.close();
    clientSocket.close();
  });

  test("transaction", (done) => {
    clientSocket.on("transaction", (arg) => {
      expect(arg).toBe("contrat_address");
      done();
    });
    serverSocket.emit("transaction", "contrat_address");
  });

});