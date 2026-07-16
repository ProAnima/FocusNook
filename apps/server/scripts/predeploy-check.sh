#!/usr/bin/env sh
set -eu

script_dir="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
server_dir="$(dirname "$script_dir")"
env_file="${FOCUSNOOK_ENV_FILE:-/opt/focusnook/.env}"
compose_file="${FOCUSNOOK_COMPOSE_FILE:-${server_dir}/compose.vds-nginx.yml}"
project="${FOCUSNOOK_COMPOSE_PROJECT:-focusnook}"

test -r "$env_file" || { echo "env file is not readable: ${env_file}" >&2; exit 1; }
required_names="POSTGRES_DB POSTGRES_USER POSTGRES_PASSWORD FOCUSNOOK_DATABASE_URL FOCUSNOOK_ADMIN_TOKEN FOCUSNOOK_WEB_SECONDARY_PASSWORD FOCUSNOOK_TOKEN_PEPPER_B64 FOCUSNOOK_ENCRYPTION_KEY_B64 FOCUSNOOK_PUBLIC_BASE_URL"
for name in $required_names; do
  value="$(sed -n "s/^${name}=//p" "$env_file" | tail -n 1)"
  test -n "$value" || { echo "missing ${name} in ${env_file}" >&2; exit 1; }
  case "$value" in
    change-me*|replace-with*)
      echo "placeholder value is forbidden for ${name}" >&2
      exit 1
      ;;
  esac
done

legal_count=0
for name in FOCUSNOOK_LEGAL_NAME FOCUSNOOK_LEGAL_TAX_ID FOCUSNOOK_LEGAL_ADDRESS FOCUSNOOK_SUPPORT_EMAIL; do
  value="$(sed -n "s/^${name}=//p" "$env_file" | tail -n 1)"
  if [ -n "$value" ]; then
    legal_count=$((legal_count + 1))
    case "$value" in
      change-me*|replace-with*)
        echo "placeholder value is forbidden for ${name}" >&2
        exit 1
        ;;
    esac
  fi
done
test "$legal_count" -eq 0 || test "$legal_count" -eq 4 || {
  echo "legal identity must be fully configured or fully omitted" >&2
  exit 1
}

docker compose -p "$project" --env-file "$env_file" -f "$compose_file" config --quiet
docker compose -p "$project" --env-file "$env_file" -f "$compose_file" build server
nginx -t
echo "Predeploy checks passed"
