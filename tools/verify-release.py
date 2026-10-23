#!/usr/bin/env python3
"""Create/verify an exact release inventory; extract only flat, regular transfer files."""
import argparse
import configparser
import hashlib
import json
import os
import re
import shutil
import stat
import subprocess
import tarfile
import tempfile
import xml.etree.ElementTree as ET
from pathlib import Path

APP_ID = "io.scenedeck.app"
MACHINES = {"amd64": 62, "arm64": 183}
MANDATORY = ("", ".tar.gz", ".AppImage")
OPTIONAL = {"flatpak": ".flatpak", "zsync": ".AppImage.zsync"}
FLATPAK_ARCHES = {"amd64": "x86_64", "arm64": "aarch64"}


def require(condition, message):
    if not condition:
        raise ValueError(message)


def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def prefix(version, arch):
    require(re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", version), "invalid version")
    require(arch in MACHINES, "unsupported architecture")
    return f"scenedeck-{version}-linux-{arch}"


def elf(data, arch, appimage=False):
    require(len(data) >= 64 and data[:4] == b"\x7fELF" and data[4:6] == b"\x02\x01",
            "expected a nonempty little-endian ELF64 executable")
    require(int.from_bytes(data[18:20], "little") == MACHINES[arch], "ELF architecture mismatch")
    if appimage:
        require(data[8:11] == b"AI\x02", "expected a type-2 AppImage")


def flatpak_identity(ref, metadata, appstream, version, arch):
    """Validate native-import readback; this seam alone does not validate bytes."""
    native_arch = FLATPAK_ARCHES[arch]
    require(ref.strip() == f"app/{APP_ID}/{native_arch}/master", "Flatpak ref identity mismatch")
    try:
        config = configparser.ConfigParser(interpolation=None)
        config.read_string(metadata)
        application = config["Application"]
        require(application["name"] == APP_ID and application["command"] == "scenedeck",
                "Flatpak application identity mismatch")
        runtime = application["runtime"].split("/")
        require(len(runtime) == 3 and runtime[1] == native_arch, "Flatpak runtime architecture mismatch")
        component = ET.fromstring(appstream)
        require(component.tag == "component" and component.findtext("id") == APP_ID,
                "Flatpak AppStream identity mismatch")
        release = component.find("releases/release")
        require(release is not None and release.attrib.get("version") == version,
                "Flatpak AppStream version mismatch")
    except (configparser.Error, KeyError, ET.ParseError) as error:
        raise ValueError("malformed Flatpak identity metadata") from error


def inspect_flatpak(path, version, arch):
    # Native import verifies the actual single-file format/objects, offline.
    # https://docs.flatpak.org/en/latest/single-file-bundles.html
    require(all(shutil.which(tool) for tool in ("flatpak", "ostree")),
            "Flatpak verification requires native flatpak and ostree tools; refusing optional bundle")
    with tempfile.TemporaryDirectory(prefix="scenedeck-flatpak-verify-") as tmp:
        root = Path(tmp)
        env = os.environ.copy()
        for key in ("FLATPAK_USER_DIR", "FLATPAK_SYSTEM_DIR", "FLATPAK_CONFIG_DIR",
                    "XDG_DATA_HOME", "XDG_CONFIG_HOME", "XDG_CACHE_HOME", "XDG_RUNTIME_DIR"):
            location = root / key
            location.mkdir(mode=0o700)
            env[key] = str(location)
        repo = root / "repo"

        def run(*command):
            try:
                return subprocess.run(command, env=env, check=True, capture_output=True,
                                      text=True, timeout=120).stdout
            except (OSError, subprocess.SubprocessError, UnicodeError) as error:
                raise ValueError("native Flatpak bundle inspection failed") from error

        run("ostree", f"--repo={repo}", "init", "--mode=archive-z2")
        run("flatpak", "build-import-bundle", str(repo), str(path.resolve()))
        ref = run("ostree", f"--repo={repo}", "refs").strip()
        require(ref == f"app/{APP_ID}/{FLATPAK_ARCHES[arch]}/master", "Flatpak ref identity mismatch")
        metadata = run("flatpak", "repo", f"--metadata={ref}", str(repo))
        appstream = run("ostree", f"--repo={repo}", "cat", ref,
                        f"/files/share/metainfo/{APP_ID}.metainfo.xml")
        flatpak_identity(ref, metadata, appstream, version, arch)


def inspect_payloads(directory, base, arch, optional, check_modes=True):
    binary = directory / base
    for name in (base, base + ".AppImage"):
        path = directory / name
        if check_modes:
            require(path.stat().st_mode & 0o111, f"not executable: {name}")
        with path.open("rb") as stream:
            elf(stream.read(64), arch, name.endswith(".AppImage"))
    files = {
        f"{base}/usr/bin/scenedeck",
        f"{base}/usr/share/applications/{APP_ID}.desktop",
        f"{base}/usr/share/metainfo/{APP_ID}.metainfo.xml",
        f"{base}/usr/share/icons/hicolor/scalable/apps/{APP_ID}.svg",
        f"{base}/usr/share/icons/hicolor/256x256/apps/{APP_ID}.png",
    }
    directories = {str(parent) for name in files for parent in Path(name).parents if str(parent) != "."}
    with tarfile.open(directory / (base + ".tar.gz"), "r:gz") as archive:
        seen = set()
        found = set()
        for member in archive:
            name = member.name.rstrip("/")
            require(name not in seen, "duplicate tarball member")
            seen.add(name)
            require((member.isdir() and name in directories) or (member.isfile() and name in files),
                    f"unexpected or linked tarball member: {name}")
            if not member.isfile():
                continue
            found.add(name)
            require(member.size > 0, "empty tarball payload")
            stream = archive.extractfile(member)
            assert stream is not None
            with stream:
                if name.endswith("/usr/bin/scenedeck"):
                    require(member.mode & 0o111, "tarball binary is not executable")
                    require(hashlib.file_digest(stream, "sha256").hexdigest() == digest(binary),
                            "tarball and standalone binary differ")
                elif name.endswith(".metainfo.xml"):
                    release = ET.fromstring(stream.read()).find("releases/release")
                    version = base.removeprefix("scenedeck-").removesuffix(f"-linux-{arch}")
                    require(release is not None and release.attrib.get("version") == version,
                            "tarball AppStream version mismatch")
        require(found == files, "tarball missing required payloads")
    if optional["flatpak"]:
        version = base.removeprefix("scenedeck-").removesuffix(f"-linux-{arch}")
        inspect_flatpak(directory / (base + ".flatpak"), version, arch)
    if optional["zsync"]:
        with (directory / (base + ".AppImage.zsync")).open("rb") as stream:
            header = stream.read(8192).split(b"\n\n", 1)[0].decode("ascii")
        filenames = re.findall(r"^Filename: (.+)$", header, re.MULTILINE)
        require(filenames == [base + ".AppImage"], "zsync names a different AppImage")


def manifest(directory, version, arch, commit, optional, create, check_modes=True):
    require(re.fullmatch(r"[0-9a-f]{40}", commit), "invalid source commit")
    base = prefix(version, arch)
    path = directory / (base + ".manifest.json")
    payloads = {base + suffix for suffix in MANDATORY}
    payloads.update(base + suffix for key, suffix in OPTIONAL.items() if optional[key])
    for name in payloads:
        item = directory / name
        require(item.is_file() and not item.is_symlink() and item.stat().st_size > 0,
                f"missing, linked or empty artifact: {name}")
    inspect_payloads(directory, base, arch, optional, check_modes)
    data = {"schema": 1, "version": version, "arch": arch, "commit": commit,
            "optional": optional,
            "assets": [{"name": name, "size": (directory / name).stat().st_size,
                        "sha256": digest(directory / name)} for name in sorted(payloads)]}
    if create:
        require(not path.exists(), "refuse replacing an existing manifest")
        path.write_text(json.dumps(data, indent=2) + "\n")
    else:
        require(json.loads(path.read_text()) == data, "release manifest differs from actual bytes or source identity")
    checksums = "".join(f"{digest(directory / name)}  {name}\n" for name in sorted(payloads | {path.name}))
    sums = directory / (base + ".SHA256SUMS")
    if create:
        require(not sums.exists(), "refuse replacing existing checksums")
        sums.write_text(checksums)
    else:
        require(sums.read_text() == checksums, "checksum inventory or bytes do not match")
    return payloads | {path.name, sums.name}


def verify(directory, version, arches, commit, check_modes=True):
    require(directory.is_dir() and not directory.is_symlink(), "expected a real artifact directory")
    require(all(stat.S_ISREG(item.lstat().st_mode) and item.stat().st_nlink == 1
                for item in directory.iterdir()), "linked or non-file artifact")
    expected = set()
    for arch in arches:
        data = json.loads((directory / (prefix(version, arch) + ".manifest.json")).read_text())
        optional = data.get("optional", {})
        require(set(optional) == set(OPTIONAL) and all(type(v) is bool for v in optional.values()),
                "invalid optional-format inventory")
        expected |= manifest(directory, version, arch, commit, optional, False, check_modes)
    actual = {item.name for item in directory.iterdir()}
    require(actual == expected, f"artifact set mismatch: missing={expected - actual}, unexpected={actual - expected}")
    return sorted(expected)


def extract(archive_path, directory, version, arch):
    base = prefix(version, arch)
    allowed = {base + suffix for suffix in (*MANDATORY, *OPTIONAL.values(), ".manifest.json", ".SHA256SUMS")}
    directory.mkdir(parents=True, exist_ok=True)
    with tarfile.open(archive_path, "r:") as archive:
        members = archive.getmembers()
        names = [member.name for member in members]
        require(len(names) == len(set(names)), "duplicate transfer members")
        require(set(names) <= allowed, "unexpected transfer filename")
        for member in members:
            require(member.isfile() and member.size > 0, "linked, empty or non-file transfer member")
            target = directory / member.name
            require(not target.exists() and not target.is_symlink(), "refuse overwriting transfer target")
        archive.extractall(directory, members=members, filter="data")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=["create", "verify", "extract"])
    parser.add_argument("--directory", type=Path, required=True)
    parser.add_argument("--version", required=True)
    parser.add_argument("--commit", required=True)
    parser.add_argument("--arch", action="append", choices=MACHINES, required=True)
    parser.add_argument("--flatpak", action="store_true")
    parser.add_argument("--zsync", action="store_true")
    parser.add_argument("--archive", type=Path)
    parser.add_argument("--downloaded", action="store_true",
                        help="HTTP downloads have no Unix modes; still check tar payload executability")
    args = parser.parse_args()
    require(len(args.arch) == len(set(args.arch)), "duplicate architectures")
    if args.command == "extract":
        require(len(args.arch) == 1 and args.archive, "extract requires one arch and archive")
        extract(args.archive, args.directory, args.version, args.arch[0])
    elif args.command == "create":
        require(len(args.arch) == 1, "create requires one arch")
        manifest(args.directory, args.version, args.arch[0], args.commit,
                 {"flatpak": args.flatpak, "zsync": args.zsync}, True)
        verify(args.directory, args.version, args.arch, args.commit)
    else:
        for name in verify(args.directory, args.version, args.arch, args.commit, not args.downloaded):
            print(name)


if __name__ == "__main__":
    main()
