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

docker build . -t web3cache-read && docker tag web3cache-read ghcr.io/mintstatelabs/web3cache-read:<version> && docker push ghcr.io/mintstatelabs/web3cache-read:<version>


# Tag-based Deployment Pipeline Documentation

**Note to Developers:** If you're primarily looking for the commands to manage tags in this new system, you can directly jump to Section 5.

## Table of Contents

1. Introduction
2. The Tag-Based Deployment Solution
3. Branch Protection Rules
4. Environment Protection Rules
5. Developer Guide: Tag Management
6. Migration Steps

## 1. Introduction

In any development cycle, one of the significant bottlenecks can be the deployment process. Our existing approach has been to use a branch-based strategy, which has its own set of problems.

Our previous method required multiple pull requests for the same code: initially to the `main` branch and then to environment-specific branches like `env/dev` or `env/prod`. This often resulted in delayed deployments and non-synchronized branches, as we repeatedly approved the same code and waited for CI processes. To address this, we're transitioning to a streamlined, tag-based deployment system.

## 2. The Tag-Based Deployment Solution

### Overview

Adopting a tag-based deployment streamlines our workflow by eliminating redundant approval processes and preventing branch discrepancies. Tags serve as fixed markers to specific commits, offering both stability and traceability in deployments. The updated process involves developing new features in a branch that's rebased with `origin/main`. After coding, apply a `env/dev` or `env/stage` tag to test in the respective environments. Following successful testing, create a pull request for peer review and approval. Once the changes are merged into `main`, use the `env/prod` tag to deploy to production.

### Environment Access

Use tags named after the target environment to trigger deployments:

- `env/dev` for Development
- `env/stage` for Staging
- `env/prod` for Production

### Advantages

1. **Single Approval**: Only one approval needed for the deployment, saving time.
2. **Immutability**: Tags ensure the code won't change after approval.
3. **Simpler CI/CD**: Easier to manage without multiple branches.
4. **Code Revision**: Code is reviewed once in the pull request to `main`.

### Steps

1. **Create and Push Tag**: Tag the approved commit and push it to trigger deployment.

   ```bash
   git tag <ENV_TAG> <COMMIT_HASH>
   git push origin <ENV_TAG>
   ```

2. **Monitor**: Inspect action logs to track deployment status.

## Branch Protection Rules

For the integrity and stability of our codebase, we have applied protection rules to our primary branch, `main`. It's crucial to note that the `main` branch is the only branch with these protection measures. Here are the established rules:

1. **Require a Pull Request Before Merging**
2. **Require Approvals**
3. **Require Approval of the Most Recent Reviewable Push**
4. **Require Status Checks to Pass Before Merging**
5. **Require Branches to be Up to Date Before Merging**
6. **Require Linear History**

By applying these rules, we ensure that the `main` branch consistently represents a stable and thoroughly reviewed version of the codebase.

## 4. Environment Protection Rules

You can find the rules of the environments in the `Environment` section of the setting in the project. For each environment you need to have the following configuration:

- **Required reviewers**: this should be set according to the environment in question.
- **Deployment branches and tags**: this should have only the correspondent tag, the title must state `0 branches and 1 tag allowed`.
- **Environment secrets and variables**: These values should be inserted according to the deployment environment.

## 5. Developer Guide: Managing Tags

In our tag-based deployment approach, tags serve as references to specific points in the codebase. Here's how you can effectively manage and utilize tags for your day-to-day tasks, we will be using `env/dev` environment as example:

### Creating a Tag

**Tag the Current State (HEAD)**

This command places a tag on the latest commit.

```bash
git tag env/dev
```

or

**Tag a Specific Commit**

To tag a particular commit, use its hash:

```bash
git tag env/dev <commit_hash>
```

### Deleting a Local Tag

If you need to remove or update a tag, start by deleting it from your local repository:

```bash
git tag -d env/dev
```

### Updating a Tag

There will be occasions when a tag needs to be shifted to a different commit.

```bash
git tag -f env/dev
```

or

```bash
git tag -f env/dev <commit_hash>
```

### Pushing a Tag to the Remote Repository

After creating or updating a tag locally, you'll need to push it to the remote repository:

```bash
git push origin env/dev
```

### Overwriting a Tag in the Remote Repository

If a tag has been updated locally and needs to replace the one in the remote repository, use:

```bash
git push origin env/dev --force
```

**Note**: Always ensure proper communication with your team when updating tags, to maintain consistency and prevent conflicts.

## 6. Migration Steps

- Delete the protection rules for the `env/*` branches.
- Add the protection rules for `main` described above
- Locally delete all `env/*`, this is important for the push command to work
- On the environments' settings, considering the name of the deployment environment to be `ENV`, change from `branch env/ENV` to `tag env/ENV`, disallowing branch permissions to access the variables and provide this access to the respective tag.

# Deploy kube-deployer service in a new cluster

For this section lets assume we want to deploy to environment `ENV` where this environment is `dev`, `stage` or `prod`.

1. Choose a domain to host the service, for example `cicd.something.com`.
2. Create and validate a certificate for this domain on AWS (validation is done in cloudflare).
3. In the `kube-deployer` Github repository [https://github.com/OrangeComet/kube-deployer](https://github.com/OrangeComet/kube-deployer), go to deployment/`ENV`/ingress-https.yaml:
   - Modify the domain name in spec.rules[0].host.
   - Modify the certificate-arn in metadata.annotations['alb.ingress.kubernetes.io/certificate-arn']
   - Now you need to manually apply the service with kubectl using the following command:
     ```
     kubectl apply -f deployment/ENV
     ```
4. Go to Cloudflare, and route the chosen domain name to the k8s domain generated from the ingress configuration, this domain can be checked using the AWS console, in the Load Balancers section.

   Example:
   | Type | Name | Content |
   |--------|------|------------------------------|
   | CNAME | cicd | {k8s-load-balancer-domain-name} |

5. Ensure that `kube-deployer` Github project settings contain the necessary secrets for the `ENV` environment. For this, there is a need of a specific cloudflare authentication token generated for only for the chosen domain. To perform this task it is possible to do it manually editing the secrets in the environment or take advantage of the tool that we developed in the [https://github.com/OrangeComet/github-branching-automation](https://github.com/OrangeComet/github-branching-automation).

   The list of the environment secrets is the following:

   - AWS_ACCESS_KEY
   - AWS_REGION
   - AWS_SECRET_ACCESS_KEY
   - CF_ACCESS_CLIENT_ID
   - CF_ACCESS_CLIENT_SECRET
   - DEPLOYER_URL

6. In AWS secrets insert the secrets in the correct AWS account and region and the name `ENV`/kube-deployer:

   - docker-auth-token
   - SKOPEO_USER
   - SKOPEO_TOKEN

7. Now we just have to push it to env/`ENV` tag. There is no need to push it to any branch until the service deployment is completed (advantage of using tag-based deployment)

8. In Kubernetes, give access to the user `user-controller` on the `deployment-manager` namespace to read the AWS secrets. This can be achieved using a script in the [https://github.com/OrangeComet/oc-aws-cdk](https://github.com/OrangeComet/oc-aws-cdk).

```bash
bash scripts/createServiceAccount.sh -n deployment-manager -u user-controller
```

This skopeo user and token is a pair of Github authentication with a single permission, `package:read`. This authentication is used for the skopeo tool to download the partial images of other services.

The docker-auth-token can be found in
Example: `{ "auths": { "ghcr.io": { "auth": "token" } } }`

9. Accept deployment in Github actions

10. As last step, if the changes are correct and as expected, update the main branch to match your successfully deployed service from the env/`ENV` tag.
