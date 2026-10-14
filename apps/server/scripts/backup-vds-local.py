#!/usr/bin/env python3
"""Create a FocusNook PostgreSQL dump and keep the result only locally."""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
import secrets
import shlex
import socket
from pathlib import Path

import paramiko


def parse_access_file(path: Path) -> tuple[str, str, str]:
    for line in reversed(path.read_text(encoding="utf-8").splitlines()):
        fields = shlex.split(line.strip())
        if len(fields) == 3 and not line.lstrip().startswith("#"):
            return fields[0], fields[1], fields[2]
    raise ValueError("access file must contain a 'host user password' line")


def connect(ssh: paramiko.SSHClient, access_file: Path | None, ssh_host: str | None) -> None:
    ssh.load_system_host_keys()
    ssh.set_missing_host_key_policy(paramiko.RejectPolicy())
    if access_file is not None:
        host, user, password = parse_access_file(access_file)
        ssh.connect(host, username=user, password=password, timeout=15)
        return

    if not ssh_host:
        raise ValueError("either --access-file or --ssh-host is required")
    config_path = Path.home() / ".ssh" / "config"
    lookup: dict[str, object] = {}
    if config_path.is_file():
        with config_path.open(encoding="utf-8") as source:
            lookup = paramiko.SSHConfig().from_file(source).lookup(ssh_host)
    hostname = str(lookup.get("hostname", ssh_host))
    username = lookup.get("user")
    identity_files = lookup.get("identityfile")
    key_filename = identity_files if isinstance(identity_files, list) else None
    ssh.connect(
        hostname,
        username=str(username) if username else None,
        key_filename=key_filename,
        look_for_keys=True,
        allow_agent=True,
        timeout=15,
    )


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
    access = parser.add_mutually_exclusive_group(required=True)
    access.add_argument("--access-file", type=Path)
    access.add_argument(
        "--ssh-host",
        help="OpenSSH host alias or hostname; uses ~/.ssh/config, keys, and known_hosts",
    )
    parser.add_argument(
        "--destination",
        type=Path,
        default=Path.home() / "Documents" / "FocusNook-backups",
    )
    parser.add_argument("--container", default="focusnook-postgres-1")
    args = parser.parse_args()

    timestamp = dt.datetime.now(dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    filename = f"focusnook-{timestamp}.dump"
    destination = args.destination / filename
    destination.parent.mkdir(parents=True, exist_ok=True)
    remote_path = f"/tmp/focusnook-local-backup-{secrets.token_hex(8)}.dump"
    quoted_container = shlex.quote(args.container)
    quoted_remote = shlex.quote(remote_path)

    ssh = paramiko.SSHClient()
    try:
        connect(ssh, args.access_file, args.ssh_host)
    except (OSError, paramiko.SSHException, socket.error) as err:
        raise RuntimeError(f"SSH connection failed: {err}") from err
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
