# web3cache-realtime

web3cache web3socket publisher

# docker commands for development

docker build . -f Dockerfile.dev -t web3cache-realtimedev
docker run -v $(pwd)/src:/app/src -v $(pwd)/.env:/app/.env --network host -it -e RUST_LOG=info web3cache-realtimedev

# docker commands for deployment

docker build . -t web3cache-realtime --no-cache
docker run -v $(pwd)/.env:/app/.env -it --network host -e RUST_LOG=info web3cache-realtime

# push image to github docker repo: ghcr.io

export CR_PAT=<github_token>
echo $CR_PAT | docker login ghcr.io -u <username> --password-stdin
docker tag web3cache-realtime ghcr.io/mintstatelabs/web3cache-realtime:<version>
docker push ghcr.io/mintstatelabs/web3cache-realtime:<version>

# start kubernetes service

kubectl apply -f k8s

# stop kubernetes service

kubectl delete -f k8s

# build and push image all at once

docker build . -t web3cache-realtime && docker tag web3cache-realtime ghcr.io/mintstatelabs/web3cache-realtime:<version> && docker push ghcr.io/mintstatelabs/web3cache-realtime:<version>
