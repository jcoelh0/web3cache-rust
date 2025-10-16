import { RESTDataSource } from "apollo-datasource-rest";
import config from "config";

class ContractRegistrationAPI extends RESTDataSource {
  willSendRequest(request: any) {
    request.headers.set("x-webhook-api-key", this.context.apiKey);
  }

  constructor() {
    super();
    this.baseURL = config.get("SUBSCRIPTION_URL");
  }

  async contractRegistration(subscriptionInput: any) {
    try {
      const data = await this.post(
        `contract-registration`,
        subscriptionInput
      );

      return {
        userErrors: [],
        result: data.result,
      };
    } catch (err: any) {
      console.log(JSON.stringify(err));
      let returnError = {
        field: [""],
        message: JSON.stringify(err.extensions.response, null, 2),
      };

      return {
        userErrors: [returnError],
        result: err.extensions.response.body.message,
      };
    }
  }

  async contractInvalidation(contract_id: string) {
    try {
      const data = await this.post(
        `contract-invalidation/` + contract_id // path
      );

      return {
        userErrors: [],
        result: data.message,
      };
    } catch (err: any) {
      console.log(JSON.stringify(err));
      let returnError = {
        field: [""],
        message: JSON.stringify(err.extensions.response, null, 2),
      };
      
      return {
        userErrors: [returnError],
        result: err.extensions.response.body.message,
      };
    }
  }
}

export default ContractRegistrationAPI;
