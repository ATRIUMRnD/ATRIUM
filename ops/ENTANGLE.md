# ENTANGLE: laptop <-> PC room sync over Tailscale

Two machines hold the same rooms (ATRIUM, DUCTEI, LIMEN, Qallow, VEYN).
They are *entangled* when every room's HEAD commit is identical on both.
`ops/entangle.ps1` measures that and, when asked, fast-forwards one side
to the other. It never force-pushes and never copies loose files.

## Why git over Tailscale SSH, not a folder sync

- Tamper evidence is built in: every commit hash covers the full tree and
  its history. If either side is altered, the hashes stop matching and the
  script reports DIVERGED instead of quietly overwriting.
- Transport is end-to-end encrypted by Tailscale (WireGuard), and the SSH
  key pins which machine may write.
- It respects AGENTS.md: a change still lands in exactly one owning repo,
  and every run leaves a plain-English witness in `ops/witness/`.

Tailscale's own folder sharing (Taildrive) is not enabled on this tailnet
and would sync files without hashes, so it is not used.

## One-time setup

1. Tailscale is signed in on both machines. Peers seen from the laptop:
   `xing-pc`, `xings-mac-mini`, `iphone-15`. SSH port 22 on `xing-pc`
   already answers.
2. On the PC, append the laptop's SSH public key (`~/.ssh/id_ed25519.pub`
   on the laptop) to `C:\Users\<user>\.ssh\authorized_keys`. For an admin
   account use `C:\ProgramData\ssh\administrators_authorized_keys`.
3. Repeat in the other direction if the PC should also drive syncs.

## Use

Defaults are the real layout: peer `xing-pc`, user `xing`, root `D:/ATRIUM`.
Git is pinned to the Windows OpenSSH client (the one that holds the key);
Git for Windows' bundled ssh does not see it and fails with "Permission
denied (publickey)".

```powershell
.\ops\entangle.ps1          # verify only
.\ops\entangle.ps1 -Fill    # clone rooms the PC lacks
.\ops\entangle.ps1 -Push    # fast-forward PC to laptop
.\ops\entangle.ps1 -Pull    # fast-forward laptop to PC
```

The PC repos carry `receive.denyCurrentBranch = updateInstead`, so a
fast-forward push into the branch the PC has checked out updates its
working tree too (refused if that tree is dirty, nothing is overwritten).

Layout rule both machines follow: `master`/`main` in every room stays
equal to `origin`; work in progress lives on named branches, and both
machines check out the same branch per room.

Exit 0 means ENTANGLED. Exit 1 means at least one room DIVERGED and needs
a human; the witness file names the room and both hashes. Exit 2 means the
peer was unreachable or refused SSH.

## Open item

The request named a protocol "QFLTL" for the tamper guard. Nothing in the
workspace or the rooms defines that term, so this document uses commit
hashes as the entanglement check. If QFLTL means something specific,
describe it here; the state check in the script is where it plugs in.
