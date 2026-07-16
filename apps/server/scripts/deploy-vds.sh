#!/usr/bin/env sh
set -eu
umask 077

if [ "${1:-}" != "--apply" ]; then
  echo "Dry stop: run $0 --apply only after reviewing access, backup target, and legal variables" >&2
  exit 2
fi

script_dir="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
server_dir="$(dirname "$script_dir")"
env_file="${FOCUSNOOK_ENV_FILE:-/opt/focusnook/.env}"
compose_file="${FOCUSNOOK_COMPOSE_FILE:-${server_dir}/compose.vds-nginx.yml}"
project="${FOCUSNOOK_COMPOSE_PROJECT:-focusnook}"
base_url="${FOCUSNOOK_BASE_URL:-https://focus.proanima.net}"
nginx_source="${FOCUSNOOK_NGINX_SOURCE:-${server_dir}/nginx/focusnook.conf}"
nginx_target="${FOCUSNOOK_NGINX_TARGET:-/etc/nginx/sites-available/focusnook.conf}"
local_backup_sha="${FOCUSNOOK_LOCAL_BACKUP_SHA256:-}"
test "${#local_backup_sha}" -eq 64 || {
  echo "set FOCUSNOOK_LOCAL_BACKUP_SHA256 from a verified local backup" >&2
  exit 2
}
case "$local_backup_sha" in
  *[!0-9a-fA-F]*)
    echo "FOCUSNOOK_LOCAL_BACKUP_SHA256 must be a SHA-256 digest" >&2
    exit 2
    ;;
esac
expect_legal=0
if grep -q '^FOCUSNOOK_LEGAL_NAME=.' "$env_file"; then
  expect_legal=1
fi

compose() {
  docker compose -p "$project" --env-file "$env_file" -f "$compose_file" "$@"
}

running_container="$(compose ps -q server)"
test -n "$running_container" || { echo "running server container not found" >&2; exit 1; }
previous_image="$(docker inspect --format '{{.Image}}' "$running_container")"
test -n "$previous_image" || { echo "current server image not found" >&2; exit 1; }
nginx_backup="$(mktemp /tmp/focusnook-nginx.XXXXXX)"
cp "$nginx_target" "$nginx_backup"
image_rollback_needed=0
nginx_rollback_needed=0

rollback_on_exit() {
  status=$?
  trap - EXIT INT TERM
  if [ "$image_rollback_needed" -eq 1 ]; then
    echo "Deploy failed; restoring previous server image" >&2
    docker tag "$previous_image" "${project}-server:latest" || true
    current_container="$(compose ps -q server 2>/dev/null || true)"
    current_image="$(docker inspect --format '{{.Image}}' "$current_container" 2>/dev/null || true)"
    if [ "$current_image" != "$previous_image" ]; then
      compose up -d --no-deps --no-build --force-recreate server || true
    fi
  fi
  if [ "$nginx_rollback_needed" -eq 1 ]; then
    echo "Restoring previous nginx configuration" >&2
    cp "$nginx_backup" "$nginx_target" || true
    if nginx -t; then
      nginx -s reload || true
    fi
  fi
  rm -f "$nginx_backup"
  exit "$status"
}
trap rollback_on_exit EXIT INT TERM

FOCUSNOOK_ENV_FILE="$env_file" FOCUSNOOK_COMPOSE_FILE="$compose_file" \
  FOCUSNOOK_COMPOSE_PROJECT="$project" sh "$script_dir/predeploy-check.sh"
image_rollback_needed=1
echo "Verified local backup acknowledged: ${local_backup_sha}"

nginx_rollback_needed=1
cp "$nginx_source" "$nginx_target"
if ! nginx -t; then
  echo "candidate nginx validation failed" >&2
  exit 1
fi

compose up -d --no-deps server
nginx -s reload

if FOCUSNOOK_EXPECT_LEGAL="$expect_legal" sh "$script_dir/smoke-test.sh" "$base_url"; then
  current_container="$(compose ps -q server)"
  current_image="$(docker inspect --format '{{.Image}}' "$current_container")"
  image_rollback_needed=0
  nginx_rollback_needed=0
  trap - EXIT INT TERM
  rm -f "$nginx_backup"
  if [ "$previous_image" != "$current_image" ]; then
    docker image rm "$previous_image" >/dev/null || {
      echo "Warning: previous FocusNook image is still referenced and was not removed" >&2
    }
  fi
  echo "Deploy completed with local backup ${local_backup_sha}"
  exit 0
fi

echo "Smoke test failed" >&2
exit 1
