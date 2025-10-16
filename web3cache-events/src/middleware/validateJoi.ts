import express from "express";
import Joi, { ObjectSchema } from "joi";
import debug from "debug";
const debugJoi = debug("app:joi");

var re_weburl = new RegExp(
  "^" +
    // protocol identifier (optional)
    // short syntax // still required
    "(?:(?:(?:https?|ftp):)?\\/\\/)" +
    // user:pass BasicAuth (optional)
    "(?:\\S+(?::\\S*)?@)?" +
    "(?:" +
    // IP address exclusion
    // private & local networks
    "(?!(?:10|127)(?:\\.\\d{1,3}){3})" +
    "(?!(?:169\\.254|192\\.168)(?:\\.\\d{1,3}){2})" +
    "(?!172\\.(?:1[6-9]|2\\d|3[0-1])(?:\\.\\d{1,3}){2})" +
    // IP address dotted notation octets
    // excludes loopback network 0.0.0.0
    // excludes reserved space >= 224.0.0.0
    // excludes network & broadcast addresses
    // (first & last IP address of each class)
    "(?:[1-9]\\d?|1\\d\\d|2[01]\\d|22[0-3])" +
    "(?:\\.(?:1?\\d{1,2}|2[0-4]\\d|25[0-5])){2}" +
    "(?:\\.(?:[1-9]\\d?|1\\d\\d|2[0-4]\\d|25[0-4]))" +
    "|" +
    // host & domain names, may end with dot
    // can be replaced by a shortest alternative
    // (?![-_])(?:[-\\w\\u00a1-\\uffff]{0,63}[^-_]\\.)+
    "(?:" +
    "(?:" +
    "[a-z0-9\\u00a1-\\uffff]" +
    "[a-z0-9\\u00a1-\\uffff_-]{0,62}" +
    ")?" +
    "[a-z0-9\\u00a1-\\uffff]\\." +
    ")+" +
    // TLD identifier name, may end with dot
    "(?:[a-z\\u00a1-\\uffff]{2,}\\.?)" +
    ")" +
    // port number (optional)
    "(?::\\d{2,5})?" +
    // resource path (optional)
    "(?:[/?#]\\S*)?" +
    "$",
  "i"
);

const schemas: any = {
  RestartSub: Joi.object({
    block_number: Joi.number()
      .integer()
      .min(0)
      .max(9223372036854775807) // 2**63 - 1
      .required(),
  }),
  StateSub: Joi.object({
    activate: Joi.boolean().optional(),
  }),
  UpdateSub: Joi.object({
    url: Joi.string()
      .regex(re_weburl)
      .label("URL")
      .messages({ url: "Invalid url" }),
    add_topics: Joi.array(),
    remove_topics: Joi.array(),
    set_topics: Joi.array(),
  }).or("url", "add_topics", "remove_topics", "set_topics"),
  RegisterSub: Joi.object({
    url: Joi.string()
      .regex(re_weburl)
      .label("url")
      .messages({ url: "Invalid url" })
      .required(),
    topics: Joi.array()
      .items(Joi.string().optional())
      .min(0)
      .label("topics")
      .messages({ topics: "Must be an array of strings" })
      .optional(),
    contract_id: Joi.string().required(),
    block_number: Joi.number().optional(),
    format: Joi.string().valid("JSON").optional(),
  }),
  SecretAndApi: Joi.object({
    "x-webhook-api-key": Joi.string().alphanum().required(),
    "x-webhook-api-secret": Joi.string().alphanum().optional(),
  }).unknown(true),
  WebHookAPI: Joi.object({
    // check length when auth is done
    "x-webhook-api-key": Joi.string().alphanum().required(),
  }).unknown(true),
};

export const validate =
  (schema: any, headerSchema: any) =>
  async (
    req: express.Request,
    res: express.Response,
    next: express.NextFunction
  ) => {
    try {
      if (schema) {
        debugJoi("Validating ", req.body);
        const { error } = schemas[schema].validate(req.body);

        if (error) {
          return res.status(400).send(error.details[0].message);
        }
      }
      if (headerSchema) {
        const { error } = schemas[headerSchema].validate(req.headers);
        debugJoi("Validating headers");
        if (error) {
          return res.status(400).send(error.details[0].message);
        }
      }

      next();
    } catch (error: any) {
      debugJoi(error);
      return res.status(400).send(error.message);
    }
  };
