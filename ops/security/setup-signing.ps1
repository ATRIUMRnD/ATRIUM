# One-time setup: sign every commit with a dedicated SSH key.
# Run in an ELEVATED PowerShell (the ssh-agent service change needs admin).
# ssh-keygen will ask you for a passphrase. Use one: it is what keeps a stolen
# key file from being usable. Nothing here is run for you.

# 1. Make the ssh-agent service available, so you type the passphrase once per session.
Set-Service ssh-agent -StartupType Automatic
Start-Service ssh-agent

# 2. A key used only for signing (separate from any key used to log in).
ssh-keygen -t ed25519 -C "xingxerx git signing" -f "$HOME\.ssh\id_ed25519_signing"
ssh-add "$HOME\.ssh\id_ed25519_signing"

# 3. Tell git to sign with it, and to verify against a local allowed-signers list.
git config --global gpg.format ssh
git config --global user.signingkey "$HOME\.ssh\id_ed25519_signing.pub"
git config --global commit.gpgsign true
git config --global tag.gpgsign true
git config --global gpg.ssh.allowedSignersFile "$HOME\.ssh\allowed_signers"
"xingxerx@gmail.com $(Get-Content "$HOME\.ssh\id_ed25519_signing.pub")" | Set-Content -Encoding ascii "$HOME\.ssh\allowed_signers"

# 4. Register the public key with GitHub as a SIGNING key (this opens a browser once
#    to grant the extra scope), so GitHub shows "Verified" on your commits.
gh auth refresh -h github.com -s admin:ssh_signing_key
gh ssh-key add "$HOME\.ssh\id_ed25519_signing.pub" --type signing --title "xing-pc git signing"

# 5. Check: make any commit, then confirm the signature (look for "Good ... signature").
#    git log -1 --show-signature
#
# Only after commits show as Verified on GitHub, optionally require signatures on the
# protected branches (see enable-github-security.ps1, last section).