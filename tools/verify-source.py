#!/usr/bin/env python3
"""Validate package/tag identity without resolving or building Cargo dependencies."""
import argparse
import re
import subprocess
import tomllib
import xml.etree.ElementTree as ET
from pathlib import Path


def git(*args):
    return subprocess.check_output(["git", *args], text=True).strip()


def verify(expected_commit, tag=None, remote=False):
    if not re.fullmatch(r"[0-9a-f]{40}", expected_commit):
        raise ValueError("expected a full immutable commit SHA")
    commit = git("rev-parse", "HEAD")
    if commit != expected_commit:
        raise ValueError(f"checkout {commit} != expected {expected_commit}")
    version = tomllib.loads(Path("Cargo.toml").read_text())["package"]["version"]
    if not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", version):
        raise ValueError("only numeric stable package versions may be released")
    release = ET.parse("data/io.scenedeck.app.metainfo.xml").find("releases/release")
    if release is None or release.attrib.get("version") != version:
        raise ValueError("top AppStream release and Cargo versions disagree")
    if tag is not None:
        if tag != f"v{version}":
            raise ValueError(f"tag {tag!r} != v{version}")
        if remote:
            refs = dict(line.split()[::-1] for line in git(
                "ls-remote", "--tags", "origin", f"refs/tags/{tag}", f"refs/tags/{tag}^{{}}"
            ).splitlines())
            resolved = refs.get(f"refs/tags/{tag}^{{}}", refs.get(f"refs/tags/{tag}"))
        else:
            resolved = git("rev-parse", f"refs/tags/{tag}^{{commit}}")
        if resolved != commit:
            raise ValueError("tag moved or does not identify the validated commit")
    return version, commit


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--commit", required=True)
    parser.add_argument("--tag")
    parser.add_argument("--remote", action="store_true")
    args = parser.parse_args()
    version, commit = verify(args.commit, args.tag, args.remote)
    print(f"version={version}\ncommit={commit}")
