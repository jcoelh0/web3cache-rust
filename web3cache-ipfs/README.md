# ipfs-events

ipfs file cacher

# docker commands for development

docker build . -f Dockerfile.dev -t web3cacheipfsdev
docker run -v $(pwd)/src:/app/src -v $(pwd)/.env:/app/.env -v $(pwd)/static:/app/static --network host -it -e RUST_LOG=info web3cacheipfsdev

# docker commands for deployment

docker build . -t web3cacheipfs --no-cache
docker run -it --network host -e RUST_LOG=info web3cacheipfs

# push image to github docker repo: ghcr.io

export CR_PAT=<github_token>
echo $CR_PAT | docker login ghcr.io -u <username> --password-stdin
docker tag web3cacheipfs ghcr.io/mintstatelabs/web3cache-ipfs:<version>
docker push ghcr.io/mintstatelabs/web3cache-ipfs:<version>

# setup credentials and secrets

echo -n <username>:<token> | base64
https://stackoverflow.com/questions/61912589/how-can-i-use-github-packages-docker-registry-in-kubernetes-dockerconfigjson

# start kubernetes service

kubectl apply -f k8s

# stop kubernetes service

kubectl delete -f k8s
