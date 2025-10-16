import mongoose, { Schema } from "mongoose";
import ILogs from "../interface/logs";

const LogsSchema: Schema = new Schema(
  {
    transaction_hash: { type: String, required: true },
    contract_id: { type: String, required: true },
    log_index: { type: Number, required: true },
    timestamp: { type: Number, required: true },
  },
  {
    timestamps: false,
  }
);

export default mongoose.model<ILogs>("Logs", LogsSchema);
