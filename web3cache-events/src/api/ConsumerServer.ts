import compression from "compression";
import config from "config";
import debug from "debug";
import express from "express";
import helmet from "helmet";
import Subscription from "../models/subscription";
import errorRoute from "../server/error";
import { io, Socket } from "socket.io-client";

const debugConsumer = debug("app:consumer");
const debugWebsocket = debug("app:websocket");
const realtime_url: string = config.get("REALTIME_URL");

function addProdLibraries(app: any) {
  app.use(express.json({ limit: "500mb" }));
  app.use(express.urlencoded({ extended: true, limit: "500mb" }));
  app.use(compression());
  app.use(helmet());
}

class ConsumerServer {
  private isDemo: boolean;
  private socket: Socket;
  constructor(web3client: any, web3Partner: any) {
    this.socket = io(realtime_url, {
      reconnection: true,
      reconnectionAttempts: 20,
    });
    this.socket.on("connect", () => {
      debugWebsocket("connected!!");
    });

    const app = express();
    addProdLibraries(app);

    this.isDemo = config.get("demo") == "true";

    app.post(
      "/push-transactions",
      async (req: express.Request, res: express.Response) => {
        const contract_id: String = req.body.contract_id;
        const data: any[] = req.body.data;
        debugConsumer(JSON.stringify(data));

        const result: any[] = await Subscription.find({
          contract_id,
          isActive: true,
        });

        for (const record of data) {
          const transactions = record.transactions;
          //debugConsumer("Transactions: ", transactions);
          for (const subscription of result) {
            await web3Partner.notify(
              transactions,
              subscription.url,
              subscription.secret,
              subscription._id.toString()
            );
          }

          for (const transaction of transactions) {
            /* debugConsumer(
              "dispatching " + transaction.event_name + " transaction: ",
              transaction.block_number
            ); */
            this.socket.emit("transaction", transaction);
          }
        }
        res.send("OK");

        /* const blockNumber: number = req.body.block_number;
        const transactions: any[] = req.body.transactions;
        debugConsumer("Receiving array: ", transactions);
        debugConsumer(
          "transactions[0].contract_id: ",
          transactions[0].contract_id
        );
        debugConsumer("Subscriptions: ", result.length);
        for (const subscription of result) {
          await web3Partner.notify(
            transactions,
            subscription.url,
            subscription.secret,
            subscription._id.toString()
          );
        }
        res.send("OK");

        for (const transaction of transactions) {
          debugConsumer(
            "dispatching " + transaction.event_name + " transaction: ",
            transaction.block_number
          );
          this.socket.emit("transaction", transaction);
        } */
      }
    );

    const internal_port: number = config.get("RECEIVERPORT") || 3003;
    app.listen(internal_port, () =>
      debugConsumer(`Internal server listening on port ${internal_port}.`)
    );
  }
}

export default ConsumerServer;
