# Web3Cache

A high-performance, distributed caching and event management system for Web3/blockchain applications, built in Rust using the Actix-web framework.

## Overview

Web3Cache is a microservices-based platform designed to:
- **Cache and serve IPFS files** with local storage and automatic MIME type detection
- **Track blockchain events** across multiple EVM-compatible chains (Ethereum, Polygon, etc.) and Sui
- **Manage webhook subscriptions** for real-time event notifications
- **Dispatch transactions** to subscribers with JWT-signed webhooks and retry logic
- **Dynamically manage Kubernetes deployments** for write services based on contract requirements
- **Provide read APIs** for querying NFT and smart contract data

## Architecture

```
                                    ┌─────────────────────────────────────┐
                                    │         External Clients            │
                                    └─────────────────┬───────────────────┘
                                                      │
                    ┌─────────────────────────────────┼─────────────────────────────────┐
                    │                                 │                                 │
                    ▼                                 ▼                                 ▼
        ┌───────────────────┐           ┌───────────────────┐           ┌───────────────────┐
        │  web3cache-ipfs   │           │ web3cache-read    │           │web3cache-subscript│
        │    (Port 3000)    │           │    (Port 3000)    │           │    (Port 3000)    │
        │                   │           │                   │           │                   │
        │  IPFS File Cache  │           │  NFT/Contract     │           │  Event/Webhook    │
        │  & Gateway        │           │  Query API        │           │  Management       │
        └───────────────────┘           └─────────┬─────────┘           └─────────┬─────────┘
                                                  │                               │
                                                  │                               │
                                                  ▼                               ▼
                                        ┌───────────────────────────────────────────────────┐
                                        │                   MongoDB                         │
                                        │  (contracts, subscriptions, transactionblocks,   │
                                        │   apikeys, metadatachains, events_info)          │
                                        └───────────────────────────────────────────────────┘
                                                  │                               │
                                                  │                               │
        ┌───────────────────┐                     │                               │
        │web3cache-dispatch │◄────────────────────┴───────────────────────────────┘
        │    (Port 3001)    │
        │                   │                     ┌───────────────────┐
        │  Transaction      │                     │ web3cache-control │
        │  Queue & Webhook  │                     │    (Port 3000)    │
        │  Dispatcher       │                     │                   │
        └─────────┬─────────┘                     │  K8s Deployment   │
                  │                               │  Controller       │
                  │                               └─────────┬─────────┘
                  │                                         │
                  ▼                                         ▼
        ┌───────────────────┐                     ┌───────────────────┐
        │ Client Webhooks   │                     │ Kubernetes Cluster│
        │ (JWT-signed)      │                     │ (web3cache-write  │
        └───────────────────┘                     │  deployments)     │
                                                  └───────────────────┘
```

## Services

### 1. web3cache-ipfs (IPFS Caching Service)

A file caching gateway for IPFS content that reduces latency and bandwidth costs by maintaining a local cache.

**Features:**
- Fetches files from IPFS gateways (ipfs.io, cloudflare-ipfs.com)
- Stores files locally with base64-encoded filenames
- Automatic MIME type detection using `tree_magic`
- Configurable maximum file size (default: 1GB)
- Cleans up incomplete `.temp` files on startup

**Endpoints:**
| Method | Path | Description |
|--------|------|-------------|
| GET | `/ipfs/{hash}` | Fetch IPFS file by CID hash |
| GET | `/ipfs/file?file=ipfs://...` | Fetch IPFS file by URI |
| GET | `/ipfs/healthcheck` | Health check endpoint |

**Example:**
```bash
# Fetch by hash
curl http://localhost:3000/ipfs/QmUM9kxKvozDKdVbh5Dpi4pT64saEdK7RPkSfWZ3ZGJx6B/image.png

# Fetch by URI
curl "http://localhost:3000/ipfs/file?file=ipfs://QmUM9kxKvozDKdVbh5Dpi4pT64saEdK7RPkSfWZ3ZGJx6B/image.png"
```

---

### 2. web3cache-subscriptions (Event Subscription Service)

Manages smart contract registrations and webhook subscriptions for blockchain events.

**Features:**
- Register EVM and Sui smart contracts
- Automatic ABI retrieval from Etherscan/Polygonscan APIs
- Automatic detection of contract deployment block number
- Subscription management with topic filtering
- Replay historical events from a specific block number
- Supports multiple chains via `metadatachains` collection

**Endpoints:**
| Method | Path | Description |
|--------|------|-------------|
| POST | `/web3cache/events/contract-registration` | Register a new contract |
| POST | `/web3cache/events/contract-invalidation/{contract_id}` | Mark contract as offline |
| GET | `/web3cache/events/get-contract/{contract_id}` | Get contract details |
| GET | `/web3cache/events/get-contracts` | List all contracts |
| GET | `/web3cache/events/get-contract-metadata/{contract_id}` | Get full contract metadata |
| POST | `/web3cache/events/subscription-registration` | Create a new subscription |
| GET | `/web3cache/events/subscriptions` | List all subscriptions |
| GET | `/web3cache/events/subscription/{sub_id}` | Get subscription by ID |
| POST | `/web3cache/events/update-subscription/{sub_id}` | Update subscription settings |
| POST | `/web3cache/events/subscription-state/{sub_id}` | Activate/deactivate subscription |
| POST | `/web3cache/events/delete-subscription/{sub_id}` | Delete a subscription |
| POST | `/web3cache/events/replay-subscription/{sub_id}` | Replay events from block |
| GET | `/web3cache/events/healthcheck` | Health check endpoint |

**Contract Registration Payload (EVM):**
```json
{
  "contract_id": "my_contract_v1",
  "chain": "ethereum",
  "contract_address": "0x1234...",
  "contract_abi": "[...]",  // Optional - auto-fetched if not provided
  "events": "Transfer,Approval"  // Optional - extracted from ABI if not provided
}
```

**Contract Registration Payload (Sui):**
```json
{
  "contract_id": "sui_my_contract",
  "chain": "sui",
  "contract_address": "0x...",
  "events": "event1,event2",
  "modules": "module1,module2"
}
```

**Subscription Registration Payload:**
```json
{
  "contract_id": "my_contract_v1",
  "url": "https://my-server.com/webhook",
  "topics": ["Transfer", "Approval"],
  "block_number": 12345678  // Optional - start from specific block
}
```

**Authentication:**
All endpoints require the `x-webhook-api-key` header with a valid API key stored in the `apikeys` collection.

---

### 3. web3cache-dispatcher (Transaction Dispatcher Service)

Receives blockchain events from write services and dispatches them to subscribed webhooks with guaranteed delivery.

**Features:**
- Queue-based transaction dispatch with LinkedList + HashMap for O(1) operations
- Exponential backoff retry logic (up to 15 retries, max 10-second delay)
- JWT-signed webhook headers using HMAC-SHA256
- Locking mechanism to prevent duplicate deliveries
- Automatic cleanup of orphaned transaction blocks
- Batched delivery (up to 50 transaction blocks per request)

**Endpoints:**
| Method | Path | Description |
|--------|------|-------------|
| POST | `/push-transactions` | Receive transactions from write services |
| GET | `/healthcheck` | Health check endpoint |

**Transaction Payload:**
```json
{
  "contract_id": "my_contract_v1",
  "reset_nonce": 1,
  "data": [
    {
      "block_number": 12345678,
      "event_name": "Transfer",
      "transactions": [
        {
          "from": "0x...",
          "to": "0x...",
          "value": "1000000000000000000"
        }
      ]
    }
  ]
}
```

**Webhook Headers:**
```
Content-Type: application/json
x-msl-webhook-id: <subscription_id>
x-msl-webhook-type: web3.standard.events.v1
x-msl-webhook-format: JSON
x-msl-webhook-signature-type: jwt.light.v1
x-msl-webhook-nonce: -1
x-msl-webhook-timestamp: <ISO8601 timestamp>
x-msl-webhook-jwt-signature: <JWT token>
```

**Webhook Payload:**
```json
{
  "metadata": {
    "contract_id": "my_contract_v1"
  },
  "payload_count": 2,
  "payload": [
    {
      "transactions": [...],
      "block_number": 12345678,
      "event_name": "Transfer"
    }
  ]
}
```

---

### 4. web3cache-read (Read API Service)

Provides query APIs for retrieving NFT and smart contract data.

**Features:**
- Query NFTs by contract address
- Query NFTs by owner address
- Transaction history lookup
- API key authentication

**Endpoints:**
| Method | Path | Description |
|--------|------|-------------|
| GET/POST | `/web3cache/read/get-contract-nft` | Get NFTs for a contract |
| GET/POST | `/web3cache/read/get-snapshot-contract-nft` | Get snapshot of contract NFTs |
| GET/POST | `/web3cache/read/get-owner-nft` | Get NFTs owned by address |
| GET/POST | `/web3cache/read/get-user-transaction` | Get user transactions |
| GET/POST | `/web3cache/read/get-user-transaction-history` | Get transaction history |
| GET/POST | `/web3cache/read/get-contracts` | List all contracts |
| GET/POST | `/web3cache/read/get-contract` | Get contract details |
| GET/POST | `/web3cache/read/get-owners` | Get NFT owners |
| GET | `/web3cache/read/healthcheck` | Health check endpoint |

**Authentication:**
All endpoints require the `x-read-api-key` header.

---

### 5. web3cache-controller (Kubernetes Deployment Controller)

Automatically manages Kubernetes deployments for write services based on contract status in MongoDB.

**Features:**
- Polls MongoDB every 30 seconds for contract status changes
- Creates K8s deployments for contracts with `status_requirement: "online"`
- Deletes K8s deployments for contracts with `status_requirement: "offline"`
- Configurable deployment templates via `deployments/deployment.json`
- Integrates with AWS Secrets Store CSI driver

**Endpoints:**
| Method | Path | Description |
|--------|------|-------------|
| GET | `/web3cache/controller/start-write-service/{contract_id}` | Manually start a write service |

**Deployment Naming Convention:**
- Contract ID: `my_contract_v1`
- Deployment Name: `web3cache-write-my-contract-v1`

---

## Supported Blockchain Networks

The system supports multiple EVM-compatible chains via the `metadatachains` MongoDB collection:

| Chain | Chain ID | Explorer API |
|-------|----------|--------------|
| Ethereum Mainnet | 1 | api.etherscan.io |
| Goerli Testnet | 5 | api-goerli.etherscan.io |
| Sepolia Testnet | 11155111 | api-sepolia.etherscan.io |
| Polygon Mainnet | 137 | api.polygonscan.com |
| Polygon Mumbai | 80001 | api-testnet.polygonscan.com |
| Rinkeby (deprecated) | 4 | api-rinkeby.etherscan.io |
| Ropsten (deprecated) | 3 | api-ropsten.etherscan.io |
| Kovan (deprecated) | 42 | api-kovan.etherscan.io |

Additionally, **Sui Network** is supported with a different registration flow.

---

## MongoDB Collections

| Collection | Purpose |
|------------|---------|
| `contracts` | Registered smart contracts with ABI, chain info, block numbers |
| `subscriptions` | Webhook subscriptions linked to contracts |
| `transactionblocks` | Pending transaction blocks for dispatch |
| `apikeys` | API key authentication |
| `metadatachains` | Chain metadata (RPC URLs, API keys) |
| `events_info` | Block number tracking per contract/event |

---

## Environment Variables

### Common
| Variable | Description | Default |
|----------|-------------|---------|
| `PORT` | HTTP server port | 3000 |
| `MONGOURI` | MongoDB connection string | Required |
| `RUST_LOG` | Log level (info, debug, error) | info |

### web3cache-ipfs
| Variable | Description | Default |
|----------|-------------|---------|
| `FILESIZE` | Maximum file size in bytes | 1073741824 (1GB) |

### web3cache-dispatcher
| Variable | Description | Default |
|----------|-------------|---------|
| `CONSUMER_PORT` | Consumer API port | 3001 |
| `REALTIME_URL` | WebSocket realtime service URL | Required |

### web3cache-subscriptions
| Variable | Description | Default |
|----------|-------------|---------|
| `SUBSCRIPTION_PORT` | Subscriptions API port | 3000 |
| `CONTROLLERURL` | Controller service URL | Required |
| `READURL` | Read service URL | Required |

---

## Development

### Prerequisites
- Rust 1.70+
- Docker & Docker Compose
- MongoDB 5.0+
- Kubernetes cluster (for controller)

### Building

```bash
# Build all services
cd web3cache-controller && cargo build --release
cd web3cache-dispatcher && cargo build --release
cd web3cache-ipfs && cargo build --release
cd web3cache-read && cargo build --release
cd web3cache-subscriptions && cargo build --release
```

### Running Locally (Development)

Each service has a `Dockerfile.dev` for development with hot-reloading:

```bash
# IPFS Service
cd web3cache-ipfs
docker build . -f Dockerfile.dev -t web3cacheipfsdev
docker run -v $(pwd)/src:/app/src -v $(pwd)/.env:/app/.env -v $(pwd)/static:/app/static \
  --network host -it -e RUST_LOG=info web3cacheipfsdev

# Subscriptions Service
cd web3cache-subscriptions
docker build . -f Dockerfile.dev -t web3cache-subscriptions-dev
docker run -v $(pwd)/src:/app/src -v $(pwd)/.env:/app/.env \
  --network host -it -e RUST_LOG=info web3cache-subscriptions-dev
```

### Running Tests

```bash
# Run tests for each service
cd web3cache-dispatcher/web3cache && cargo test
cd web3cache-read/web3cache && cargo test
cd web3cache-subscriptions/web3cache && cargo test
cd web3cache-controller && cargo test
```

---

## Deployment

### Docker Images

```bash
# Build production image
docker build . -t web3cache-<service> --no-cache

# Push to GitHub Container Registry
export CR_PAT=<github_token>
echo $CR_PAT | docker login ghcr.io -u <username> --password-stdin
docker tag web3cache-<service> ghcr.io/<org>/web3cache-<service>:<version>
docker push ghcr.io/<org>/web3cache-<service>:<version>
```

### Kubernetes

```bash
# Create secrets
kubectl create secret generic web3cacheread --from-literal MONGOURI=<MONGOURI>

# Apply Kubernetes manifests
kubectl apply -f k8s/
```

### Tag-based Deployment Pipeline

The project uses a tag-based deployment strategy:

| Tag | Environment |
|-----|-------------|
| `env/dev` | Development |
| `env/stage` | Staging |
| `env/prod` | Production |

```bash
# Tag and deploy
git tag env/dev
git push origin env/dev

# Update tag to new commit
git tag -f env/dev <commit_hash>
git push origin env/dev --force
```

---

## Security

- **API Key Authentication**: All services validate API keys against the `apikeys` collection
- **JWT-Signed Webhooks**: Webhook deliveries include HMAC-SHA256 signed JWT tokens
- **Transaction Locking**: Prevents duplicate event delivery with MongoDB-based locks
- **Secrets Management**: Kubernetes integration with AWS Secrets Store CSI driver

---

## Related Services (Node.js/TypeScript)

The ecosystem also includes Node.js services (not covered in this Rust codebase):
- `web3cache-events`: Event publisher that monitors blockchain for events
- `web3cache-realtime`: WebSocket server for real-time event notifications
- `web3cache-graphql`: GraphQL API layer


---

## Contributing

1. Create a feature branch from `main`
2. Make changes and write tests
3. Push to your branch
4. Create a pull request to `main`
5. After approval, use tag-based deployment for testing
