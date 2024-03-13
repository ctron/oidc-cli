# OIDC CLI

> A command line tool for working with OIDC

## Installation

```bash
cargo install oidc-cli
```

## Example

Creating a new (confidential) client:

```bash
oidc create confidential my-client -issuer https://example.com/realm --client-id foo --client-secret bar
```

Then, get an access token:

```bash
oidc token my-client
```

Or combine it with e.g., HTTPie:

```bash
http example.com/api "Authorization:$(oidc token my-client --bearer)"
```
