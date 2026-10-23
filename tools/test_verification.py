"""Offline fixtures for CI verification (never presented as built release artifacts)."""
import contextlib
import copy
import importlib.util
import io
import json
import os
import shutil
import subprocess
import sys
import tarfile
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

TOOLS = Path(__file__).resolve().parent
VERSION = "1.2.3"
COMMIT = "a" * 40


def load(name):
    spec = importlib.util.spec_from_file_location(name.replace("-", "_"), TOOLS / (name + ".py"))
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


release = load("verify-release")
snap = load("verify-snap")
native = load("test-native")
source = load("verify-source")


def elf(arch, appimage=False):
    data = bytearray(64)
    data[:6] = b"\x7fELF\x02\x01"
    data[18:20] = release.MACHINES[arch].to_bytes(2, "little")
    if appimage:
        data[8:11] = b"AI\x02"
    return bytes(data)


def payloads(base, arch):
    app = release.APP_ID
    return {
        f"{base}/usr/bin/scenedeck": (elf(arch), 0o755),
        f"{base}/usr/share/applications/{app}.desktop": (b"[Desktop Entry]\nExec=scenedeck\n", 0o644),
        f"{base}/usr/share/metainfo/{app}.metainfo.xml": (
            f'<component><releases><release version="{VERSION}"/></releases></component>'.encode(), 0o644),
        f"{base}/usr/share/icons/hicolor/scalable/apps/{app}.svg": (b"<svg/>", 0o644),
        f"{base}/usr/share/icons/hicolor/256x256/apps/{app}.png": (b"fixture-png", 0o644),
    }


def tarball(path, contents):
    with tarfile.open(path, "w:gz") as archive:
        for name, (data, mode) in contents.items():
            info = tarfile.TarInfo(name)
            info.size, info.mode = len(data), mode
            archive.addfile(info, io.BytesIO(data))


NATIVE_FLATPAK = bool(shutil.which("flatpak") and shutil.which("ostree"))


def flatpak_metadata(app=release.APP_ID, arch="amd64", version=VERSION):
    native_arch = release.FLATPAK_ARCHES[arch]
    ref = f"app/{app}/{native_arch}/master"
    metadata = (f"[Application]\nname={app}\nruntime=org.gnome.Platform/{native_arch}/48\n"
                f"sdk=org.gnome.Sdk/{native_arch}/48\ncommand=scenedeck\n")
    xml = (f'<component type="desktop-application"><id>{app}</id><name>SceneDeck fixture</name>'
           '<summary>Native container fixture only, not an application build</summary>'
           f'<releases><release version="{version}"/></releases></component>')
    return ref, metadata, xml


def native_flatpak(path, arch="amd64", app=release.APP_ID, version=VERSION):
    """Build a REAL native container with metadata only; never install or run it."""
    with tempfile.TemporaryDirectory(prefix="scenedeck-flatpak-fixture-") as tmp:
        root = Path(tmp)
        build = root / "build"
        metadata_dir = build / "files/share/metainfo"
        metadata_dir.mkdir(parents=True)
        (build / "export").mkdir()
        _, metadata, xml = flatpak_metadata(app, arch, version)
        (build / "metadata").write_text(metadata)
        (metadata_dir / f"{app}.metainfo.xml").write_text(xml)
        repo = root / "repo"
        # Native fixture creation must not scan or mutate host Flatpak state.
        env = os.environ.copy()
        for key in ("FLATPAK_USER_DIR", "FLATPAK_SYSTEM_DIR", "FLATPAK_CONFIG_DIR",
                    "XDG_DATA_HOME", "XDG_CONFIG_HOME", "XDG_CACHE_HOME", "XDG_RUNTIME_DIR"):
            location = root / key
            location.mkdir(mode=0o700)
            env[key] = str(location)
        for command in (["flatpak", "build-export", "--disable-sandbox",
                         f"--arch={release.FLATPAK_ARCHES[arch]}", str(repo), str(build)],
                        ["flatpak", "build-bundle", f"--arch={release.FLATPAK_ARCHES[arch]}",
                         str(repo), str(path), app]):
            subprocess.run(command, env=env, check=True, capture_output=True, text=True, timeout=30)


def fixture(directory, arch="amd64", optional=None):
    optional = optional or {"flatpak": False, "zsync": False}
    base = release.prefix(VERSION, arch)
    (directory / base).write_bytes(elf(arch))
    (directory / base).chmod(0o755)
    (directory / (base + ".AppImage")).write_bytes(elf(arch, True))
    (directory / (base + ".AppImage")).chmod(0o755)
    tarball(directory / (base + ".tar.gz"), payloads(base, arch))
    if optional["flatpak"]:
        native_flatpak(directory / (base + ".flatpak"), arch)
    if optional["zsync"]:
        (directory / (base + ".AppImage.zsync")).write_bytes(f"zsync: 0.6.2\nFilename: {base}.AppImage\n\nfixture".encode())
    return base


class ReleaseVerification(unittest.TestCase):
    def test_native_flatpak_fixture_isolates_build_state(self):
        calls = []

        def inspect(command, **kwargs):
            env = kwargs.get("env", {})
            root = Path(command[-1]).parent if command[1] == "build-export" else Path(command[-3]).parent
            for key in ("FLATPAK_USER_DIR", "FLATPAK_SYSTEM_DIR", "FLATPAK_CONFIG_DIR",
                        "XDG_DATA_HOME", "XDG_CONFIG_HOME", "XDG_CACHE_HOME", "XDG_RUNTIME_DIR"):
                self.assertIn(key, env)
                self.assertTrue(Path(env[key]).is_relative_to(root))
                self.assertTrue(Path(env[key]).is_dir())
            calls.append(command[1])

        with tempfile.TemporaryDirectory() as tmp, patch.object(subprocess, "run", side_effect=inspect):
            native_flatpak(Path(tmp) / "fixture.flatpak")
        self.assertEqual(calls, ["build-export", "build-bundle"])

    def test_flatpak_text_is_rejected_before_inventory_creation(self):
        with tempfile.TemporaryDirectory() as tmp:
            directory = Path(tmp)
            base = fixture(directory)
            (directory / (base + ".flatpak")).write_text("not a native bundle")
            with self.assertRaises(ValueError):
                release.manifest(directory, VERSION, "amd64", COMMIT,
                                 {"flatpak": True, "zsync": False}, True)
            self.assertFalse((directory / (base + ".manifest.json")).exists())

    def test_flatpak_metadata_readback_seam_rejects_wrong_and_malformed_identity(self):
        ref, metadata, xml = flatpak_metadata()
        release.flatpak_identity(ref, metadata, xml, VERSION, "amd64")
        for changed in [
            (ref.replace(release.APP_ID, "io.other.app"), metadata, xml),
            (ref.replace("x86_64", "aarch64"), metadata, xml),
            (ref, metadata.replace(release.APP_ID, "io.other.app"), xml),
            (ref, metadata.replace("x86_64", "aarch64"), xml),
            (ref, metadata, xml.replace(VERSION, "0.0.1")),
            (ref, metadata, xml.replace(release.APP_ID, "io.other.app")),
            (ref, "malformed", xml), (ref, metadata, "<malformed"),
        ]:
            with self.subTest(changed=changed), self.assertRaises(ValueError):
                release.flatpak_identity(*changed, VERSION, "amd64")

    def test_flatpak_missing_tools_fail_closed(self):
        with patch.object(release.shutil, "which", return_value=None):
            with self.assertRaisesRegex(ValueError, "requires native flatpak and ostree"):
                release.inspect_flatpak(Path("untrusted.flatpak"), VERSION, "amd64")

    @unittest.skipUnless(NATIVE_FLATPAK, "native flatpak/ostree tools unavailable; optional bundles fail closed")
    def test_native_flatpak_wrong_identity_arch_version_and_corruption(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "fixture.flatpak"
            native_flatpak(path)
            release.inspect_flatpak(path, VERSION, "amd64")
            with self.assertRaisesRegex(ValueError, "ref identity mismatch"):
                release.inspect_flatpak(path, VERSION, "arm64")
            with self.assertRaisesRegex(ValueError, "version mismatch"):
                release.inspect_flatpak(path, "0.0.1", "amd64")
            original = path.read_bytes()
            path.write_bytes(original[:64])
            with self.assertRaisesRegex(ValueError, "native Flatpak bundle inspection failed"):
                release.inspect_flatpak(path, VERSION, "amd64")
            native_flatpak(path, app="io.other.app")
            with self.assertRaisesRegex(ValueError, "ref identity mismatch"):
                release.inspect_flatpak(path, VERSION, "amd64")

    def test_cli_all_architectures_and_optional_combinations(self):
        self.check_cli_combinations((False,))

    @unittest.skipUnless(NATIVE_FLATPAK, "native flatpak/ostree tools unavailable; optional bundles fail closed")
    def test_cli_native_flatpak_combinations(self):
        self.check_cli_combinations((True,))

    def check_cli_combinations(self, flatpaks):
        for flatpak in flatpaks:
            for zsync in (False, True):
                with self.subTest(flatpak=flatpak, zsync=zsync), tempfile.TemporaryDirectory() as tmp:
                    directory = Path(tmp)
                    for arch in release.MACHINES:
                        arch_dir = directory / arch
                        arch_dir.mkdir()
                        fixture(arch_dir, arch, {"flatpak": flatpak, "zsync": zsync})
                        args = [sys.executable, str(TOOLS / "verify-release.py"), "create", "--directory", str(arch_dir),
                                "--version", VERSION, "--commit", COMMIT, "--arch", arch]
                        subprocess.run(args + (["--flatpak"] if flatpak else []) + (["--zsync"] if zsync else []), check=True)
                        for path in arch_dir.iterdir():
                            path.rename(directory / path.name)
                        arch_dir.rmdir()
                    names = release.verify(directory, VERSION, list(release.MACHINES), COMMIT)
                    self.assertEqual(len(names), 10 + 2 * (flatpak + zsync))
                    for arch in release.MACHINES:
                        sums = (directory / (release.prefix(VERSION, arch) + ".SHA256SUMS")).read_text()
                        self.assertEqual(len(sums.splitlines()), 4 + flatpak + zsync)
                    if flatpak:
                        bundle = directory / (release.prefix(VERSION, "amd64") + ".flatpak")
                        bundle.write_bytes(b"not a native bundle")
                        with self.assertRaisesRegex(ValueError, "native Flatpak bundle inspection failed"):
                            release.verify(directory, VERSION, list(release.MACHINES), COMMIT)

    def test_missing_empty_extra_link_corruption_modes_identity(self):
        def mutate(directory, base, case):
            if case == "missing": (directory / base).unlink()
            elif case == "empty": (directory / base).write_bytes(b"")
            elif case == "extra": (directory / "unexpected").write_text("extra")
            elif case == "symlink": (directory / (base + ".AppImage")).unlink(); (directory / (base + ".AppImage")).symlink_to(directory / base)
            elif case == "hardlink": os.link(directory / base, directory / "hardlink")
            elif case == "corrupt": (directory / (base + ".AppImage")).write_bytes(elf("arm64", True))
            elif case == "not-executable": (directory / base).chmod(0o644)
            elif case == "checksum": (directory / (base + ".SHA256SUMS")).write_text("incorrect checksum")
            elif case == "optional-tamper": (directory / (base + ".AppImage.zsync")).write_bytes(b"tampered")
        for case in ("missing", "empty", "extra", "symlink", "hardlink", "corrupt", "not-executable", "checksum", "optional-tamper"):
            with self.subTest(case=case), tempfile.TemporaryDirectory() as tmp:
                directory = Path(tmp)
                optional = {"flatpak": False, "zsync": True}
                base = fixture(directory, optional=optional)
                release.manifest(directory, VERSION, "amd64", COMMIT, optional, True)
                mutate(directory, base, case)
                with self.assertRaises((ValueError, FileNotFoundError)):
                    release.verify(directory, VERSION, ["amd64"], COMMIT)
        with tempfile.TemporaryDirectory() as tmp:
            directory = Path(tmp)
            fixture(directory)
            release.manifest(directory, VERSION, "amd64", COMMIT, {"flatpak": False, "zsync": False}, True)
            with self.assertRaises(ValueError): release.verify(directory, VERSION, ["amd64"], "b" * 40)
            with self.assertRaises(FileNotFoundError): release.verify(directory, VERSION, ["amd64", "arm64"], COMMIT)

    def test_tarball_payload_rejections_before_checksum_creation(self):
        for case in ("missing", "unexpected", "wrong-version", "not-executable", "different-binary", "symlink", "duplicate"):
            with self.subTest(case=case), tempfile.TemporaryDirectory() as tmp:
                directory = Path(tmp)
                base = fixture(directory)
                files = payloads(base, "amd64")
                binary = base + "/usr/bin/scenedeck"
                metadata = base + f"/usr/share/metainfo/{release.APP_ID}.metainfo.xml"
                if case == "missing": files.pop(binary)
                elif case == "unexpected": files[base + "/extra"] = (b"extra", 0o644)
                elif case == "wrong-version": files[metadata] = (b'<component><releases><release version="0.0.1"/></releases></component>', 0o644)
                elif case == "not-executable": files[binary] = (elf("amd64"), 0o644)
                elif case == "different-binary": files[binary] = (elf("arm64"), 0o755)
                tarball(directory / (base + ".tar.gz"), files)
                if case in ("symlink", "duplicate"):
                    with tarfile.open(directory / (base + ".tar.gz"), "w:gz") as archive:
                        info = tarfile.TarInfo(binary)
                        if case == "symlink": info.type = tarfile.SYMTYPE; info.linkname = "/etc/passwd"
                        archive.addfile(info)
                        if case == "duplicate": archive.addfile(info)
                with self.assertRaises(ValueError):
                    release.manifest(directory, VERSION, "amd64", COMMIT, {"flatpak": False, "zsync": False}, True)

    def test_optional_inventory_and_zsync_target(self):
        with tempfile.TemporaryDirectory() as tmp:
            directory = Path(tmp)
            base = fixture(directory)
            with self.assertRaises(ValueError):
                release.manifest(directory, VERSION, "amd64", COMMIT, {"flatpak": True, "zsync": False}, True)
            (directory / (base + ".AppImage.zsync")).write_text("Filename: wrong.AppImage\n\n")
            with self.assertRaises(ValueError):
                release.manifest(directory, VERSION, "amd64", COMMIT, {"flatpak": False, "zsync": True}, True)

    def test_transfer_preserves_modes_and_rejects_traversal_links_duplicates(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            directory = root / "source"
            directory.mkdir()
            base = fixture(directory)
            release.manifest(directory, VERSION, "amd64", COMMIT, {"flatpak": False, "zsync": False}, True)
            transfer = root / "transfer.tar"
            with tarfile.open(transfer, "w:") as archive:
                for path in directory.iterdir(): archive.add(path, arcname=path.name)
            target = root / "target"
            release.extract(transfer, target, VERSION, "amd64")
            release.verify(target, VERSION, ["amd64"], COMMIT)
            (target / base).chmod(0o644)
            (target / (base + ".AppImage")).chmod(0o644)
            # HTTP downloads have no mode bits; embedded tar modes remain mandatory.
            release.verify(target, VERSION, ["amd64"], COMMIT, check_modes=False)
            for case in ("traversal", "symlink", "duplicate"):
                with self.subTest(case=case), tarfile.open(transfer, "w:") as archive:
                    info = tarfile.TarInfo("../escape" if case == "traversal" else base)
                    info.size = 1
                    if case == "symlink": info.type = tarfile.SYMTYPE; info.linkname = "/etc/passwd"
                    archive.addfile(info, io.BytesIO(b"x"))
                    if case == "duplicate": archive.addfile(info, io.BytesIO(b"x"))
                with self.assertRaises(ValueError): release.extract(transfer, root / case, VERSION, "amd64")


class NativeAndSourceVerification(unittest.TestCase):
    def test_missing_native_prerequisite_fails_before_any_cargo_process(self):
        with patch.object(native.shutil, "which", return_value=None), patch.object(native.subprocess, "check_output") as cargo:
            with self.assertRaisesRegex(SystemExit, "Missing native test prerequisites"):
                native.main()
            cargo.assert_not_called()

    def test_inventory_is_dynamic_and_rejects_unclassified_or_missing_locale(self):
        listing = native.LOCALE + ": test\n" + "\n".join(f"ui::tests::display_{i}: test" for i in range(9))
        self.assertEqual(len(native.native_tests(listing)), 9)
        for invalid in ("", listing + "\nother::ignored: test", listing + "\n" + native.LOCALE + ": test"):
            with self.assertRaises(ValueError): native.native_tests(invalid)

    def test_native_app_lifecycle_tests_are_classified_without_accepting_other_app_tests(self):
        listing = native.LOCALE + ": test\napp::tests::native_close_waits_for_saves: test\n"
        self.assertEqual(native.native_tests(listing), ["app::tests::native_close_waits_for_saves"])
        with self.assertRaises(ValueError):
            native.native_tests(listing + "app::tests::ignored_non_native: test\n")

    def test_fresh_cargo_display_session_and_xdg_for_each_test(self):
        listing = native.LOCALE + ": test\nui::tests::first: test\nui::tests::second: test\n"
        calls = []
        def record(command, **kwargs):
            self.assertEqual(command.count("cargo"), 1)
            self.assertIn("xvfb-run", command)
            self.assertIn("dbus-run-session", command)
            bus_config = Path(command[command.index("--config-file") + 1]).read_text()
            self.assertNotIn("servicedir", bus_config)
            self.assertIn("unix:abstract=scenedeck-native-", bus_config)
            self.assertEqual(kwargs["env"]["GIO_USE_VFS"], "local")
            self.assertIn("--exact", command)
            self.assertEqual(command[-3:], ["--ignored", "--test-threads=1", "--nocapture"])
            self.assertTrue(Path(kwargs["env"]["XDG_RUNTIME_DIR"]).is_dir())
            calls.append(kwargs["env"]["XDG_RUNTIME_DIR"])
        with patch.object(native.shutil, "which", return_value="/fixture"), patch.object(native.subprocess, "check_output", return_value=listing), patch.object(native.subprocess, "run", side_effect=record):
            native.main()
        self.assertEqual(len(set(calls)), 2)
        self.assertTrue(all(not Path(path).exists() for path in calls))

    def test_source_rejects_malformed_mismatched_or_moved_identity(self):
        with tempfile.TemporaryDirectory() as tmp, contextlib.chdir(tmp):
            Path("Cargo.toml").write_text(f'[package]\nname="scenedeck"\nversion="{VERSION}"\n')
            Path("data").mkdir()
            Path("data/io.scenedeck.app.metainfo.xml").write_text(f'<component><releases><release version="{VERSION}"/></releases></component>')
            with patch.object(source, "git", return_value=COMMIT):
                self.assertEqual(source.verify(COMMIT, "v" + VERSION), (VERSION, COMMIT))
                with self.assertRaises(ValueError): source.verify("short")
                with self.assertRaises(ValueError): source.verify("b" * 40)
                with self.assertRaises(ValueError): source.verify(COMMIT, "v1.2.3;echo injection")
            with patch.object(source, "git", side_effect=[COMMIT, f'{"b" * 40}\trefs/tags/v{VERSION}']):
                with self.assertRaises(ValueError): source.verify(COMMIT, "v" + VERSION, True)
            with patch.object(source, "git", side_effect=[COMMIT, f'{"b" * 40}\trefs/tags/v{VERSION}\n{COMMIT}\trefs/tags/v{VERSION}^{{}}']):
                self.assertEqual(source.verify(COMMIT, "v" + VERSION, True), (VERSION, COMMIT))


class SnapVerification(unittest.TestCase):
    def test_manifest_exact_pair_and_tampering(self):
        with tempfile.TemporaryDirectory() as tmp:
            directory = Path(tmp)
            for arch in snap.MACHINES:
                arch_dir = directory / arch
                arch_dir.mkdir()
                (arch_dir / f"scenedeck_{VERSION}_{arch}.snap").write_bytes(b"Snap checksum fixture")
                with patch.object(snap, "inspect"):
                    snap.inventory(arch_dir, VERSION, [arch], COMMIT, True)
                for path in arch_dir.iterdir(): path.rename(directory / path.name)
                arch_dir.rmdir()
            with patch.object(snap, "inspect"):
                snap.inventory(directory, VERSION, list(snap.MACHINES), COMMIT)
                (directory / f"scenedeck_{VERSION}_amd64.snap").write_bytes(b"tampered")
                with self.assertRaises(ValueError): snap.inventory(directory, VERSION, list(snap.MACHINES), COMMIT)

    def test_snap_metadata_and_elf_rejections(self):
        metadata = {"name": "scenedeck", "version": VERSION, "architectures": ["amd64"], "confinement": "strict",
                    "base": "core24", "apps": {"scenedeck": {"command": "bin/scenedeck"}}}
        import yaml
        for field, value in [(None, None), ("name", "other"), ("version", "0.0.1"), ("architectures", ["arm64"]), ("confinement", "classic")]:
            changed = copy.deepcopy(metadata)
            if field: changed[field] = value
            with self.subTest(field=field), patch.object(snap.subprocess, "check_output", side_effect=[yaml.safe_dump(changed).encode(), elf("amd64")]):
                if field:
                    with self.assertRaises(ValueError): snap.inspect(Path("fixture.snap"), VERSION, "amd64")
                else: snap.inspect(Path("fixture.snap"), VERSION, "amd64")
        with patch.object(snap.subprocess, "check_output", side_effect=[yaml.safe_dump(metadata).encode(), elf("arm64")]):
            with self.assertRaises(ValueError): snap.inspect(Path("fixture.snap"), VERSION, "amd64")

    def test_store_downgrade_and_exact_readback(self):
        data = {"name": "scenedeck", "channel-map": []}
        with tempfile.TemporaryDirectory() as tmp:
            directory = Path(tmp)
            for arch in snap.MACHINES:
                path = directory / f"scenedeck_{VERSION}_{arch}.snap"
                path.write_bytes(arch.encode())
                data["channel-map"].append({"channel": {"track": "latest", "risk": "stable", "architecture": arch},
                                            "version": VERSION, "download": {"sha3-384": snap.digest(path, "sha3_384"), "size": path.stat().st_size}})
            snap.verify_store(data, "stable", VERSION)
            snap.verify_store(data, "stable", VERSION, directory)
            with self.assertRaises(ValueError): snap.verify_store(data, "stable", "1.2.2")
            changed = copy.deepcopy(data)
            changed["channel-map"][0]["download"]["sha3-384"] = "bad"
            with self.assertRaises(ValueError): snap.verify_store(changed, "stable", VERSION, directory)
            with self.assertRaises(ValueError): snap.verify_store(data, "edge", VERSION, directory)


if __name__ == "__main__":
    unittest.main()
