# Configuration

SceneDeck stores local settings under the XDG config directory.

## Config File

Default path:

```text
$XDG_CONFIG_HOME/scenedeck/config.json
```

Fallback path:

```text
$HOME/.config/scenedeck/config.json
```

Current schema version: `1`.

Example:

```json
{
  "version": 1,
  "obs": {
    "host": "127.0.0.1",
    "port": 4455
  },
  "live": {
    "show_roles": ["primary"],
    "audio_inputs": [],
    "allow_switching_only": ["primary"]
  },
  "theme_mode": "system"
}
```

Fields:

- `obs.host`: OBS WebSocket host.
- `obs.port`: OBS WebSocket port.
- `live.show_roles`: roles intended for Live display. Current Live filtering is
  driven by the local registry role semantics.
- `live.audio_inputs`: optional allow-list of OBS input names for scene audio.
  Empty means discover all active scene audio inputs.
- `live.allow_switching_only`: roles intended for scene switching. Current Live
  switching uses scenes whose role is `Primary`.
- `theme_mode`: `system`, `light`, or `dark`.

Unknown or missing fields fall back through Serde defaults. If the app cannot
read or parse the config, it starts with defaults and reports a startup notice.

## Registry File

Default path:

```text
$XDG_CONFIG_HOME/scenedeck/registry.json
```

Fallback path:

```text
$HOME/.config/scenedeck/registry.json
```

The registry stores local metadata for OBS scenes.

Example:

```json
{
  "scenes": {
    "Main": {
      "role": "primary",
      "tags": [],
      "protected": false
    },
    "Camera Frame": {
      "role": "module",
      "tags": ["camera"],
      "protected": true
    }
  },
  "rules": {
    "primary_can_depend_on": [],
    "module_can_depend_on": [],
    "forbidden_edges": []
  }
}
```

Roles:

- `primary`: live-switchable scene.
- `secondary`: valid scene hidden from Live by default.
- `module`: reusable nested scene.
- `raw`: hardware or source wrapper scene.
- `debug`: temporary test scene.
- `archive`: preserved but excluded from workflows.

The UI currently edits scene roles and stale entries. Tags, protected flags, and
custom rule fields are available in the file for deeper workflows and Doctor
logic.

## YAML Import and Export

Inventory can export the scene registry to YAML and import it back from YAML.
The YAML structure mirrors `registry.json`, so it preserves scenes, roles, tags,
protected flags, and rule fields.

Example:

```yaml
scenes:
  Main:
    role: primary
    tags:
      - live
    protected: false
rules:
  primary_can_depend_on: []
  module_can_depend_on: []
  forbidden_edges:
    - [primary, debug]
```

Import replaces the local registry file with the parsed YAML content.

## Secrets

The OBS password is stored through the system Secret Service keyring. It is not
written to `config.json` or `registry.json`.

If keyring access fails, SceneDeck reports the error in Settings and logs a
warning.
