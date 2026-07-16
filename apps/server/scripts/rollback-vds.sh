#!/usr/bin/env sh
set -eu

state_file="${1:-}"
test -r "$state_file" || { echo "usage: $0 /opt/focusnook/releases/rollback-image-TIMESTAMP.txt" >&2; exit 2; }
test "${2:-}" = "--apply" || { echo "add --apply after reviewing the selected image" >&2; exit 2; }

script_dir="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
server_dir="$(dirname "$script_dir")"
env_file="${FOCUSNOOK_ENV_FILE:-/opt/focusnook/.env}"
compose_file="${FOCUSNOOK_COMPOSE_FILE:-${server_dir}/compose.vds-nginx.yml}"
project="${FOCUSNOOK_COMPOSE_PROJECT:-focusnook}"
base_url="${FOCUSNOOK_BASE_URL:-https://focus.proanima.net}"
nginx_state_file="${FOCUSNOOK_NGINX_ROLLBACK_FILE:-}"
nginx_target="${FOCUSNOOK_NGINX_TARGET:-/etc/nginx/sites-available/focusnook.conf}"
image="$(sed -n '1p' "$state_file")"
expect_legal=0
if grep -q '^FOCUSNOOK_LEGAL_NAME=.' "$env_file"; then
  expect_legal=1
fi
test -n "$image" || { echo "rollback state is empty" >&2; exit 1; }

docker image inspect "$image" >/dev/null
docker tag "$image" "${project}-server:latest"
docker compose -p "$project" --env-file "$env_file" -f "$compose_file" up -d --no-deps --no-build --force-recreate server
if [ -n "$nginx_state_file" ]; then
  test -r "$nginx_state_file" || { echo "nginx rollback file is not readable" >&2; exit 1; }
  cp "$nginx_state_file" "$nginx_target"
  nginx -t
  nginx -s reload
fi
FOCUSNOOK_EXPECT_LEGAL="$expect_legal" sh "$script_dir/smoke-test.sh" "$base_url"
echo "Rollback completed with image ${image}"
