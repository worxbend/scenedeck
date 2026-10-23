#!/usr/bin/env python3
"""Promote numeric stable releases monotonically under the workflow's shared lock."""
import argparse
import json
import os
import re
import urllib.error
import urllib.request


def require(condition, message):
    if not condition:
        raise ValueError(message)


def version(tag):
    require(isinstance(tag, str) and re.fullmatch(r"v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)", tag),
            "latest promotion requires a canonical numeric vX.Y.Z tag")
    return tuple(map(int, tag[1:].split(".")))


def api(path, method="GET", data=None, missing=False):
    request = urllib.request.Request(
        "https://api.github.com/" + path, method=method,
        data=None if data is None else json.dumps(data).encode(),
        headers={"Authorization": "Bearer " + os.environ["GH_TOKEN"],
                 "Accept": "application/vnd.github+json", "X-GitHub-Api-Version": "2022-11-28",
                 "Content-Type": "application/json", "User-Agent": "scenedeck-release"})
    try:
        with urllib.request.urlopen(request, timeout=30) as response:
            return json.load(response)
    except urllib.error.HTTPError as error:
        if missing and error.code == 404:
            error.close()
            return None
        raise


def stable(row):
    require(isinstance(row, dict) and type(row.get("draft")) is bool
            and type(row.get("prerelease")) is bool, "malformed release status")
    if row["draft"] or row["prerelease"]:
        return False
    try:
        version(row.get("tag_name"))
    except ValueError:
        return False
    require(type(row.get("id")) is int and row["id"] > 0, "malformed release ID")
    return True


def promote(repository, tag, request=api):
    """Call only inside release-latest concurrency; it is not an API CAS lock.

    Scan all published versions so replacement of pending Actions promotion jobs
    still promotes the highest verified publication, not the last finisher.
    """
    require(re.fullmatch(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", repository), "invalid repository")
    version(tag)
    endpoint = f"repos/{repository}/releases"
    releases = []
    page = 1
    while True:
        rows = request(f"{endpoint}?per_page=100&page={page}")
        require(isinstance(rows, list), "malformed releases page")
        releases.extend(row for row in rows if stable(row))
        if len(rows) < 100:
            break
        page += 1
    require(any(row["tag_name"] == tag for row in releases), "triggering release is not published and stable")
    newest = max(releases, key=lambda row: version(row["tag_name"]))
    current = request(endpoint + "/latest", missing=True)
    if current is not None:
        require(stable(current), "current latest has an unsupported identity; refusing promotion")
        if version(current["tag_name"]) >= version(newest["tag_name"]):
            return current["tag_name"]
    # make_latest is a string enum, not a JSON boolean. Never use legacy ordering.
    # https://docs.github.com/en/rest/releases/releases#update-a-release
    request(f"{endpoint}/{newest['id']}", method="PATCH", data={"make_latest": "true"})
    readback = request(endpoint + "/latest")
    require(stable(readback) and readback["id"] == newest["id"]
            and readback["tag_name"] == newest["tag_name"], "latest pointer readback mismatch")
    return readback["tag_name"]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repository", required=True)
    parser.add_argument("--tag", required=True)
    args = parser.parse_args()
    print("latest=" + promote(args.repository, args.tag))


if __name__ == "__main__":
    main()
