import debug from "debug";
import fs from "fs";
import express from "express";

const errorDebug = debug("app:error");
const debugsession = debug("app:session");

/* async function abortSession(session) {
  if (!session.hasEnd) {
    await session.abortTransaction();
    session.endSession();
    debugsession('session closed');
  }
}
 */
// eslint-disable-next-line no-unused-vars
export default (
  err: express.ErrorRequestHandler,
  req: express.Request,
  res: express.Response,
  _next: express.NextFunction
) => {
  fs.appendFile(
    "logs/logfile.txt",
    `${new Date().toISOString()} - ${err}, requestpath: ${
      req.originalUrl
    }, request body: ${JSON.stringify(req.body)}\n`,
    () => {
      // Do nothing
    }
  );
  errorDebug(err);
  //if (req.session && !req.session.hasEnded) {
  //  abortSession(req.session);
  //}
  res.status(500).send("Something failed.");
};
