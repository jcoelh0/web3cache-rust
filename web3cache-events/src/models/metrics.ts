import mongoose, { Schema } from "mongoose";
import IMetrics from "../interface/apikey";

const MetricsSchema: Schema = new Schema(
  {
    description: { type: String, required: true },
    type: { type: String, required: true },
    time: { type: Number, required: true },
    date: { type: Date, required: true },
  },
  {
    timestamps: false,
  }
);

export default mongoose.model<IMetrics>("Metrics", MetricsSchema);
