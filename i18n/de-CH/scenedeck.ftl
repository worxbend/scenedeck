## SceneDeck UI strings (English, source locale).
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
edge-status-warning-tooltip = Verbindungen ausserhalb einer Positivliste
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
role-secondary-desc = Gültige Szene, standardmässig auf der Live-Seite ausgeblendet
role-module-desc = Wiederverwendbare verschachtelte Szene, nicht direkt umschaltbar
role-raw-desc = Hardware- oder Quellen-Wrapper-Szene
role-debug-desc = Temporäre Testszene
role-archive-desc = Aufbewahrt, aber von allen Arbeitsabläufen ausgeschlossen

## domain/mixer.rs — MixerMode / MixerGrouping labels
mixer-mode-active = Aktiv
mixer-mode-selected = Ausgewählt
mixer-mode-pinned = Angeheftet
mixer-mode-active-desc = Folgt der Programmszene von OBS.
mixer-mode-selected-desc = Ausgewählte Szene prüfen, ohne OBS zu folgen.
mixer-mode-pinned-desc = Die ausgewählte Szene während des Betriebs stabil halten.
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
doctor-no-role = Der Szene ist in der lokalen Registry keine Rolle zugewiesen.
doctor-no-role-suggestion = Inventar öffnen und eine Rolle zuweisen.
doctor-stale-entry = Der Registry-Eintrag verweist auf eine Szene, die in OBS nicht gefunden wurde.
doctor-stale-entry-suggestion = Den Eintrag aus dem Inventar entfernen.
doctor-protected-switchable = Geschützte Szene befindet sich in der umschaltbaren Rolle '{ $role }'.
doctor-protected-switchable-suggestion = Geschützte Szenen sind meist Bausteine; Modul oder Roh in Betracht ziehen.
doctor-cycle = Zirkulärer Szenenverweis zwischen '{ $parent }' und '{ $child }'.
doctor-cycle-suggestion = Die Verschachtelungsschleife entfernen; OBS kann Zyklen nicht darstellen.
doctor-edge-primary-debug = Primäre Szene hängt von einer Debug-Szene ab. (→ '{ $child }')
doctor-edge-primary-debug-suggestion = Die Debug-Szene vor dem Livegang aus dem Live-Pfad entfernen.
doctor-edge-primary-raw = Primäre Szene umschliesst direkt eine Roh-Quelle. (→ '{ $child }')
doctor-edge-primary-raw-suggestion = Die Roh-Quelle zur Wiederverwendung und Übersichtlichkeit in eine Modul-Szene einbetten.
doctor-edge-module-primary = Modul hängt von einer primären Szene ab und kehrt damit die Hierarchie um. (→ '{ $child }')
doctor-edge-module-primary-suggestion = Module sollten Bausteine sein, keine Verbraucher primärer Szenen.
doctor-edge-raw-nests = Roh-Szene verschachtelt eine weitere Szene. (→ '{ $child }')
doctor-edge-raw-nests-suggestion = Roh-Szenen sollten reine Quellen-Wrapper ohne verschachtelte Szenen sein.
doctor-edge-forbidden = Die Szenenabhängigkeit ist durch die Graph-Richtlinie verboten. (→ '{ $child }')
doctor-edge-outside-policy = Die Szenenabhängigkeit liegt ausserhalb der konfigurierten Graph-Richtlinie. (→ '{ $child }')
doctor-edge-adjust-suggestion = Die Verschachtelungsbeziehung anpassen oder die Graph-Regeln der Registry aktualisieren.

## controller/app_controller.rs
controller-not-connected = Nicht mit OBS verbunden

## controller/state.rs — Page titles and ObsStatus labels
page-live = Live
page-mixer = Mixer
page-graph = Graph
page-inventory = Inventar
page-doctor = Diagnose
page-settings = Einstellungen
obs-status-disconnected = Getrennt
obs-status-connecting = Verbindung wird hergestellt…
obs-status-connected = Verbunden
obs-status-error = Fehler

## storage/config.rs — ConfigStartupNotice
config-first-launch = Noch keine gespeicherten Einstellungen. Standardwerte werden geladen.
config-read-failed = Einstellungen konnten nicht gelesen werden: { $detail }
config-parse-failed = Einstellungen konnten nicht verarbeitet werden: { $detail }

## graph.rs

graph-empty-title = Keine Abhängigkeiten
graph-empty-description = Keine Szenen verschachteln andere Szenen, oder OBS ist nicht verbunden. Verbinden und verschachtelte Szenenquellen hinzufügen, um den Abhängigkeitsgraphen zu sehen.
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
mixer-no-scene-description = Eine Szene wählen, um deren Mixer-Audio zu laden.
mixer-loading-title = Mixer-Audio wird geladen
mixer-loading-description = Audioquellen für { $scene } werden geladen.

## Audio-source empty states
mixer-current-scene-fallback = Die aktuelle Szene
mixer-no-audio-sources-title = Keine Audioquellen
mixer-no-audio-sources-description = { $scene } hat keine passenden konfigurierten OBS-Audioquellen.
mixer-no-matching-title = Keine passenden Audioquellen
mixer-no-matching-description = Suchfilter anpassen, um verfügbare Audioquellen anzuzeigen.

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
mixer-summary-selected-fallback = Keine ausgewählte Szene festgelegt; verwendet aktive OBS-Szene: { $scene }
mixer-summary-pinned-selected-fallback = Keine angeheftete Szene festgelegt; verwendet ausgewählte Szene: { $scene }
mixer-summary-pinned-active-fallback = Keine angeheftete oder ausgewählte Szene festgelegt; verwendet aktive OBS-Szene: { $scene }

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
inventory-scenes-group-description = Rollen zuweisen, um zu steuern, welche Szenen auf der Live-Seite erscheinen.
inventory-stale-group-title = Veraltete Registry-Einträge
inventory-stale-group-description = Diese Szenen befinden sich in der lokalen Registry, existieren aber in OBS nicht mehr.
inventory-remove-stale-tooltip = Veralteten Eintrag entfernen
inventory-yaml-row-title = Szenen-Registry-YAML
inventory-yaml-row-subtitle = Szenenrollen, Tags, Schutzkennzeichen und Graph-Regeln exportieren oder importieren.
inventory-yaml-filter-name = YAML-Dateien

# Shared between the row's Export button, the export FileChooserNative's
# accept label, and (for Cancel) both the export and import dialogs.
inventory-export-button-label = Export
inventory-export-tooltip = Szenen-Registry als YAML exportieren
inventory-import-button-label = Import
inventory-import-tooltip = Szenen-Registry aus YAML importieren
inventory-dialog-cancel-label = Abbrechen

inventory-export-dialog-title = Szenen-Registry exportieren
inventory-export-success = Szenen-Registry nach { $path } exportiert.
inventory-export-error = Export fehlgeschlagen: { $error }
inventory-export-no-file = Export fehlgeschlagen: Es wurde keine Datei ausgewählt.

inventory-import-dialog-title = Szenen-Registry importieren
inventory-import-error = Import fehlgeschlagen: { $error }
inventory-import-no-file = Import fehlgeschlagen: Es wurde keine Datei ausgewählt.

## window.rs

window-stream-live-tooltip = Live-Streaming
window-about-tooltip = Über SceneDeck
window-refresh-tooltip = Aktuelle Seite aktualisieren

window-stream-status-line = Stream: { $state }{ $elapsed }
window-record-status-line = Aufnahme: { $state }{ $elapsed }

window-status-connecting = Verbindung zu OBS wird hergestellt…
window-connect-btn-connecting = Verbindung wird hergestellt…
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

window-sidebar-output-starting = Wird gestartet…
window-sidebar-output-stopping = Wird gestoppt…
window-sidebar-output-reconnecting = Verbindung wird wiederhergestellt…
window-sidebar-output-working = Wird verarbeitet…

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
live-stream-toggle-tooltip = Streaming starten oder stoppen
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
live-disconnected-detail = Die Verbindungssteuerung am unteren Rand der Seitenleiste verwenden.
live-stream-command-error-label = Stream-Befehl fehlgeschlagen
live-recording-command-error-label = Aufnahme-Befehl fehlgeschlagen
live-last-recording-detail = Letzte Aufnahme: { $path }
live-starting-stream = Stream wird gestartet…
live-stopping-stream = Stream wird gestoppt…
live-reconnecting-stream = Stream-Verbindung wird wiederhergestellt…
live-starting-recording = Aufnahme wird gestartet…
live-stopping-recording = Aufnahme wird gestoppt…
live-reconnecting-recording = Aufnahme-Verbindung wird wiederhergestellt…
live-button-starting = Wird gestartet…
live-button-stopping = Wird gestoppt…
live-button-reconnecting = Verbindung wird wiederhergestellt…
live-button-working = Wird verarbeitet…
live-output-kind-stream = Stream
live-output-kind-record = Aufnahme
live-output-label = { $kind }: { $state }
live-output-label-with-elapsed = { $kind }: { $state } · { $elapsed }
live-scenes-no-primary-hint = Keine Szenen mit der Rolle Primär gefunden. Rollen im Inventar zuweisen.
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
live-stop-recording-confirm-body = OBS beendet die aktuelle Aufnahme.
live-stop-recording-confirm-label = Aufnahme stoppen

## audio_card.rs
audio-card-mute-tooltip = Eingang stummschalten
audio-card-source-path-tooltip = { $scope }: { $path }
audio-card-fader-tooltip = Lautstärkeregler
audio-card-lock-tooltip = Lautstärkeregler sperren
audio-card-fine-plus-tooltip = +1 dB
audio-card-fine-reset-tooltip = Auf 0.0 dB zurücksetzen
audio-card-fine-minus-tooltip = -1 dB

## scene_card.rs
scene-card-tooltip = { $status } ({ $role })
scene-card-role-suffix = { $role }-Szene

## status_bar.rs
status-bar-stream-inactive = Stream: Inaktiv
status-bar-record-inactive = Aufnahme: Inaktiv
status-bar-fps-placeholder = FPS —
status-bar-cpu-placeholder = CPU —
status-bar-bitrate-placeholder = Bitrate —
status-bar-fps = FPS { $value }
status-bar-cpu = CPU { $value }%
status-bar-bitrate = Bitrate { $value } kbps
status-bar-dropped = { $count } verworfen

## settings.rs

settings-page-title = Einstellungen
settings-appearance-title = Erscheinungsbild
settings-appearance-description = GNOME-Anwendungen sollten standardmässig dem Systemstil folgen.
settings-theme-mode-system = System
settings-theme-mode-light = Hell
settings-theme-mode-dark = Dunkel
settings-color-scheme-title = Farbschema
settings-color-scheme-subtitle = Der Systemeinstellung folgen oder Hell/Dunkel erzwingen
settings-theme-title = Motiv
settings-theme-status-title = Motiv-Status
settings-theme-status-initial = Motiv geladen.
settings-failed-to-save = Speichern fehlgeschlagen: { $err }
settings-custom-css-title = Eigenes CSS
settings-custom-css-subtitle = Separate benutzerdefinierte CSS-Dateien für Hell- und Dunkelmodus laden
settings-custom-light-css-title = Pfad für eigenes Hell-CSS
settings-custom-dark-css-title = Pfad für eigenes Dunkel-CSS
settings-reload-css-title = Eigenes CSS neu laden
settings-reload-css-subtitle = Das gewählte Motiv und die passende Hell-/Dunkel-CSS-Datei erneut anwenden.
settings-reload-button = Neu laden
settings-language-title = Sprache
settings-language-description = Änderungen wirken sich erst nach einem Neustart von SceneDeck aus.
settings-display-language-title = Anzeigesprache
settings-display-language-subtitle = Eine Sprache wählen oder der Systemsprache folgen.
settings-language-status-title = Sprachstatus
settings-language-status-initial = Neu starten, um eine geänderte Sprache anzuwenden.
settings-language-saved = Sprache gespeichert. SceneDeck neu starten, um sie anzuwenden.
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
settings-confirm-start-stream-title = Streamstart bestätigen
settings-confirm-start-stream-subtitle = Vor dem Starten des Live-Streams nachfragen.
settings-confirm-stop-stream-title = Streamstopp bestätigen
settings-confirm-stop-stream-subtitle = Vor dem Stoppen des Live-Streams nachfragen.
settings-confirm-start-recording-title = Aufnahmestart bestätigen
settings-confirm-start-recording-subtitle = Vor dem Starten einer Aufnahme nachfragen.
settings-confirm-stop-recording-title = Aufnahmestopp bestätigen
settings-confirm-stop-recording-subtitle = Vor dem Stoppen einer Aufnahme nachfragen.
settings-obs-not-connected = Nicht mit OBS verbunden.
settings-obs-connecting = Verbindung zu OBS wird hergestellt…
settings-obs-connected = Verbunden — OBS { $version }
settings-obs-error = Fehler: { $err }
settings-theme-subtitle = { $description } Farbmuster: { $swatches }
settings-theme-loaded = { $theme } geladen ({ $variant }).
settings-theme-loaded-with-warnings = Motiv mit Warnungen geladen.

## theme.rs

theme-adwaita-default-name = Adwaita Default
theme-adwaita-default-desc = Neutrale Gestaltung, die den GNOME-Standardvorgaben folgt.
theme-scenedeck-dark-name = SceneDeck Dark
theme-scenedeck-dark-desc = Ein zurückhaltendes dunkles Konsolen-Motiv für den Live-Betrieb.
theme-scenedeck-light-name = SceneDeck Light
theme-scenedeck-light-desc = Ein klares helles Konsolen-Motiv mit zurückhaltendem Kontrast.
theme-obsidian-name = Obsidian
theme-obsidian-desc = Gut lesbare Graphit-Flächen mit kühlen Akzenten.
theme-nord-name = Nord
theme-nord-desc = Kühle blaugraue Flächen mit frostigen Akzenten.
theme-dracula-inspired-name = Dracula Inspired
theme-dracula-inspired-desc = Eine dunkle, ausdrucksstarke Palette im Original-CSS.
theme-solarized-dark-name = Solarized Dark
theme-solarized-dark-desc = Blendarmer Kontrast mit Türkis- und Bernsteinakzenten.
theme-high-contrast-name = High Contrast
theme-high-contrast-desc = Kräftigere Konturen und Kontrast für kritische Bedienelemente.
theme-stream-red-name = Stream Red
theme-stream-red-desc = Sendungsorientierte Rotakzente für Live-Zustände.
theme-studio-purple-name = Studio Purple
theme-studio-purple-desc = Kontrollierte Violettakzente ohne dominante Flächen.
theme-ubuntu-violet-name = Ubuntu Violet
theme-ubuntu-violet-desc = Von Ubuntu inspirierte violette Flächen mit einem warmen Live-Akzent.
theme-custom-css-read-failed = Eigenes CSS konnte nicht von { $path } gelesen werden: { $err }
theme-custom-css-no-matching-file = Eigenes CSS ist aktiviert, aber es ist keine passende Hell-/Dunkel-Datei festgelegt.
theme-css-no-display = { $label } wurde nicht geladen, da kein GTK-Display verfügbar ist.
theme-css-parse-error = { $label } CSS-Parsefehler: { $message }
