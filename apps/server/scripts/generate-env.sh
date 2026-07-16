#!/usr/bin/env sh
set -eu

domain="${1:-sync.example.com}"
postgres_user="${POSTGRES_USER:-focusnook}"
postgres_db="${POSTGRES_DB:-focusnook}"

need_openssl() {
  if ! command -v openssl >/dev/null 2>&1; then
    echo "openssl is required" >&2
    exit 1
  fi
}

secret() {
  openssl rand -base64 32 | tr -d '\n'
}

need_openssl
umask 077

postgres_password="$(openssl rand -base64 32 | tr '+/' '-_' | tr -d '=\n')"
admin_token="fnk_admin_$(openssl rand -base64 32 | tr '+/' '-_' | tr -d '=\n')"
web_secondary_password="fnk_web_$(openssl rand -base64 24 | tr '+/' '-_' | tr -d '=\n')"
token_pepper="$(secret)"
encryption_key="$(secret)"
legal_name="${FOCUSNOOK_LEGAL_NAME:-}"
legal_tax_id="${FOCUSNOOK_LEGAL_TAX_ID:-}"
legal_address="${FOCUSNOOK_LEGAL_ADDRESS:-}"
support_email="${FOCUSNOOK_SUPPORT_EMAIL:-}"

legal_count=0
for value in "$legal_name" "$legal_tax_id" "$legal_address" "$support_email"; do
  if [ -n "$value" ]; then legal_count=$((legal_count + 1)); fi
done
if [ "$legal_count" -ne 0 ] && [ "$legal_count" -ne 4 ]; then
  echo "legal identity must be fully configured or fully omitted" >&2
  exit 1
fi

cat > .env <<EOF
POSTGRES_DB=${postgres_db}
POSTGRES_USER=${postgres_user}
POSTGRES_PASSWORD=${postgres_password}

FOCUSNOOK_DATABASE_URL=postgres://${postgres_user}:${postgres_password}@postgres:5432/${postgres_db}
FOCUSNOOK_BIND_ADDR=0.0.0.0:8080
FOCUSNOOK_PUBLIC_BASE_URL=https://${domain}
FOCUSNOOK_DOMAIN=${domain}

FOCUSNOOK_ADMIN_TOKEN=${admin_token}
FOCUSNOOK_WEB_SECONDARY_PASSWORD=${web_secondary_password}
FOCUSNOOK_TOKEN_PEPPER_B64=${token_pepper}
FOCUSNOOK_ENCRYPTION_KEY_B64=${encryption_key}
FOCUSNOOK_LEGAL_NAME=${legal_name}
FOCUSNOOK_LEGAL_TAX_ID=${legal_tax_id}
FOCUSNOOK_LEGAL_ADDRESS=${legal_address}
FOCUSNOOK_SUPPORT_EMAIL=${support_email}

FOCUSNOOK_MAX_OPS_PER_EXCHANGE=500
FOCUSNOOK_MAX_OPERATION_PAYLOAD_BYTES=262144
FOCUSNOOK_MAX_BLOB_BYTES=15728640
RUST_LOG=focusnook_sync_server=info,tower_http=info
EOF

echo ".env generated for ${domain}"
echo "Admin token is in .env as FOCUSNOOK_ADMIN_TOKEN. Keep it private."
echo "Web console secondary password is in .env as FOCUSNOOK_WEB_SECONDARY_PASSWORD."
