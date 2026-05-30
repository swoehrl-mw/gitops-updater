# GitOps Updater

Gitops-updater is a small command line tool to simplify component updates in a [FluxCD-based](https://fluxcd.io/flux/) GitOps setup. It assumes a structure as I describe it in my [blog post on multi-cluster GitOps setups](https://blog.sebastianwoehrl.name/blog/2025-06-gitops-with-fluxcd/). The basic structure: Manifests for applications/components are organized into folders. These are then deployed into a cluster by placing a FluxCD `Kustomization` referencing that app folder in the folder for a cluster. Any customization, especially versions, is done using [postBuild variables](https://fluxcd.io/flux/components/kustomize/kustomizations/#post-build-variable-substitution). You can also find a simple [example in this repo](./example/).

This tool can automatically determine the newest version for all used components and bundle them into an updateset. You can then apply this updateset to all your clusters in stages. With earlier stages being the testing ground for later stages (at minimum you would have a dev stage for all your test/dev clusters and a prod stage for all your production clusters).

## Quickstart

1. Clone this repository and navigate into it with a terminal
2. Download the CLI binary for your platform from the [Releases page](https://github.com/swoehrl-mw/gitops-updater/releases). Rename the file to `gitops-updater` (or `gitops-updater.exe` for Windows) and make it executable if needed (`chmod +x gitops-updater` on Linux).
3. Run `./gitops-updater updateset new`. This will generate a file `updateset-<random>.json`. It will have a list of components and versions.
4. Run `./gitops-updater deploy --record`. This will update all versions for the `dev` cluster
5. Use `git diff` or your favorite Git tool to check any changes made. You should see new versions set in files in the `example/clusters/dev` folder.
6. Run `./gitops-updater deploy --record`.
7. If you run `git diff` again you will see that the same versions have now been set for the `prod` cluster.

In a real setup you would commit after each stage to deploy the changes to the clusters. Once you had verified the new version works, you would then update the next stage.

## Configuration

The tool is configured using a configuration file normally called `updater.yaml`. You can call it anything else but then must pass `-c <filename>` to each CLI call.

It has the following structure:

```yaml
baseDir: example  # The base directory for your GitOps structure. Leave out if it is the repo root
# A list of components
releases:
  # A component from a HelmRelease
  - helm:
      repo: https://kubernetes.github.io/ingress-nginx  # The Helm repository URL
      chart: ingress-nginx  # The name of the chart
      versionRegex: \d+\.\d+\.\d+ # An optional regex to filter out versions (e.g. pre-releases)
    target:
      file: nginx.yaml  # The name of the file in the cluster folder containing the Kustomization for the component/app
      postBuildVariable: nginxVersion  # The postBuild variable that contains the version
  - helm:
      repo: oci://ghcr.io/prometheus-community/charts  # Helm repositories can also be read from OCI
      chart: kube-prometheus-stack
      versionRegex: \d+\.\d+\.\d+ # An optional regex to filter out versions (e.g. pre-releases)
    target:
      file: kps.yaml
      postBuildVariable: kpsVersion
    name: prometheus  # Optional name
  # A component with a container image
  - container:
      image: hivemq/hivemq4  # The registry/image name
      versionRegex: \d+\.\d+\.\d+  # An optional regex to filter out versions
    target:
      file: hivemq.yaml
      postBuildVariable: hivemqVersion
    name: hivemq  # Optional name
# A list of stages and clusters. These are processed in order
stages:
  - name: dev  # Name of the stage
    # List of clusters in that stage (names must correspond to the folder names in the clusters folder)
    clusters:
      - dev
  - name: prod
    clusters:
      - prod
```

You can have gitops-updater analyze your structure and automatically generate the configuration file for you. Run `gitops-updater generate-config example` for that (add `-c <configfile>` to write the config to a different file).

`container` components can not be detected automatically, you need to manually add them. The tool also does not know about stages and simply places all clusters in a default stage. You need to edit and add stages based on your setup. For nicer reading you can optionally give each release a `name`. If none is configured the helm chart name or container image will be used when the list of releases that have updates is printed by the `updateset` command.

## Available commands

* `generate-config [-c <path>] [<root>]`: Generates a new config file, optionally provide a filename and a root directory for the GitOps structure
* `updateset [-c <path>] new`: Generate a new updateset
* `deploy [-c <path>] [-u <id>] [-s <stage>] [--record]`: Update versions from an updateset for a stage. If no stage id (`-u`) is given you will be prompted to selected one. If `--record` is given the stage that was deployed is stored in the updateset. If no stage (`-s`) is given it will either use the first stage or the next stage not yet recorded in the updateset.

## Download

Binaries are published via the [GitHub Releases page](https://github.com/swoehrl-mw/gitops-updater/releases), currently for Linux (amd64/x86_64 and ARM), MacOS (ARM) and Windows (amd64/x86_64).

You can also build the binary yourself by checking out the repo and running `cargo build --release`.

## Development

This tools is developed in [Rust](https://rust-lang.org/learn/get-started/). You need a current Rust+Cargo toolchain for local development.

Simply run `cargo run -- <args>` to run the CLI tool locally.
