import mongoose, { Schema } from "mongoose";
import ISubscription from "../interface/subscription";

const SubscriptionSchema: Schema = new Schema(
  {
    apikey: { type: String, required: true },
    url: { type: String, required: true },
    topics: { type: Array, of: Object, required: true },
    secret: { type: String, required: true },
    isActive: { type: Boolean, required: false },
    contract_id: { type: String, required: false },
    isSui: { type: Boolean, required: false },
    block_number: { type: Number, required: false },
  },
  {
    timestamps: true,
    versionKey: false,
  }
);
export default mongoose.model<ISubscription>(
  "Subscription",
  SubscriptionSchema
);
