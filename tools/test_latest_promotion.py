"""Offline latest-pointer state-machine tests; no GitHub requests are issued."""
import copy
from email.message import Message
import importlib.util
import io
import itertools
import json
import unittest
import urllib.error
from pathlib import Path
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("promote_latest", Path(__file__).with_name("promote-latest.py"))
assert spec and spec.loader
promotion = importlib.util.module_from_spec(spec)
spec.loader.exec_module(promotion)


def release(tag, identity=1, **status):
    return {"id": identity, "tag_name": tag, "draft": False, "prerelease": False, **status}


class Repository:
    def __init__(self, rows=(), current=None):
        self.rows = list(rows)
        self.current = current
        self.writes = []
        self.calls = []
        self.bad_readback = False

    def request(self, path, method="GET", data=None, missing=False):
        self.calls.append((path, method, data, missing))
        if method == "PATCH":
            self.writes.append((path, data))
            row = next(row for row in self.rows if str(row["id"]) == path.rsplit("/", 1)[1])
            if not self.bad_readback:
                self.current = row
            return copy.deepcopy(row)
        if path.endswith("/latest"):
            return copy.deepcopy(self.current)
        page = int(path.rsplit("=", 1)[1])
        return copy.deepcopy(self.rows[(page - 1) * 100:page * 100])


class LatestPromotion(unittest.TestCase):
    def test_out_of_order_completions_and_backfills_never_regress(self):
        versions = ["v1.2.3", "v1.10.0", "v2.0.0"]
        for order in itertools.permutations(versions):
            with self.subTest(order=order):
                repo = Repository()
                highest = (0, 0, 0)
                for index, tag in enumerate(order, 1):
                    repo.rows.append(release(tag, index))
                    result = promotion.promote("owner/repo", tag, repo.request)
                    highest = max(highest, promotion.version(tag))
                    self.assertEqual(promotion.version(result), highest)
                self.assertEqual(promotion.promote("owner/repo", order[0], repo.request), "v2.0.0")
                self.assertTrue(all(data == {"make_latest": "true"} for _, data in repo.writes))

    def test_pending_job_replacement_reconciles_highest_published_not_trigger(self):
        repo = Repository([release("v1.2.3"), release("v1.10.0", 2)], release("v1.0.0", 9))
        self.assertEqual(promotion.promote("owner/repo", "v1.2.3", repo.request), "v1.10.0")
        self.assertEqual(len(repo.writes), 1)
        promotion.promote("owner/repo", "v1.10.0", repo.request)
        self.assertEqual(len(repo.writes), 1, "equal versions must be idempotent")

    def test_pagination_filters_drafts_prereleases_and_nonnumeric_tags(self):
        rows = [release("legacy", index + 1) for index in range(100)]
        rows += [release("v99.0.0", 101, draft=True), release("v98.0.0", 102, prerelease=True),
                 release("v3.0.0-rc1", 103), release("v2.0.0", 104)]
        repo = Repository(rows)
        self.assertEqual(promotion.promote("owner/repo", "v2.0.0", repo.request), "v2.0.0")
        self.assertTrue(any("page=2" in path for path, *_ in repo.calls))

    def test_invalid_or_unpublished_trigger_and_invalid_latest_fail_closed(self):
        for tag in ["v01.2.3", "v1.2.3-rc1", "v1.2.3;injection", "1.2.3", "v9.0.0"]:
            repo = Repository([release("v1.2.3")])
            with self.subTest(tag=tag), self.assertRaises(ValueError):
                promotion.promote("owner/repo", tag, repo.request)
            self.assertEqual(repo.writes, [])
        for current in [release("legacy"), release("v9.0.0", draft=True), {"tag_name": "v1.0.0"}]:
            repo = Repository([release("v1.2.3")], current)
            with self.assertRaises(ValueError):
                promotion.promote("owner/repo", "v1.2.3", repo.request)
            self.assertEqual(repo.writes, [])

    def test_newer_current_is_never_downgraded_even_if_list_is_stale(self):
        repo = Repository([release("v1.2.3")], release("v9.0.0", 9))
        self.assertEqual(promotion.promote("owner/repo", "v1.2.3", repo.request), "v9.0.0")
        self.assertEqual(repo.writes, [])

    def test_failed_reads_or_failed_pointer_readback_are_not_success(self):
        repo = Repository([release("v1.2.3")], release("v1.0.0", 9))
        repo.bad_readback = True
        with self.assertRaisesRegex(ValueError, "readback mismatch"):
            promotion.promote("owner/repo", "v1.2.3", repo.request)
        with self.assertRaises(OSError):
            promotion.promote("owner/repo", "v1.2.3", lambda *a, **k: (_ for _ in ()).throw(OSError("offline")))

    def test_api_only_treats_exact_latest_404_as_missing(self):
        for status in [404, 403, 500]:
            error = urllib.error.HTTPError("https://api.github.com/test", status, "fixture", Message(), io.BytesIO(b""))
            self.addCleanup(error.close)
            with patch.dict(promotion.os.environ, {"GH_TOKEN": "offline-fixture"}), patch.object(promotion.urllib.request, "urlopen", side_effect=error):
                if status == 404:
                    self.assertIsNone(promotion.api("test", missing=True))
                else:
                    with self.assertRaises(urllib.error.HTTPError): promotion.api("test", missing=True)
        with patch.dict(promotion.os.environ, {"GH_TOKEN": "offline-fixture"}), patch.object(promotion.urllib.request, "urlopen", return_value=io.BytesIO(b'{}')) as request:
            promotion.api("test", method="PATCH", data={"make_latest": "true"})
            self.assertEqual(json.loads(request.call_args.args[0].data), {"make_latest": "true"})


if __name__ == "__main__":
    unittest.main()
