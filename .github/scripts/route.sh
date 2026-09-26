#!/usr/bin/env bash
# Pick the runner of the Linux and the macOS jobs from the two free pools of the
# organisation: GitHub-hosted runners (2,000 minutes a month, macOS counting 10×) and
# Blacksmith (3,000, macOS counting 20×). Each kind of job has a home pool, which keeps
# its cache warm: Linux on Blacksmith, macOS on GitHub. It moves to the other pool only
# once its home passed THRESHOLD percent of its quota and the other has not.
#
# Usage is estimated from this month's jobs across the organisation's repositories,
# rounded up to the minute per job as both providers bill. Without GH_TOKEN, or with
# PROVIDER set to github or blacksmith, the estimate is skipped.
#
# Writes `linux` and `macos` to $GITHUB_OUTPUT and a summary to $GITHUB_STEP_SUMMARY.
set -euo pipefail

org=${ORG:?ORG must name the organisation}
provider=${PROVIDER:-auto}
threshold=${THRESHOLD:-85}
github_quota=2000
blacksmith_quota=3000
declare -A runners=(
    [github-linux]=ubuntu-latest
    [github-macos]=macos-latest
    [blacksmith-linux]=blacksmith-4vcpu-ubuntu-2404-arm
    [blacksmith-macos]=blacksmith-6vcpu-macos-latest
)
output=${GITHUB_OUTPUT:-/dev/stdout}
summary=${GITHUB_STEP_SUMMARY:-/dev/stderr}

# Print "<labels> <billed minutes> <private>" for every finished job of the month.
jobs_this_month() {
    local since repo private run
    since=$(date -u +%Y-%m-01)
    gh api --paginate "orgs/$org/repos?per_page=100" --jq '.[] | "\(.name) \(.private)"' |
        while read -r repo private; do
            gh api --paginate "repos/$org/$repo/actions/runs?created=>=$since&status=completed&per_page=100" \
                --jq '.workflow_runs[].id' |
                while read -r run; do
                    gh api --paginate "repos/$org/$repo/actions/runs/$run/jobs?filter=all&per_page=100" \
                        --jq '.jobs[] | select(.started_at != null and .completed_at != null)
                              | "\(.labels | join(",")) \((((.completed_at | fromdate) - (.started_at | fromdate)) / 60) | ceil) '"$private"'"'
                done
        done
}

# Print "<github units> <blacksmith units>", in each provider's free-minute units.
usage() {
    jobs_this_month | awk '
        {
            labels = $1; minutes = $2; private = $3
            if (match(labels, /blacksmith-[0-9]+vcpu/)) {
                vcpu = substr(labels, RSTART + 11, RLENGTH - 15) + 0
                if (labels ~ /macos/) blacksmith += minutes * 20 * vcpu / 6
                else if (labels ~ /-arm/) blacksmith += minutes * 0.625 * vcpu / 2
                else blacksmith += minutes * vcpu / 2
            } else if (private == "true") {
                if (labels ~ /macos/) github += minutes * 10
                else if (labels ~ /windows/) github += minutes * 2
                else if (labels ~ /ubuntu|linux/) github += minutes
            }
        }
        END { printf "%d %d\n", github, blacksmith }'
}

linux=blacksmith macos=github
case "$provider" in
    github | blacksmith) linux=$provider macos=$provider reason="forced to $provider" ;;
    auto)
        if [[ -z "${GH_TOKEN:-}" ]]; then
            reason="no usage token: home pools"
        else
            read -r github blacksmith < <(usage)
            github_pct=$((github * 100 / github_quota))
            blacksmith_pct=$((blacksmith * 100 / blacksmith_quota))
            reason="GitHub ${github}/${github_quota} (${github_pct}%), Blacksmith ${blacksmith}/${blacksmith_quota} (${blacksmith_pct}%)"
            if ((blacksmith_pct >= threshold && github_pct < threshold)); then linux=github; fi
            if ((github_pct >= threshold && blacksmith_pct < threshold)); then macos=blacksmith; fi
        fi
        ;;
    *) echo "unknown provider $provider: auto, github or blacksmith" >&2; exit 2 ;;
esac

{
    echo "linux=${runners[$linux-linux]}"
    echo "macos=${runners[$macos-macos]}"
} >>"$output"
echo "Runners: Linux on ${runners[$linux-linux]}, macOS on ${runners[$macos-macos]} ($reason)." >>"$summary"
