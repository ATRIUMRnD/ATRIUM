# ATRIUM security baseline

What every ATRIUM repository, service and CI workflow must have, where each repository stands,
and what only the account owner can switch on. Status is as of October 27, 2026 and describes
the `chore/security-baseline` pull requests, so a row is only true once its PR is merged.

## The baseline

1. **No root, least privilege.** Daemons run as a dedicated unprivileged user, with no capabilities,
   a read-only system and a short list of writable paths (`NoNewPrivileges`, `ProtectSystem=strict`,
   `ReadWritePaths=`). Containers run as a non-root uid with a read-only root filesystem and
   `cap_drop: [ALL]`.
2. **Loopback by default.** A listener binds `127.0.0.1` unless it has a reason not to. A listener
   reachable from another machine needs TLS (mutual TLS where peers are known) or an explicit,
   logged opt-in. Never a wildcard address in a default.
3. **Authentication on.** `--no-auth` style switches are for throwaway local runs only, never in a
   Dockerfile, unit file or compose file.
4. **Secrets stay out of git.** Credentials come from an env file or secret store (mode 0600, root
   only), never inline in a unit, compose file or workflow. Key material is git-ignored.
5. **CI is least privilege and pinned.** Workflows set `permissions: contents: read` at the top and
   widen per job only when needed. Every action is pinned to a full commit SHA with the tag in a
   comment. Checkouts use `persist-credentials: false`. Test tooling comes from a pinned file.
6. **Dependencies are watched.** `.github/dependabot.yml` covers GitHub Actions, Cargo, pip and
   Docker where present. Dependabot alerts, secret scanning and push protection are on.
7. **Reporting path.** A `SECURITY.md` says how to report privately, and private vulnerability
   reporting is on for public repositories.
8. **Provenance.** Commits are signed with a dedicated SSH key. Protected branches require the
   CI checks and, once signing works, signed commits.
9. **Nothing authoritative is a bare file.** A file that grants an action (an accepted policy
   verdict) must match what the append-only ledger recorded.

## Status by repository

| Repository | Unit or image | Bind and auth | CI pinned, read-only token | Dependabot | SECURITY.md | Key files ignored |
|---|---|---|---|---|---|---|
| LIMEN | `limend.service` and `deploy_node.sh` sandboxed, unprivileged | gRPC loopback default, cleartext refused off-loopback | yes | yes | yes | yes |
| DUCTEI | none | already refuses wildcard listeners | yes (PAT checkout no longer persists) | yes | yes | yes |
| Qallow | none | not reviewed | yes | yes | yes | yes |
| VEYN | Dockerfile non-root, auth on, digests pinned; compose hardened | daemon binds loopback by design | yes | yes (incl. Docker) | yes | yes |
| brain | none | not reviewed | no workflows | yes | yes | yes |
| QFLTL | none | security tools bind wildcard on purpose | no workflows | yes | yes | yes |
| player-harness | none | not reviewed | yes | yes | yes | yes |
| jspace-lab | none | not reviewed | no workflows | yes | yes | yes |
| oneiro | none | UDP listener on all interfaces (see gaps) | yes | yes | yes | yes |
| ATRIUM | none | n/a | no workflows | n/a | yes | yes |

## Only the account owner can do these

- **Turn on the GitHub features** (Dependabot alerts and security updates for all repositories,
  secret scanning and push protection on the public ones, private vulnerability reporting):
  review and run `ops/security/enable-github-security.ps1`. Today Dependabot alerts are off
  everywhere, and secret scanning is on only for LIMEN and jspace-lab.
- **Set up commit signing:** review and run `ops/security/setup-signing.ps1` (it needs a
  passphrase and an elevated shell for the ssh-agent service). Then optionally require signed
  commits on `main` and `master`.
- **Deploy the hardened units** on the host that runs limend or a coordination node, then check
  `systemd-analyze verify` and `systemd-analyze security` there. The LIMEN unit scores 3.1 (OK)
  against 9.4 (UNSAFE) for the old one when measured offline.

## Known gaps and accepted risks

- **Unbuilt Docker image.** The VEYN image and compose stack were validated with
  `docker compose config` only; the Docker engine was not running. Build and run them once:
  `docker compose up --build`, then `docker compose logs consumer`.
- **VEYN `Cargo.lock` is stale** against its manifests (the DUCTEI git dependency), so
  `cargo build --locked` fails. Refresh the lock, then add `--locked` to the Dockerfile.
- **UDP listeners on all interfaces** in VEYN's EEG adapter and oneiro's `phase_sync` are there for
  devices on the LAN. Bind them to a specific interface or Tailscale address, and firewall the port.
- **QFLTL's network tools** (sniffer, guard, attacker) bind wildcards because that is their job.
  Keep them off shared hosts.
- **The proposal ledger is append-only but not signed.** `apply_proposal.py` refuses a verdict that
  differs from the ledger, which stops casual edits, not someone who can rewrite both. Review of the
  commit, signed commits and branch protection are the defence.
- **Dependabot and path or git dependencies.** Cargo manifests that point outside the repository
  (DUCTEI to VEYN) can make Dependabot report errors for that directory.
- **No audit gate in CI yet.** `cargo audit`, `cargo deny` and `pip-audit` are not installed or
  required. Dependabot alerts cover known advisories until they are.
- **Not reviewed:** Qallow's C persistence layer, brain's sidecar, and the code of the repositories
  marked "not reviewed" above. This baseline is configuration and defaults, not a code audit.

## Check it yourself (PowerShell)

Unpinned actions across a checkout:

```powershell
Select-String -Path .github\workflows\*.yml -Pattern '^\s*(- )?uses:\s*[\w./-]+@(?![0-9a-f]{40}\b)'
```

Secret-shaped strings in tracked files:

```powershell
git grep -InE 'AKIA[0-9A-Z]{16}|ghp_[A-Za-z0-9]{30,}|github_pat_[A-Za-z0-9_]{30,}|-----BEGIN (RSA |EC |OPENSSH )?PRIVATE KEY-----'
```

Sandbox score of a unit file (needs systemd 250 or newer, for example in WSL):

```powershell
wsl -e systemd-analyze security --offline=true --no-pager /mnt/d/LIMEN/scripts/limend.service
```