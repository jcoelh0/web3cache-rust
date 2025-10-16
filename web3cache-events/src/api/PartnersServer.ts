import axios from "axios";
import debug from "debug";
import express from "express";
import config from "config";

import Subscription from "../models/subscription";
import Dispatcher from "./Dispatcher";

import ISubscription from "../interface/subscription";
import ITransaction, { SingleTransaction } from "../interface/transaction";
import { setMetrics } from "../middleware/metrics";
import { validate } from "../middleware/validateJoi";
import mongoose from "mongoose";

const {
  getChainId,
  getChainAdress,
  getContractAbiIfAvailable,
  getInitialBlockNumberByContractAddress,
} = require("./ContractRegistration");

const debugAuth = debug("app:auth");
const debugSubscription = debug("app:subscription");
const debugMongo = debug("app:mongo");
const mongo_url: string = config.get("DB_CONN_STRING");

const read_api_url: string = config.get("READURL");

interface Transaction {
  from: string;
  to: string;
  contract_id: string;
}

async function getHistoryBlockNumber(
  blockNumber: number,
  sub: ISubscription,
  dispatcher: Dispatcher,
  res: express.Response,
  status: number,
  message: any
) {
  const headers = {
    block_number: blockNumber,
    contract_id: sub.contract_id,
  };
  try {
    const response = await axios.get(read_api_url + "/transactions_history", {
      headers,
    });

    const fullContent: any[] = [];
    const sendTransactions: any[] = [];
    for (const transaction of response.data) {
      if (
        sendTransactions.length > 0 &&
        (transaction.block_number !=
          sendTransactions[sendTransactions.length - 1].block_number ||
          transaction.event_name !=
            sendTransactions[sendTransactions.length - 1].event_name)
      ) {
        fullContent.push([...sendTransactions]);
        sendTransactions.length = 0;
      }
      sendTransactions.push(transaction);
    }

    if (sendTransactions.length) {
      fullContent.push([...sendTransactions]);
    }
    await dispatcher.bulkStore(
      fullContent,
      blockNumber,
      sub._id.toString(),
      sub.secret
    );

    res.status(status).send(message);
  } catch (error: any) {
    debugSubscription(error.message);
    return res
      .status(500)
      .send("Internal error, we were not able to restart the blocknumber");
  }
}

class PartnersServer {
  private dispatcher: Dispatcher;
  private app: any;

  constructor(app: any) {
    const router = express.Router();
    this.dispatcher = new Dispatcher();

    this.app = app;

    router.get(
      "/get-contracts",
      async (req: express.Request, res: express.Response) => {
        try {
          const response = await axios.get(read_api_url + "/contracts", {});
          res.send(response.data);
        } catch (error) {
          res.status(500).send("Internal Error");
        }
      }
    );

    router.get(
      "/get-contract/:contract_id",
      async (req: express.Request, res: express.Response) => {
        const headers = {
          contract_id: req.params.contract_id,
        };
        const response = await axios.get(read_api_url + "/contract", {
          headers,
        });
        res.send(response.data);
      }
    );

    router.post(
      "/subscription-move-registration",
      validate("RegisterSub", "SecretAndApi"),
      async (req: express.Request, res: express.Response) => {
        const api_key = req.headers["x-webhook-api-key"];
        const callback_secret = req.body.secret;
        const topics = req.body.topics;
        const format = req.body.format || "JSON";
        const contract_id = req.body.contract_id;
        const url = req.body.url;
        let block_number = req.body.block_number;

        const headers = {
          contract_id: contract_id,
        };
        const response = await axios.get(read_api_url + "/contract", {
          headers,
        });
        // verify if response is valid , else return no contract found try register ir 1st
        if (response.data.length === 0) {
          return res
            .status(400)
            .send(
              "No contract found for contract_id: " +
                contract_id +
                " Please register your contract."
            );
        } else {
          //verify api_key
          //verify block_number , present if (null), if given block_number >current will also be present,
          //if given block number <= start block number of contract, it'll start at that block number
          //else start at the given block number
          const subscription = new Subscription({
            apikey: api_key,
            topics: topics,
            contract_id: contract_id,
            secret: callback_secret || "default secret",
            isActive: true,
            isSui: true,
            url,
          });
          await subscription.save();

          if (block_number != null) {
            return getHistoryBlockNumber(
              block_number,
              subscription,
              this.dispatcher,
              res,
              201,
              { subscription }
            );
          } else {
            return res.status(201).send({ subscription });
          }
        }
      }
    );

    router.post(
      "/contract-registration",
      async (req: express.Request, res: express.Response) => {
        const api_key = req.headers["x-webhook-api-key"];
        const contract_id = req.body.contract_id;
        const chain = req.body.chain;
        const contract_address = req.body.contract_address;
        let contract_abi = req.body.contract_abi;

        const chain_id: number = getChainId(chain);
        debugSubscription("chain_id", chain_id);

        if (!contract_abi) {
          contract_abi = await getContractAbiIfAvailable(
            contract_address,
            chain_id
          );
          if (!contract_abi)
            return res
              .status(404)
              .send("Contract abi not provided and not found through API");
        }

        debugSubscription(getChainAdress("wss", chain_id));

        const initial_block_number =
          await getInitialBlockNumberByContractAddress(
            contract_address,
            chain_id
          );
        if (!initial_block_number)
          return res.status(404).send("Could not find initial block number.");

        let contract_to_add = {
          contract_address,
          contract_abi,
          contract_block_number: initial_block_number,
          owner_block_number: initial_block_number,
          transfer_block_number: initial_block_number,
          chain_address_wss: getChainAdress("wss", chain_id),
          chain_address_https: getChainAdress("https", chain_id),
        };

        /* mongoose.createConnection(
          mongo_url + "/" + contract_id + "?retryWrites=true&w=majority"
        ); */
        /* .then((result) => {
            debugMongo("connected to mongoDB");
          })
          .catch((error) => {
            debugMongo(error.message, error);
          }); */

        /* try {
          //verify api_key
          //verify url
          const subscription = new Subscription({
            apikey: api_key,
            topics: topics,
            contractid: contract_id,
            isActive: true,
            url,
          });
          await subscription.save();
          return res.status(201).send(subscription.id);
        } catch (error) {
          return res.status(500).send("Internal Error");
        } */
        return res.status(200).send(contract_to_add);
      }
    );

    router.post(
      "/subscription-registration",
      validate("RegisterSub", "SecretAndApi"),
      async (req: express.Request, res: express.Response) => {
        const api_key = req.headers["x-webhook-api-key"];
        const callback_secret = req.headers["x-webhook-api-secret"];
        const topics = req.body.topics;
        const contract_id = req.body.contract_id;
        const url = req.body.url;
        let block_number = req.body.block_number;
        let timer1 = new Date();
        const headers = {
          contract_id: contract_id,
        };
        const response = await axios.get(read_api_url + "/contract", {
          headers,
        });
        // verify if response is valid , else return no contract found try register ir 1st
        if (response.data.length === 0) {
          return res
            .status(400)
            .send(
              "No contract found for contract_id: " +
                contract_id +
                " Please register your contract."
            );
        } else {
          //verify api_key
          //verify block_number , present if (null), if given block_number >current will also be present,
          //if given block number <= start block number of contract, it'll start at that block number
          //else start at the given block number
          const subscription = new Subscription({
            apikey: api_key,
            topics: topics || [],
            contract_id: contract_id,
            secret: callback_secret || "default secret",
            isActive: true,
            url,
          });
          await subscription.save();
          const subscriptionApi: any = { ...subscription };
          delete subscriptionApi._doc.apikey;
          setMetrics(
            "Time measured since start subscribe until giving a response",
            "time-to-subscribe",
            new Date().getTime() - timer1.getTime()
          );
          if (block_number != null) {
            return getHistoryBlockNumber(
              block_number,
              subscription,
              this.dispatcher,
              res,
              201,
              { subscription: subscriptionApi._doc }
            );
          } else {
            return res.status(201).send({ subscription: subscriptionApi._doc });
          }
        }
      }
    );

    //if(process.env.NODE_ENV == 'development') {
    router.post(
      "/black-hole-endpoint",
      async (req: express.Request, res: express.Response) => {
        debugSubscription("webhook destroyed");
        //debugSubscription("headers: ", JSON.stringify(req.headers, null, 2));
        debugSubscription(req.body);
        res.send("OK");
      }
    );

    router.post(
      "/update-subscription/:subid",
      validate("UpdateSub", "WebHookAPI"),
      async (req: express.Request, res: express.Response) => {
        // await collections.subscription.find({token:req.query.apikey})
        // req.body
        debugSubscription(
          `apikey: ${req.headers["x-webhook-api-key"]} url: ${req.body.url} subscription ID ${req.params.subid}`
        );
        const api_key = req.headers["x-webhook-api-key"];
        const subid = req.params.subid;
        const { url, add_topics, remove_topics, set_topics } = req.body;

        const setObject: any = { isActive: true };
        if (url) {
          setObject.url = url;
        }
        if (set_topics) {
          const result = await Subscription.updateOne(
            { apikey: api_key, _id: subid },
            { $set: { topics: set_topics, ...setObject } }
          );
        } else if (add_topics || remove_topics) {
          if (add_topics) {
            await Subscription.updateOne(
              { apikey: api_key, _id: subid },
              {
                $addToSet: { topics: { $each: add_topics || [] } },
                $set: setObject,
              }
            );
          }
          if (remove_topics) {
            await Subscription.updateOne(
              { apikey: api_key, _id: subid },
              {
                $pull: { topics: { $in: remove_topics || [] } },
                $set: setObject,
              }
            );
          }
        } else {
          await Subscription.updateOne(
            { apikey: api_key, _id: subid },
            { $set: setObject }
          );
        }
        const subscripton = await Subscription.findOne({
          apikey: api_key,
          _id: subid,
        }).select({
          apikey: 0,
          secret: 0,
          __v: 0,
        });
        if (subscripton) {
          return res.send({ subscripton });
        } else {
          return res.status(404).send("Subscription-apikey pair not found");
        }
      }
    );

    router.post(
      "/subscription-state/:subid",
      validate("StateSub", "WebHookAPI"),
      async (req: express.Request, res: express.Response) => {
        const api_key = req.headers["x-webhook-api-key"];
        const subid = req.params.subid;
        const state: boolean = req.body.activate ? req.body.activate : true;

        const result = await Subscription.updateOne(
          { apikey: api_key, _id: subid },
          { $set: { isActive: state } }
        );

        if (result.matchedCount != 0 && result.modifiedCount != 0) {
          debugSubscription(
            `Sucess subscription updated ${JSON.stringify(result)}`
          );
          const subscripton = await Subscription.findOne({
            apikey: api_key,
            _id: subid,
          }).select({
            apikey: 0,
            secret: 0,
            __v: 0,
          });
          return res.send({ subscripton });
        } else {
          return res.status(401).send("Not authorized");
        }
      }
    );

    router.post(
      "/delete-subscription/:subid",
      validate(null, "WebHookAPI"),
      async (req: express.Request, res: express.Response) => {
        debugSubscription(
          `DELETE apikey: ${req.headers["x-webhook-api-key"]} subscription ID ${req.params.subid} `
        );

        const delete_res = await Subscription.deleteOne({
          _id: req.params.subid,
          apikey: req.headers["x-webhook-api-key"],
        });
        if (delete_res.deletedCount != 0) {
          return res.send("Ok");
        } else {
          return res.status(401).send("not authorized");
        }
      }
    );

    router.get(
      "/subscriptions",
      validate(null, "WebHookAPI"),
      async (req: express.Request, res: express.Response) => {
        debugSubscription(`get apikey: ${req.headers["x-webhook-api-key"]}`);
        const apikey = req.headers["x-webhook-api-key"];
        const subscriptions = await Subscription.find({ apikey }).select({
          apikey: 0,
          secret: 0,
          __v: 0,
        });

        return res.send({ subscriptions });
      }
    );

    router.get(
      "/subscription/:subid",
      validate(null, "WebHookAPI"),
      async (req: express.Request, res: express.Response) => {
        debugSubscription(`get subscription: ${req.params.subid}`);
        const subscription = await Subscription.findOne({
          _id: req.params.subid,
          apikey: req.headers["x-webhook-api-key"],
        }).select({ apikey: 0, secret: 0, __v: 0 });
        if (!subscription) {
          return res.status(404).send("Subscription not found");
        } else {
          return res.status(200).send({ subscription });
        }
      }
    );

    router.post(
      "/replay-subscription/:subid",
      validate("RestartSub", "WebHookAPI"),
      async (req: express.Request, res: express.Response) => {
        const blockNumber: number = req.body.block_number;
        const subscription: any = await Subscription.findOne({
          _id: req.params.subid,
          apikey: req.headers["x-webhook-api-key"],
        }).select({ apikey: 0, secret: 0, __v: 0 });
        if (!subscription) {
          return res.status(404).send("Subscription not found");
        } else {
          return getHistoryBlockNumber(
            blockNumber,
            subscription,
            this.dispatcher,
            res,
            200,
            { subscription }
          );
        }
      }
    );

    app.use("/web3cache/events", router);
    this.init();
  }

  async init() {
    await this.dispatcher.init();
  }

  public async notify(
    transactions: SingleTransaction[],
    webhook_url: string,
    secret: string,
    subid: string
  ) {
    try {
      await this.dispatcher.notify(transactions, webhook_url, secret, subid);
    } catch (ex) {
      debugSubscription("ERROR on dispatching!");
    }
  }
}

export default PartnersServer;
