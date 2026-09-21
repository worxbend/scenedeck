## SceneDeck UI strings (German).
##
## Grouped by the module each message is used from. Message ids are prefixed
## with the module name to keep them unambiguous in this single shared file.

## Internal — used only by the i18n loader's own regression test, not shown
## in the UI. Every locale must define this so the smoke test can confirm the
## locale's bundle loaded (not just the `en` fallback).
i18n-loader-smoke-test = Lokalisierung geladen.

## infra/error.rs — user-facing renderings of AppError. `detail` is raw
## upstream text (often from OBS or the OS) and is never translated.
error-connection = OBS-Verbindung fehlgeschlagen: { $detail }
error-request = OBS-Anfrage fehlgeschlagen: { $detail }
error-config = Konfigurationsfehler: { $detail }
error-storage = Speicherfehler: { $detail }
error-notification-title = SceneDeck-Fehler: { $message }

## domain/audio.rs — AudioSourceScope::label()
audio-scope-global = Global
audio-scope-active = Szene
audio-scope-nested = Verschachtelt
audio-scope-group = Gruppe

## domain/graph.rs — EdgeStatus summary label/tooltip
edge-status-ok-label = OK
edge-status-warning-label = Warnung
edge-status-forbidden-label = Verboten
edge-status-ok-tooltip = Verbindungen, die der Graph-Richtlinie entsprechen
edge-status-warning-tooltip = Verbindungen außerhalb einer Positivliste
edge-status-forbidden-tooltip = Verbindungen, die durch die Graph-Richtlinie verboten sind

## domain/output.rs — OutputRunState::label(), OutputStatus::summary()
output-state-inactive = Inaktiv
output-state-starting = Wird gestartet
output-state-active = Aktiv
output-state-stopping = Wird gestoppt
output-state-reconnecting = Verbindung wird wiederhergestellt
output-state-paused = Pausiert
output-state-unknown = Unbekannt
output-summary = { $name }: { $state }

## domain/role.rs — SceneRole::label()/description()
role-primary = Primär
role-secondary = Sekundär
role-module = Modul
role-raw = Roh
role-debug = Debug
role-archive = Archiv
role-unassigned = Nicht zugewiesen
role-primary-desc = Live umschaltbare Szene
role-secondary-desc = Gültige Szene, standardmäßig auf der Live-Seite ausgeblendet
role-module-desc = Wiederverwendbare verschachtelte Szene, nicht direkt umschaltbar
role-raw-desc = Szene zur Kapselung von Hardware oder Quellen
role-debug-desc = Temporäre Testszene
role-archive-desc = Aufbewahrt, aber von allen Arbeitsabläufen ausgeschlossen

## domain/mixer.rs — MixerMode / MixerGrouping labels
mixer-mode-active = Aktiv
mixer-mode-selected = Ausgewählt
mixer-mode-pinned = Angeheftet
mixer-mode-active-desc = Folgt der Programmszene von OBS.
mixer-mode-selected-desc = Ausgewählte Szene prüfen, ohne OBS zu folgen.
mixer-mode-pinned-desc = Hält die ausgewählte Szene während des Betriebs stabil.
mixer-grouping-scope = Bereich
mixer-grouping-scene-path = Szenenpfad
mixer-grouping-none = Keine

## domain/diagnostic.rs — DiagnosticSeverity label/count
diag-label-info = Info
diag-label-warning = Warnungen
diag-label-error = Fehler
diag-count-info = { $count ->
    [one] { $count } Info-Eintrag
   *[other] { $count } Info-Einträge
}
diag-count-warning = { $count ->
    [one] { $count } Warnung
   *[other] { $count } Warnungen
}
diag-count-error = { $count ->
    [one] { $count } Fehler
   *[other] { $count } Fehler
}

## ui/pages/inventory.rs
inventory-no-role-assigned = Keine Rolle zugewiesen

## services/doctor_service.rs — diagnostic message/suggestion templates
doctor-no-role = Der Szene ist im lokalen Register keine Rolle zugewiesen.
doctor-no-role-suggestion = Öffnen Sie das Inventar und weisen Sie eine Rolle zu.
doctor-stale-entry = Der Registereintrag verweist auf eine Szene, die in OBS nicht gefunden wurde.
doctor-stale-entry-suggestion = Entfernen Sie den Eintrag aus dem Inventar.
doctor-protected-switchable = Die geschützte Szene hat die umschaltbare Rolle „{ $role }“.
doctor-protected-switchable-suggestion = Geschützte Szenen sind meist Bausteine; erwägen Sie Modul oder Roh.
doctor-cycle = Zirkulärer Szenenverweis zwischen „{ $parent }“ und „{ $child }“.
doctor-cycle-suggestion = Entfernen Sie die Schleife verschachtelter Szenen; OBS kann keine Zyklen darstellen.
doctor-edge-primary-debug = Eine Primärszene hängt von einer Debug-Szene ab. (→ „{ $child }“)
doctor-edge-primary-debug-suggestion = Entfernen Sie die Debug-Szene aus dem Live-Pfad, bevor Sie live gehen.
doctor-edge-primary-raw = Eine Primärszene kapselt direkt eine Roh-Quelle. (→ „{ $child }“)
doctor-edge-primary-raw-suggestion = Kapseln Sie die Roh-Quelle in einer Modul-Szene für Wiederverwendbarkeit und Übersichtlichkeit.
doctor-edge-module-primary = Ein Modul hängt von einer Primärszene ab und kehrt damit die Hierarchie um. (→ „{ $child }“)
doctor-edge-module-primary-suggestion = Module sollten Bausteine sein, keine Konsumenten von Primärszenen.
doctor-edge-raw-nests = Eine Roh-Szene verschachtelt eine andere Szene. (→ „{ $child }“)
doctor-edge-raw-nests-suggestion = Roh-Szenen sollten reine Quell-Wrapper ohne verschachtelte Szenen sein.
doctor-edge-forbidden = Die Szenenabhängigkeit ist durch die Graph-Richtlinie verboten. (→ „{ $child }“)
doctor-edge-outside-policy = Die Szenenabhängigkeit liegt außerhalb der konfigurierten Graph-Richtlinie. (→ „{ $child }“)
doctor-edge-adjust-suggestion = Passen Sie die Beziehung der verschachtelten Szene an oder aktualisieren Sie die Graph-Regeln im Register.

## controller/app_controller.rs
controller-not-connected = Nicht mit OBS verbunden

## controller/state.rs — Page titles and ObsStatus labels
page-live = Live
page-stats = Statistik
page-mixer = Mixer
page-graph = Graph
page-inventory = Inventar
page-doctor = Diagnose
page-settings = Einstellungen
obs-status-disconnected = Getrennt
obs-status-connecting = Verbindung wird hergestellt …
obs-status-connected = Verbunden
obs-status-error = Fehler

## storage/config.rs — ConfigStartupNotice
config-first-launch = Noch keine gespeicherten Einstellungen. Standardwerte werden geladen.
config-read-failed = Einstellungen konnten nicht gelesen werden: { $detail }
config-parse-failed = Einstellungen konnten nicht verarbeitet werden: { $detail }

## graph.rs

graph-empty-title = Keine Abhängigkeiten
graph-empty-description = Keine Szene verschachtelt andere Szenen, oder OBS ist nicht verbunden. Verbinden Sie sich und fügen Sie verschachtelte Szenenquellen hinzu, um den Abhängigkeitsgraphen zu sehen.
graph-page-title = Szenenabhängigkeiten
graph-reset-tooltip = Graph-Layout zurücksetzen
graph-edge-summary-count = { $label } { $count }

## mixer.rs

## Page chrome / empty state
mixer-empty-title = Keine Mixer-Daten
mixer-empty-description = Mit OBS verbinden, um Szenen und Audioquellen zu laden.
mixer-page-title = Mixer
mixer-controls-title = Mixer-Steuerung
mixer-summary-title = Aktuelle Mixer-Quelle

## Control rows (ComboRow / EntryRow titles+subtitles)
mixer-mode-row-title = Modus
mixer-mode-row-subtitle = Aktiv folgt OBS; Ausgewählt und Angeheftet halten die gewählte Szene stabil.
mixer-scene-row-title = Szene
mixer-scene-row-subtitle = Wird von den Modi Ausgewählt und Angeheftet verwendet.
mixer-grouping-row-title = Gruppieren nach
mixer-grouping-row-subtitle = Legt fest, wie die Audioquellen unten angeordnet werden.
mixer-search-row-title = Suche

## Scene-loading / no-scene placeholders
mixer-no-scene-title = Keine Szene ausgewählt
mixer-no-scene-description = Wählen Sie eine Szene aus, um deren Mixer-Audio zu laden.
mixer-loading-title = Mixer-Audio wird geladen
mixer-loading-description = Audioquellen für { $scene } werden geladen.

## Audio-source empty states
mixer-current-scene-fallback = Die aktuelle Szene
mixer-no-audio-sources-title = Keine Audioquellen
mixer-no-audio-sources-description = { $scene } hat keine passenden konfigurierten OBS-Audioquellen.
mixer-no-matching-title = Keine passenden Audioquellen
mixer-no-matching-description = Passen Sie den Suchfilter an, um verfügbare Audioquellen anzuzeigen.

## Group titles
mixer-group-all-sources = Alle Quellen
mixer-group-global-fallback = Global

## Error placeholder + retry
mixer-error-title = Mixer-Audio nicht verfügbar
mixer-error-description = Audioquellen für { $scene } konnten nicht geladen werden: { $message }
mixer-retry-button-label = Erneut versuchen
mixer-retry-button-tooltip = Laden des Mixer-Audios erneut versuchen

## Current-source summary row (source_summary / scene_target_summary)
mixer-summary-following-active = Folgt der aktiven OBS-Szene: { $scene }
mixer-summary-no-scene-selected = Keine Szene ausgewählt
mixer-summary-selected-scene = Ausgewählte Szene: { $scene }
mixer-summary-pinned-scene = Angeheftete Szene: { $scene }
mixer-summary-selected-fallback = Keine Szene ausgewählt; verwende aktive OBS-Szene: { $scene }
mixer-summary-pinned-selected-fallback = Keine Szene angeheftet; verwende ausgewählte Szene: { $scene }
mixer-summary-pinned-active-fallback = Keine angeheftete oder ausgewählte Szene; verwende aktive OBS-Szene: { $scene }

## doctor.rs — page chrome, empty state, and all-clear text for the Doctor
## page. Diagnostic finding messages themselves (`doctor-no-role`,
## `doctor-cycle*`, `doctor-edge-*`, etc.) already exist in
## src/services/doctor_service.rs and are NOT redefined here.
doctor-page-title = Diagnose
doctor-empty-state-title = Nichts zu prüfen
doctor-empty-state-description = Mit OBS verbinden, um die Architekturdiagnose auszuführen.
doctor-summary-row-title = Diagnose
doctor-rerun-tooltip = Diagnose erneut ausführen
doctor-all-clear-title = Keine Probleme gefunden
doctor-all-clear-detail = Die Szenenarchitektur erfüllt alle Prüfungen.

## inventory.rs — page chrome, group titles, YAML import/export UI, and
## status messages. Role names/descriptions themselves are NOT redefined
## here — they already exist in src/domain/role.rs (`role-*`,
## `role-*-desc`, `role-unassigned`) and are reused directly via
## SceneRole::label()/description()/unassigned_label(). The already-present
## `inventory-no-role-assigned` message is also left untouched.
inventory-page-title = Inventar
inventory-empty-state-title = Keine Szenen
inventory-empty-state-description = Mit OBS verbinden, um die Szenenliste zu laden.
inventory-scenes-group-title = OBS-Szenen
inventory-scenes-group-description = Weisen Sie Rollen zu, um zu steuern, welche Szenen auf der Live-Seite erscheinen.
inventory-stale-group-title = Veraltete Registereinträge
inventory-stale-group-description = Diese Szenen befinden sich in Ihrem lokalen Register, existieren aber in OBS nicht mehr.
inventory-remove-stale-tooltip = Veralteten Eintrag entfernen
inventory-yaml-row-title = Szenenregister-YAML
inventory-yaml-row-subtitle = Szenenrollen, Tags, Schutzkennzeichen und Graph-Regeln exportieren oder importieren.
inventory-yaml-filter-name = YAML-Dateien
inventory-accent-clear-tooltip = Akzentfarbe der Szene entfernen
inventory-drag-tooltip = Ziehen, um die Szene neu anzuordnen
inventory-accent-dialog-title = Akzentfarbe der Szene
inventory-accent-choose-tooltip = Akzentfarbe der Szene wählen

# Shared between the row's Export button, the export FileChooserNative's
# accept label, and (for Cancel) both the export and import dialogs.
inventory-export-button-label = Exportieren
inventory-export-tooltip = Szenenregister als YAML exportieren
inventory-import-button-label = Importieren
inventory-import-tooltip = Szenenregister aus YAML importieren
inventory-dialog-cancel-label = Abbrechen

inventory-export-dialog-title = Szenenregister exportieren
inventory-export-success = Szenenregister nach { $path } exportiert.
inventory-export-error = Export fehlgeschlagen: { $error }
inventory-export-no-file = Export fehlgeschlagen: Es wurde keine Datei ausgewählt.

inventory-import-dialog-title = Szenenregister importieren
inventory-import-error = Import fehlgeschlagen: { $error }
inventory-import-no-file = Import fehlgeschlagen: Es wurde keine Datei ausgewählt.

## window.rs

window-stream-live-tooltip = Live-Stream aktiv
window-about-tooltip = Über SceneDeck
window-refresh-tooltip = Aktuelle Seite aktualisieren

window-stream-status-line = Stream: { $state }{ $elapsed }
window-record-status-line = Aufnahme: { $state }{ $elapsed }

window-status-connecting = Verbindung zu OBS wird hergestellt …
window-connect-btn-connecting = Verbindung wird hergestellt …
window-current-scene-none = Aktuelle Szene: —
window-status-connected = Verbunden — OBS { $version }
window-connect-btn-disconnect = Trennen
window-status-disconnected = Getrennt
window-connect-btn-connect = Mit OBS verbinden
window-live-disconnected-hint = Mit OBS verbinden, um die Live-Steuerung zu nutzen
window-current-scene = Aktuelle Szene: { $scene }
window-status-error = Fehler: { $error }
window-connect-btn-retry = Erneut versuchen
window-obs-connection-failed = OBS-Verbindung fehlgeschlagen
window-toast-obs-error = OBS-Fehler: { $error }

window-output-kind-stream = Stream
window-output-kind-record = Aufnahme

window-sidebar-output-starting = Wird gestartet …
window-sidebar-output-stopping = Wird gestoppt …
window-sidebar-output-reconnecting = Verbindung wird wiederhergestellt …
window-sidebar-output-working = Wird verarbeitet …

window-sidebar-start-stream = Stream starten
window-sidebar-stop-stream = Stream stoppen
window-sidebar-start-recording = Aufnahme starten
window-sidebar-stop-recording = Aufnahme stoppen

window-selector-profile-label = Profil
window-selector-profile-tooltip = OBS-Profil wechseln
window-selector-collection-label = Sammlung
window-selector-collection-tooltip = OBS-Szenensammlung wechseln

## live.rs

live-start-stream-label = Stream starten
live-stop-stream-label = Stream stoppen
live-start-record-label = Aufnahme starten
live-stop-record-label = Aufnahme stoppen
live-stream-toggle-tooltip = Stream starten oder stoppen
live-record-toggle-tooltip = Aufnahme starten oder stoppen
live-stream-inactive-label = Stream: Inaktiv
live-record-inactive-label = Aufnahme: Inaktiv
live-copy-last-recording-path-tooltip = Pfad der letzten Aufnahme kopieren
live-copied-recording-path-tooltip = Pfad der letzten Aufnahme kopiert
live-copy-recording-path-with-value-tooltip = Aufnahmepfad kopieren: { $path }
live-stream-card-title = Stream
live-recording-card-title = Aufnahme
live-current-scene-placeholder = Aktuelle Szene: —
live-scenes-section-label = Szenen
live-scenes-connect-hint = Mit OBS verbinden, um Szenen zu laden.
live-audio-section-label = Audio
live-disconnected-title = Mit OBS verbinden, um die Live-Steuerung zu nutzen
live-disconnected-detail = Verwenden Sie die Verbindungssteuerung am unteren Rand der Seitenleiste.
live-stream-command-error-label = Stream-Befehl fehlgeschlagen
live-recording-command-error-label = Aufnahme-Befehl fehlgeschlagen
live-last-recording-detail = Letzte Aufnahme: { $path }
live-starting-stream = Stream wird gestartet …
live-stopping-stream = Stream wird gestoppt …
live-reconnecting-stream = Stream-Verbindung wird wiederhergestellt …
live-starting-recording = Aufnahme wird gestartet …
live-stopping-recording = Aufnahme wird gestoppt …
live-reconnecting-recording = Aufnahme-Verbindung wird wiederhergestellt …
live-button-starting = Wird gestartet …
live-button-stopping = Wird gestoppt …
live-button-reconnecting = Verbindung wird wiederhergestellt …
live-button-working = Wird verarbeitet …
live-output-kind-stream = Stream
live-output-kind-record = Aufnahme
live-output-label = { $kind }: { $state }
live-output-label-with-elapsed = { $kind }: { $state } · { $elapsed }
live-scenes-no-primary-hint = Keine Szenen mit der Rolle Primär gefunden. Weisen Sie Rollen im Inventar zu.
live-audio-empty-hint = Keine Audioeingänge konfiguriert.
live-cancel-button-label = Abbrechen
live-start-stream-confirm-heading = Stream starten?
live-start-stream-confirm-body = OBS beginnt mit dem Senden des Live-Streams.
live-stop-stream-confirm-heading = Stream stoppen?
live-stop-stream-confirm-body = OBS beendet das Senden des Live-Streams.
live-start-recording-confirm-heading = Aufnahme starten?
live-start-recording-confirm-body = OBS startet eine neue Aufnahme.
live-start-recording-confirm-label = Aufnahme starten
live-stop-recording-confirm-heading = Aufnahme stoppen?
live-stop-recording-confirm-body = OBS stoppt die aktuelle Aufnahme.
live-stop-recording-confirm-label = Aufnahme stoppen

## audio_card.rs
audio-card-mute-tooltip = Eingang stummschalten
audio-card-source-path-tooltip = { $scope }: { $path }
audio-card-fader-tooltip = Lautstärkeregler
audio-card-lock-tooltip = Lautstärkeregler sperren
audio-card-fine-plus-tooltip = +1 dB
audio-card-fine-reset-tooltip = Auf 0,0 dB zurücksetzen
audio-card-fine-minus-tooltip = -1 dB
audio-card-meter-tooltip-title = Aussteuerungsanzeige: { $channels }
audio-card-meter-tooltip-zones = Grün unter -20 dB · Gelb bis -9 dB · Rot darüber, nahe der Übersteuerung
audio-card-meter-tooltip-indicators = Balken: Spitzenpegel mit Abfall · Linie: lauteste Spitze in 20 s · Punkt: Lautheit · Fuß: Pegel vom Gerät
audio-card-meter-tooltip-waiting = Aussteuerungsanzeige: warte auf OBS-Pegel

## icon.rs
icon-none = Kein Symbol
icon-camera = Kamera
icon-desktop = Desktop
icon-game = Spiel
icon-film = Film
icon-images = Bilder
icon-television = Fernsehen
icon-browser = Browser
icon-terminal = Terminal
icon-code = Code
icon-chat = Chat
icon-guests = Gäste
icon-star = Stern
icon-alert = Hinweis
icon-break = Pause
icon-countdown = Countdown
icon-start = Start
icon-pause = Anhalten
icon-stop = Stopp
icon-settings = Einstellungen
icon-layers = Ebenen
icon-microphone = Mikrofon
icon-headset = Headset
icon-headphones = Kopfhörer
icon-speaker = Lautsprecher
icon-volume = Lautstärke
icon-music = Musik
icon-instrument = Instrument
icon-radio = Radio
icon-call = Anruf
icon-waveform = Wellenform
inventory-scene-icon-tooltip = Symbol für diese Szene wählen
mixer-input-icon-tooltip = Symbol für diese Audioquelle wählen

## meter.rs
audio-meter-zone-nominal = Grün
audio-meter-zone-warning = Gelb
audio-meter-zone-error = Rot
audio-meter-channel-mono = Mono
audio-meter-channel-left = Links
audio-meter-channel-right = Rechts
audio-meter-channel-front-left = Vorne links
audio-meter-channel-front-right = Vorne rechts
audio-meter-channel-front-center = Vorne Mitte
audio-meter-channel-lfe = LFE
audio-meter-channel-rear-left = Hinten links
audio-meter-channel-rear-right = Hinten rechts
audio-meter-channel-side-left = Seite links
audio-meter-channel-side-right = Seite rechts
audio-meter-channel-numbered = Kanal { $index }

## hotkey.rs
hotkey-modifier-ctrl = Strg+
hotkey-modifier-alt = Alt+
hotkey-modifier-shift = Umschalt+
hotkey-modifier-super = Super+
hotkey-modifier-ctrl-alt = Strg+Alt+
hotkey-modifier-ctrl-shift = Strg+Umschalt+
hotkey-style-plain = Nur Ziffer
hotkey-style-ctrl = Strg + Ziffer
hotkey-style-alt = Alt + Ziffer
hotkey-style-shift = Umschalt + Ziffer
hotkey-style-super = Super + Ziffer
hotkey-style-ctrl-alt = Strg+Alt + Ziffer
hotkey-style-ctrl-shift = Strg+Umschalt + Ziffer
hotkey-style-leader = Leader-Taste, dann Ziffer
hotkey-leader-space = Leertaste
hotkey-leader-comma = Komma
hotkey-leader-semicolon = Semikolon
hotkey-leader-backslash = Backslash
hotkey-leader-grave = Backtick
hotkey-shortcut-leader = { $leader } dann { $digit }
hotkey-hint-plain = 1 … 0 drücken
hotkey-hint-modifier = { $modifier }1 … 0 drücken
hotkey-hint-leader = { $leader } drücken, dann 1 … 0
hotkey-hint-leader-armed = Leader aktiv — 1 … 0 drücken
hotkey-hint-empty-slot = Keine Szene auf Platz { $slot }

## scene_card.rs
scene-card-tooltip = { $status } ({ $role })
scene-card-tooltip-with-hotkey = { $status } ({ $role }) · { $hotkey }
scene-card-role-suffix = { $role }-Szene
scene-card-tooltip-active = Aktuelle Programmszene
scene-card-tooltip-previous = Zuvor aktive Szene
scene-card-tooltip-ready = Zu dieser Szene wechseln
scene-card-status-active = Aktiv
scene-card-status-previous = Vorherige
scene-card-status-ready = Bereit
scene-card-marker-active = An
scene-card-marker-previous = Zuletzt

## status_bar.rs
status-bar-stream-inactive = Stream: Inaktiv
status-bar-record-inactive = Aufnahme: Inaktiv
status-bar-fps-placeholder = FPS —
status-bar-cpu-placeholder = CPU —
status-bar-bitrate-placeholder = Bitrate —
status-bar-fps = FPS { $value }
status-bar-cpu = CPU { $value } %
status-bar-bitrate = Bitrate { $value } kbps
status-bar-dropped = { $count } verworfen
status-bar-dropped-placeholder = Verworfen —

## settings.rs

settings-page-title = Einstellungen
settings-appearance-title = Erscheinungsbild
settings-appearance-description = GNOME-Anwendungen sollten standardmäßig dem Systemstil folgen.
settings-theme-mode-system = System
settings-theme-mode-light = Hell
settings-theme-mode-dark = Dunkel
settings-color-scheme-title = Farbschema
settings-color-scheme-subtitle = Der Systemeinstellung folgen oder Hell/Dunkel erzwingen
settings-motion-title = Bewegung
settings-motion-subtitle = Wie stark die Oberfläche animiert wird. Live-Anzeigen bleiben auf jeder Stufe lesbar.
settings-motion-full = Vollständig
settings-motion-reduced = Reduziert
settings-motion-off = Aus
settings-theme-title = Thema
settings-theme-status-title = Themenstatus
settings-theme-status-initial = Thema geladen.
settings-failed-to-save = Speichern fehlgeschlagen: { $err }
settings-custom-css-title = Benutzerdefiniertes CSS
settings-custom-css-subtitle = Separate CSS-Dateien für den hellen und dunklen Modus laden
settings-custom-light-css-title = Pfad für benutzerdefiniertes helles CSS
settings-custom-dark-css-title = Pfad für benutzerdefiniertes dunkles CSS
settings-reload-css-title = Benutzerdefiniertes CSS neu laden
settings-reload-css-subtitle = Das gewählte Thema und die passende helle/dunkle CSS-Datei erneut anwenden.
settings-reload-button = Neu laden
settings-language-title = Sprache
settings-language-description = Änderungen werden nach einem Neustart von SceneDeck wirksam.
settings-display-language-title = Anzeigesprache
settings-display-language-subtitle = Wählen Sie eine Sprache oder folgen Sie der Systemsprache.
language-system-default = Systemstandard
settings-language-status-title = Sprachstatus
settings-language-status-initial = Neu starten, um eine geänderte Sprache anzuwenden.
settings-language-saved = Sprache gespeichert. Starten Sie SceneDeck neu, um sie anzuwenden.
settings-obs-connection-title = OBS-Verbindung
settings-obs-connection-description = WebSocket-Einstellungen für OBS Studio (Standardport: 4455).
settings-host-title = Host
settings-port-title = Port
settings-password-title = Passwort (optional)
settings-obs-status-title = OBS-Status
settings-invalid-port = Ungültige Portnummer.
settings-saved = Einstellungen gespeichert.
settings-password-saved = Passwort im Schlüsselbund gespeichert.
settings-keyring-error = Schlüsselbund-Fehler: { $err }
settings-output-safety-title = Ausgabesicherheit
settings-output-safety-description = Optionale Bestätigungen für kritische Stream- und Aufnahmeaktionen.
settings-confirm-start-stream-title = Stream-Start bestätigen
settings-confirm-start-stream-subtitle = Vor dem Start des Live-Streams nachfragen.
settings-confirm-stop-stream-title = Stream-Stopp bestätigen
settings-confirm-stop-stream-subtitle = Vor dem Stoppen des Live-Streams nachfragen.
settings-confirm-start-recording-title = Aufnahmestart bestätigen
settings-confirm-start-recording-subtitle = Vor dem Start einer Aufnahme nachfragen.
settings-confirm-stop-recording-title = Aufnahmestopp bestätigen
settings-confirm-stop-recording-subtitle = Vor dem Stoppen einer Aufnahme nachfragen.
settings-hotkeys-title = Szenen-Tastenkürzel
settings-hotkeys-description = Live-Szenen über die Tastatur wechseln. Die Platznummern folgen der in der Inventur festgelegten Szenenreihenfolge.
settings-hotkeys-enabled-title = Szenen-Tastenkürzel aktivieren
settings-hotkeys-enabled-subtitle = Zifferntasten wechseln die Szenenkarten der Live-Seite in Kartenreihenfolge.
settings-hotkeys-style-title = Tastenkombination
settings-hotkeys-style-subtitle = Welche Tasten eine Szene wechseln. Reine Ziffern pausieren, solange ein Textfeld den Fokus hat.
settings-hotkeys-leader-title = Leader-Taste
settings-hotkeys-leader-subtitle = Erste Taste des zweistufigen Kürzels, nach Vim-Art.
settings-hotkeys-timeout-title = Leader-Zeitfenster
settings-hotkeys-timeout-subtitle = Wie lange der Leader auf seine Ziffer wartet, in Millisekunden.
settings-hotkeys-preview-title = Aktuelle Belegung
settings-hotkeys-preview-subtitle = { $first } … { $last } wechseln die ersten { $count } Szenen auf Live.
settings-hotkeys-preview-disabled = Szenen-Tastenkürzel sind ausgeschaltet.
settings-obs-not-connected = Nicht mit OBS verbunden.
settings-obs-connecting = Verbindung zu OBS wird hergestellt …
settings-obs-connected = Verbunden — OBS { $version }
settings-obs-error = Fehler: { $err }
settings-theme-subtitle = { $description } Farbmuster: { $swatches }
settings-theme-loaded = { $theme } geladen ({ $variant }).
settings-theme-loaded-with-warnings = Thema mit Warnungen geladen.
theme-variant-light = Hell
theme-variant-dark = Dunkel

## theme.rs

theme-adwaita-default-name = Adwaita Standard
theme-adwaita-default-desc = Neutrales Erscheinungsbild nach GNOME-Standard.
theme-scenedeck-dark-name = SceneDeck Dunkel
theme-scenedeck-dark-desc = Ein zurückhaltendes dunkles Konsolen-Thema für den Live-Betrieb.
theme-scenedeck-light-name = SceneDeck Hell
theme-scenedeck-light-desc = Ein klares helles Konsolen-Thema mit dezentem Kontrast.
theme-obs-name = OBS
theme-obs-desc = Der Standard-Look von OBS Studio, in libadwaita umgesetzt.
theme-obsidian-name = Obsidian
theme-obsidian-desc = Gut lesbare Graphit-Oberflächen mit kühlen Akzenten.
theme-nord-name = Nord
theme-nord-desc = Kühle blaugraue Oberflächen mit frostigen Akzenten.
theme-dracula-inspired-name = Dracula-inspiriert
theme-dracula-inspired-desc = Eine dunkle, ausdrucksstarke Palette mit eigenem CSS.
theme-solarized-dark-name = Solarized Dunkel
theme-solarized-dark-desc = Blendarmer Kontrast mit türkisen und bernsteinfarbenen Akzenten.
theme-high-contrast-name = Hoher Kontrast
theme-high-contrast-desc = Stärkere Umrandungen und Kontrast für kritische Bedienelemente.
theme-stream-red-name = Stream-Rot
theme-stream-red-desc = Broadcast-orientierte rote Akzente für Live-Zustände.
theme-studio-purple-name = Studio-Violett
theme-studio-purple-desc = Dezente violette Akzente, ohne die Oberflächen zu dominieren.
theme-ubuntu-violet-name = Ubuntu-Violett
theme-ubuntu-violet-desc = Von Ubuntu inspirierte violette Oberflächen mit warmem Live-Akzent.
theme-custom-css-read-failed = Benutzerdefiniertes CSS konnte nicht aus { $path } gelesen werden: { $err }
theme-custom-css-no-matching-file = Benutzerdefiniertes CSS ist aktiviert, aber es ist keine passende helle/dunkle Datei festgelegt.
theme-css-no-display = { $label } wurde nicht geladen, da kein GTK-Display verfügbar ist.
theme-css-parse-error = { $label } CSS-Parsingfehler: { $message }
## stats.rs — live streaming telemetry
stats-page-title = Stream-Statistik
stats-page-subtitle = Live-Telemetrie, die bei bestehender Verbindung einmal pro Sekunde von OBS abgefragt wird.
stats-gauge-fps = FPS
stats-gauge-frame-time = Bildzeit (ms)
stats-gauge-dropped = Verworfene Bilder
stats-gauge-congestion = Auslastung
stats-chart-fps = Bilder pro Sekunde
stats-chart-frame-time = Durchschnittliche Renderzeit pro Bild (ms)
stats-chart-output-skipped = Übersprungene Ausgabebilder pro Messung
stats-chart-render-skipped = Verpasste Renderbilder pro Messung
stats-card-render-frames = Verpasste Renderbilder
stats-card-output-frames = Übersprungene Ausgabebilder
stats-card-stream-frames = Verworfene Stream-Bilder
stats-card-frame-time = Durchschnittliche Renderzeit pro Bild
stats-card-cpu = OBS-CPU-Auslastung
stats-card-memory = OBS-Speichernutzung
stats-card-bitrate = Stream-Bitrate
stats-value-placeholder = —
stats-value-frames = { $skipped } von { $total } ({ $percent } %)
stats-value-ms = { $value } ms
stats-value-percent = { $value } %
stats-value-mb = { $value } MB
stats-value-kbps = { $value } kbit/s

## Help, Onboarding und ergänzende Statusmeldungen
page-help = Hilfe
config-parse-failed-backed-up = Einstellungen konnten nicht gelesen werden: { $detail } — die Datei wurde unter { $path } gesichert und die Standardwerte wurden geladen.
stats-popout-button = Statistik in einem separaten Fenster öffnen

help-page-title = Hilfe & Einführung
help-hero-title = Willkommen bei SceneDeck
help-hero-description = Eine native Linux-Steueroberfläche für OBS Studio. Dieser Leitfaden führt durch die erste Verbindung, zeigt, wie auf der Live-Seite nur die tatsächlich geschalteten Szenen erscheinen, und erklärt jede Seite der Seitenleiste. Klappen Sie ein Thema auf, um es zu lesen.
help-expand-hint = Klicken Sie auf ein Thema, um es aufzuklappen.
help-open-settings = Einstellungen öffnen
help-open-inventory = Inventar öffnen
help-open-doctor = Doctor öffnen
help-open-live = Live öffnen
help-open-mixer = Mixer öffnen
help-open-graph = Graph öffnen
help-open-stats = Statistik öffnen

help-group-start-title = Erste Schritte
help-group-start-description = Der kürzeste Weg von der Neuinstallation zum ersten Szenenwechsel.
help-quickstart-title = In fünf Schritten zum ersten Szenenwechsel
help-quickstart-subtitle = Führen Sie diese Schritte beim ersten Mal der Reihe nach aus
help-quickstart-body =
    1. Öffnen Sie in OBS Studio Werkzeuge → WebSocket-Servereinstellungen und aktivieren Sie „WebSocket-Server aktivieren“. Lassen Sie OBS laufen.
    2. Klicken Sie im selben OBS-Dialog auf „Verbindungsinformationen anzeigen“ und notieren Sie Serverport (standardmäßig 4455) und Serverpasswort.
    3. Öffnen Sie in SceneDeck die Einstellungen und tragen Sie Host, Port und Passwort ein. Der Host bleibt 127.0.0.1, wenn OBS auf demselben Computer läuft.
    4. Klicken Sie unten in der Seitenleiste auf Verbinden. Die Statuszeile darüber wird grün und zeigt „Verbunden“.
    5. Öffnen Sie das Inventar und weisen Sie den Szenen, die Sie während einer Sendung schalten möchten, die Rolle „Primär“ zu. Genau diese Szenen werden als Karten auf der Live-Seite angezeigt.

help-concepts-title = Wie SceneDeck Ihren Aufbau versteht
help-concepts-subtitle = Rollen, die Registrierung und was OBS nie verändert
help-concepts-body =
    SceneDeck benennt in OBS nichts um, löscht nichts und ändert keine Reihenfolge. Es liest Ihre Szenen über die OBS-WebSocket-Verbindung und führt eigene Notizen dazu.
    Eine „Rolle“ ist eine solche Notiz: Ihre eigene Kennzeichnung für den Zweck einer Szene — etwa eine Szene für die Sendung, eine wiederverwendbare Einblendung oder eine übrig gebliebene Testszene.
    Diese Notizen liegen in registry.json neben der Konfigurationsdatei. Sie bleiben nach Neustarts erhalten und können über die Inventarseite exportiert und auf einen anderen Computer übertragen werden.
    Da die Notizen lokal gespeichert sind, können zwei Personen denselben OBS-Aufbau nutzen und jeweils eine andere Live-Seite führen.

help-group-connect-title = Verbindung mit OBS
help-group-connect-description = Auf diesem Computer oder quer durch den Raum.
help-connect-local-title = Verbindung mit OBS auf diesem Computer
help-connect-local-subtitle = Der Standardfall — Host 127.0.0.1, Port 4455
help-connect-local-body =
    127.0.0.1 ist die Adresse, über die ein Computer mit sich selbst spricht. Sie ist daher der richtige Host, wenn OBS und SceneDeck nebeneinander laufen.
    Der Port muss dem Serverport in den WebSocket-Servereinstellungen von OBS entsprechen. OBS verwendet 4455, sofern Sie ihn nicht geändert haben.
    Wenn in OBS „Authentifizierung aktivieren“ gewählt ist, fügen Sie das Passwort unter Einstellungen → Passwort ein. SceneDeck speichert es im Schlüsselbund Ihres Desktops (dort, wo auch Ihr Browser gespeicherte Anmeldedaten ablegt), niemals in der unverschlüsselten Konfigurationsdatei.
    Klicken Sie in der Seitenleiste auf Verbinden oder drücken Sie jederzeit Strg+R, um die Verbindung erneut herzustellen.

help-connect-remote-title = Verbindung mit OBS auf einem anderen Computer
help-connect-remote-subtitle = Streaming-PC in der Ecke, Steuerung vom Laptop
help-connect-remote-body =
    Dies ist der Aufbau mit zwei Computern: OBS läuft auf dem Aufnahme- und Kodierrechner, SceneDeck auf dem Computer vor Ihnen. Beide müssen sich im selben Netzwerk befinden.
    Aktivieren Sie auf dem OBS-Computer unter Werkzeuge → WebSocket-Servereinstellungen den WebSocket-Server und die Authentifizierung und legen Sie ein Passwort fest. Lassen Sie die Authentifizierung nicht ausgeschaltet — sonst könnte jeder im Netzwerk, der den Port erreicht, Ihren Stream starten oder stoppen.
    Ermitteln Sie die Adresse des OBS-Computers auf diesem Gerät. Führen Sie unter Linux `ip addr` aus und suchen Sie eine Adresse wie 192.168.1.42; führen Sie unter Windows `ipconfig` aus und lesen Sie die IPv4-Adresse; unter macOS steht sie in Systemeinstellungen → Netzwerk. Gemeint ist die Adresse des Computers, nicht von OBS.
    Geben Sie den Port in der Firewall des OBS-Computers frei. Unter Linux mit ufw lautet der Befehl `sudo ufw allow 4455/tcp`; unter Windows erlauben Sie OBS in der Windows-Defender-Firewall für private Netzwerke.
    Tragen Sie in den SceneDeck-Einstellungen diese Adresse als Host ein (zum Beispiel 192.168.1.42), belassen Sie den Port bei 4455 und fügen Sie das Passwort ein. Klicken Sie auf Verbinden.
    Verwenden Sie für den OBS-Computer möglichst eine Kabelverbindung. Szenenwechsel funktionieren auch über WLAN, aber ein verlorenes Paket verzögert den Schnitt.
    Ein Tipp, der eine Sendung retten kann: Reservieren Sie in den DHCP-Einstellungen Ihres Routers eine feste Adresse für den OBS-Computer, damit der gespeicherte Host auch nach einem Neustart funktioniert.

help-connect-share-title = Einem Co-Host oder Moderator die Steuerung überlassen
help-connect-share-subtitle = Ein zweites SceneDeck für denselben Aufbau
help-connect-share-body =
    Der WebSocket-Server von OBS akzeptiert mehrere Clients gleichzeitig. Eine zweite Person kann daher auf einem zweiten Computer ihr eigenes SceneDeck mit demselben Aufbau verbinden — genau wie im vorherigen Thema, nur zweimal.
    Es gibt keine persönlichen Anmeldungen: Wer Host, Port und Passwort in den Einstellungen hat, besitzt dieselbe Kontrolle wie Sie, einschließlich Starten und Stoppen des Streams. Geben Sie diese Daten nur jemandem, dem Sie auch Ihre OBS-Sitzung anvertrauen würden.
    Begrenzen Sie zunächst die Ansicht. Rollen und ausgeblendete Szenen liegen im lokalen Inventar, nicht in OBS. Richten Sie diese auf Ihrem Computer ein und lassen Sie Ihre Mitwirkenden die Szenenregistrierungs-YAML exportieren oder nachbilden, statt mit einer vollständigen ungefilterten Szenenliste zu beginnen.
    Leiten Sie den WebSocket-Port nicht ins Internet weiter. Verbinden Sie beide Geräte über dasselbe VPN oder Tailscale-Netzwerk oder tunneln Sie die Verbindung über SSH und tragen Sie die Tunneladresse als Host ein.
    Alle verbundenen Personen sehen denselben Live-Zustand — ein Szenenwechsel oder eine Reglerbewegung auf einer Seite erscheint sofort auch auf der anderen, als säßen beide an derselben Tastatur.

help-connect-trouble-title = Wenn Verbinden nicht funktioniert
help-connect-trouble-subtitle = Lesen Sie den Fehler und prüfen Sie dann diese Liste
help-connect-trouble-body =
    „Verbindung abgelehnt“ bedeutet fast immer, dass der WebSocket-Server in OBS nicht aktiviert ist oder der Port nicht übereinstimmt. Prüfen Sie beides unter Werkzeuge → WebSocket-Servereinstellungen.
    Wenn eine Verbindung hängen bleibt und dann abläuft, verwirft meist eine Firewall den Datenverkehr oder die Hostadresse gehört zu einem anderen Computer als gedacht.
    „Authentifizierung fehlgeschlagen“ bedeutet, dass das Passwort falsch ist. Geben Sie es in den Einstellungen erneut ein; das Feld ist nur zum Schreiben gedacht und sieht daher leer aus, selbst wenn ein Passwort gespeichert ist.
    Zeigt der Status in der Seitenleiste gar nichts an? Prüfen Sie, ob OBS wirklich läuft und nicht durch einen eigenen modalen Dialog blockiert ist.
    SceneDeck verbindet sich nach einem Abbruch automatisch erneut; mit Strg+R starten Sie den Versuch sofort.

help-group-scenes-title = Szenen ordnen
help-group-scenes-description = Die Live-Seite sollte nur das zeigen, worauf Sie tatsächlich schalten.
help-scenes-hide-title = Szenen ausblenden, auf die Sie nie schalten
help-scenes-hide-subtitle = Die wichtigste erste Einrichtung
help-scenes-hide-body =
    In einem gewachsenen OBS-Aufbau sammeln sich Szenen, die nur in andere Szenen eingebettet werden, oder die für einen einzelnen Test erstellt und nie gelöscht wurden. Auf einer Live-Steueroberfläche können sie zu einem falschen Schnitt führen.
    SceneDeck zeigt eine Live-Karte nur für Szenen mit der Rolle Primär. Alle anderen Rollen sind auf Live ausgeblendet. Eine Szene auszublenden bedeutet daher einfach, ihr eine andere Rolle als Primär zu geben.
    Öffnen Sie das Inventar. Jede OBS-Szene erhält eine Zeile mit einer Rollenauswahl auf der rechten Seite.
    Weisen Sie den wenigen Szenen, auf die Sie tatsächlich schalten, Primär zu. Alles andere erhält eine dieser Rollen: Sekundär (eine echte Szene, die manchmal gebraucht wird, aber nicht auf Live erscheinen soll), Modul (eine Einblendung oder Bauchbinde, die nur in andere Szenen eingebettet wird), Roh (eine reine Kamera- oder Aufnahmehülle), Debug (eine Testszene) oder Archiv (für später aufbewahrt und aus dem Weg).
    Szenen ohne zugewiesene Rolle bleiben ebenfalls von Live fern; Nicht zugewiesen blendet sie also auch aus. Eine bewusste Zuweisung ist dennoch besser: Doctor meldet nicht zugewiesene Szenen, damit neue auffallen.
    Die Änderung wirkt sofort — kehren Sie zu Live zurück und die Karte ist verschwunden. In OBS wurde nichts verändert.

help-scenes-order-title = Reihenfolge, Farben und Symbole
help-scenes-order-subtitle = Die richtige Karte unübersehbar machen
help-scenes-order-body =
    Ziehen Sie eine Szene am Griff links in ihrer Inventarzeile, um die Reihenfolge festzulegen. Die Live-Karten und Ziffernkürzel folgen dieser Reihenfolge; die Karte für Taste 1 steht also ganz oben.
    Die Akzentfarbe färbt die Live-Karte der Szene ein. Verwenden Sie eine kräftige Farbe für Szenen mit Folgen — etwa „Wir sind live“ oder eine Sponsorenbotschaft — damit Ihr Blick sofort dorthin fällt.
    Über die Symbolauswahl links in jeder Zeile erhält die Live-Karte ein Symbol. Es stehen dreißig Symbole sowie „Kein Symbol“ zum Entfernen zur Verfügung.
    Reihenfolge, Farben und Symbole werden in der lokalen Registrierung gespeichert, nicht in OBS.

help-scenes-registry-title = Einrichtung sichern und übertragen
help-scenes-registry-subtitle = Die Zeile „Szenenregistrierungs-YAML“ im Inventar
help-scenes-registry-body =
    Der Export schreibt Rollen, Reihenfolge, Akzentfarben, Symbole, Tags und Graphregeln in eine einzige YAML-Datei — ein lesbares Textformat, das Sie in einer Versionsverwaltung ablegen können.
    Der Import ersetzt die lokale Registrierung durch den Inhalt einer solchen Datei. Damit übertragen Sie eine fertige Einrichtung auf einen zweiten Computer oder kehren nach einem Experiment zu einem früheren Stand zurück.
    Szenennamen verbinden die Datei mit OBS. Eine in OBS umbenannte Szene erscheint daher als veralteter Eintrag. Das Inventar listet solche Einträge auf und kann sie entfernen.

help-group-operate-title = Eine Sendung durchführen
help-group-operate-description = Die Seiten, die Sie während eines Streams tatsächlich verwenden.
help-live-title = Die Live-Seite
help-live-subtitle = Szenenkarten, Audio und die Programmszene
help-live-body =
    Live ist die Bedienansicht: oben die aktuelle Programmszene, auf einer Seite die Szenenkarten und auf der anderen kompakte Audiokarten. Ziehen Sie die Trennlinie, um der benötigten Hälfte mehr Platz zu geben.
    Ein Klick auf eine Szenenkarte schaltet OBS auf diese Szene. Die aktuelle Szene ist als Aktiv markiert, die übrigen als Bereit.
    Nach dem Verbinden sind keine Karten zu sehen? Dann hat noch keine Szene die Rolle Primär — siehe oben „Szenen ausblenden, auf die Sie nie schalten“.
    Die Statusleiste am unteren Rand bleibt auf jeder Seite sichtbar. Sie zeigt Verbindung, Stream- und Aufnahmestatus mit Laufzeit sowie aktuelle Werte für FPS, verworfene Frames, CPU und Bitrate.

help-hotkeys-title = Szenen über die Tastatur wechseln
help-hotkeys-subtitle = Standardmäßig Strg+1 … Strg+0, frei konfigurierbar
help-hotkeys-body =
    Jede der ersten zehn Live-Karten trägt ein kleines Ziffernabzeichen: die erste Karte 1, die neunte 9 und die zehnte 0. Der Text neben der Überschrift „Szenen“ zeigt immer die aktuelle Belegung.
    Die Platznummern folgen der Reihenfolge im Inventar. Wenn Sie die Karten dort neu anordnen, ändern sich die Kürzel entsprechend.
    Unter Einstellungen → Szenen-Tastenkürzel wählen Sie, wie die Ziffer gedrückt wird. Eine Modifikatortaste plus Ziffer (standardmäßig Strg) kann beim Tippen nicht versehentlich auslösen. Eine einzelne Ziffer ist am schnellsten. Der Leader-Stil funktioniert wie in Vim: Leader-Taste drücken, loslassen und dann die Ziffer drücken; währenddessen zeigt der Text „Leader bereit“.
    Kürzel wirken nur auf der Live-Seite. Varianten ohne Modifikatortaste pausieren, solange ein Textfeld den Fokus hat. Ist einer Ziffer keine Szene zugeordnet, meldet der Text dies, statt etwas zu schalten.

help-audio-title = Audio: Mixer-Seite und Pegelanzeigen
help-audio-subtitle = Was die farbigen Balken bedeuten
help-audio-body =
    Audiokarten erscheinen auf Live und mit mehr Platz und Bedienelementen auf der Mixer-Seite. Globale OBS-Audiogeräte stehen zuerst, danach audiofähige Quellen der aktuellen Szene — einschließlich Quellen in verschachtelten Szenen und Gruppen.
    Der Mixer-Modus bestimmt, welche Szene Sie hören und sehen. Aktiv folgt der OBS-Programmszene. Ausgewählt lädt eine Szene und bleibt dort. Angeheftet hält eine gewählte Szene als festes Ziel, während OBS weiterschaltet.
    Die Anzeige neben jedem Regler reicht von −60 dB unten bis 0 dB oben und verwendet die OBS-Grenzwerte: Grün unter −20 dB für Musik und Hintergrund, Gelb von −20 bis −9 dB für Sprache, Rot über −9 dB, wo Übersteuerung beginnt. Nichts sollte dauerhaft im roten Bereich liegen.
    Eine Spalte steht für eine Monoquelle, zwei für Stereo — links, dann rechts. Bewegt sich nur die linke Spalte, hört die Hälfte Ihres Publikums von dieser Quelle nichts.
    Die Linie über der Füllung zeigt den lautesten Spitzenwert der letzten zwanzig Sekunden und macht eine übersehene Übersteuerung schnell sichtbar. Das Quadrat ganz unten zeigt den Pegel, der vor dem Regler vom Gerät eintrifft — ist er zu hoch, hilft keine Reglerbewegung.
    Die Sperrtaste einer Karte sperrt nur den SceneDeck-Regler. In OBS selbst wird nichts gesperrt.

help-outputs-title = Stream und Aufnahme starten und stoppen
help-outputs-subtitle = Und Bestätigungen gegen versehentliche Aktionen
help-outputs-body =
    Die Schaltflächen zum Starten und Stoppen von Stream und Aufnahme befinden sich unten in der Seitenleiste und sind von jeder Seite erreichbar. Die Statusleiste zeigt Zustand und Laufzeit.
    Unter Einstellungen → Ausgabesicherheit legen Sie fest, welche der vier Aktionen zuerst nachfragt. Standardmäßig muss das Stoppen bestätigt werden, das Starten nicht — früh zu starten kostet wenig, früh zu stoppen beendet die Sendung.
    Direkt in OBS vorgenommene Änderungen erscheinen ebenfalls hier: SceneDeck folgt den OBS-Ereignissen, statt anzunehmen, dass der eigene Tastendruck erfolgreich war.

help-group-inspect-title = Einrichtung prüfen
help-group-inspect-description = Finden Sie die Überraschung, bevor sie auf Sendung passiert.
help-doctor-title = Doctor
help-doctor-subtitle = Strukturprobleme, nach Schweregrad sortiert
help-doctor-body =
    Doctor liest Szenenliste, Rollenzuweisungen und Szenenverschachtelung und meldet Auffälligkeiten als Fehler, Warnungen und Informationen.
    Typische Befunde: Szenen ohne Rolle, gespeicherte Szenen, die in OBS nicht mehr existieren, zirkuläre Verschachtelungen und Szenen, die entgegen den Rollenregeln ineinander verschachtelt sind.
    Die Prüfung läuft bei jedem Öffnen der Seite erneut; die Schaltfläche Erneut ausführen erzwingt einen weiteren Lauf.
    Prüfen Sie die Seite nach jeder Änderung am OBS-Aufbau und noch einmal vor dem Sendestart.

help-graph-title = Graph
help-graph-subtitle = Welche Szenen in welchen enthalten sind
help-graph-body =
    Durch das Verschachteln von Szenen entstehen in OBS Einblendungen und gemeinsame Layouts. Dadurch kann eine Szene aber auch von etwas abhängen, das Sie vergessen haben.
    Graph listet jede übergeordnete Szene und ihren Inhalt auf und bewertet jede Beziehung anhand Ihrer Rollenregeln als in Ordnung, fragwürdig oder verboten.
    Damit beantworten Sie vor einer Änderung die Frage: „Was geht kaputt, wenn ich diese Szene ändere?“

help-stats-title = Statistik
help-stats-subtitle = Ob der Computer Schritt hält
help-stats-body =
    Anzeigen für FPS, Frame-Renderzeit, verworfene Frames und Netzwerküberlastung werden bei schlechteren Werten zuerst gelb und dann rot — verworfene Frames warnen ab 1 %, Überlastung ab 30 %.
    Verlaufsgrafiken speichern ungefähr die letzten zwei Minuten. Balkendiagramme zeigen, wann Frames verloren gingen, statt nur eine laufende Summe anzuzeigen. So erkennen Sie, ob ein Ruckler ein einzelner schlechter Moment oder ein Trend war.
    Solange die Verbindung besteht, werden fortlaufend Messwerte gesammelt. Wenn Sie Statistik mitten im Stream öffnen, sehen Sie daher die vorherigen Minuten, statt mit einer leeren Ansicht zu beginnen.
    Frame-Zähler stammen aus OBS und werden zurückgesetzt, wenn OBS oder die Stream-Ausgabe neu startet.

help-group-personalise-title = Persönlich gestalten
help-group-personalise-description = Erscheinungsbild, Sprache und Speicherorte Ihrer Dateien.
help-appearance-title = Themen und Erscheinungsbild
help-appearance-subtitle = Einschließlich eines Looks passend zu OBS
help-appearance-body =
    Das Farbschema folgt standardmäßig der Hell-/Dunkel-Einstellung Ihres Desktops; Sie können eine Variante auch erzwingen.
    Themen sind Familien mit heller und dunkler Variante: Wählen Sie eine aus, und die zum aktuellen Farbschema passende Variante wird angewendet. Die OBS-Familie entspricht dem Erscheinungsbild von OBS Studio, damit die Steueroberfläche zur gesteuerten Anwendung passt.
    Benutzerdefiniertes CSS verwendet getrennte Dateien für Hell und Dunkel und folgt damit ebenfalls dem Farbschema. „Benutzerdefiniertes CSS neu laden“ übernimmt Änderungen ohne Neustart.
    Bewegung steuert, wie stark die Oberfläche animiert wird. Wählen Sie Reduziert oder Aus, wenn Bewegung ablenkt oder der Computer stark ausgelastet ist.

help-files-title = Wo SceneDeck Daten speichert
help-files-subtitle = Konfiguration, Registrierung und Ihr OBS-Passwort
help-files-body =
    Einstellungen einschließlich OBS-Host und -Port liegen unter $XDG_CONFIG_HOME/scenedeck/config.json — normalerweise ~/.config/scenedeck/config.json.
    Szenenrollen, Reihenfolge, Akzentfarben und Symbole liegen in registry.json im selben Ordner.
    Das OBS-Passwort steht in keiner der beiden Dateien. Es wird im Secret-Service-Schlüsselbund Ihres Desktops gespeichert, demselben Tresor, den Ihr Browser nutzt.
    Sichern Sie beide JSON-Dateien, um eine vollständige Einrichtung auf einen anderen Computer zu übertragen, oder verwenden Sie den YAML-Export im Inventar nur für den Szenenteil.

help-shortcuts-title = Tastenkürzel
help-shortcuts-subtitle = Die vollständige Liste
help-shortcuts-body =
    F1 — diesen Leitfaden öffnen.
    Strg+R — erneut mit OBS verbinden.
    Strg+, — Einstellungen öffnen.
    Strg+Q — SceneDeck beenden.
    Strg+1 … Strg+0 auf der Live-Seite — zu den ersten zehn Szenenkarten wechseln. Die Kombination ist unter Einstellungen → Szenen-Tastenkürzel konfigurierbar.

welcome-dialog-heading = Willkommen bei SceneDeck
welcome-dialog-body = Dies scheint Ihr erster Start zu sein. Die Hilfeseite erklärt die Verbindung mit OBS — auch auf einem anderen Computer — und zeigt, wie auf der Live-Seite nur die Szenen erscheinen, auf die Sie tatsächlich schalten. Das dauert nur wenige Minuten und vermeidet einige Fehler.
welcome-dialog-later = Nicht jetzt
welcome-dialog-open = Leitfaden lesen
window-help-tooltip = Hilfe und Einführungsleitfaden
