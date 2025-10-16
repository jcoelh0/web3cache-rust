import { Document } from "mongoose";

export default interface Apikey extends Document {
  apikey: string;
  apisecret: string;
  partner_name: string;
  subid: string;
}
