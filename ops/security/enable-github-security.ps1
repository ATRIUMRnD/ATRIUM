# Turn on GitHub's built-in security features for every ATRIUM repository.
# Review, then run in PowerShell with the gh CLI logged in as an account that
# administers these repositories. Nothing here is run for you. Every call is
# reversible from the repository's Settings > Code security page.

$all = 'xingxerx/LIMEN', 'ATRIUMRnD/DUCTEI', 'xingxerx/QALLOW', 'ATRIUMRnD/VEYN', 'ATRIUMRnD/ATRIUM',
       'xingxerx/brain', 'xingxerx/QFLTL', 'xingxerx/player-harness', 'xingxerx/jspace-lab', 'xingxerx/oneiro'

# Public repositories: secret scanning and push protection are free there.
$public = 'xingxerx/LIMEN', 'ATRIUMRnD/DUCTEI', 'xingxerx/QALLOW', 'ATRIUMRnD/ATRIUM', 'xingxerx/jspace-lab'

# 1. Dependabot alerts and security updates (free on public and private repositories).
foreach ($r in $all) {
    gh api -X PUT "repos/$r/vulnerability-alerts"
    gh api -X PUT "repos/$r/automated-security-fixes"
}

# 2. Secret scanning and push protection on the public repositories.
foreach ($r in $public) {
    gh api -X PATCH "repos/$r" `
        -f 'security_and_analysis[secret_scanning][status]=enabled' `
        -f 'security_and_analysis[secret_scanning_push_protection][status]=enabled'
}

# 3. Private vulnerability reporting, so the SECURITY.md link works for outside reporters.
foreach ($r in $public) {
    gh api -X PUT "repos/$r/private-vulnerability-reporting"
}

# 4. OPTIONAL, only after setup-signing.ps1 and commits show as Verified on GitHub:
#    require signed commits on the protected branches. GitHub's own web merges are
#    signed by GitHub, so PR merges keep working.
# gh api -X POST "repos/xingxerx/LIMEN/branches/main/protection/required_signatures"
# gh api -X POST "repos/ATRIUMRnD/DUCTEI/branches/main/protection/required_signatures"
# gh api -X POST "repos/ATRIUMRnD/ATRIUM/branches/master/protection/required_signatures"

# Check what is on now:
foreach ($r in $all) {
    $info = gh api "repos/$r" | ConvertFrom-Json
    '{0,-26} secret_scanning={1} push_protection={2}' -f $r, $info.security_and_analysis.secret_scanning.status, $info.security_and_analysis.secret_scanning_push_protection.status
}