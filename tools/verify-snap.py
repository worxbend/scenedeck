#!/usr/bin/env python3
"""Verify Snap identity/checksums, prevent channel downgrades and read back Store bytes."""
import argparse
import hashlib
import json
import re
import subprocess
import time
import urllib.request
from pathlib import Path

MACHINES = {"amd64": 62, "arm64": 183}


def require(condition, message):
    if not condition:
        raise ValueError(message)


def digest(path, algorithm="sha256"):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, algorithm).hexdigest()


def version_tuple(version):
    require(re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", version), "expected numeric stable version")
    return tuple(map(int, version.split(".")))


def inspect(path, version, arch):
    import yaml  # Ubuntu's python3-yaml; no remote runtime dependencies.
    metadata = yaml.safe_load(subprocess.check_output(["unsquashfs", "-cat", str(path), "meta/snap.yaml"]))
    require(metadata["name"] == "scenedeck" and str(metadata["version"]) == version,
            "Snap name/version mismatch")
    require(metadata["architectures"] == [arch], "Snap architecture mismatch")
    require(metadata["confinement"] == "strict" and metadata["base"] == "core24", "Snap confinement/base changed")
    require(metadata["apps"]["scenedeck"]["command"] == "bin/scenedeck", "unexpected Snap launcher")
    binary = subprocess.check_output(["unsquashfs", "-cat", str(path), "bin/scenedeck"])
    require(len(binary) >= 64 and binary[:6] == b"\x7fELF\x02\x01", "Snap binary is not ELF64")
    require(int.from_bytes(binary[18:20], "little") == MACHINES[arch], "Snap ELF architecture mismatch")


def inventory(directory, version, arches, commit, create=False):
    version_tuple(version)
    require(re.fullmatch(r"[0-9a-f]{40}", commit), "invalid commit")
    expected = set()
    for arch in arches:
        name = f"scenedeck_{version}_{arch}.snap"
        path = directory / name
        require(path.is_file() and not path.is_symlink() and path.stat().st_size > 0, "missing or empty Snap")
        inspect(path, version, arch)
        manifest = {"schema": 1, "commit": commit, "name": "scenedeck", "version": version,
                    "arch": arch, "filename": name, "size": path.stat().st_size, "sha256": digest(path)}
        manifest_path = directory / (name + ".manifest.json")
        sums_path = directory / (name + ".SHA256SUMS")
        if create:
            require(not manifest_path.exists() and not sums_path.exists(), "refuse replacing verification data")
            manifest_path.write_text(json.dumps(manifest, indent=2) + "\n")
        else:
            require(json.loads(manifest_path.read_text()) == manifest, "Snap manifest/source mismatch")
        sums = f"{digest(path)}  {name}\n{digest(manifest_path)}  {manifest_path.name}\n"
        if create:
            sums_path.write_text(sums)
        else:
            require(sums_path.read_text() == sums, "Snap checksums mismatch")
        expected.update([name, manifest_path.name, sums_path.name])
    require({p.name for p in directory.iterdir()} == expected, "unexpected or missing Snap artifacts")
    require(all(p.is_file() and not p.is_symlink() for p in directory.iterdir()), "linked Snap artifact")


def store_entries(data, channel):
    require(data.get("name") == "scenedeck", "unexpected Store name")
    entries = {}
    for entry in data["channel-map"]:
        item = entry["channel"]
        if item["track"] == "latest" and item["risk"] == channel:
            arch = item["architecture"]
            require(arch not in entries, "duplicate Store architecture/channel")
            entries[arch] = entry
    return entries


def verify_store(data, channel, version, directory=None):
    entries = store_entries(data, channel)
    for arch in MACHINES:
        entry = entries.get(arch)
        if directory is None:
            if entry:
                require(version_tuple(entry["version"]) <= version_tuple(version), "refuse Snap channel downgrade")
        else:
            require(entry is not None and entry["version"] == version, "Store version is not yet the candidate")
            assert entry is not None
            path = directory / f"scenedeck_{version}_{arch}.snap"
            require(entry["download"]["sha3-384"] == digest(path, "sha3_384"), "Store bytes differ from validated Snap")
            require(entry["download"]["size"] == path.stat().st_size, "Store size mismatch")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=["create", "verify", "store-check", "store-readback"])
    parser.add_argument("--directory", type=Path, required=True)
    parser.add_argument("--version", required=True)
    parser.add_argument("--commit", required=True)
    parser.add_argument("--arch", choices=MACHINES, action="append", required=True)
    parser.add_argument("--channel", choices=["edge", "beta", "candidate", "stable"])
    args = parser.parse_args()
    require(len(args.arch) == len(set(args.arch)), "duplicate architectures")
    inventory(args.directory, args.version, args.arch, args.commit, args.command == "create")
    if args.command.startswith("store-"):
        require(args.channel is not None and set(args.arch) == set(MACHINES), "Store verification requires both architectures/channel")
        attempts = 20 if args.command == "store-readback" else 1
        for attempt in range(attempts):
            try:
                request = urllib.request.Request("https://api.snapcraft.io/v2/snaps/info/scenedeck",
                                                 headers={"Snap-Device-Series": "16"})
                with urllib.request.urlopen(request, timeout=30) as response:
                    data = json.load(response)
                verify_store(data, args.channel, args.version,
                             args.directory if args.command == "store-readback" else None)
                break
            except (ValueError, OSError):
                if attempt + 1 == attempts:
                    raise
                time.sleep(15)
    print(f"Verified {args.command}: SceneDeck {args.version}, {', '.join(args.arch)}")


if __name__ == "__main__":
    main()
