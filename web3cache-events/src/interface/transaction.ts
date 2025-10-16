import { ObjectId } from "mongodb";
import { Document } from "mongoose";

export interface SingleTransaction {
  _id: string;
  contract_id: string;
  from: string;
  to: string;
  token_id: number;
  block_number: number;
  transaction_hash: string;
  log_index: number;
}

export default interface TransactionBlock extends Document {
  subid: string;
  transactions: SingleTransaction[];
  secret: string;
  block_number: number;
  locked_until: Date;
}
