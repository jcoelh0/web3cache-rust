import http from "http";
import socketIO from "socket.io";
import * as jwt from "jsonwebtoken";
import config from "config";
import { promisify } from "util";
import fs from "fs";
import axios from "axios";
import debug from "debug";
import express from "express";

import errorRoute from "../server/error";

const readFile = promisify(fs.readFile);

const debugAuth = debug("app:auth");
const debugWebsocket = debug("app:websocket");
const debugSubscription = debug("app:subscription");
const debugMongo = debug("app:mongo");

interface Transaction {
  from: string;
  to: string;
  contract_id: string;
}

class ClientsServer {
  private pubKey: any;

  constructor(app: any) {
    app.get(
      "/web3cache/events/healthcheck",
      (req: express.Request, res: express.Response) => {
        res.send("OK");
      }
    );

    app.use(errorRoute);
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
    const token = value["x-auth-token"];
    //debugauth("socketAuth", token);

    const auth =
      process.env.NODE_ENV == "test"
        ? jwt.decode(token)
        : (jwt.verify(token, pubkey) as any);
    //const validSessionToken = await Token.findOne({ token });
    return auth.address;
  } catch (ex: any) {
    return null;
  }
}
