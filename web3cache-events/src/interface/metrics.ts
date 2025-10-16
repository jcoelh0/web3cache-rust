import { Document } from "mongoose";

export default interface Metrics extends Document {
  description: string;
  type: string;
  time: number;
  date: Date;
}
