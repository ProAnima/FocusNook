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

for name in FOCUSNOOK_LEGAL_NAME FOCUSNOOK_SUPPORT_EMAIL FOCUSNOOK_LEGAL_TAX_ID FOCUSNOOK_LEGAL_ADDRESS; do
  value="$(sed -n "s/^${name}=//p" "$env_file" | tail -n 1)"
  if [ -n "$value" ]; then
    case "$value" in
      change-me*|replace-with*)
        echo "placeholder value is forbidden for ${name}" >&2
        exit 1
        ;;
    esac
  fi
done
operator_name="$(sed -n 's/^FOCUSNOOK_LEGAL_NAME=//p' "$env_file" | tail -n 1)"
support_email="$(sed -n 's/^FOCUSNOOK_SUPPORT_EMAIL=//p' "$env_file" | tail -n 1)"
tax_id="$(sed -n 's/^FOCUSNOOK_LEGAL_TAX_ID=//p' "$env_file" | tail -n 1)"
legal_address="$(sed -n 's/^FOCUSNOOK_LEGAL_ADDRESS=//p' "$env_file" | tail -n 1)"
test -z "$operator_name" && test -z "$support_email" || {
  test -n "$operator_name" && test -n "$support_email"
} || {
  echo "public registration requires FOCUSNOOK_LEGAL_NAME and FOCUSNOOK_SUPPORT_EMAIL" >&2
  exit 1
}
test -n "$operator_name" || {
  test -z "$tax_id" && test -z "$legal_address"
} || {
  echo "tax id and address require public registration to be enabled" >&2
  exit 1
}

docker compose -p "$project" --env-file "$env_file" -f "$compose_file" config --quiet
docker compose -p "$project" --env-file "$env_file" -f "$compose_file" build server
nginx -t
echo "Predeploy checks passed"
