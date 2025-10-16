import {Document} from 'mongoose'

export default interface Subscription extends Document {
    
    apikey: string
    url : string
    isActive: boolean
    secret: string
    topics: string[]
    isSui: boolean
    contract_id : string
    block_number : number
}