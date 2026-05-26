# Vulnerable & High-Risk Package Watchlist

Load this during Step 2 (Dependency Audit). Check versions in the project's lock files.

---

## Rust / Cargo

| Crate | Issue |
|-------|-------|
| openssl | Check advisory db for current version |
| hyper | Check advisory db for current version |

Reference: https://rustsec.org/advisories/

---

## Java / Maven

| Package | Vulnerable Versions | Issue |
|---------|-------------------|-------|
| log4j-core | 2.0-2.14.1 | Log4Shell RCE (CVE-2021-44228) — CRITICAL |
| log4j-core | 2.15.0 | Incomplete fix — still vulnerable |
| Spring Framework | < 5.3.28, < 6.0.13 | Various CVEs |
| Spring Boot | < 3.1.4 | Various |
| Jackson-databind | < 2.14.0 | Deserialization |
| Apache Commons Text | < 1.10.0 | Text4Shell RCE (CVE-2022-42889) |
| Apache Struts | < 6.3.0 | Various RCE |
| Netty | < 4.1.94 | HTTP request smuggling |

---

## General Red Flags (Any Ecosystem)

Flag any dependency that:
1. Has not been updated in > 2 years AND has > 10 open security issues
2. Has been deprecated by its maintainer with a security advisory
3. Is a fork of a known package from an unknown publisher (typosquatting)
4. Has a name that's one character off from a popular package (e.g., `lodash` vs `1odash`)
5. Was recently transferred to a new owner (check git history / npm transfer notices)