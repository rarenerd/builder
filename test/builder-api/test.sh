#!/bin/bash

# This script exists to run our integration tests.

set -euo pipefail

# make mocha happy by running from the directory it expects
cd "$(dirname "${BASH_SOURCE[0]}")"

clean_test_artifacts() {
  if ! command -v op >/dev/null 2>&1; then
    hab pkg install -b habitat/builder-op
  fi

  local sql origins
  origins=( neurosis xmen )

  # clean origins
  local origins origin_id_tables origin_tables
  origin_id_tables=( origin_members origin_channels origin_invitations origin_packages origin_projects origin_public_keys origin_secret_keys )
  origin_tables=( origin_integrations origin_project_integrations )
  sql=

  for origin in "${origins[@]}"; do
    sql+="SET SEARCH_PATH TO shard_$(op shard "$origin");"
    sql+="DELETE FROM origin_channel_packages WHERE channel_id IN (SELECT id FROM origin_channels WHERE origin_id=(SELECT id FROM origins WHERE name='$origin'));"

    for table in "${origin_id_tables[@]}"; do
      sql+="DELETE FROM $table WHERE origin_id=(SELECT id FROM origins WHERE name='$origin');"
    done

    for table in "${origin_tables[@]}"; do
      sql+="DELETE FROM $table WHERE origin='$origin';"
    done

    sql+="DELETE FROM origins WHERE name='$origin';"
  done

  psql builder_originsrv -q -c "$sql"

  # clean users
  local users account_tables
  users=( bobo mystique )
  account_tables=( account_invitations account_origins )
  sql=

  for user in "${users[@]}"; do
    sql+="SET SEARCH_PATH TO shard_$(op shard "$user");"

    for table in "${account_tables[@]}"; do
      sql+="DELETE FROM $table WHERE account_id=(SELECT id FROM accounts WHERE name='$user');"
    done

    sql+="DELETE FROM accounts WHERE name='$user';"
  done

  psql builder_sessionsrv -q -c "$sql"

  # clean jobs
  sql=

  for origin in "${origins[@]}"; do
    sql+="SET SEARCH_PATH TO shard_0;"
    sql+="DELETE FROM busy_workers WHERE job_id IN (SELECT id FROM jobs WHERE project_name LIKE '$origin%');"
    sql+="DELETE FROM graph_packages WHERE ident LIKE '$origin%';"
    sql+="DELETE FROM group_projects WHERE project_name LIKE '$origin%';"
    sql+="DELETE FROM groups WHERE project_name LIKE '$origin%';"
    sql+="DELETE FROM jobs WHERE project_name LIKE '$origin%';"
  done

  psql builder_jobsrv -q -c "$sql"
}

if [ -n "${TRAVIS:-}" ]; then
  pushd "$(git rev-parse --show-toplevel)"
  cp /tmp/builder-github-app.pem .secrets/
  cp .secrets/habitat-env{.sample,}
  support/linux/provision.sh
  set +u; eval "$(direnv hook bash)"; set -u
  direnv allow

  # Do what `hab setup` would do
  hab origin key generate "$(id -nu)"
  mkdir -p "$HOME/.hab/etc"
  cat <<EOT > "$HOME/.hab/etc/cli.toml"
origin = "$(id -nu)"
EOT
  mkdir -p "$HOME/.hab/cache/analytics"
  touch "$HOME/.hab/cache/analytics/OPTED_OUT"
  # end hab setup

  cat <<EOT >> .studiorc
set -x
set -euo pipefail

HAB_FUNC_TEST=arg-to-sup-run sup-run

until hab sup status; do echo "waiting for hab sup to start"; sleep 1; done

if ! hab sup status; then
  echo "SUPERVISOR FAILED TO START"
  exit 2
fi

start-builder

echo "BUILDING BUILDER"
build-builder > /dev/null
echo "BUILDER BUILT build-builder returned \$?"

hab sup status

hab pkg install -b core/node
cd /src/test/builder-api
npm install mocha
hab pkg binlink core/coreutils -d /usr/bin env

hab sup status
npm run mocha
exit \$?
EOT
  HAB_STUDIO_SUP=false hab studio enter
else
  clean_test_artifacts # start with a clean slate

  if ! command -v npm >/dev/null 2>&1; then
    hab pkg install -b core/node
  fi

  if ! [ -f /usr/bin/env ]; then
    hab pkg binlink core/coreutils -d /usr/bin env
  fi

  if ! [ -d node_modules/mocha ]; then
    npm install mocha
  fi

  if npm run mocha; then
    echo "All tests passed, performing DB cleanup"
    clean_test_artifacts
  else
    mocha_exit_code=$?
    echo "Tests failed; skipping cleanup to facilitate investigation"
  fi

  exit ${mocha_exit_code:-0}
fi