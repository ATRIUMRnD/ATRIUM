# Product surface

VEYN is the commercial room. ATRIUM does not sell.

## What is for sale

- **VEYN** is the local sensor data bus (daemon on `:7700`). Source is
  public under Elastic License 2.0: others may use and modify it, they
  may not offer it as a hosted service, and they may not strip license
  keys or notices.
- **Intero** (in the VEYN tree) is the first-party app on that bus:
  intent, biometric memory, MCP, desktop app. That is the product a
  customer runs.
- LIMEN stays a library (`limen-compiler`). DUCTEI and Qallow stay
  plumbing. ATRIUM stays the constitution.

## Live state (August 29 2026)

- License file is ELv2, copyright XINGXERX / ATRIUM.
- GitHub description previously said "open-source". That was wrong for
  ELv2. Corrected on the VEYN repo to source-available / Intero.
- Security PR https://github.com/xingxerx/VEYN/pull/77 merged with all
  CI jobs green (format, three OS builds, Intero macOS, harness).
- No GitHub Releases yet. Main had been red on Format + Clippy before
  that PR.
- ELv2 talks about license-key functionality. Confirm whether Intero
  actually has a key check before charging anyone. If it does not,
  adding one is VEYN-owned application code, not ATRIUM.

## Routing

Work that is "make money with the bus" goes to VEYN (Intero). Work that
is "quantum routing as a library" goes to LIMEN. Do not open commercial
copy in ATRIUM, DUCTEI, or Qallow.
