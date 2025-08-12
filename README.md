# OIDC CLI

[![crates.io](https://img.shields.io/crates/v/oidc-cli.svg)](https://crates.io/crates/oidc-cli)
[![GitHub release (latest SemVer)](https://img.shields.io/github/v/tag/ctron/oidc-cli?sort=semver)](https://github.com/ctron/oidc-cli/releases)
[![CI](https://github.com/ctron/oidc-cli/actions/workflows/ci.yaml/badge.svg)](https://github.com/ctron/oidc-cli/actions/workflows/ci.yaml)

> A command line tool for working with OIDC

## Installation

From source with `cargo`:

```bash
cargo install oidc-cli
```

A binary with `cargo-binstall`:

```bash
cargo binstall oidc-cli
```

Download a released binary: https://github.com/ctron/oidc-cli/releases

On Windows, you can use `winget`:

```bash
winget install ctron.oidc
```

With `brew` to you can:

```bash
brew tap ctron/tap
brew install ctron/tap/oidc
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

## More examples

Create a public client from an initial refresh token. This can be useful if you have a frontend application, but no
means
of performing the authorization code flow with a local server. In case you have access to the refresh token, e.g via
the browsers developer console, you can initialize the public client with that:

```bash
oidc create public my-client --issuer https://example.com/realm --client-id foo --refresh-token <refresh-token>
```
