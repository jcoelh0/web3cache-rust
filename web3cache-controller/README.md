# web3cache-read

web3cache controller rust-actix

# docker commands for development

docker build . -f Dockerfile.dev -t web3cache-controllerdev
docker run -v $(pwd)/deployments:/app/deployments -v $(pwd)/src:/app/src -v $(pwd)/.env:/app/.env --network host -it -e RUST_LOG=info web3cache-controllerdev

# docker commands for deployment

docker build . -t web3cache-controller --no-cache
docker run -it --network host -e RUST_LOG=info web3cache-controller

# push image to github docker repo: ghcr.io

export CR_PAT=<github_token>
echo $CR_PAT | docker login ghcr.io -u <username> --password-stdin
docker tag web3cache-controller ghcr.io/mintstatelabs/web3cache-controller:<version>
docker push ghcr.io/mintstatelabs/web3cache-controller:<version>

# setup credentials and secrets

kubectl create secret generic web3cacheread --from-literal MONGOURI=<MONGOURI>
echo -n <username>:<token> | base64
https://stackoverflow.com/questions/61912589/how-can-i-use-github-packages-docker-registry-in-kubernetes-dockerconfigjson

# start kubernetes service

kubectl apply -f k8s

# stop kubernetes service

kubectl delete -f k8s

# build and push image all at once

docker build . -t web3cache-controller && docker tag web3cache-controller ghcr.io/mintstatelabs/web3cache-controller:<version> && docker push ghcr.io/mintstatelabs/web3cache-read:<version>


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
