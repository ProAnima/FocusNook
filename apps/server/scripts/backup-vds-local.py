#!/usr/bin/env python3
"""Create a FocusNook PostgreSQL dump and keep the result only locally."""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
import secrets
import shlex
from pathlib import Path

import paramiko


def parse_access_file(path: Path) -> tuple[str, str, str]:
    for line in reversed(path.read_text(encoding="utf-8").splitlines()):
        fields = shlex.split(line.strip())
        if len(fields) == 3 and not line.lstrip().startswith("#"):
            return fields[0], fields[1], fields[2]
    raise ValueError("access file must contain a 'host user password' line")


def run(ssh: paramiko.SSHClient, command: str) -> str:
    _, stdout, stderr = ssh.exec_command(command, timeout=120)
    output = stdout.read().decode("utf-8", errors="replace")
    error = stderr.read().decode("utf-8", errors="replace")
    status = stdout.channel.recv_exit_status()
    if status != 0:
        raise RuntimeError(error.strip() or f"remote command failed with {status}")
    return output.strip()


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--access-file", required=True, type=Path)
    parser.add_argument(
        "--destination",
        type=Path,
        default=Path.home() / "Documents" / "FocusNook-backups",
    )
    parser.add_argument("--container", default="focusnook-postgres-1")
    args = parser.parse_args()

    host, user, password = parse_access_file(args.access_file)
    timestamp = dt.datetime.now(dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    filename = f"focusnook-{timestamp}.dump"
    destination = args.destination / filename
    destination.parent.mkdir(parents=True, exist_ok=True)
    remote_path = f"/tmp/focusnook-local-backup-{secrets.token_hex(8)}.dump"
    quoted_container = shlex.quote(args.container)
    quoted_remote = shlex.quote(remote_path)

    ssh = paramiko.SSHClient()
    ssh.load_system_host_keys()
    ssh.set_missing_host_key_policy(paramiko.WarningPolicy())
    ssh.connect(host, username=user, password=password, timeout=15)
    try:
        labels = run(
            ssh,
            "docker inspect --format "
            + shlex.quote(
                '{{ index .Config.Labels "com.docker.compose.project" }}|'
                '{{ index .Config.Labels "com.docker.compose.service" }}'
            )
            + f" {quoted_container}",
        )
        if labels != "focusnook|postgres":
            raise RuntimeError("refusing to back up a non-FocusNook postgres container")

        run(
            ssh,
            "umask 077; docker exec "
            f"{quoted_container} sh -ec "
            + shlex.quote('pg_dump -U "$POSTGRES_USER" -d "$POSTGRES_DB" -Fc')
            + f" > {quoted_remote}",
        )
        run(
            ssh,
            f"test -s {quoted_remote}; docker exec -i {quoted_container} "
            f"pg_restore --list < {quoted_remote} >/dev/null",
        )
        remote_hash = run(ssh, f"sha256sum {quoted_remote}").split()[0]
        with ssh.open_sftp() as sftp:
            sftp.get(remote_path, str(destination))

        local_hash = sha256(destination)
        if local_hash != remote_hash:
            destination.unlink(missing_ok=True)
            raise RuntimeError("downloaded backup SHA-256 does not match the VDS")

        manifest = {
            "created_utc": dt.datetime.now(dt.timezone.utc).isoformat(),
            "file": destination.name,
            "size_bytes": destination.stat().st_size,
            "sha256": local_hash,
            "remote_retention": "temporary file removed after verified download",
        }
        destination.with_suffix(".json").write_text(
            json.dumps(manifest, indent=2) + "\n", encoding="utf-8"
        )
        print(f"BACKUP={destination}")
        print(f"SHA256={local_hash}")
    finally:
        try:
            run(ssh, f"rm -f -- {quoted_remote}")
        finally:
            ssh.close()


if __name__ == "__main__":
    main()
