#!/usr/bin/env sh
set -eu

base_url="${1:-https://focus.proanima.net}"
base_url="${base_url%/}"

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

check_json /healthz
check_json /readyz
check_page /privacy "Политика конфиденциальности FocusNook"
check_page /terms "Пользовательское соглашение FocusNook"

headers="$(curl --fail --silent --show-error --head --max-time 15 "${base_url}/privacy")"
printf '%s' "$headers" | grep -qi '^strict-transport-security:'
printf '%s' "$headers" | grep -qi '^x-content-type-options:.*nosniff'
echo "Smoke test passed for ${base_url}"
