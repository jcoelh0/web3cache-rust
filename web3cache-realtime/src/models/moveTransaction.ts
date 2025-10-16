
import mongoose, {Schema} from 'mongoose'
import ITransactions  from "../interface/transaction"


const TransactionSchema: Schema = new Schema(
    {
        subid: {type : String  , required: true},
        transactions: { type: Array, of: Object, required: true},
        block_number: { type: Number, required: true},
        locked_until: {type: Date, require: true}
    },
    {
        timestamps:true,
        versionKey: false
    }
);

TransactionSchema.index({
    subid: 1,
    block_number: 1
  
})


export default mongoose.model<ITransactions>('Transactionblock',TransactionSchema);