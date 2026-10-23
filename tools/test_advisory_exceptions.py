"""Offline policy fixtures; clock injection is only available to Python unit tests."""
import copy
import importlib.util
import os
import shutil
import subprocess
import sys
import tempfile
import tomllib
import unittest
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "tools/check-advisory-exceptions.py"
ADVISORY = "RUSTSEC-2026-0374"


class AdvisoryExceptions(unittest.TestCase):
    def setUp(self):
        self.deny = tomllib.loads((ROOT / "deny.toml").read_text())
        self.lock = tomllib.loads((ROOT / "Cargo.lock").read_text())
        self.exception = {
            "id": ADVISORY,
            "owner": "SceneDeck maintainer",
            "reason": "Maintenance-only; published localization proc macros have no compatible fixed release.",
            "expires": "2026-11-20",
        }
        self.deny["advisories"] = {
            "version": 2,
            "yanked": "deny",
            "unused-ignored-advisory": "deny",
            "ignore": [{"id": ADVISORY, "reason": self.deny_reason()}],
        }
        self.before = datetime(2026, 11, 19, 23, 59, 59, tzinfo=timezone.utc)

    def deny_reason(self):
        return (f"{self.exception['reason']} Owner: {self.exception['owner']}. "
                f"Expires: {self.exception['expires']} 00:00:00 UTC.")

    def checker(self):
        self.assertTrue(SCRIPT.is_file(), "the expiry checker must exist")
        spec = importlib.util.spec_from_file_location("advisory_exceptions", SCRIPT)
        assert spec and spec.loader
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        return module

    def test_utc_expiry_boundary(self):
        checker = self.checker()
        for timestamp, expired in [
            ("2026-11-19T23:59:59.999999+00:00", False),
            ("2026-11-20T00:00:00+00:00", True),
            ("2026-11-21T12:00:00+00:00", True),
            ("2026-11-20T03:00:00+03:00", True),
            ("2026-11-19T23:00:00-01:00", True),
        ]:
            with self.subTest(timestamp=timestamp):
                now = datetime.fromisoformat(timestamp)
                if expired:
                    with self.assertRaisesRegex(ValueError, "expired"):
                        checker.validate(self.deny, self.lock, self.exception, now)
                else:
                    checker.validate(self.deny, self.lock, self.exception, now)

    def test_missing_or_changed_approval_metadata_fails_closed(self):
        checker = self.checker()
        for key in self.exception:
            for replacement in [None, "", "   ", 42, False]:
                with self.subTest(key=key, replacement=replacement):
                    exception = copy.deepcopy(self.exception)
                    if replacement is None:
                        del exception[key]
                    else:
                        exception[key] = replacement
                    with self.assertRaisesRegex(ValueError, "approval"):
                        checker.validate(self.deny, self.lock, exception, self.before)
        for changes in [
            {"id": "RUSTSEC-2026-9999"},
            {"expires": "2026-11-21"},
            {"expires": "2026-11-19"},
            {"expires": "not-a-date"},
            {"expires": "2026-11-20T23:59:59Z"},
            {"extra": "another approval"},
        ]:
            with self.subTest(changes=changes):
                exception = {**self.exception, **changes}
                with self.assertRaisesRegex(ValueError, "approval"):
                    checker.validate(self.deny, self.lock, exception, self.before)

    def test_deny_ignore_set_and_reasons_must_match_the_single_approval(self):
        approved = self.deny["advisories"]["ignore"][0]
        for ignored in [
            [], [ADVISORY], [{"id": ADVISORY}],
            [{"id": ADVISORY, "reason": ""}],
            [{"id": ADVISORY, "reason": "unrelated acceptance"}],
            [{"id": "RUSTSEC-2026-9999", "reason": approved["reason"]}],
            [approved, {"id": "RUSTSEC-2026-9999", "reason": "unapproved"}],
            [approved, approved],
            [{**approved, "expiry": "2099-01-01"}],
        ]:
            with self.subTest(ignored=ignored):
                deny = copy.deepcopy(self.deny)
                deny["advisories"]["ignore"] = ignored
                with self.assertRaisesRegex(ValueError, "deny.toml"):
                    self.checker().validate(deny, self.lock, self.exception, self.before)

    def test_advisory_protections_cannot_be_disabled(self):
        for key, replacement in [
            ("version", 1), ("yanked", "allow"),
            ("unused-ignored-advisory", "allow"), ("unmaintained", "none"),
        ]:
            for missing in (False, True):
                with self.subTest(key=key, replacement=replacement, missing=missing):
                    deny = copy.deepcopy(self.deny)
                    if missing:
                        if key not in deny["advisories"]:
                            continue
                        del deny["advisories"][key]
                    else:
                        deny["advisories"][key] = replacement
                    with self.assertRaisesRegex(ValueError, "deny.toml"):
                        self.checker().validate(deny, self.lock, self.exception, self.before)

    def test_reviewed_registry_package_identities_cannot_drift(self):
        names = ["basic-toml", "i18n-config", "i18n-embed", "i18n-embed-fl", "i18n-embed-impl"]
        for name in names:
            for key, replacement in [
                ("version", "99.0.0"),
                ("source", None),
                ("source", "git+https://example.invalid/replacement#" + "a" * 40),
                ("source", "registry+https://example.invalid/index"),
                ("checksum", None), ("checksum", "0" * 64),
            ]:
                with self.subTest(name=name, key=key, replacement=replacement):
                    lock = copy.deepcopy(self.lock)
                    package = next(p for p in lock["package"] if p["name"] == name)
                    if replacement is None:
                        del package[key]
                    else:
                        package[key] = replacement
                    with self.assertRaisesRegex(ValueError, "locked graph"):
                        self.checker().validate(self.deny, lock, self.exception, self.before)

    def test_reviewed_packages_must_exist_once(self):
        for name in ["scenedeck", "basic-toml", "i18n-config", "i18n-embed", "i18n-embed-fl", "i18n-embed-impl"]:
            for change in ["missing", "duplicate", "another-version"]:
                with self.subTest(name=name, change=change):
                    lock = copy.deepcopy(self.lock)
                    package = next(p for p in lock["package"] if p["name"] == name)
                    if change == "missing":
                        lock["package"].remove(package)
                    else:
                        duplicate = copy.deepcopy(package)
                        if change == "another-version":
                            duplicate["version"] = "99.0.0"
                        lock["package"].append(duplicate)
                    with self.assertRaisesRegex(ValueError, "locked graph"):
                        self.checker().validate(self.deny, lock, self.exception, self.before)

    def test_root_package_cannot_be_replaced_by_a_registry_package(self):
        lock = copy.deepcopy(self.lock)
        root = next(p for p in lock["package"] if p["name"] == "scenedeck")
        root["source"] = "registry+https://github.com/rust-lang/crates.io-index"
        with self.assertRaisesRegex(ValueError, "locked graph"):
            self.checker().validate(self.deny, lock, self.exception, self.before)

    def test_only_reviewed_proc_macro_reverse_paths_are_allowed(self):
        edges = [
            ("i18n-config", "basic-toml"),
            ("i18n-embed-fl", "i18n-config"), ("i18n-embed-impl", "i18n-config"),
            ("i18n-embed", "i18n-embed-impl"), ("i18n-embed-fl", "i18n-embed"),
            ("scenedeck", "i18n-embed"), ("scenedeck", "i18n-embed-fl"),
        ]
        for parent, child in edges:
            for change in ["remove", "duplicate", "wrong-version", "wrong-source"]:
                with self.subTest(parent=parent, child=child, change=change):
                    lock = copy.deepcopy(self.lock)
                    package = next(p for p in lock["package"] if p["name"] == parent)
                    if change == "remove":
                        package["dependencies"].remove(child)
                    elif change == "duplicate":
                        package["dependencies"].append(child)
                    else:
                        replacement = (child + " 99.0.0" if change == "wrong-version"
                                       else child + " 0.1.10 (git+https://example.invalid/source)")
                        package["dependencies"][package["dependencies"].index(child)] = replacement
                    with self.assertRaisesRegex(ValueError, "locked graph"):
                        self.checker().validate(self.deny, lock, self.exception, self.before)
        for child in ["basic-toml", "i18n-config", "i18n-embed", "i18n-embed-fl", "i18n-embed-impl"]:
            with self.subTest(new_parent=child):
                lock = copy.deepcopy(self.lock)
                lock["package"].append({"name": "unreviewed-parent", "version": "1.0.0", "dependencies": [child]})
                with self.assertRaisesRegex(ValueError, "locked graph"):
                    self.checker().validate(self.deny, lock, self.exception, self.before)
        lock = copy.deepcopy(self.lock)
        next(p for p in lock["package"] if p["name"] == "scenedeck")["dependencies"].append("basic-toml")
        with self.assertRaisesRegex(ValueError, "locked graph"):
            self.checker().validate(self.deny, lock, self.exception, self.before)

    def test_cargo_disambiguated_references_keep_the_same_reviewed_path(self):
        lock = copy.deepcopy(self.lock)
        child = next(p for p in lock["package"] if p["name"] == "basic-toml")
        parent = next(p for p in lock["package"] if p["name"] == "i18n-config")
        for reference in [f"basic-toml {child['version']}", f"basic-toml {child['version']} ({child['source']})"]:
            with self.subTest(reference=reference):
                parent["dependencies"][0] = reference
                self.checker().validate(self.deny, lock, self.exception, self.before)

    def test_unrelated_locked_packages_are_not_frozen(self):
        lock = copy.deepcopy(self.lock)
        lock["package"].append({"name": "unrelated", "version": "1.0.0", "dependencies": ["serde"]})
        self.checker().validate(self.deny, lock, self.exception, self.before)

    def test_repository_policy_matches_the_approved_exception(self):
        record = ROOT / "tools/advisory-exception.toml"
        self.assertTrue(record.is_file(), "the machine-readable approval record must exist")
        exception = tomllib.loads(record.read_text())
        self.assertEqual(exception["id"], ADVISORY)
        self.assertEqual(exception["owner"], "SceneDeck maintainer")
        self.assertEqual(exception["expires"], "2026-11-20")
        self.checker().validate(tomllib.loads((ROOT / "deny.toml").read_text()), self.lock, exception, self.before)

    def test_cli_uses_actual_utc_clock_independent_of_working_directory(self):
        before = datetime.now(timezone.utc)
        completed = subprocess.run(
            [sys.executable, str(SCRIPT)], cwd=ROOT.parent, text=True, capture_output=True,
            env={**os.environ, "TZ": "Pacific/Kiritimati", "ADVISORY_NOW": "2026-01-01"},
        )
        after = datetime.now(timezone.utc)
        self.assertIn("checked_at=", completed.stdout)
        checked_at = datetime.fromisoformat(completed.stdout.splitlines()[0].removeprefix("checked_at="))
        self.assertEqual(checked_at.utcoffset(), timezone.utc.utcoffset(checked_at))
        self.assertLessEqual(before, checked_at)
        self.assertLessEqual(checked_at, after)
        expired = checked_at >= datetime(2026, 11, 20, tzinfo=timezone.utc)
        self.assertEqual(completed.returncode, int(expired), completed.stderr)
        self.assertIn("expired" if expired else "valid", completed.stderr if expired else completed.stdout)

    def test_cli_rejects_clock_override_arguments(self):
        for option in ["--now", "--date", "--today", "--as-of", "--expires", "2026-01-01"]:
            with self.subTest(option=option):
                completed = subprocess.run(
                    [sys.executable, str(SCRIPT), option, "2026-01-01"], text=True, capture_output=True,
                )
                self.assertEqual(completed.returncode, 2, completed.stdout + completed.stderr)
                self.assertIn("unrecognized arguments", completed.stderr)

    def test_cli_fails_closed_on_missing_or_malformed_policy_files(self):
        for filename in ["deny.toml", "Cargo.lock", "tools/advisory-exception.toml"]:
            for change in ["missing", "malformed", "empty"]:
                with self.subTest(filename=filename, change=change), tempfile.TemporaryDirectory() as tmp:
                    root = Path(tmp)
                    (root / "tools").mkdir()
                    for source in ["tools/check-advisory-exceptions.py", "deny.toml", "Cargo.lock", "tools/advisory-exception.toml"]:
                        shutil.copyfile(ROOT / source, root / source)
                    target = root / filename
                    if change == "missing":
                        target.unlink()
                    else:
                        target.write_text("[broken" if change == "malformed" else "")
                    completed = subprocess.run(
                        [sys.executable, str(root / "tools/check-advisory-exceptions.py")], text=True, capture_output=True,
                    )
                    self.assertEqual(completed.returncode, 1, completed.stdout + completed.stderr)
                    self.assertIn("advisory exception check failed", completed.stderr)

    def test_naive_clock_is_not_accepted(self):
        with self.assertRaisesRegex(ValueError, "UTC|timezone"):
            self.checker().validate(self.deny, self.lock, self.exception, datetime(2026, 11, 19))


if __name__ == "__main__":
    unittest.main()
