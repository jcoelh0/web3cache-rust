# web3cache-read

web3cache read rust-actix

# docker commands for development

docker build . -f Dockerfile.dev -t web3cachereaddev
docker run -v $(pwd)/src:/app/src -v $(pwd)/.env:/app/.env --network host -it -e RUST_LOG=info web3cachereaddev

# docker commands for deployment

docker build . -t web3cacheread --no-cache
docker run -it --network host -e RUST_LOG=info web3cacheread

# push image to github docker repo: ghcr.io

export CR_PAT=<github_token>
echo $CR_PAT | docker login ghcr.io -u <username> --password-stdin
docker tag web3cacheread ghcr.io/mintstatelabs/web3cache-read:<version>
docker push ghcr.io/mintstatelabs/web3cache-read:<version>

# setup credentials and secrets

kubectl create secret generic web3cacheread --from-literal MONGOURI=<MONGOURI>
echo -n <username>:<token> | base64
https://stackoverflow.com/questions/61912589/how-can-i-use-github-packages-docker-registry-in-kubernetes-dockerconfigjson

# start kubernetes service

kubectl apply -f k8s

# stop kubernetes service

kubectl delete -f k8s

# build and push image all at once

docker build . -t web3cache-read && docker tag web3cache-read ghcr.io/orangecomet/web3cache-read:<version> && docker push ghcr.io/orangecomet/web3cache-read:<version>
