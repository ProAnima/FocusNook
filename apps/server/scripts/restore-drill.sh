#!/usr/bin/env sh
set -eu

backup_file="${1:-}"
test -r "$backup_file" || { echo "usage: $0 BACKUP.dump --apply" >&2; exit 2; }
test "${2:-}" = "--apply" || { echo "add --apply to create and remove a temporary restore database" >&2; exit 2; }

script_dir="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
server_dir="$(dirname "$script_dir")"
env_file="${FOCUSNOOK_ENV_FILE:-/opt/focusnook/.env}"
compose_file="${FOCUSNOOK_COMPOSE_FILE:-${server_dir}/compose.vds-nginx.yml}"
project="${FOCUSNOOK_COMPOSE_PROJECT:-focusnook}"
drill_db="focusnook_restore_drill_$(date -u +%Y%m%d%H%M%S)"

compose() {
  docker compose -p "$project" --env-file "$env_file" -f "$compose_file" "$@"
}

db_user="$(compose exec -T postgres printenv POSTGRES_USER | tr -d '\r')"

cleanup() {
  compose exec -T postgres dropdb -U "$db_user" --if-exists "$drill_db" >/dev/null 2>&1 || true
}
trap cleanup EXIT INT TERM

compose exec -T postgres createdb -U "$db_user" "$drill_db"
compose exec -T postgres pg_restore -U "$db_user" -d "$drill_db" --no-owner --no-privileges < "$backup_file"
tables="$(compose exec -T postgres psql -U "$db_user" -d "$drill_db" -Atc "select count(*) from information_schema.tables where table_schema='public'")"
test "$tables" -ge 8 || { echo "restored database has too few tables: ${tables}" >&2; exit 1; }
echo "Restore drill passed with ${tables} public tables"
