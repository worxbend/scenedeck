#!/usr/bin/env python3
"""Enforce the single approved basic-toml maintenance-advisory exception."""
import argparse
import sys
import tomllib
from datetime import datetime, timezone
from pathlib import Path

EXPIRES = datetime(2026, 11, 20, tzinfo=timezone.utc)
REGISTRY = "registry+https://github.com/rust-lang/crates.io-index"
# These checksums pin the published manifests, including both proc-macro declarations.
REVIEWED = {
    "basic-toml": ("0.1.10", "ba62675e8242a4c4e806d12f11d136e626e6c8361d6b829310732241652a178a"),
    "i18n-config": ("0.4.8", "3e06b90c8a0d252e203c94344b21e35a30f3a3a85dc7db5af8f8df9f3e0c63ef"),
    "i18n-embed": ("0.16.0", "a217bbb075dcaefb292efa78897fc0678245ca67f265d12c351e42268fcb0305"),
    "i18n-embed-fl": ("0.10.1", "602e6bd30c3db2749e13e38b363a3d98d9d41de1d8de7a79c31bb69e45b47cda"),
    "i18n-embed-impl": ("0.8.4", "0f2cc0e0523d1fe6fc2c6f66e5038624ea8091b3e7748b5e8e0c84b1698db6c2"),
}
PARENTS = {
    "basic-toml": ["i18n-config"],
    "i18n-config": ["i18n-embed-fl", "i18n-embed-impl"],
    "i18n-embed": ["i18n-embed-fl", "scenedeck"],
    "i18n-embed-fl": ["scenedeck"],
    "i18n-embed-impl": ["i18n-embed"],
}


def validate_package_identities(packages):
    for name in ["scenedeck", *REVIEWED]:
        matches = [package for package in packages if package.get("name") == name]
        if len(matches) != 1:
            raise ValueError(f"locked graph requires exactly one {name}")
        package = matches[0]
        if name == "scenedeck":
            if "source" in package:
                raise ValueError("locked graph root must remain the local scenedeck package")
        elif (package.get("version"), package.get("checksum")) != REVIEWED[name] or package.get("source") != REGISTRY:
            raise ValueError(f"locked graph identity changed for {name}; re-review or remove the exception")


def validate_graph(lock):
    packages = lock.get("package", [])
    validate_package_identities(packages)
    for name, parents in PARENTS.items():
        version = REVIEWED[name][0]
        references = {name, f"{name} {version}", f"{name} {version} ({REGISTRY})"}
        incoming = []
        for package in packages:
            for dependency in package.get("dependencies", []):
                if dependency.split()[0] == name:
                    if dependency not in references:
                        raise ValueError(f"locked graph reference changed for {name}: {dependency}")
                    incoming.append(package["name"])
        if sorted(incoming) != parents:
            raise ValueError(f"locked graph proc-macro paths changed for {name}: {incoming}")


def validate(deny, lock, exception, now):
    if now.utcoffset() is None:
        raise ValueError("the policy clock must be timezone-aware (UTC)")
    if now >= EXPIRES:
        raise ValueError("RUSTSEC-2026-0374 exception expired at 2026-11-20 00:00:00 UTC")
    fields = {"id", "owner", "reason", "expires"}
    if set(exception) != fields or any(
        not isinstance(exception[field], str) or not exception[field].strip() for field in fields
    ):
        raise ValueError("approval must contain a nonempty id, owner, reason and expiry")
    if exception["id"] != "RUSTSEC-2026-0374" or exception["expires"] != "2026-11-20":
        raise ValueError("approval is only for RUSTSEC-2026-0374 until 2026-11-20 UTC")
    reason = (f"{exception['reason']} Owner: {exception['owner']}. "
              f"Expires: {exception['expires']} 00:00:00 UTC.")
    if deny.get("advisories") != {
        "version": 2,
        "yanked": "deny",
        "unused-ignored-advisory": "deny",
        "ignore": [{"id": exception["id"], "reason": reason}],
    }:
        raise ValueError("deny.toml advisories must match the single approved exception exactly")
    validate_graph(lock)


def main():
    argparse.ArgumentParser(description=__doc__).parse_args()
    root = Path(__file__).resolve().parents[1]
    now = datetime.now(timezone.utc)
    print(f"checked_at={now.isoformat()}")
    try:
        deny = tomllib.loads((root / "deny.toml").read_text(encoding="utf-8"))
        lock = tomllib.loads((root / "Cargo.lock").read_text(encoding="utf-8"))
        exception = tomllib.loads((root / "tools/advisory-exception.toml").read_text(encoding="utf-8"))
        validate(deny, lock, exception, now)
    except (OSError, ValueError) as error:
        print(f"advisory exception check failed: {error}", file=sys.stderr)
        return 1
    print("RUSTSEC-2026-0374 exception valid until 2026-11-20 00:00:00 UTC")
    return 0


if __name__ == "__main__":
    sys.exit(main())
