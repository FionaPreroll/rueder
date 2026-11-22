# hardened-http

The HTTP client for addresses that others name, shared by the rueder server
(`../server`) and RSSifier (`../rssifier`), so that the rules against reaching
into the own network (SSRF) are in one place:

- only `http` and `https`, also as the target of a redirect;
- every address a name has is checked against the IANA lists of addresses that
  are not on the public internet (`address::internal`), and the connection goes
  to exactly the checked addresses;
- redirects are followed one at a time and checked like the first address;
- limits for the time of the whole fetch, the number of redirects and the size
  of the body after decompression;
- no proxy from the environment, no cookies, no referrer; TLS with rustls and
  Mozilla's roots compiled in.

A program names itself in `Config::agent`. `Config::allow` lets the operator
open single internal networks, e.g. for tests against a server on the same
machine.

Both programs take it by its path; it goes to crates.io when RSSifier moves to
a repository of its own.

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test        # the address rules with proptest, the client against local servers
```
