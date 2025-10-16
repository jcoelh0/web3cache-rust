import http from "http";
import socketIO from "socket.io";
import * as jwt from "jsonwebtoken";
import config from "config";
import { promisify } from "util";
import fs from "fs";
import axios from "axios";
import debug from "debug";
import express from "express";

const debugExternal = debug("app:external");
const debugWebsocket = debug("app:websocket");
const debugAuth = debug("app:auth");

const external_port: number = config.get("external_port");

interface Transaction {
  from: string;
  to: string;
  contract_id: string;
}

class ClientsServer {
  private io: socketIO.Server;
  private pubKey: any;

  constructor() {
    const app = express();
    const server = http.createServer(app);
    app.get("/web3cache/realtime/healthcheck", (req, res) => {
      res.send({
        uptime: process.uptime(),
        date: new Date(),
      });
    });
    this.io = new socketIO.Server(server, {
      cors: {
        origin: config.get("frontend"),
        methods: ["GET", "POST"],
        allowedHeaders: ["x-auth-token"],
        // transports: ["websocket", "polling"],
        credentials: true,
      },
      allowEIO3: true,
    });
    this.init().then(() => {
      this.io.on("connection", (socket) => {
        socket.on("login", async (data) => {
          const auth = await socketAuth(socket.handshake, this.pubKey);
          if (!auth) {
            debugAuth("Not authorized");
            return;
          }
          socket.data.auth = auth;
          debugWebsocket("auth", auth);
          socket.join(auth);
        });
        socket.on("logout", () => {
          if (socket.data.auth) {
            socket.leave(socket.data.auth);
            socket.data.auth = null;
          }
        });
        socket.on("disconnect", () => {
          if (socket.data.auth) {
            socket.leave(socket.data.auth);
            socket.data.auth = null;
          }
        });
      });
      server.listen(external_port);
      debugExternal(`Listening externally in port ${external_port}`);
    });
  }

  public notify(transaction: Transaction) {
    debugWebsocket(`Transaction : ${transaction.to}, ${transaction.from} `);
    //emit transaction only for the clients envolved (use of rooms)
    this.io.to(transaction.from).emit("transaction", transaction);
    this.io.to(transaction.to).emit("transaction", transaction);
  }

  public notifyAll(transaction: Transaction) {
    debugWebsocket(`Transaction: ${transaction.to}, ${transaction.from}`);
    this.io.emit("transaction", transaction);
  }

  public async init() {
    try {
      const pubKey = (
        await axios.get(`${config.get("iam")}/security/getPublicKey`)
      )?.data;
      if (!pubKey) {
        debugAuth("Pubkey not available:", pubKey);
      }
      this.pubKey = pubKey;
      debugAuth("Pub key was retrevied from the iam service!");
    } catch (ex: any) {
      debugAuth("Error accessing iam key: ", ex.message);
      //process.exit(1);
    }
  }
}

export default ClientsServer;

async function socketAuth(handshake: any, pubkey: any) {
  try {
    debugAuth("handshake.headers.cookie: ", handshake.headers.cookie);
    const value = handshake.headers.cookie
      .split("; ")
      .reduce((prev: any, current: String) => {
        const [name, ...value] = current.split("=");
        prev[name] = value.join("=");
        return prev;
      }, {});
    const token: string = value["x-auth-token"];
    debugAuth("socketAuth", token);

    const auth =
      process.env.NODE_ENV == "test"
        ? jwt.decode(token)
        : (jwt.verify(token, Buffer.from(pubkey.publicKey), {
            algorithms: ["RS256"],
          }) as any);
    //const validSessionToken = await Token.findOne({ token });
    return auth.address;
  } catch (ex: any) {
    debugAuth("Error: ", ex.message);
    return null;
  }
}
