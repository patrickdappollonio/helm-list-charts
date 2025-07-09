# `helm-list-charts`

[![Github Downloads](https://img.shields.io/github/downloads/patrickdappollonio/helm-list-charts/total?color=orange&label=github%20downloads)](https://github.com/patrickdappollonio/helm-list-charts/releases)

This is a tiny Rust application that allows you to navigate Helm repository indexes.

Normally, you can use something like `artifacthub.io` to search for Helm charts, but unfortunately the navigation is a bit more point-and-click there than programatical. This tool allows you to provide a target Helm repository and fetch all available charts, and then navigate through them.

The charts are printed to `stdout` in a table format (think, the output of `docker ps` or `kubectl get pods`) and all Helm charts are grouped by name. The output will print the chart name, the version, the app version, and the description.

Since some repositories might host so many charts, the output of the program is piped to a pager automatically (per your `$PAGER` environment variable, defaulting to `less`). You can disable the pager with the `--no-pager` flag.

You can zoom in on a specific chart by providing the chart name as an argument to the program with the `--chart` flag. This will print the same output, just limited to the chart you're interested in.

## Installation

[Download a binary from the releases page](https://github.com/patrickdappollonio/helm-list-charts/releases) and place it in a folder that is in your `$PATH`.

Alternatively, if you're on macOS or Linux and you're a Homebrew user, you can install it via Homebrew:

```shell
brew install patrickdappollonio/tap/helm-list-charts
```

## Usage

```shell
helm-list-charts --source https://prometheus-community.github.io/helm-charts
```

You'll see an output like this at the time of writing this README (truncated for brevity):

```bash
CHART                                       TYPE           VERSION  DESCRIPTION                                           APP VERSION    CREATED                KUBE VERSION
prometheus-operator-crds                    application    19.0.0   A Helm chart that collects custom resource...         v0.81.0        Mar 15, 2025 6:08 pm   >=1.16.0-0
prometheus-operator-crds                    application    18.0.1   A Helm chart that collects custom resource...         v0.80.1        Feb 25, 2025 3:36 am   >=1.16.0-0
prometheus-operator-crds                    application    18.0.0   A Helm chart that collects custom resource...         v0.80.0        Feb 6, 2025 11:13 am   >=1.16.0-0
prometheus-operator-crds                    application    17.0.2   A Helm chart that collects custom resource...         v0.79.2        Dec 18, 2024 1:36 pm   >=1.16.0-0
prometheus-postgres-exporter                application    6.10.0   A Helm chart for prometheus postgres-exporter         v0.17.0        Mar 3, 2025 1:36 pm    <unspecified>
prometheus-postgres-exporter                application    6.9.0    A Helm chart for prometheus postgres-exporter         v0.17.0        Feb 25, 2025 3:20 pm   <unspecified>
prometheus-postgres-exporter                application    6.8.1    A Helm chart for prometheus postgres-exporter         v0.16.0        Dec 23, 2024 12:22 pm  <unspecified>
prometheus-postgres-exporter                application    6.8.0    A Helm chart for prometheus postgres-exporter         v0.16.0        Dec 18, 2024 9:01 pm   <unspecified>
prometheus-blackbox-exporter                application    9.4.0    Prometheus Blackbox Exporter                          v0.26.0        Mar 20, 2025 10:05 am  >=1.21.0-0
prometheus-blackbox-exporter                application    9.3.0    Prometheus Blackbox Exporter                          v0.26.0        Feb 28, 2025 8:37 am   >=1.21.0-0
prometheus-blackbox-exporter                application    9.2.0    Prometheus Blackbox Exporter                          v0.25.0        Jan 31, 2025 4:20 pm   >=1.21.0-0
prometheus-blackbox-exporter                application    9.1.0    Prometheus Blackbox Exporter                          v0.25.0        Nov 6, 2024 1:04 pm    >=1.21.0-0
# ... and so on
```

You can filter for just one of all the charts by providing the `--chart` flag:

```shell
helm-list-charts --source https://prometheus-community.github.io/helm-charts --chart prometheus-operator-crds
```

This will only show the `prometheus-operator-crds` charts.

### Supported flags

```shell
CLI tool for listing Helm charts from a chart repository (chartmuseum-style)

Usage: helm-list-charts [OPTIONS] --source <SOURCE>

Options:
      --source <SOURCE>    The Helm chart repository source URL (e.g. https://bitnami-labs.github.io/sealed-secrets)
      --chart <CHART>      (Optional) Filter by a specific chart name (case insensitive)
      --type <CHART_TYPE>  (Optional) Filter by chart type (case insensitive, e.g. "application" or "library")
      --no-pager           Disable the pager (enabled by default on outputs longer than 25 lines)
  -h, --help               Print help
  -V, --version            Print version
```

You can also disable the pager permanently by setting either the `NO_PAGER` environment variable to true for a more CI-friendly way, or the specific environment variable `HELM_LIST_CHARTS_NO_PAGER` to true. By default, pager is enabled for more than 25 lines of output.

## ... But why?

While it's possible to add a Helm index to your local machine and use `helm search repo`, it's not always the case that you want to add a repository to your local machine. That, and in CI is just the extra set of steps that you might not want to deal with.

This tool is just a quick-and-dirty way to navigate Helm repositories without having to add them to your local machine or your CI environment. The output is also a bit more friendly than the raw YAML output of the Helm repository index, and while you could potentially use `yq` to parse the YAML index, this tool is a bit more user-friendly and pipe-friendly.
