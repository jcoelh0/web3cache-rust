# web3cache-events

web3cache event publisher

# prepare database with contract info

MONGORUI=<MONGOURI> bash initmongoDB.sh

# docker commands for development

docker build . -f Dockerfile.dev -t web3cacheeventsdev
docker run -v $(pwd)/src:/app/src -v $(pwd)/.env:/app/.env --network host -it -e RUST_LOG=info web3cacheeventsdev

# docker commands for deployment

docker build . -t web3cacheevents --no-cache
docker run -v $(pwd)/.env:/app/.env -it --network host -e RUST_LOG=info web3cacheevents

# push image to github docker repo: ghcr.io

export CR_PAT=<github_token>
echo $CR_PAT | docker login ghcr.io -u <username> --password-stdin
docker tag web3cacheevents ghcr.io/mintstatelabs/web3cache-events:<version>
docker push ghcr.io/mintstatelabs/web3cache-events:<version>

# start kubernetes service

kubectl apply -f k8s

# stop kubernetes service

kubectl delete -f k8s

# build and push image all at once

docker build . -t web3cache-events && docker tag web3cache-events ghcr.io/mintstatelabs/web3cache-events:<version> && docker push ghcr.io/mintstatelabs/web3cache-events:<version>
