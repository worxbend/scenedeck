"""Static workflow contracts; these checks never build, upload or publish anything."""
import re
import subprocess
import tomllib
import unittest
from pathlib import Path

import yaml

ROOT = Path(__file__).resolve().parents[1]


def workflow(name):
    # YAML 1.1's SafeLoader treats `on` as a bool; BaseLoader keeps Actions keys.
    return yaml.load((ROOT / ".github/workflows" / name).read_text(), Loader=yaml.BaseLoader)


class WorkflowContracts(unittest.TestCase):
    def test_pins_timeouts_credentials_permissions_and_shell_syntax(self):
        for path in (ROOT / ".github/workflows").glob("*.yml"):
            parsed = workflow(path.name)
            self.assertEqual(parsed["permissions"], {"contents": "read"}, path.name)
            for name, job in parsed["jobs"].items():
                if "uses" in job:
                    self.assertNotIn("secrets", job, f"{path.name}:{name}")
                    continue
                self.assertIn("timeout-minutes", job, f"{path.name}:{name}")
                for step in job["steps"]:
                    if "uses" in step:
                        self.assertRegex(step["uses"], r"^[\w.-]+/[\w.-]+@[0-9a-f]{40}$")
                        if step["uses"].startswith("actions/checkout@"):
                            self.assertEqual(step["with"]["persist-credentials"], "false")
                    if "run" in step:
                        self.assertNotIn("${{", step["run"], f"shell interpolation in {path.name}:{name}")
                        subprocess.run(["bash", "-n"], input=step["run"], text=True, check=True)

    def test_release_and_snap_require_all_reusable_quality_gates(self):
        ci = workflow("ci.yml")
        self.assertEqual(ci["on"]["workflow_call"]["inputs"]["commit"]["required"], "true")
        commands = "\n".join(step.get("run", "") for job in ci["jobs"].values() for step in job["steps"])
        for command in ["cargo fmt --all -- --check", "cargo clippy --workspace --all-targets --all-features --locked -- -D warnings",
                        "cargo test --workspace --all-features --locked\n", "cargo audit",
                        "cargo deny --locked --all-features check advisories bans licenses sources", "python3 tools/test-native.py"]:
            self.assertIn(command, commands)
        self.assertIn("infra::i18n::tests::every_shipped_locale_parses_and_loads -- --exact --ignored --test-threads=1", commands)
        self.assertNotIn("RUSTSEC-2026-0173", commands)
        self.assertIn("schedule", ci["on"])
        for filename, build, publish in [("release.yml", "build-linux", "publish-release"), ("snap.yml", "build", "publish")]:
            jobs = workflow(filename)["jobs"]
            self.assertEqual(jobs["validate"]["uses"], "./.github/workflows/ci.yml")
            self.assertEqual(jobs["validate"]["with"]["commit"], "${{ needs.prepare.outputs.commit }}")
            for job in [build, publish]:
                self.assertIn("validate", jobs[job]["needs"])
                self.assertIn("prepare", jobs[job]["needs"])
            self.assertIn(build, jobs[publish]["needs"])
            self.assertEqual({row["runner"] for row in jobs[build]["strategy"]["matrix"]["include"]}, {"ubuntu-24.04", "ubuntu-24.04-arm"})
            self.assertNotIn("permissions", jobs[build])
            self.assertNotIn("secrets.", str(jobs[build]))
        self.assertIn("github.event_name == 'push'", workflow("release.yml")["jobs"]["publish-release"]["if"])
        self.assertIn("github.event_name != 'pull_request'", workflow("snap.yml")["jobs"]["publish"]["if"])

    def test_dependency_exception_check_cannot_be_skipped_before_scanners(self):
        ci = workflow("ci.yml")
        dependencies = ci["jobs"]["dependencies"]
        self.assertNotIn("if", dependencies)
        self.assertNotIn("continue-on-error", dependencies)
        steps = dependencies["steps"]
        runs = [step.get("run", "") for step in steps]
        command = "python3 tools/check-advisory-exceptions.py"
        self.assertEqual(runs.count(command), 1, "dependency CI must enforce UTC exception expiry")
        checked = runs.index(command)
        for key in ["if", "continue-on-error", "env", "working-directory"]:
            self.assertNotIn(key, steps[checked])
        for scanner in ["cargo audit", "cargo deny --locked --all-features check advisories bans licenses sources"]:
            scanned = runs.index(scanner)
            self.assertLess(checked, scanned)
            self.assertNotIn("continue-on-error", steps[scanned])
            self.assertNotIn("if", steps[scanned])
        for filename in ["release.yml", "snap.yml"]:
            self.assertEqual(workflow(filename)["jobs"]["validate"]["uses"], "./.github/workflows/ci.yml")

    def test_site_validation_gates_ci_and_pages_before_upload(self):
        ci = workflow("ci.yml")
        self.assertIn("pull_request", ci["on"])
        self.assertIn("workflow_call", ci["on"])
        site = ci["jobs"]["site"]
        checkout = next(step for step in site["steps"] if step.get("uses", "").startswith("actions/checkout@"))
        self.assertEqual(checkout["with"]["ref"], "${{ inputs.commit || github.sha }}")
        pages = workflow("pages.yml")
        self.assertEqual(pages["jobs"]["deploy"]["needs"], "build")
        for name, job in [("ci:site", site), ("pages:build", pages["jobs"]["build"])]:
            self.assertNotIn("permissions", job, name)  # inherit read-only workflow token
            self.assertNotIn("secrets.", str(job), name)
            setup = next(step for step in job["steps"] if step.get("uses", "").startswith("actions/setup-node@"))
            self.assertEqual(setup["with"]["node-version"], "22")
            for command in ["npm ci --ignore-scripts", "npm test", "npm run check", "npm audit"]:
                step = next(step for step in job["steps"] if step.get("run") == command)
                self.assertEqual(step["working-directory"], "site/tools", name)
                self.assertNotIn("continue-on-error", step, name)
        steps = pages["jobs"]["build"]["steps"]
        runs = [step.get("run", "") for step in steps]
        stamp = runs.index("site/tools/stamp-release.sh")
        checked = runs.index('node site/tools/check.mjs --build "$(git rev-parse HEAD)"')
        strip = next(i for i, run in enumerate(runs) if run.startswith("rm -rf site/tools"))
        upload = next(i for i, step in enumerate(steps) if step.get("uses", "").startswith("actions/upload-pages-artifact@"))
        for command in ["npm ci --ignore-scripts", "npm test", "npm run check", "npm audit"]:
            self.assertLess(runs.index(command), stamp)
        self.assertLess(stamp, checked)
        self.assertLess(checked, strip)
        self.assertLess(strip, upload)
        self.assertNotIn("continue-on-error", steps[checked])

    def test_dependabot_monitors_locked_site_tools(self):
        config = yaml.safe_load((ROOT / ".github/dependabot.yml").read_text())
        npm = [entry for entry in config["updates"] if entry["package-ecosystem"] == "npm"]
        self.assertEqual(len(npm), 1)
        self.assertEqual(npm[0]["directory"], "/site/tools")
        self.assertEqual(npm[0]["schedule"]["interval"], "weekly")

    def test_latest_promotion_is_serialized_across_tags_and_never_implicit(self):
        jobs = workflow("release.yml")["jobs"]
        publish = jobs["publish-release"]
        commands = "\n".join(step.get("run", "") for step in publish["steps"])
        self.assertNotIn("--latest=true", commands)
        self.assertIn("--draft --prerelease=false --latest=false", commands)
        self.assertIn("--draft=false --latest=false", commands)
        promote = jobs["promote-latest"]
        self.assertEqual(promote["concurrency"], {"group": "release-latest", "cancel-in-progress": "false"})
        self.assertEqual(set(promote["needs"]), {"prepare", "validate", "publish-release"})
        self.assertIn("github.event_name == 'push'", promote["if"])
        self.assertEqual(promote["permissions"], {"contents": "write"})
        self.assertIn('python3 tools/promote-latest.py --repository "$GITHUB_REPOSITORY" --tag "$GITHUB_REF_NAME"',
                      "\n".join(step.get("run", "") for step in promote["steps"]))

    def test_flatpak_native_inspection_dependencies_are_available_for_release_readback(self):
        jobs = workflow("release.yml")["jobs"]
        for name in ["build-linux", "publish-release"]:
            commands = "\n".join(step.get("run", "") for step in jobs[name]["steps"])
            self.assertIn("flatpak", commands)
            self.assertIn("ostree", commands)

    def test_toolchain_and_snap_bootstrap_pin_consistency(self):
        channel = tomllib.loads((ROOT / "rust-toolchain.toml").read_text())["toolchain"]["channel"]
        self.assertEqual(channel, "1.99.1")
        for filename in ("ci.yml", "release.yml"):
            for job in workflow(filename)["jobs"].values():
                for step in job.get("steps", []):
                    if step.get("uses", "").startswith("dtolnay/rust-toolchain@"):
                        self.assertEqual(step["with"]["toolchain"], channel)
        recipe = yaml.safe_load((ROOT / "snap/snapcraft.yaml").read_text())
        bootstrap = recipe["parts"]["rust-deps"]["override-pull"]
        self.assertIn("--default-toolchain " + channel, bootstrap)
        self.assertNotIn("sh.rustup.rs", bootstrap)
        self.assertNotIn("build-snaps", recipe["parts"]["rust-deps"])
        self.assertEqual(len(re.findall(r"checksum=[0-9a-f]{64}", bootstrap)), 2)
        self.assertLess(bootstrap.index("sha256sum --check --strict"), bootstrap.index("./rustup-init -y"))
        # snapcraft's rust plugin adds --locked itself; a second copy is an error.
        self.assertNotIn("--locked", recipe["parts"]["scenedeck"]["rust-cargo-parameters"])
        for part in recipe["parts"].values():
            for key, script in part.items():
                if key.startswith("override-"):
                    subprocess.run(["bash", "-n"], input=script, text=True, check=True)


if __name__ == "__main__":
    unittest.main()
