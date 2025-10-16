import express from "express";
import axios from "axios";

const port = 3002;
const webhookURL_register =
  "http://localhost:3000/web3cache/events/subscription-registration";
const webhookURL_restart =
  "http://localhost:3000/web3cache/events/replay-subscription";
const app = express();
app.use(express.json());
let subid_arr: number[] = [];
const webhook_api_key = "";
const n_partners = 10;

async function registerSub(
  contract_id: string,
  block_number: number,
  url: string,
  topics: string[],
  webhook_api_key: string
): Promise<number> {
  var data = {
    contract_id: contract_id,
    url: url,
    topics: topics,
    block_number: block_number,
  };

  const headers = {
    "Content-Type": "application/json",
    "x-webhook-api-key": webhook_api_key,
  };

  try {
    const res = await axios.post(webhookURL_register, data, { headers });
    console.log(`subscribe response ${res.data}`);
    return res.data;
  } catch (error: any) {
    console.log(error.response.data);
    //console.log(error.message);
    return error.message;
  }
}

async function restartSub(
  block_number: number,
  webhook_api_key: string,
  webhook_sub_id: number
) {
  var data = {
    block_number: block_number,
  };
  const headers = {
    "Content-Type": "application/json",
    "x-webhook-api-key": webhook_api_key,
  };
  try {
    const res = await axios.post(
      webhookURL_restart + "/" + webhook_sub_id,
      data,
      { headers }
    );
    console.log(`replay response ${res.data}`);
    return res.data;
  } catch (error: any) {
    console.log(error.message);
  }
}

const runpartners = async () => {
  try {
    let result = await registerSub(
      "polychain_monsters_mainnet",
      34175816,
      "https://web3cache.mintstatelabs.org/web3cache/events/black-hole-endpoint",
      ["transfer"],
      webhook_api_key
    );
    console.log(result);
    let sub_id: number = Object(Object(result)["subscription"])["_id"];
    console.log("subscription ID " + sub_id);
    if (sub_id) {
      subid_arr.push(sub_id);
    }
  } catch (error) {
    console.log(error);
  }
};
for (let i = 0; i < n_partners; i++) {
  runpartners();
}

const replayAllpartners = async () => {
  for (let i of subid_arr) {
    try {
      let result_replay = await restartSub(34175816, webhook_api_key, i);
      console.log("replay response:" + result_replay);
    } catch (error) {
      console.log(error);
    }
  }
};

replayAllpartners();
