# OIDC CLI

> A command line tool for working with OIDC

## Installation

```bash
cargo install oidc-cli
```

## Example

Creating a new (confidential) client:

```bash
oidc create confidential --name test -issuer http://example.com/realm --client-id foo --client-secret bar
```

Then, getting an access token:

```bash
oidc token --name test
```

