# Flatpak packaging

SceneDeck builds as a GNOME-runtime Flatpak.

## Prerequisites

```sh
flatpak install flathub org.gnome.Platform//50 org.gnome.Sdk//50 \
    org.freedesktop.Sdk.Extension.rust-stable
```

## 1. Vendor Cargo dependencies

The release workflow vendors Cargo dependencies before invoking
`flatpak-builder`. Use the same approach locally when testing the release
manifest:

```sh
cd ../..
mkdir -p .cargo
cargo vendor vendor > .cargo/config.toml
cd packaging/flatpak
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
- The runtime version (`50`) should track the GNOME platform used for
  development; bump it together with the libadwaita feature level.
