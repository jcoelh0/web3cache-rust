import http from "http";
import config from "config";
import { promisify } from "util";
import fs from "fs";
import axios from "axios";
import debug from "debug";

import compression from "compression";

import setUncaught from "./logging";
import ClientsServer from "../api/ClientsServer";
import InternalServer from "../api/InternalServer";

const readFile = promisify(fs.readFile);

const debugWebsocket = debug("app:websocket");

setUncaught();
const externclient = new ClientsServer();
const internclient = new InternalServer(externclient);

module.exports = {
  externclient,
  internclient,
};
