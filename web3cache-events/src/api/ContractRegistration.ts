import axios from "axios";
import config from "config";
import debug from "debug";

const debugContractRegistration = debug("app:contractRegist");
const { chains } = require("./chains");

function getChainId(chain: string) {
  const chainJson = chains.find((x: any) =>
    x.name.toLowerCase().includes(chain.toLowerCase())
  );
  return chainJson?.chainId;
}

function getChainApi(chain_id: number) {
  switch (chain_id) {
    case 1:
      return "https://api.etherscan.io/";
    case 5:
      return "https://api-goerli.etherscan.io/";
    case 42:
      return "https://api-kovan.etherscan.io/";
    case 4:
      return "https://api-rinkeby.etherscan.io/";
    case 3:
      return "https://api-ropsten.etherscan.io/";
    case 11155111:
      return "https://api-sepolia.etherscan.io/";
    case 137:
      return "https://api.polygonscan.com/";
    case 80001:
      return "https://api-testnet.polygonscan.com/";
    default:
      return null;
  }
}

function getChainAdress(address_type: string, chain_id: number) {
  if (address_type === "wss") {
    const urls: any = config.get("WSS_URLS");
    return urls[chain_id.toString()];
  } else {
    const urls: any = config.get("HTTPS_URLS");
    return urls[chain_id.toString()];
  }
}

async function getContractAbiIfAvailable(
  contract_address: string,
  chain_id: number
) {
  const get_abi_url =
    getChainApi(chain_id) +
    "/api?module=contract&action=getabi&address=" +
    contract_address +
    "&apikey=" +
    config.get("ETHERSCAN_API_KEY");

  debugContractRegistration(get_abi_url);
  try {
    const response = await axios.get(get_abi_url);
    //debugContractRegistration(response.data);
    return response.data.result;
  } catch (err) {
    debugContractRegistration(err);
    return null;
  }
}

async function getInitialBlockNumberByContractAddress(
  contract_address: string,
  chain_id: number
) {
  const etherscan_api_key = config.get("ETHERSCAN_API_KEY");
  const chain_api_url = getChainApi(chain_id);

  const get_contract_creation_url = chain_api_url +
    "/api?module=contract&action=getcontractcreation&contractaddresses=" +
    contract_address +
    "&apikey=" +
    etherscan_api_key;

  try {
    let response = await axios.get(get_contract_creation_url);
    //debugContractRegistration(response.data);

    const get_transaction_by_hash_url = chain_api_url +
      "/api?module=proxy&action=eth_getTransactionByHash&txhash=" +
      response.data.result[0].txHash +
      "&apikey=" +
      etherscan_api_key;
      
    response = await axios.get(get_transaction_by_hash_url);
    debugContractRegistration(response.data.result.blockNumber);
    
    return parseInt(response.data.result.blockNumber, 16);
  } catch (err) {
    debugContractRegistration(err);
    return null;
  }
}

module.exports = {
  getChainId,
  getChainApi,
  getChainAdress,
  getInitialBlockNumberByContractAddress,
  getContractAbiIfAvailable,
};
