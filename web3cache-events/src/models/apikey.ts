import mongoose, {Schema} from 'mongoose'
import IApikey  from "../interface/apikey"


const ApikeySchema : Schema = new Schema(
    {
        apikey :{type : String , required: true},
        partner_name: {type: String , required: true},
        subid : {type : Number , required : true}

    },
    {
        timestamps: true
    }
)


export default mongoose.model<IApikey>('Apikey',ApikeySchema);