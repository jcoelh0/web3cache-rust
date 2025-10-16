import { RESTDataSource } from "apollo-datasource-rest";
import { stringify } from "querystring";
import config from "config";

function formatSubscription(inputSubscription: any) {
  let subscription = {
    createdAt: inputSubscription.createdAt,
    contractId: inputSubscription.contract_id || "",
    endpoint: {
      __typename: "WebhookHttpEndpoint",
      callbackUrl: inputSubscription.url,
    },
    format: "JSON",
    id: inputSubscription._id,
    includedEvents: inputSubscription.topics,
    updatedAt: inputSubscription.updatedAt,
  };
  return subscription;
}

class WebhookContractApi extends RESTDataSource {
  constructor() {
    super();
    this.baseURL = config.get("SUBSCRIPTION_URL"); //"https://web3cache.mintstatelabs.org/web3cache/events";
  }

  willSendRequest(request: any) {
    request.headers.set("x-webhook-api-key", this.context.apiKey);
    //request.headers.set('x-webhook-api-secret', this.context.apiSecret);
  }

  async getEVMWebhook(subscriptionInput: any) {
    const data = await this.get(
      `subscription/` + subscriptionInput.id // path
    );

    return formatSubscription(data.subscription);
  }

  async getEVMWebhooks(subscriptionInput: any) {
    try {
      const data = await this.get(
        `subscriptions` // path
      );
      return data.subscriptions.map(formatSubscription);
    } catch (err) {
      return [];
    }
  }

  async createEVMWebhook(subscriptionInput: any) {
    let inputBody = {
      contract_id: subscriptionInput.contractRegistrationId,
      url: subscriptionInput.webhookSubscription.callbackUrl,
      //format:  subscriptionInput.webhookSubscription.format,
      topics: subscriptionInput.webhookSubscription.includeEvents,
    };

    try {
      const data = await this.post(
        `subscription-registration`, // path
        inputBody // request body
      );

      return {
        userErrors: [],
        webhookContractEventSubscription: formatSubscription(data.subscription),
      };
    } catch (err: any) {
      let returnError = {
        field: [""],
        message: JSON.stringify(err.extensions.response, null, 2),
      };

      return {
        userErrors: [returnError],
        webhookContractEventSubscription: null,
      };
    }
  }

  async replayEVMWebhook(subscriptionInput: any) {
    let inputBody = {
      block_number: subscriptionInput.webhookSubscription.blockNumber,
    };

    try {
      const data = await this.post(
        `replay-subscription/` + subscriptionInput.id, // path
        inputBody // request body
      );
      // { transaction_blocks_replay: i64  }

      return {
        userErrors: [],
        webhookContractEventSubscription: formatSubscription(data),
      };
    } catch (err: any) {
      console.log("err: ", err);
      let returnError = {
        field: [""],
        message: JSON.stringify(err.extensions.response, null, 2),
      };

      return {
        userErrors: [returnError],
        webhookContractEventSubscription: null,
      };
    }
  }

  async updateEVMWebhook(subscriptionInput: any) {
    let inputBody = {
      url: subscriptionInput.webhookSubscription.callbackUrl,
      //format:  subscriptionInput.webhookSubscription.format,
      topics: subscriptionInput.webhookSubscription.includeEvents,
    };

    try {
      const data = await this.post(
        `subscription-update/` + subscriptionInput.id, // path
        inputBody // request body
      );

      return {
        userErrors: [],
        webhookContractEventSubscription: formatSubscription(data),
      };
    } catch (err: any) {
      let returnError = {
        field: [""],
        message: JSON.stringify(err.extensions.response, null, 2),
      };

      return {
        userErrors: [returnError],
        webhookContractEventSubscription: null,
      };
    }
  }

  async deleteWebhook(subscriptionInput: any) {
    try {
      const data = await this.post(
        `delete-subscription/` + subscriptionInput.id // path
      );

      return {
        userErrors: [],
        deletedWebhookSubscriptionId: subscriptionInput.id,
      };
    } catch (err: any) {
      let returnError = {
        field: [],
        message: JSON.stringify(err.extensions.response, null, 2),
      };

      return {
        userErrors: returnError,
        deletedWebhookSubscriptionId: null,
      };
    }
  }
}

export default WebhookContractApi;
