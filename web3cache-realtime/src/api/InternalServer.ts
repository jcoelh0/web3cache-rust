import http from "http";
import socketIO, { ServerOptions } from "socket.io";
import * as jwt from "jsonwebtoken";
import config from "config";
import { promisify } from "util";
import fs from "fs";
import axios from "axios";
import debug from "debug";
import ClientsServer from "./ClientsServer";
import fastify, { FastifyInstance } from "fastify";
import socketioServer from "fastify-socket.io";

const readFile = promisify(fs.readFile);

const debugInternal = debug("app:internalservice");

const isDemo: boolean = config.get("demo") || false;
const internal_port: number = config.get("internal_port");

interface Transaction {
  from: string;
  to: string;
  contract_id: string;
}

interface RequestBody {
  transactions: any[];
}

class InternalServer {
  private client: ClientsServer;
  private app: FastifyInstance;
  constructor(client: ClientsServer) {
    this.app = fastify().withTypeProvider();
    this.app.register(socketioServer, {
      // put your options here
    });

    this.app.post<{ Body: RequestBody }>(
      "/web3cache/realtime/notify-transactions",
      async (req, reply) => {
        debugInternal(`Received ${req.body.transactions.length} transactions!`);
        if (isDemo) {
          const { transactions } = req.body;
          for (const transaction of transactions) client.notifyAll(transaction);
        } else {
          const { transactions } = req.body;
          for (const transaction of transactions) client.notify(transaction);
        }
        return "OK";
      }
    );

    this.app.ready((err) => {
      if (err) throw err;

      this.app.io.on("connection", (socket) => {
        socket.on("transaction", async (transaction: Transaction) => {
          if (isDemo) {
            client.notifyAll(transaction);
          } else {
            client.notify(transaction);
          }
        });
      });
    });

    this.app.listen({ port: internal_port, host: "0.0.0.0" });
    this.client = client;
    debugInternal(`Listening internatlly in port ${internal_port}`);
  }
}

export default InternalServer;
