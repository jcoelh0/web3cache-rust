import axios from "axios";
import debug from "debug";
import Subscription from "../models/subscription";
import MoveTransaction from "../models/moveTransaction";
import Logs from "../models/logs";
import ILogs from "../interface/logs";
import ITransaction, { SingleTransaction } from "../interface/transaction";
import jwt from "jsonwebtoken";
const debugDispatcher = debug("app:Dispatcher");
const debugQueue = debug("app:queue");
const debugLogs = debug("app:logs");

const timeout = (time: number) =>
  new Promise((resolve) => {
    setTimeout(resolve, time);
  });
const MAX_RETRIES = 15;

function mergeQueues(
  currentQueue: string[],
  queueMap: any,
  newItems: string[]
) {
  debugDispatcher("merging queues!");
  const currentDate = new Date();
  debugQueue("Before: ", currentQueue);
  for (const item of newItems) {
    if (!(item in queueMap)) {
      debugQueue("M!Adding subid: ", item);
      debugQueue("M!State []: ", currentQueue);
      debugQueue("M!State {}: ", Object.keys(queueMap));
      currentQueue.push(item);
      queueMap[item] = [100, currentDate];
      debugQueue("M!queue: ", currentQueue);
      debugQueue("M!queueMap: ", queueMap);
    }
  }
  debugQueue(
    "After: ",
    currentQueue.map((q) => q[0])
  );
}

class Dispatcher {
  private queue: string[] = [];
  private queueMap: any = {};
  private queueOn: boolean = false;

  constructor() {}

  public async forceQueue() {
    if (this.queueOn) return;
    this.queueOn = true;
    debugDispatcher("Queue on");

    while (true) {
      let usefullWork: number = MAX_RETRIES;
      while (this.queue.length > 0) {
        const nextsubid: string = this.queue.shift() as string;
        const nextitem: any[] = this.queueMap[nextsubid];
        delete this.queueMap[nextsubid];
        const increaseTimeout: number = nextitem[0];
        const waitUntil: Date = nextitem[0];//erro
        const currentDate: Date = new Date();
        if (currentDate < waitUntil) {
          this.queue.push(nextsubid);
          this.queueMap[nextsubid] = [increaseTimeout, waitUntil];
          //debugDispatcher(`Still need to wait ${Number(waitUntil) - Number(currentDate)} to send! queue: ${this.queue}`)
          usefullWork -= 1;
          if (usefullWork <= 0) {
            await mergeQueues(
              this.queue,
              this.queueMap,
              (
                await Subscription.find({ isActive: true }).select({ _id: 1 })
              ).map((s) => s._id.toString())
            );
            usefullWork = MAX_RETRIES;
          }

          await timeout(50);
          continue;
        }
        usefullWork = MAX_RETRIES;

        if (await this.anyPending(nextsubid)) {
          await this.trySendTransactions(nextsubid, increaseTimeout);
        } else {
        }
        await timeout(200);
      }
      debugDispatcher("Queue cool off");
      await Promise.all([this.init(), await timeout(1000)]);
    }
    this.queueOn = false;
  }

  public async init() {
    this.queue = (
      await Subscription.find({ isActive: true }).select({ _id: 1 })
    ).map((s) => s._id.toString());

    this.queueMap = this.queue.reduce((T: any, elem: string) => {
      T[elem] = [100, new Date()];
      return T;
    }, {});

    this.forceQueue();
  }

  public async trySendTransactions(subid: string, currentTimeIncrease: number) {
    debugDispatcher("trySendTransactions called on subid", subid);
    const transactionGroup: any = await MoveTransaction.find({ subid })
      .sort({ subid: 1, block_number: 1 })
      .limit(50);
    debugDispatcher(
      "transactionGroup to send in batch: ",
      transactionGroup.length
    );
    let withProblems: boolean = false;

    const currentDate = new Date();
    const newDate = new Date(Number(new Date()) + 10000);
    const updateResult: any = await Promise.all(
      transactionGroup.map((t: any) =>
        MoveTransaction.updateOne(
          { _id: t._id, locked_until: { $lte: currentDate } },
          { $set: { locked_until: newDate } }
        )
      )
    );

    debugDispatcher(updateResult.map((u: any) => u.matchedCount));
    const transactionArray: any[] = [];
    const subscription: any = await Subscription.findOne({ _id: subid });
    const ackIds: string[] = [];
    for (const idx in transactionGroup) {
      const transactionBlock: any = transactionGroup[idx];

      const lock_result = updateResult[idx];
      if (lock_result.matchedCount == 0) {
        // already locked, unlock all transactions blocked by this dispatcher
        debugDispatcher("Already lock");
        withProblems = true;

        // unblock
        const startIdx = Number(idx);
        const listOfUpdates: any[] = [];
        for (var i = startIdx + 1; i < transactionGroup.length; i++) {
          if (updateResult[i].matchedCount == 1) {
            // locked by this dispatcher
            listOfUpdates.push(transactionGroup[i]._id.toString());
          }
        }
        if (listOfUpdates.length > 0) {
          await MoveTransaction.updateMany(
            { _id: { $in: listOfUpdates }, locked_until: newDate },
            { $set: { locked_until: currentDate } }
          );
        }
        break;
      }

      transactionArray.push({
        transactions: transactionBlock.transactions,
        block_number: transactionBlock.transactions[0].block_number,
      });
      ackIds.push(transactionBlock._id.toString());
    }
    if (transactionArray.length) {
      if (
        await this.dispatchTransaction(
          transactionArray,
          subscription,
          subid,
          true
        )
      ) {
        await MoveTransaction.deleteMany({ _id: { $in: ackIds } });
      } else {
        withProblems = true;
      }
    } else {
      withProblems = true;
    }
    if (await this.anyPending(subid)) {
      debugDispatcher("Automatic added subid", subid, " to the queue");
      const nextDelay: number = withProblems
        ? Math.min(currentTimeIncrease * 2, 10000)
        : 150;
      if (!(subid in this.queueMap)) {
        this.queue.push(subid);
      }
      this.queueMap[subid] = [
        nextDelay,
        new Date(Number(currentDate) + nextDelay),
      ];
    }
  }

  public async anyPending(subid: string) {
    try {
      const result: boolean =
        (await MoveTransaction.findOne({ subid })) != null;
      debugDispatcher("Pending transactions on", subid, "to send: ", result);
      return result;
    } catch (ex) {
      return false;
    }
  }

  public async storeTransaction(
    transactions: SingleTransaction[],
    secret: string,
    subid: string
  ) {
    try {
      const transaction = new MoveTransaction({
        subid: subid,
        transactions: transactions,
        secret: secret,
        block_number: transactions[0].block_number,
        locked_until: new Date(),
      });

      await MoveTransaction.create(transaction);
      if (!(subid in this.queueMap)) {
        debugDispatcher("Manually adding subid to the queue");
        debugQueue("Adding subid: ", subid);
        debugQueue("State []: ", this.queue);
        debugQueue("State {}: ", Object.keys(this.queueMap));
        this.queue.push(subid);
        this.queueMap[subid] = [100, new Date()];
        debugQueue("queue: ", this.queue);
        debugQueue("queueMap: ", this.queueMap);
      }
      this.forceQueue();
    } catch (error) {
      debugDispatcher(error);
    }
  }

  public async bulkStore(
    transactions: any[],
    blockNumber: number,
    subid: string,
    secret: string
  ) {
    if (transactions.length == 0) return;

    const result = await MoveTransaction.deleteMany({
      subid,
    });

    debugDispatcher("Delete result: ", result);
    debugDispatcher("Secret", secret);
    const currentDate = new Date();
    await MoveTransaction.insertMany(
      transactions.map(
        (t) =>
          ({
            subid: subid,
            transactions: t,
            secret: secret,
            block_number: t[0].block_number,
            locked_until: currentDate,
          } as ITransaction)
      )
    );
    if (!(subid in this.queueMap)) {
      debugDispatcher("Manually adding subid to the queue");
      debugQueue("Adding subid: ", subid);
      debugQueue("State []: ", this.queue);
      debugQueue("State {}: ", Object.keys(this.queueMap));
      this.queue.push(subid);
      this.queueMap[subid] = [150, new Date()];
      debugQueue("queue: ", this.queue);
      debugQueue("queueMap: ", this.queue);
    }
    this.forceQueue();
  }

  public async notify(
    transactions: SingleTransaction[],
    webhook_url: string,
    secret: string,
    subid: string
  ) {
    await this.storeTransaction(transactions, secret, subid.toString());
  }

  public async dispatchTransaction(
    transactionArray: any[],
    { url: webhook_url, secret: secret, contract_id, apikey: apikey }: any,
    subid: string,
    ignore_store: boolean = false
  ) {
    try {
      Logs.insertMany(
        transactionArray.reduce(
          (T: any, transactionBlock: any) =>
            T.concat(
              transactionBlock.transactions.map((t: SingleTransaction) => ({
                contract_id: t.contract_id,
                transaction_hash: t.transaction_hash,
                log_index: t.log_index,
                timestamp: Math.round(new Date().getTime()),
              }))
            ),
          []
        )
      );

      const headers = {
        "Content-Type": "application/json",
        "x-msl-webhook-id": subid,
        "x-msl-webhook-type": "web3.standard.events.v1",
        "x-msl-webhook-format": "JSON",
        "x-msl-webhook-signature-type": "jwt.light.v1",
        "x-msl-webhook-nounce": -1,
        "x-msl-webhook-timestamp": new Date().toISOString(),
        "x-msl-webhook-jwt-signature": jwt.sign(
          {
            contract_id: contract_id,
            timestamp: new Date().toISOString(),
            subcription_id: subid,
            //last_block_number: transactions[0].block_number,
          },
          apikey,
          {
            expiresIn: 60 * 60,
          }
        ),
      };

      const res = await axios({
        url: webhook_url,
        method: "POST",
        data: {
          metadata: {
            contract_id: contract_id,
            chain: "mumbai",
          },
          payload_count: transactionArray.length,
          payload: transactionArray,
        },
        headers: headers,
        timeout: 5000,
      });
      debugDispatcher(`transaction was sent with success`);
      return true;
    } catch (error) {
      debugDispatcher(error);
      if (!ignore_store) {
        await Promise.all(
          transactionArray.map((transactionObject: any) =>
            this.storeTransaction(transactionObject.transactions, secret, subid)
          )
        );

        //await ;
      }
    }
    return false;
  }
}

export default Dispatcher;
