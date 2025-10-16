import Kafka from 'kafkajs'


const kafka = new Kafka.Kafka({
    clientId: 'transaction-producer',
    brokers: ['localhost:9092'],
  })



async function queueMessage() {
    const producer = kafka.producer()
    await producer.connect()
    await producer.send({
      topic: 'transactions',
      messages: [
        { value: '[{ "contract_id": "renga_mainnet", "from": "0x0000000000000000000000000000000000000000", "to": "0x0cd6f17146c9799372e457af2d6c0fa92f5ac83d", "token_id": 3047, "block_number": 15463873, "transaction_hash": "0x4d8be7cfee5850e6f56a3f0b042ef84476ed627a3cd0628daa90b9d3c04338fb", "log_index": 234 },{ "contract_id": "renga_mainnet", "from": "0x0000000000000000000000000000000000000000", "to": "0xdd69b0c3531127c9fc5f56a329e72a9da1cc9e19", "token_id": 1810, "block_number": 15463873, "transaction_hash": "0x2b15898fcf72ca8c6de051b507c5354d453fd67bac4b55c62b59fcedd6d68dcc", "log_index": 241 },{ "contract_id": "renga_mainnet", "from": "0x0000000000000000000000000000000000000000", "to": "0x5f747cf7aa9e03dc2bfed25fa8cce89bf06488b8", "token_id": 3046, "block_number": 15463873, "transaction_hash": "0xf1e8bbd12b29592ea62d1b26e86fdf88a7561da3538f054b26b130998a6e231e", "log_index": 310 }]' },
      ],
    })
    await producer.disconnect()
}   
setInterval(() =>{

    queueMessage();

},3000)
