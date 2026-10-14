#!/usr/bin/env sh
set -eu

base_url="${1:-https://focus.proanima.net}"
base_url="${base_url%/}"
expect_legal="${FOCUSNOOK_EXPECT_LEGAL:-0}"

check_json() {
  path="$1"
  body="$(curl --fail --silent --show-error --max-time 15 "${base_url}${path}")"
  printf '%s' "$body" | grep -q '"ok":true'
  echo "OK ${path}"
}

check_page() {
  path="$1"
  marker="$2"
  body="$(curl --fail --silent --show-error --max-time 15 "${base_url}${path}")"
  printf '%s' "$body" | grep -q "$marker"
  if printf '%s' "$body" | grep -qi 'replace-with\|change-me'; then
    echo "placeholder found on ${path}" >&2
    exit 1
  fi
  echo "OK ${path}"
}

check_code() {
  path="$1"
  expected="$2"
  code="$(curl --silent --output /dev/null --write-out '%{http_code}' --max-time 15 "${base_url}${path}")"
  test "$code" = "$expected" || {
    echo "expected ${path} to return ${expected}, got ${code}" >&2
    exit 1
  }
  echo "OK ${path} ${code}"
}

check_json /healthz
check_json /readyz
check_code /v1/admin/stats 401
check_code /v1/sync/events 401
if [ "$expect_legal" -eq 1 ]; then
  check_page /privacy "Политика конфиденциальности FocusNook"
  check_page /terms "Пользовательское соглашение FocusNook"
  headers="$(curl --fail --silent --show-error --head --max-time 15 "${base_url}/privacy")"
  printf '%s' "$headers" | grep -qi '^strict-transport-security:'
  printf '%s' "$headers" | grep -qi '^x-content-type-options:.*nosniff'
else
  for path in privacy terms; do
    code="$(curl --silent --output /dev/null --write-out '%{http_code}' --max-time 15 "${base_url}/${path}")"
    test "$code" = "404" || { echo "expected /${path} to stay disabled, got ${code}" >&2; exit 1; }
    echo "DISABLED /${path}"
  done
fi
http_url="http://${base_url#https://}"
redirect="$(curl --silent --output /dev/null --write-out '%{http_code}|%{redirect_url}' --max-time 15 "$http_url")"
case "$redirect" in
  301\|https://*) echo "OK HTTP to HTTPS redirect" ;;
  *) echo "invalid HTTP redirect: ${redirect}" >&2; exit 1 ;;
esac
echo "Smoke test passed for ${base_url}"
