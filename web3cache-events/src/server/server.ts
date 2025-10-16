import http from "http";
import config from "config";
import { promisify } from "util";
import fs from "fs";
import axios from "axios";
import debug from "debug";
import express from "express";
import mongoose from "mongoose";

import compression from "compression";
import helmet from "helmet";

import PartnersServer from "../api/PartnersServer";
import setUncaught from "./logging";
import errorRoute from "./error";
import ClientsServer from "../api/ClientsServer";
import ConsumerServer from "../api/ConsumerServer";

const readFile = promisify(fs.readFile);

const debugExpress = debug("app:express");
const debugMongo = debug("app:mongo");

const external_port: number = config.get("PORT");
const mongo_url: string = config.get("DB_CONN_STRING");

const app = express();
addProdLibraries(app);
app.use(express.json());
const server: http.Server = http.createServer(app);

debugMongo("mongo_url: ", mongo_url);

mongoose
  .connect(mongo_url)
  .then((result) => {
    debugMongo("connected to mongoDB");
  })
  .catch((error) => {
    debugMongo(error.message, error);
  });

setUncaught();
const web3client = new ClientsServer(app);
const web3Partner = new PartnersServer(app);
const consumerServer = new ConsumerServer(web3client, web3Partner);

app.use(errorRoute);

app.listen(external_port, () =>
  debugExpress(`Server listening on port ${external_port}.`)
);

function addProdLibraries(app: any) {
  app.use(express.json({ limit: "500mb" }));
  app.use(express.urlencoded({ limit: "500mb" }));
  app.use(compression());
  app.use(helmet());
}

module.exports = {
  web3client,
  addProdLibraries,
};
