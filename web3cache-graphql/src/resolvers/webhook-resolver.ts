export const resolvers = {
  Query: {
    webhookEVMSubscription(parent: any, args: any, { dataSources }: any) {
      return dataSources.webhookAPI.getEVMWebhook(args);
    },
    webhookEVMSubscriptions(parent: any, args: any, { dataSources }: any) {
      return dataSources.webhookAPI.getEVMWebhooks(args);
    },
  },
  Mutation: {
    webhookEVMSubscriptionCreate(parent: any, args: any, { dataSources }: any) {
      return dataSources.webhookAPI.createEVMWebhook(args);
    },
    webhookEVMSubscriptionReplay(parent: any, args: any, { dataSources }: any) {
      return dataSources.webhookAPI.replayEVMWebhook(args);
    },
    webhookSubscriptionDelete(parent: any, args: any, { dataSources }: any) {
      return dataSources.webhookAPI.deleteWebhook(args);
    },
    contractRegistration(_: any, { input }: any, { dataSources }: any) {
      return dataSources.contractAPI.contractRegistration(input);
    },
    contractInvalidation(_: any, { input }: any, { dataSources }: any) {
      return dataSources.contractAPI.contractInvalidation(input.contract_id);
    },
  },
};