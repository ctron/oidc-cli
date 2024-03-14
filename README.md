# OIDC CLI

[![CI](https://github.com/ctron/oidc-cli/actions/workflows/ci.yaml/badge.svg)](https://github.com/ctron/oidc-cli/actions/workflows/ci.yaml)

> A command line tool for working with OIDC

## Installation

```bash
cargo install oidc-cli
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
