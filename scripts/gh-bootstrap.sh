set -euo pipefail

REPO="${SENTIL_REPO:-sedislab/SENTIL}"

gh repo edit "$REPO" \
  --description "Runtime verification for probabilistic Signal Temporal Logic" \
  --homepage "https://sentil.pages.dev" \
  --enable-issues \
  --enable-discussions \
  --enable-wiki=false \
  --enable-projects=false \
  --delete-branch-on-merge \
  --allow-update-branch

gh api -X PUT "repos/$REPO/topics" \
  -f 'names[]=runtime-verification' \
  -f 'names[]=signal-temporal-logic' \
  -f 'names[]=formal-methods' \
  -f 'names[]=stl' \
  -f 'names[]=prstl' \
  -f 'names[]=rust' \
  -f 'names[]=monitoring' \
  -f 'names[]=synthesis'