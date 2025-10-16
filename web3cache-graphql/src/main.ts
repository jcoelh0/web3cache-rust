import { useApolloDataSources } from "@envelop/apollo-datasources";
import { useGraphQlJit } from "@envelop/graphql-jit";
import { createServer } from "@graphql-yoga/node";
import config from "config";
import fastify, { FastifyReply, FastifyRequest } from "fastify";
import ContractRegistrationAPI from "./datasources/contract-registration-api";
import WebhookContractApi from "./datasources/webhook-contract-api";

import { buildSubgraphSchema } from "@apollo/subgraph";
import { join } from "path";
const { mergeTypeDefs } = require("@graphql-tools/merge");
const { loadFiles } = require("@graphql-tools/load-files");

const { resolvers } = require("./resolvers/webhook-resolver");

async function main() {
  // This is the fastify instance you have created
  const app = fastify({ logger: true });

  const typeDefsArray = await loadFiles(join(__dirname, "schemas"));
  const typeDefs = mergeTypeDefs(typeDefsArray);

  const schema = buildSubgraphSchema({
    typeDefs: typeDefs,
    resolvers: resolvers,
  });

  const graphQLServer = createServer<{
    req: FastifyRequest;
    reply: FastifyReply;
  }>({
    schema: schema,
    // Integrate Fastify logger
    logging: {
      debug: (...args) => args.forEach((arg) => app.log.debug(arg)),
      info: (...args) => args.forEach((arg) => app.log.info(arg)),
      warn: (...args) => args.forEach((arg) => app.log.warn(arg)),
      error: (...args) => args.forEach((arg) => app.log.error(arg)),
    },
    context: ({ req }) => {
      // Get the api key from the headers.
      const apiKey = req.headers["x-webhook-api-key"] || "";
      const apiSecret = req.headers["x-webhook-api-secret"] || "";

      // Add the user to the context
      return { apiKey, apiSecret };
    },
    graphiql: true,
    plugins: [
      useGraphQlJit(),
      // ... other plugins ...
      useApolloDataSources({
        dataSources() {
          return {
            webhookAPI: new WebhookContractApi(),
            contractAPI: new ContractRegistrationAPI(),
          };
        },
        // To provide a custom cache, you can use the following code (InMemoryLRUCache is used by default):
        // cache: new YourCustomCache()
      }),
    ],
  });

  //graphQLServer.start()

  /**
   * We pass the incoming HTTP request to GraphQL Yoga
   * and handle the response using Fastify's `reply` API
   * Learn more about `reply` https://www.fastify.io/docs/latest/Reply/
   **/
  app.route({
    url: "/web3cache/graphql",
    method: ["GET", "POST", "OPTIONS"],
    async handler(req, reply) {
      // Second parameter adds Fastify's `req` and `reply` to the GraphQL Context
      const response = await graphQLServer.handleIncomingMessage(req, {
        req,
        reply,
      });
      response.headers.forEach((value, key) => {
        reply.header(key, value);
      });

      reply.status(response.status);

      reply.send(response.body);

      return reply;
    },
  });

  app.route({
    url: "/web3cache/graphql/healthcheck",
    method: ["GET"],
    async handler(req, reply) {
      return "ok";
    },
  });

  // This will allow Fastify to forward multipart requests to GraphQL Yoga
  app.addContentTypeParser("multipart/form-data", {}, (req, payload, done) =>
    done(null)
  );

  app.listen(
    {
      port: Number(config.get("PORT")) || 4000,
      host: "0.0.0.0",
    },
    (err) => {
      if (err) throw err;
    }
  );
}

main();
