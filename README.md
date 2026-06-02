# OIDC CLI

[![crates.io](https://img.shields.io/crates/v/oidc-cli.svg)](https://crates.io/crates/oidc-cli)
[![GitHub release (latest SemVer)](https://img.shields.io/github/v/tag/ctron/oidc-cli?sort=semver)](https://github.com/ctron/oidc-cli/releases)
[![CI](https://github.com/ctron/oidc-cli/actions/workflows/ci.yaml/badge.svg)](https://github.com/ctron/oidc-cli/actions/workflows/ci.yaml)

> A command line tool for working with OIDC

## Installation

* Download a released binary: https://github.com/ctron/oidc-cli/releases

* From source with `cargo`:

  ```bash
  cargo install oidc-cli
  ```

* A binary with `cargo-binstall`:

  ```bash
  cargo binstall oidc-cli
  ```

* On Windows, you can use `winget`:

  ```commandline
  winget install ctron.oidc
  ```

* With `brew` to you can:

  ```bash
  brew tap ctron/tap
  brew install ctron/tap/oidc
  ```

* With `snap` you can:

  ```bash
  snap install oidc
  ```

## Example

Creating a new (confidential) client:

```bash
oidc create confidential my-client --issuer https://example.com/realm --client-id foo --client-secret bar
```

Creating a new (public) client:

```bash
oidc create public my-client --issuer https://example.com/realm --client-id foo
```

Then, get an access token:

```bash
oidc token my-client
```

Or combine it with e.g., HTTPie:

```bash
http example.com/api "Authorization:$(oidc token my-client --bearer)"
```

Or even shorter:

```bash
http example.com/api $(oidc token -H my-client)
```

This also works with `curl`:

```bash
curl http://example.com/api -H $(oidc token -H my-client)
```

## XH integration

Use the `xh-plugin-oidc` binary with `xh` custom auth plugins:

```bash
xh --auth-type=plugin:oidc --auth=my-client https://example.com/api
```

The `xh-plugin-oidc` binary can also discover the client name from a local config file. Starting from
the current directory, it walks up parent directories and searches for `.xh-auth-oidc.json`,
`.xh-auth-oidc.yaml`, then `.xh-auth-oidc.toml`:

```toml
client_name = "my-client"

[http]
timeout = "60s"
connect_timeout = "30s"
min_tls_version = "1.2"
disable_system_certificates = false
additional_root_certificates = []
```

Then the client name does not need to be passed to `xh`:

```bash
xh --auth-type=plugin:oidc https://example.com/api
```

## More examples

Create a public client from an initial refresh token. This can be useful if you have a frontend application, but no
means
of performing the authorization code flow with a local server. In case you have access to the refresh token, e.g via
the browsers developer console, you can initialize the public client with that:

```bash
oidc create public my-client --issuer https://example.com/realm --client-id foo --refresh-token <refresh-token>
```
