# web3cache-subscriptions

web3cache subscriptions actix-web in rust

# docker commands for development

docker build . -f Dockerfile.dev -t web3cache-subscriptions-dev
docker run -v $(pwd)/src:/app/src -v $(pwd)/.env:/app/.env --network host -it -e RUST_LOG=info web3cache-subscriptions-dev

# docker commands for deployment

docker build . -t web3cachesubscriptions --no-cache
docker run -it --network host -e RUST_LOG=info web3cachesubscriptions

# push image to github docker repo: ghcr.io

export CR_PAT=<github_token>
echo $CR_PAT | docker login ghcr.io -u <username> --password-stdin
docker tag web3cache-subscriptions ghcr.io/mintstatelabs/web3cache-subscriptions:<version>
docker push ghcr.io/mintstatelabs/web3cache-subscriptions:<version>

# setup credentials and secrets

kubectl create secret generic web3cacheevents --from-literal MONGOURI=<MONGOURI>
echo -n <username>:<token> | base64
https://stackoverflow.com/questions/61912589/how-can-i-use-github-packages-docker-registry-in-kubernetes-dockerconfigjson

# start kubernetes service

kubectl apply -f k8s

# stop kubernetes service

kubectl delete -f k8s

# build and push image all at once

docker build . -t web3cache-subscriptions && docker tag web3cache-subscriptions ghcr.io/mintstatelabs/web3cache-subscriptions:<version> && docker push ghcr.io/mintstatelabs/web3cache-subscriptions:<version>
