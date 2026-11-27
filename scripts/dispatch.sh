#!/usr/bin/env bash
# Starts a workflow on the Windows PC runner and waits for that run. Usage: dispatch.sh <workflow.yml> [-f key=value ...]
set -euo pipefail
export GH_TOKEN="${GH_TOKEN:-$(gh auth token --user andresmarpz)}"
repo=andresmarpz/greyscreen
workflow="$1"; shift
latest() { gh run list -R "$repo" --workflow "$workflow" --event workflow_dispatch --limit 1 --json databaseId --jq '.[0].databaseId // 0'; }
before=$(latest)
gh workflow run "$workflow" -R "$repo" --ref main "$@"
for _ in $(seq 60); do
  run=$(latest)
  [ "$run" != "$before" ] && exec gh run watch "$run" -R "$repo" --exit-status
  sleep 2
done
echo "the $workflow run did not appear" >&2
exit 1
