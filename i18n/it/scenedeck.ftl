## SceneDeck UI strings (Italian).
##
## Grouped by the module each message is used from. Message ids are prefixed
## with the module name to keep them unambiguous in this single shared file.

## Internal — used only by the i18n loader's own regression test, not shown
## in the UI. Every locale must define this so the smoke test can confirm the
## locale's bundle loaded (not just the `en` fallback).
i18n-loader-smoke-test = Localizzazione caricata.

## infra/error.rs — user-facing renderings of AppError. `detail` is raw
## upstream text (often from OBS or the OS) and is never translated.
error-connection = Connessione a OBS non riuscita: { $detail }
error-request = Richiesta a OBS non riuscita: { $detail }
error-config = Errore di configurazione: { $detail }
error-storage = Errore di archiviazione: { $detail }
error-notification-title = Errore di SceneDeck: { $message }

## domain/audio.rs — AudioSourceScope::label()
audio-scope-global = Globale
audio-scope-active = Scena
audio-scope-nested = Annidato
audio-scope-group = Gruppo

## domain/graph.rs — EdgeStatus summary label/tooltip
edge-status-ok-label = OK
edge-status-warning-label = Avviso
edge-status-forbidden-label = Vietato
edge-status-ok-tooltip = Collegamenti conformi alla policy del grafo
edge-status-warning-tooltip = Collegamenti non presenti in una lista consentita
edge-status-forbidden-tooltip = Collegamenti vietati dalla policy del grafo

## domain/output.rs — OutputRunState::label(), OutputStatus::summary()
output-state-inactive = Inattivo
output-state-starting = Avvio in corso
output-state-active = Attivo
output-state-stopping = Arresto in corso
output-state-reconnecting = Riconnessione in corso
output-state-paused = In pausa
output-state-unknown = Sconosciuto
output-summary = { $name }: { $state }

## domain/role.rs — SceneRole::label()/description()
role-primary = Primaria
role-secondary = Secondaria
role-module = Modulo
role-raw = Grezza
role-debug = Debug
role-archive = Archivio
role-unassigned = Non assegnato
role-primary-desc = Scena selezionabile in diretta
role-secondary-desc = Scena valida, nascosta da Diretta per impostazione predefinita
role-module-desc = Scena annidata riutilizzabile, non selezionabile direttamente
role-raw-desc = Scena contenitore per hardware o sorgente
role-debug-desc = Scena di prova temporanea
role-archive-desc = Conservata ma esclusa da tutti i flussi di lavoro

## domain/mixer.rs — MixerMode / MixerGrouping labels
mixer-mode-active = Attiva
mixer-mode-selected = Selezionata
mixer-mode-pinned = Fissata
mixer-mode-active-desc = Segue la scena di programma di OBS.
mixer-mode-selected-desc = Ispeziona la scena selezionata senza seguire OBS.
mixer-mode-pinned-desc = Mantiene stabile la scena selezionata durante l'utilizzo.
mixer-grouping-scope = Ambito
mixer-grouping-scene-path = Percorso scena
mixer-grouping-none = Nessuno

## domain/diagnostic.rs — DiagnosticSeverity label/count
diag-label-info = Info
diag-label-warning = Avvisi
diag-label-error = Errori
diag-count-info = { $count ->
    [one] { $count } elemento informativo
   *[other] { $count } elementi informativi
}
diag-count-warning = { $count ->
    [one] { $count } avviso
   *[other] { $count } avvisi
}
diag-count-error = { $count ->
    [one] { $count } errore
   *[other] { $count } errori
}

## ui/pages/inventory.rs
inventory-no-role-assigned = Nessun ruolo assegnato

## services/doctor_service.rs — diagnostic message/suggestion templates
doctor-no-role = La scena non ha un ruolo assegnato nel registro locale.
doctor-no-role-suggestion = Apri Inventario e assegna un ruolo.
doctor-stale-entry = La voce del registro fa riferimento a una scena non trovata in OBS.
doctor-stale-entry-suggestion = Rimuovi la voce da Inventario.
doctor-protected-switchable = La scena protetta ha il ruolo selezionabile '{ $role }'.
doctor-protected-switchable-suggestion = Le scene protette sono generalmente elementi di base; valuta Modulo o Grezza.
doctor-cycle = Riferimento circolare tra le scene '{ $parent }' e '{ $child }'.
doctor-cycle-suggestion = Rimuovi il ciclo di scene annidate; OBS non può renderizzare i cicli.
doctor-edge-primary-debug = La scena primaria dipende da una scena di debug. (→ '{ $child }')
doctor-edge-primary-debug-suggestion = Rimuovi la scena di debug dal percorso live prima di andare in diretta.
doctor-edge-primary-raw = La scena primaria racchiude direttamente una sorgente grezza. (→ '{ $child }')
doctor-edge-primary-raw-suggestion = Racchiudi la sorgente grezza in una scena modulo per riutilizzo e chiarezza.
doctor-edge-module-primary = Il modulo dipende da una scena primaria, invertendo la gerarchia. (→ '{ $child }')
doctor-edge-module-primary-suggestion = I moduli dovrebbero essere elementi di base, non consumatori di scene primarie.
doctor-edge-raw-nests = La scena grezza annida un'altra scena. (→ '{ $child }')
doctor-edge-raw-nests-suggestion = Le scene grezze dovrebbero essere contenitori terminali senza scene annidate.
doctor-edge-forbidden = La dipendenza tra scene è vietata dalla policy del grafo. (→ '{ $child }')
doctor-edge-outside-policy = La dipendenza tra scene non rientra nella policy del grafo configurata. (→ '{ $child }')
doctor-edge-adjust-suggestion = Modifica la relazione tra scene annidate o aggiorna le regole del grafo nel registro.

## controller/app_controller.rs
controller-not-connected = Non connesso a OBS

## controller/state.rs — Page titles and ObsStatus labels
page-live = Diretta
page-mixer = Mixer
page-graph = Grafo
page-inventory = Inventario
page-doctor = Diagnostica
page-settings = Impostazioni
obs-status-disconnected = Disconnesso
obs-status-connecting = Connessione in corso…
obs-status-connected = Connesso
obs-status-error = Errore

## storage/config.rs — ConfigStartupNotice
config-first-launch = Nessuna impostazione salvata. Vengono caricati i valori predefiniti.
config-read-failed = Impossibile leggere le impostazioni: { $detail }
config-parse-failed = Impossibile analizzare le impostazioni: { $detail }

## graph.rs

graph-empty-title = Nessuna dipendenza
graph-empty-description = Nessuna scena ne annida altre, oppure OBS non è connesso. Connettiti e aggiungi sorgenti di scena annidate per vedere il grafo delle dipendenze.
graph-page-title = Dipendenze delle scene
graph-reset-tooltip = Ripristina la disposizione del grafo
graph-edge-summary-count = { $label } { $count }

## mixer.rs

## Page chrome / empty state
mixer-empty-title = Nessun dato del mixer
mixer-empty-description = Connettiti a OBS per caricare scene e sorgenti audio.
mixer-page-title = Mixer
mixer-controls-title = Controlli mixer
mixer-summary-title = Sorgente mixer corrente

## Control rows (ComboRow / EntryRow titles+subtitles)
mixer-mode-row-title = Modalità
mixer-mode-row-subtitle = Attiva segue OBS; Selezionata e Fissata mantengono stabile la scena scelta.
mixer-scene-row-title = Scena
mixer-scene-row-subtitle = Usata dalle modalità Selezionata e Fissata.
mixer-grouping-row-title = Raggruppa per
mixer-grouping-row-subtitle = Controlla come vengono disposte le sorgenti audio qui sotto.
mixer-search-row-title = Cerca

## Scene-loading / no-scene placeholders
mixer-no-scene-title = Nessuna scena selezionata
mixer-no-scene-description = Scegli una scena per caricarne l'audio del mixer.
mixer-loading-title = Caricamento audio mixer
mixer-loading-description = Caricamento delle sorgenti audio per { $scene }.

## Audio-source empty states
mixer-current-scene-fallback = La scena corrente
mixer-no-audio-sources-title = Nessuna sorgente audio
mixer-no-audio-sources-description = { $scene } non ha sorgenti audio OBS configurate corrispondenti.
mixer-no-matching-title = Nessuna sorgente audio corrispondente
mixer-no-matching-description = Modifica il filtro di ricerca per mostrare le sorgenti audio disponibili.

## Group titles
mixer-group-all-sources = Tutte le sorgenti
mixer-group-global-fallback = Globale

## Error placeholder + retry
mixer-error-title = Audio mixer non disponibile
mixer-error-description = Impossibile caricare le sorgenti audio per { $scene }: { $message }
mixer-retry-button-label = Riprova
mixer-retry-button-tooltip = Riprova a caricare l'audio del mixer

## Current-source summary row (source_summary / scene_target_summary)
mixer-summary-following-active = Segue la scena attiva di OBS: { $scene }
mixer-summary-no-scene-selected = Nessuna scena selezionata
mixer-summary-selected-scene = Scena selezionata: { $scene }
mixer-summary-pinned-scene = Scena fissata: { $scene }
mixer-summary-selected-fallback = Scena selezionata non impostata; viene usata la scena attiva di OBS: { $scene }
mixer-summary-pinned-selected-fallback = Scena fissata non impostata; viene usata la scena selezionata: { $scene }
mixer-summary-pinned-active-fallback = Scena fissata e selezionata non impostate; viene usata la scena attiva di OBS: { $scene }

## doctor.rs — page chrome, empty state, and all-clear text for the Doctor
## page. Diagnostic finding messages themselves (`doctor-no-role`,
## `doctor-cycle*`, `doctor-edge-*`, etc.) already exist in
## src/services/doctor_service.rs and are NOT redefined here.
doctor-page-title = Diagnostica
doctor-empty-state-title = Niente da controllare
doctor-empty-state-description = Connettiti a OBS per eseguire la diagnostica dell'architettura.
doctor-summary-row-title = Diagnostica
doctor-rerun-tooltip = Esegui di nuovo la diagnostica
doctor-all-clear-title = Nessun problema rilevato
doctor-all-clear-detail = L'architettura delle scene soddisfa tutti i controlli.

## inventory.rs — page chrome, group titles, YAML import/export UI, and
## status messages. Role names/descriptions themselves are NOT redefined
## here — they already exist in src/domain/role.rs (`role-*`,
## `role-*-desc`, `role-unassigned`) and are reused directly via
## SceneRole::label()/description()/unassigned_label(). The already-present
## `inventory-no-role-assigned` message is also left untouched.
inventory-page-title = Inventario
inventory-empty-state-title = Nessuna scena
inventory-empty-state-description = Connettiti a OBS per caricare l'elenco delle scene.
inventory-scenes-group-title = Scene OBS
inventory-scenes-group-description = Assegna i ruoli per controllare quali scene appaiono nella pagina Diretta.
inventory-stale-group-title = Voci del registro obsolete
inventory-stale-group-description = Queste scene sono presenti nel registro locale ma non esistono più in OBS.
inventory-remove-stale-tooltip = Rimuovi la voce obsoleta
inventory-yaml-row-title = YAML del registro scene
inventory-yaml-row-subtitle = Esporta o importa ruoli delle scene, tag, flag di protezione e regole del grafo.
inventory-yaml-filter-name = File YAML

# Shared between the row's Export button, the export FileChooserNative's
# accept label, and (for Cancel) both the export and import dialogs.
inventory-export-button-label = Esporta
inventory-export-tooltip = Esporta il registro scene in YAML
inventory-import-button-label = Importa
inventory-import-tooltip = Importa il registro scene da YAML
inventory-dialog-cancel-label = Annulla

inventory-export-dialog-title = Esporta registro scene
inventory-export-success = Registro scene esportato in { $path }.
inventory-export-error = Esportazione non riuscita: { $error }
inventory-export-no-file = Esportazione non riuscita: nessun file selezionato.

inventory-import-dialog-title = Importa registro scene
inventory-import-error = Importazione non riuscita: { $error }
inventory-import-no-file = Importazione non riuscita: nessun file selezionato.

## window.rs

window-stream-live-tooltip = Streaming in diretta
window-about-tooltip = Informazioni su SceneDeck
window-refresh-tooltip = Aggiorna la pagina corrente

window-stream-status-line = Stream: { $state }{ $elapsed }
window-record-status-line = Registrazione: { $state }{ $elapsed }

window-status-connecting = Connessione a OBS in corso…
window-connect-btn-connecting = Connessione in corso…
window-current-scene-none = Scena corrente: —
window-status-connected = Connesso — OBS { $version }
window-connect-btn-disconnect = Disconnetti
window-status-disconnected = Disconnesso
window-connect-btn-connect = Connetti a OBS
window-live-disconnected-hint = Connettiti a OBS per usare i controlli di Diretta
window-current-scene = Scena corrente: { $scene }
window-status-error = Errore: { $error }
window-connect-btn-retry = Riprova
window-obs-connection-failed = Connessione a OBS non riuscita
window-toast-obs-error = Errore OBS: { $error }

window-output-kind-stream = Stream
window-output-kind-record = Registrazione

window-sidebar-output-starting = Avvio in corso…
window-sidebar-output-stopping = Arresto in corso…
window-sidebar-output-reconnecting = Riconnessione in corso…
window-sidebar-output-working = Operazione in corso…

window-sidebar-start-stream = Avvia stream
window-sidebar-stop-stream = Interrompi stream
window-sidebar-start-recording = Avvia registrazione
window-sidebar-stop-recording = Interrompi registrazione

window-selector-profile-label = Profilo
window-selector-profile-tooltip = Cambia profilo OBS
window-selector-collection-label = Raccolta
window-selector-collection-tooltip = Cambia raccolta di scene OBS

## live.rs

live-start-stream-label = Avvia stream
live-stop-stream-label = Interrompi stream
live-start-record-label = Avvia registrazione
live-stop-record-label = Interrompi registrazione
live-stream-toggle-tooltip = Avvia o interrompi lo streaming
live-record-toggle-tooltip = Avvia o interrompi la registrazione
live-stream-inactive-label = Stream: Inattivo
live-record-inactive-label = Registrazione: Inattiva
live-copy-last-recording-path-tooltip = Copia il percorso dell'ultima registrazione
live-copied-recording-path-tooltip = Percorso dell'ultima registrazione copiato
live-copy-recording-path-with-value-tooltip = Copia il percorso della registrazione: { $path }
live-stream-card-title = Stream
live-recording-card-title = Registrazione
live-current-scene-placeholder = Scena corrente: —
live-scenes-section-label = Scene
live-scenes-connect-hint = Connettiti a OBS per caricare le scene.
live-audio-section-label = Audio
live-disconnected-title = Connettiti a OBS per usare i controlli di Diretta
live-disconnected-detail = Usa il controllo di connessione in fondo alla barra laterale.
live-stream-command-error-label = Comando stream non riuscito
live-recording-command-error-label = Comando di registrazione non riuscito
live-last-recording-detail = Ultima registrazione: { $path }
live-starting-stream = Avvio dello stream in corso…
live-stopping-stream = Interruzione dello stream in corso…
live-reconnecting-stream = Riconnessione dello stream in corso…
live-starting-recording = Avvio della registrazione in corso…
live-stopping-recording = Interruzione della registrazione in corso…
live-reconnecting-recording = Riconnessione della registrazione in corso…
live-button-starting = Avvio in corso…
live-button-stopping = Arresto in corso…
live-button-reconnecting = Riconnessione in corso…
live-button-working = Operazione in corso…
live-output-kind-stream = Stream
live-output-kind-record = Registrazione
live-output-label = { $kind }: { $state }
live-output-label-with-elapsed = { $kind }: { $state } · { $elapsed }
live-scenes-no-primary-hint = Nessuna scena con ruolo Primaria trovata. Assegna i ruoli in Inventario.
live-audio-empty-hint = Nessun ingresso audio configurato.
live-cancel-button-label = Annulla
live-start-stream-confirm-heading = Avviare lo stream?
live-start-stream-confirm-body = OBS inizierà a inviare lo stream in diretta.
live-stop-stream-confirm-heading = Interrompere lo stream?
live-stop-stream-confirm-body = OBS smetterà di inviare lo stream in diretta.
live-start-recording-confirm-heading = Avviare la registrazione?
live-start-recording-confirm-body = OBS avvierà una nuova registrazione.
live-start-recording-confirm-label = Avvia registrazione
live-stop-recording-confirm-heading = Interrompere la registrazione?
live-stop-recording-confirm-body = OBS interromperà la registrazione in corso.
live-stop-recording-confirm-label = Interrompi registrazione

## audio_card.rs
audio-card-mute-tooltip = Disattiva ingresso audio
audio-card-source-path-tooltip = { $scope }: { $path }
audio-card-fader-tooltip = Regolatore di volume
audio-card-lock-tooltip = Blocca il cursore del volume
audio-card-fine-plus-tooltip = +1 dB
audio-card-fine-reset-tooltip = Ripristina a 0.0 dB
audio-card-fine-minus-tooltip = -1 dB

## scene_card.rs
scene-card-tooltip = { $status } ({ $role })
scene-card-role-suffix = Scena { $role }

## status_bar.rs
status-bar-stream-inactive = Stream: Inattivo
status-bar-record-inactive = Registrazione: Inattiva
status-bar-fps-placeholder = FPS —
status-bar-cpu-placeholder = CPU —
status-bar-bitrate-placeholder = Bitrate —
status-bar-fps = FPS { $value }
status-bar-cpu = CPU { $value }%
status-bar-bitrate = Bitrate { $value } kbps
status-bar-dropped = { $count } persi

## settings.rs

settings-page-title = Impostazioni
settings-appearance-title = Aspetto
settings-appearance-description = Le app GNOME dovrebbero seguire lo stile di sistema per impostazione predefinita.
settings-theme-mode-system = Sistema
settings-theme-mode-light = Chiaro
settings-theme-mode-dark = Scuro
settings-color-scheme-title = Schema colore
settings-color-scheme-subtitle = Segui la preferenza di sistema oppure forza chiaro / scuro
settings-theme-title = Tema
settings-theme-status-title = Stato del tema
settings-theme-status-initial = Tema caricato.
settings-failed-to-save = Salvataggio non riuscito: { $err }
settings-custom-css-title = CSS personalizzato
settings-custom-css-subtitle = Carica file CSS utente separati per la modalità chiara e scura
settings-custom-light-css-title = Percorso CSS personalizzato chiaro
settings-custom-dark-css-title = Percorso CSS personalizzato scuro
settings-reload-css-title = Ricarica CSS personalizzato
settings-reload-css-subtitle = Riapplica il tema selezionato e il file CSS personalizzato chiaro/scuro corrispondente.
settings-reload-button = Ricarica
settings-language-title = Lingua
settings-language-description = Le modifiche hanno effetto dopo il riavvio di SceneDeck.
settings-display-language-title = Lingua dell'interfaccia
settings-display-language-subtitle = Scegli una lingua oppure segui le impostazioni internazionali di sistema.
settings-language-status-title = Stato della lingua
settings-language-status-initial = Riavvia per applicare una lingua modificata.
settings-language-saved = Lingua salvata. Riavvia SceneDeck per applicarla.
settings-obs-connection-title = Connessione OBS
settings-obs-connection-description = Impostazioni WebSocket per OBS Studio (porta predefinita: 4455).
settings-host-title = Host
settings-port-title = Porta
settings-password-title = Password (opzionale)
settings-obs-status-title = Stato OBS
settings-invalid-port = Numero di porta non valido.
settings-saved = Impostazioni salvate.
settings-password-saved = Password salvata nel portachiavi.
settings-keyring-error = Errore del portachiavi: { $err }
settings-output-safety-title = Sicurezza output
settings-output-safety-description = Conferme facoltative per le azioni critiche di stream e registrazione.
settings-confirm-start-stream-title = Conferma avvio stream
settings-confirm-start-stream-subtitle = Chiedi conferma prima di avviare lo stream in diretta.
settings-confirm-stop-stream-title = Conferma interruzione stream
settings-confirm-stop-stream-subtitle = Chiedi conferma prima di interrompere lo stream in diretta.
settings-confirm-start-recording-title = Conferma avvio registrazione
settings-confirm-start-recording-subtitle = Chiedi conferma prima di avviare una registrazione.
settings-confirm-stop-recording-title = Conferma interruzione registrazione
settings-confirm-stop-recording-subtitle = Chiedi conferma prima di interrompere una registrazione.
settings-obs-not-connected = Non connesso a OBS.
settings-obs-connecting = Connessione a OBS in corso…
settings-obs-connected = Connesso — OBS { $version }
settings-obs-error = Errore: { $err }
settings-theme-subtitle = { $description } Campioni: { $swatches }
settings-theme-loaded = Caricato { $theme } ({ $variant }).
settings-theme-loaded-with-warnings = Tema caricato con avvisi.

## theme.rs

theme-adwaita-default-name = Adwaita predefinito
theme-adwaita-default-desc = Stile neutro che segue le impostazioni predefinite di GNOME.
theme-scenedeck-dark-name = SceneDeck Scuro
theme-scenedeck-dark-desc = Un tema console scuro e sobrio per l'uso in diretta.
theme-scenedeck-light-name = SceneDeck Chiaro
theme-scenedeck-light-desc = Un tema console chiaro e nitido con contrasto contenuto.
theme-obsidian-name = Obsidian
theme-obsidian-desc = Superfici grafite ad alta leggibilità con accenti freddi.
theme-nord-name = Nord
theme-nord-desc = Superfici blu-grigio fredde con accenti color ghiaccio.
theme-dracula-inspired-name = Ispirato a Dracula
theme-dracula-inspired-desc = Una palette scura ed espressiva con CSS originale.
theme-solarized-dark-name = Solarized Scuro
theme-solarized-dark-desc = Contrasto poco abbagliante con accenti verde acqua e ambra.
theme-high-contrast-name = Alto contrasto
theme-high-contrast-desc = Contorni e contrasto più marcati per i controlli critici.
theme-stream-red-name = Rosso Stream
theme-stream-red-desc = Accenti rossi orientati alla trasmissione per gli stati in diretta.
theme-studio-purple-name = Viola Studio
theme-studio-purple-desc = Accenti viola controllati senza sovrastare le superfici.
theme-ubuntu-violet-name = Viola Ubuntu
theme-ubuntu-violet-desc = Superfici viola ispirate a Ubuntu con un accento caldo per la diretta.
theme-custom-css-read-failed = Impossibile leggere il CSS personalizzato da { $path }: { $err }
theme-custom-css-no-matching-file = Il CSS personalizzato è abilitato ma non è impostato un file chiaro/scuro corrispondente.
theme-css-no-display = { $label } non è stato caricato perché non è disponibile alcun display GTK.
theme-css-parse-error = Errore di analisi CSS di { $label }: { $message }
