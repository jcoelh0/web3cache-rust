require('express-async-errors');
import fs from 'fs';
import debug from 'debug';

const uncaughtDebug = debug('app:uncaught');

export default function setUncaught() {
  process.on('uncaughtException', (ex) => {
    fs.appendFile(
      'logs/uncaught.txt',
      `${new Date().toISOString()} - uncaughtException - ${ex}\n`,
      () => {
        // Do nothing
      },
    );
    uncaughtDebug(ex);
  });

  process.on('unhandledRejection', (ex) => {
    fs.appendFile(
      'logs/uncaught.txt',
      `${new Date().toISOString()} - unhandledRejection - ${ex}\n`,
      () => {
        // Do nothing
      },
    );
    uncaughtDebug(ex);
  });
};
