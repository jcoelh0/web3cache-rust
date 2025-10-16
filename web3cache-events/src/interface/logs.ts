import { Document } from "mongoose";

export default interface Logs extends Document {
  transaction_hash: string;
  contract_id: string;
  log_index: number;
  timestamp: number;
}
