# Flatpak packaging

SceneDeck builds as a GNOME-runtime Flatpak.

## Prerequisites

```sh
flatpak install flathub org.gnome.Platform//47 org.gnome.Sdk//47 \
    org.freedesktop.Sdk.Extension.rust-stable//24.08
pip install aiohttp toml   # for the cargo source generator
```

## 1. Generate the offline Cargo source manifest

Flathub builds run offline, so dependencies must be vendored into a
`cargo-sources.json` generated from `Cargo.lock`:

```sh
# One-time: fetch the generator
curl -O https://raw.githubusercontent.com/flatpak/flatpak-builder-tools/master/cargo/flatpak-cargo-generator.py

python flatpak-cargo-generator.py ../../Cargo.lock -o cargo-sources.json
```

Re-run this whenever `Cargo.lock` changes.

## 2. Build and install

```sh
flatpak-builder --user --install --force-clean build io.scenedeck.app.yml
```

## 3. Run

```sh
flatpak run io.scenedeck.app
```

## Notes

- The OBS password is stored via the host Secret Service
  (`--talk-name=org.freedesktop.secrets`), never in the sandbox.
- `--share=network` is required for the OBS WebSocket connection.
- The runtime version (`47`) should track the GNOME platform used for
  development; bump it together with the libadwaita feature level.
