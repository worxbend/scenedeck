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
events-resync-warning = Alcune notifiche di OBS sono andate perse. SceneDeck si sta risincronizzando; la cronologia delle uscite potrebbe essere incompleta.

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
page-stats = Statistiche
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
inventory-stale-group-description = Controlla le voci delle scene mancanti o con conflitti di identità. Le voci conservate non si applicano alle scene attuali.
inventory-remove-stale-tooltip = Rimuovi la voce obsoleta
inventory-yaml-row-title = YAML del registro scene
inventory-yaml-row-subtitle = Esporta o importa ruoli delle scene, tag, flag di protezione e regole del grafo.
inventory-yaml-filter-name = File YAML
inventory-accent-clear-tooltip = Rimuovi il colore di accento della scena
inventory-drag-tooltip = Trascina per riordinare la scena
inventory-accent-dialog-title = Colore di accento della scena
inventory-accent-choose-tooltip = Scegli il colore di accento della scena

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
audio-card-meter-tooltip-title = Misuratore di volume: { $channels }
audio-card-meter-tooltip-zones = Verde sotto -20 dB · Giallo fino a -9 dB · Rosso sopra, vicino alla distorsione
audio-card-meter-tooltip-indicators = Barra: livello di picco con decadimento · Linea: picco più alto in 20 s · Punto: intensità sonora · Base: livello in arrivo dal dispositivo
audio-card-meter-tooltip-waiting = Misuratore di volume: in attesa dei livelli di OBS

## icon.rs
icon-none = Nessuna icona
icon-camera = Videocamera
icon-desktop = Desktop
icon-game = Gioco
icon-film = Film
icon-images = Immagini
icon-television = Televisione
icon-browser = Browser
icon-terminal = Terminale
icon-code = Codice
icon-chat = Chat
icon-guests = Ospiti
icon-star = Stella
icon-alert = Avviso
icon-break = Pausa
icon-countdown = Conto alla rovescia
icon-start = Avvio
icon-pause = Sospendi
icon-stop = Arresto
icon-settings = Impostazioni
icon-layers = Livelli
icon-microphone = Microfono
icon-headset = Cuffie con microfono
icon-headphones = Cuffie
icon-speaker = Altoparlante
icon-volume = Volume
icon-music = Musica
icon-instrument = Strumento
icon-radio = Radio
icon-call = Chiamata
icon-waveform = Forma d'onda
inventory-scene-icon-tooltip = Scegli un'icona per questa scena
mixer-input-icon-tooltip = Scegli un'icona per questa sorgente audio

## meter.rs
audio-meter-zone-nominal = Verde
audio-meter-zone-warning = Giallo
audio-meter-zone-error = Rosso
audio-meter-channel-mono = Mono
audio-meter-channel-left = Sinistro
audio-meter-channel-right = Destro
audio-meter-channel-front-left = Anteriore sinistro
audio-meter-channel-front-right = Anteriore destro
audio-meter-channel-front-center = Anteriore centrale
audio-meter-channel-lfe = LFE
audio-meter-channel-rear-left = Posteriore sinistro
audio-meter-channel-rear-right = Posteriore destro
audio-meter-channel-side-left = Laterale sinistro
audio-meter-channel-side-right = Laterale destro
audio-meter-channel-numbered = Canale { $index }

## hotkey.rs
hotkey-modifier-ctrl = Ctrl+
hotkey-modifier-alt = Alt+
hotkey-modifier-shift = Maiusc+
hotkey-modifier-super = Super+
hotkey-modifier-ctrl-alt = Ctrl+Alt+
hotkey-modifier-ctrl-shift = Ctrl+Maiusc+
hotkey-style-plain = Solo cifra
hotkey-style-ctrl = Ctrl + cifra
hotkey-style-alt = Alt + cifra
hotkey-style-shift = Maiusc + cifra
hotkey-style-super = Super + cifra
hotkey-style-ctrl-alt = Ctrl+Alt + cifra
hotkey-style-ctrl-shift = Ctrl+Maiusc + cifra
hotkey-style-leader = Tasto leader, poi cifra
hotkey-leader-space = Spazio
hotkey-leader-comma = Virgola
hotkey-leader-semicolon = Punto e virgola
hotkey-leader-backslash = Barra rovesciata
hotkey-leader-grave = Apice inverso
hotkey-shortcut-leader = { $leader } poi { $digit }
hotkey-hint-plain = Premi 1 … 0
hotkey-hint-modifier = Premi { $modifier }1 … 0
hotkey-hint-leader = Premi { $leader }, poi 1 … 0
hotkey-hint-leader-armed = Leader attivo — premi 1 … 0
hotkey-hint-empty-slot = Nessuna scena nella posizione { $slot }

## scene_card.rs
scene-card-tooltip = { $status } ({ $role })
scene-card-tooltip-with-hotkey = { $status } ({ $role }) · { $hotkey }
scene-card-role-suffix = Scena { $role }
scene-card-tooltip-ready = Passa a questa scena

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
status-bar-dropped-placeholder = Persi —

## settings.rs

settings-page-title = Impostazioni
settings-appearance-title = Aspetto
settings-appearance-description = Le app GNOME dovrebbero seguire lo stile di sistema per impostazione predefinita.
settings-theme-mode-system = Sistema
settings-theme-mode-light = Chiaro
settings-theme-mode-dark = Scuro
settings-color-scheme-title = Schema colore
settings-color-scheme-subtitle = Segui la preferenza di sistema oppure forza chiaro / scuro
settings-motion-title = Movimento
settings-motion-subtitle = Quanto si anima l'interfaccia. Gli indicatori live restano leggibili a ogni livello.
settings-motion-full = Completo
settings-motion-reduced = Ridotto
settings-motion-off = Disattivato
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
language-system-default = Predefinito di sistema
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
settings-hotkeys-title = Scorciatoie scene
settings-hotkeys-description = Cambia le scene di Live dalla tastiera. Le posizioni seguono l'ordine delle scene impostato in Inventario.
settings-hotkeys-enabled-title = Abilita le scorciatoie scene
settings-hotkeys-enabled-subtitle = I tasti numerici cambiano le schede scena della pagina Live, nell'ordine in cui compaiono.
settings-hotkeys-style-title = Combinazione di tasti
settings-hotkeys-style-subtitle = Quali tasti cambiano scena. Le cifre da sole restano inattive mentre un campo di testo ha il focus.
settings-hotkeys-leader-title = Tasto leader
settings-hotkeys-leader-subtitle = Primo tasto della scorciatoia a due battute, in stile vim.
settings-hotkeys-timeout-title = Timeout del leader
settings-hotkeys-timeout-subtitle = Quanto attende il leader la sua cifra, in millisecondi.
settings-hotkeys-preview-title = Assegnazioni attuali
settings-hotkeys-preview-subtitle = { $first } … { $last } cambiano le prime { $count } scene di Live.
settings-hotkeys-preview-disabled = Le scorciatoie scene sono disattivate.
settings-obs-not-connected = Non connesso a OBS.
settings-obs-connecting = Connessione a OBS in corso…
settings-obs-connected = Connesso — OBS { $version }
settings-obs-error = Errore: { $err }
settings-theme-loaded = Caricato { $theme } ({ $variant }).
settings-theme-loaded-with-warnings = Tema caricato con avvisi.
theme-variant-light = Chiaro
theme-variant-dark = Scuro

## theme.rs

theme-adwaita-default-name = Adwaita predefinito
theme-adwaita-default-desc = Stile neutro che segue le impostazioni predefinite di GNOME.
theme-scenedeck-dark-name = SceneDeck Scuro
theme-scenedeck-dark-desc = Un tema console scuro e sobrio per l'uso in diretta.
theme-scenedeck-light-name = SceneDeck Chiaro
theme-scenedeck-light-desc = Un tema console chiaro e nitido con contrasto contenuto.
theme-obs-name = OBS
theme-obs-desc = L'aspetto predefinito di OBS Studio, reinterpretato con libadwaita.
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
## stats.rs — live streaming telemetry
stats-page-title = Statistiche di trasmissione
stats-page-subtitle = Telemetria in tempo reale richiesta a OBS una volta al secondo quando la connessione è attiva.
stats-gauge-fps = FPS
stats-gauge-frame-time = Tempo per fotogramma (ms)
stats-gauge-dropped = Fotogrammi persi
stats-gauge-congestion = Congestione
stats-chart-fps = Fotogrammi al secondo
stats-chart-frame-time = Tempo medio di rendering per fotogramma (ms)
stats-chart-output-skipped = Fotogrammi di uscita saltati per campione
stats-chart-render-skipped = Fotogrammi di rendering persi per campione
stats-card-render-frames = Fotogrammi di rendering persi
stats-card-output-frames = Fotogrammi di uscita saltati
stats-card-stream-frames = Fotogrammi persi nella trasmissione
stats-card-frame-time = Tempo medio di rendering per fotogramma
stats-card-cpu = Utilizzo CPU di OBS
stats-card-memory = Utilizzo memoria di OBS
stats-card-bitrate = Bitrate della trasmissione
stats-value-placeholder = —
stats-value-frames = { $skipped } su { $total } ({ $percent } %)
stats-value-ms = { $value } ms
stats-value-percent = { $value } %
stats-value-mb = { $value } MB
stats-value-kbps = { $value } kb/s

## Guida, onboarding e messaggi aggiunti nella versione 0.4.
page-help = Guida
config-parse-failed-backed-up = Impossibile analizzare le impostazioni: { $detail } — il file è stato conservato in { $path } e sono stati caricati i valori predefiniti.
stats-popout-button = Apri le statistiche in una finestra separata

## ui/pages/help.rs — guida introduttiva. I corpi sono intenzionalmente su più
## righe: ogni riga viene visualizzata separatamente in un argomento espandibile.
help-page-title = Guida e introduzione
help-hero-title = Benvenuto in SceneDeck
help-hero-description = Una superficie di controllo nativa per Linux dedicata a OBS Studio. Questa guida illustra la prima connessione, mostra come limitare la pagina Diretta alle scene che usi davvero e spiega ogni pagina della barra laterale. Espandi un argomento per leggerlo.
help-expand-hint = Seleziona un argomento per espanderlo.

help-open-settings = Apri Impostazioni
help-open-inventory = Apri Inventario
help-open-doctor = Apri Diagnostica
help-open-live = Apri Diretta
help-open-mixer = Apri Mixer
help-open-graph = Apri Grafo
help-open-stats = Apri Statistiche

help-group-start-title = Per iniziare
help-group-start-description = Il percorso più breve da una nuova installazione al primo cambio di scena.

help-quickstart-title = Cinque passaggi per il primo cambio di scena
help-quickstart-subtitle = Seguili in ordine la prima volta
help-quickstart-body =
    1. In OBS Studio, apri Strumenti → Impostazioni del server WebSocket e seleziona «Abilita server WebSocket». Lascia OBS in esecuzione.
    2. Nella stessa finestra di OBS, premi «Mostra informazioni di connessione» e annota la porta del server (4455 per impostazione predefinita) e la password del server.
    3. In SceneDeck, apri Impostazioni e inserisci Host, Porta e Password. Se OBS viene eseguito sullo stesso computer, lascia 127.0.0.1 come Host.
    4. Premi Connetti in fondo alla barra laterale. La riga di stato sopra il pulsante diventa verde e mostra «Connesso».
    5. Apri Inventario e assegna il ruolo «Primaria» alle scene che vuoi selezionare durante una trasmissione. Quelle scene —e solo quelle— diventano schede nella pagina Diretta.

help-concepts-title = Come SceneDeck interpreta la tua configurazione
help-concepts-subtitle = Ruoli, registro e ciò che non modifica mai OBS
help-concepts-body =
    Le connessioni salvate hanno credenziali separate. Ogni connessione, indirizzo e raccolta di scene ha il proprio spazio di lavoro.
    Ruoli, tag, preferiti, ordine, colori, icone e viste del deck sono organizzazione locale: non modificano le scene OBS. Ricerca e viste salvate non cambiano le azioni associate.
    Gli UUID conosciuti conservano i metadati dopo una rinomina. Una sostituzione con UUID diverso non eredita l’identità.

help-group-connect-title = Connessione a OBS
help-group-connect-description = Su questo computer o dall'altra parte della stanza.

help-connect-local-title = Connessione a OBS su questo computer
help-connect-local-subtitle = Il caso predefinito: host 127.0.0.1, porta 4455
help-connect-local-body =
    127.0.0.1 è l'indirizzo usato da un computer per comunicare con sé stesso, quindi è l'Host corretto quando OBS e SceneDeck vengono eseguiti affiancati.
    La porta deve corrispondere alla porta del server nelle Impostazioni del server WebSocket di OBS. OBS usa 4455, a meno che tu non l'abbia cambiata.
    Se in OBS è selezionato «Abilita autenticazione», incolla la password in Impostazioni → Password. SceneDeck la conserva nel portachiavi del desktop (lo stesso luogo in cui il browser salva gli accessi), mai nel file di configurazione in testo semplice.
    Premi Connetti nella barra laterale oppure Ctrl+R in qualsiasi momento per riconnetterti.

help-connect-remote-title = Connessione a OBS su un altro computer
help-connect-remote-subtitle = PC di streaming in un angolo, controllo dal portatile
help-connect-remote-body =
    Questa è la configurazione a due computer: OBS viene eseguito sul computer che acquisisce e codifica, SceneDeck su quello davanti a te. Entrambi devono essere sulla stessa rete.
    Sul computer con OBS, in Strumenti → Impostazioni del server WebSocket, seleziona «Abilita server WebSocket» e «Abilita autenticazione», quindi imposta una password che puoi digitare. Non disattivare l'autenticazione: altrimenti chiunque sulla rete possa raggiungere la porta potrebbe avviare e fermare la trasmissione.
    Trova l'indirizzo del computer OBS direttamente su quel computer. Su Linux esegui `ip addr` e cerca un indirizzo come 192.168.1.42; su Windows esegui `ipconfig` e leggi l'indirizzo IPv4; su macOS si trova in Impostazioni di Sistema → Rete. È l'indirizzo del computer, non di OBS.
    Consenti la porta attraverso il firewall del computer OBS. Su Linux con ufw usa `sudo ufw allow 4455/tcp`; su Windows consenti OBS in Windows Defender Firewall per le reti private.
    Nelle Impostazioni di SceneDeck, inserisci quell'indirizzo come Host (ad esempio 192.168.1.42), lascia la Porta su 4455 e incolla la password. Premi Connetti.
    Per il computer OBS è preferibile una connessione cablata. I cambi di scena tramite Wi-Fi funzionano comunque, ma un pacchetto perso ritarda il cambio.
    Un consiglio che può salvare una trasmissione: riserva un indirizzo fisso al computer OBS nelle impostazioni DHCP del router, così l'Host salvato continuerà a funzionare dopo un riavvio.

help-connect-share-title = Consentire a un co-conduttore o moderatore di gestire il pannello
help-connect-share-subtitle = Un secondo SceneDeck collegato alla stessa regia
help-connect-share-body =
    Il server WebSocket di OBS accetta più client contemporaneamente, quindi una seconda persona su un altro computer può usare la propria copia di SceneDeck con la stessa regia: è la configurazione dell'argomento precedente, eseguita due volte.
    Non esistono accessi personali: chi dispone di Host, Porta e password nelle proprie Impostazioni ha esattamente il tuo stesso controllo, compreso l'avvio e l'arresto della trasmissione. Condividili solo con chi riceverebbe anche la tua sessione OBS.
    Riduci prima ciò che l'altra persona vedrà. Ruoli e scene nascoste risiedono nell'Inventario locale, non in OBS: configurali sul tuo computer e chiedi al collaboratore di esportare o ricreare lo YAML del registro scene invece di partire da un elenco completo e non filtrato.
    Non inoltrare la porta WebSocket su Internet per consentire l'accesso a un collaboratore remoto. Collegatevi entrambi alla stessa VPN o rete Tailscale, oppure create un tunnel SSH e impostate come Host l'indirizzo del tunnel.
    Tutti gli utenti connessi vedono lo stesso stato in diretta: un cambio di scena o lo spostamento di un fader da una parte compare immediatamente anche dall'altra, proprio come se foste davanti alla stessa tastiera.

help-connect-trouble-title = Quando Connetti non funziona
help-connect-trouble-subtitle = Leggi l'errore, poi segui questo elenco
help-connect-trouble-body =
    «Connessione rifiutata» significa quasi sempre che il server WebSocket non è abilitato in OBS o che la porta non corrisponde. Controlla entrambe in Strumenti → Impostazioni del server WebSocket.
    Una connessione che resta in attesa e poi scade indica in genere che un firewall sta bloccando il traffico o che l'indirizzo Host appartiene a un altro computer.
    «Autenticazione non riuscita» significa che la password è errata. Inseriscila di nuovo in Impostazioni; il campo è di sola scrittura, quindi appare vuoto anche quando è stata salvata una password.
    Nessuna indicazione nella barra di stato laterale? Verifica che OBS sia davvero in esecuzione e che non sia bloccato da una propria finestra di dialogo modale.
    SceneDeck si riconnette automaticamente dopo un'interruzione, mentre Ctrl+R forza subito un nuovo tentativo.

help-group-scenes-title = Organizzazione delle scene
help-group-scenes-description = La pagina Diretta deve mostrare ciò che selezioni e nient'altro.

help-scenes-hide-title = Nascondere le scene che non mandi mai in onda
help-scenes-hide-subtitle = La prima impostazione più utile
help-scenes-hide-body =
    Una configurazione OBS in uso accumula scene create soltanto per essere annidate in altre o realizzate per una prova e mai eliminate. Mostrarle su una superficie di controllo dal vivo facilita un cambio errato.
    SceneDeck mostra una scheda in Diretta solo per le scene con ruolo Primaria. Tutti gli altri ruoli sono nascosti, quindi «nascondere una scena» significa semplicemente assegnarle un ruolo diverso da Primaria.
    Apri Inventario. Ogni scena OBS ha una riga con un selettore di ruolo sulla destra.
    Imposta Primaria per le poche scene che mandi davvero in onda. Per tutte le altre scegli: Secondaria (una scena reale che talvolta serve, ma che non vuoi in Diretta), Modulo (una sovrimpressione o un sottopancia annidato in un'altra scena), Grezza (una videocamera o acquisizione di base), Debug (una scena di prova) o Archivio (conservata per dopo e tenuta da parte).
    Anche le scene prive di ruolo restano fuori da Diretta, quindi lasciarle Non assegnate le nasconde. È comunque meglio assegnare un ruolo intenzionalmente: la pagina Diagnostica segnala le scene Non assegnate per farti notare quelle nuove.
    La modifica ha effetto immediato: torna a Diretta e la scheda sarà scomparsa. In OBS non è cambiato nulla.

help-scenes-order-title = Ordine, colori e icone
help-scenes-order-subtitle = Rendi evidente la scheda giusta
help-scenes-order-body =
    Trascina una scena dalla maniglia a sinistra della sua riga in Inventario per impostare l'ordine. Le schede di Diretta e le scorciatoie numeriche seguono quell'ordine, quindi la scheda attivata con 1 è la prima.
    Il selettore del colore in evidenza tinge la scheda della scena in Diretta. Riserva un colore intenso alle scene importanti —la scena «siamo in diretta» o la lettura dello sponsor— per individuarle subito.
    Il selettore di icone a sinistra di ogni riga aggiunge un simbolo alla scheda in Diretta. Sono disponibili trenta icone, oltre a una voce «nessuna icona» per rimuoverla.
    Ordine, colori e icone vengono salvati nel registro locale, non in OBS.

help-scenes-registry-title = Backup e trasferimento della configurazione
help-scenes-registry-subtitle = La riga YAML del registro scene in Inventario
help-scenes-registry-body =
    Inventario esporta il registro dello spazio corrente in YAML, incluse viste, azioni, regole di verifica e preimpostazioni audio. Importare sostituisce solo questa organizzazione, non altri spazi né le credenziali OBS.
    Scorciatoie e azioni rapide importate sono inizialmente disabilitate. Controlla le identità prima di abilitarle. Gli UUID seguono le rinomine; lo stesso nome con altro UUID indica un altro elemento. Le voci obsolete si eliminano in Inventario.

help-group-operate-title = Durante una trasmissione
help-group-operate-description = Le pagine che usi davvero mentre sei in onda.

help-live-title = La pagina Diretta
help-live-subtitle = Parete di scene, Studio, audio e uscite
help-live-body =
    Live è una parete di scene. Anteprime delle scene nelle Impostazioni aggiunge miniature ombreggiate e schede più grandi; disattivandolo tornano i riquadri compatti.
    In modalità diretta, schede e scorciatoie cambiano Programma. L'interruttore Studio nella barra superiore segue OBS: le schede preparano Anteprima e Transizione la invia a Programma. Transizione e durata supportata appaiono solo in modalità Studio.
    Espandi la barra audio sotto le scene per altri controlli o apri Mixer. Streaming, registrazione, buffer di replay e videocamera virtuale condividono una barra di uscita.
    Sorgenti nella barra superiore apre visibilità e impostazioni dettagliate. Esaminare una scena lì non cambia mai Programma.
    Nessuna scheda dopo la connessione? Assegna il ruolo Principale in Inventario alle scene desiderate su Live.

help-hotkeys-title = Cambiare scena dalla tastiera
help-hotkeys-subtitle = Ctrl+1 … Ctrl+0 per impostazione predefinita e configurabili
help-hotkeys-body =
    Le prime dieci scene selezionabili nell’ordine salvato dell’Inventario ricevono le cifre 1–9, poi 0. Ogni scheda visibile mostra la cifra assegnata; il suggerimento della scena indica la combinazione configurata.
    La ricerca, le viste salvate e i preferiti in cima non modificano i numeri. Una scorciatoia può attivare una scena nascosta dal filtro. Riordinare l’Inventario senza filtri cambia le assegnazioni.
    Impostazioni → Scorciatoie scene consente di scegliere come premere la cifra. Un modificatore più la cifra (Ctrl per impostazione predefinita) evita attivazioni accidentali mentre digiti. La sola cifra è l'opzione più rapida. Lo stile con tasto leader funziona come in vim: premi il tasto leader, rilascialo e poi premi la cifra; mentre attende, la didascalia mostra «Leader attivo».
    Le scorciatoie delle scene funzionano solo in Live e si sospendono quando un campo di testo ha il focus. Una cifra senza scena assegnata indica una posizione vuota senza cambiare scena.

help-audio-title = Audio: la pagina Mixer e gli indicatori
help-audio-subtitle = Cosa indicano le barre colorate
help-audio-body =
    La barra audio compatta di Live si espande per modifiche rapide; Mixer offre fader completi. Prima compaiono i dispositivi audio globali di OBS, poi le sorgenti della scena attuale, comprese scene annidate e gruppi.
    Le modalità del Mixer stabiliscono l'audio di quale scena stai osservando. Attiva segue la scena di programma OBS. Selezionata carica una scena e resta su di essa. Fissata mantiene una scena scelta come destinazione permanente mentre OBS prosegue.
    L'indicatore accanto a ogni fader va da -60 dB in basso a 0 dB in alto e usa le soglie di OBS: verde sotto -20 dB per musica e sottofondo, giallo da -20 a -9 dB per la voce, rosso sopra -9 dB dove inizia la distorsione. Nessun segnale dovrebbe restare sul rosso.
    Una colonna indica una sorgente mono; due indicano stereo, sinistra e poi destra. Se si muove solo la colonna sinistra, metà degli spettatori non sentirà quella sorgente.
    La linea sopra il riempimento mostra il picco più alto degli ultimi venti secondi, il modo più rapido per individuare una distorsione sfuggita. Il quadrato in basso mostra il livello in arrivo dal dispositivo prima del fader: se è troppo alto, spostare il fader non lo correggerà.
    Il pulsante di blocco su una scheda immobilizza soltanto il cursore di SceneDeck. Non blocca nulla in OBS.

help-outputs-title = Streaming, registrazione, replay e videocamera virtuale
help-outputs-subtitle = E le conferme che evitano un incidente
help-outputs-body =
    La barra di uscita di Live riunisce streaming, registrazione, buffer di replay e videocamera virtuale. Altri controlli di uscita apre pausa/ripresa, divisione del file/capitoli e risultati recenti.
    Impostazioni → Sicurezza uscite controlla le conferme per avviare e fermare streaming e registrazione. L'arresto richiede conferma per impostazione predefinita.
    Eventi e osservazioni aggiornate di OBS determinano lo stato mostrato. Una richiesta accettata da sola non dimostra che un'uscita sia partita o che un file sia stato salvato. I percorsi appartengono al computer di OBS.

help-group-inspect-title = Verifica della configurazione
help-group-inspect-description = Trova l'imprevisto prima che accada in onda.

help-doctor-title = Diagnostica
help-doctor-subtitle = Problemi strutturali ordinati per gravità
help-doctor-body =
    Diagnostica legge l'elenco delle scene, le assegnazioni dei ruoli e gli annidamenti, quindi segnala ciò che sembra errato come Errori, Avvisi e Info.
    Tra i problemi tipici figurano scene senza ruolo, scene memorizzate che non esistono più in OBS, annidamenti circolari e scene annidate in una direzione non consentita dai ruoli.
    Viene rieseguita ogni volta che apri la pagina; il pulsante Riesegui la forza immediatamente.
    Vale la pena controllarla dopo ogni modifica alla configurazione OBS e ancora una volta prima di andare in onda.

help-graph-title = Grafo
help-graph-subtitle = Quali scene sono contenute nelle altre
help-graph-body =
    Annidare una scena in un'altra permette di creare sovrimpressioni e layout condivisi in OBS, ma può anche rendere una scena dipendente da qualcosa che hai dimenticato.
    Grafo elenca ogni scena principale e ciò che contiene, e confronta ciascuna relazione con le regole dei ruoli: corretta, dubbia o vietata.
    Usalo per rispondere a «cosa smetterà di funzionare se modifico questa scena?» prima di intervenire.

help-stats-title = Statistiche
help-stats-subtitle = Se il computer riesce a tenere il passo
help-stats-body =
    Gli indicatori di FPS, tempo di rendering dei fotogrammi, fotogrammi persi e congestione di rete diventano prima ambra e poi rossi quando ogni valore peggiora: i fotogrammi persi avvisano all'1%, la congestione al 30%.
    I grafici delle tendenze conservano circa gli ultimi due minuti, mentre gli istogrammi mostrano quando sono stati persi i fotogrammi invece di un totale progressivo, così puoi capire se uno scatto era un singolo momento negativo o una tendenza.
    I campioni vengono raccolti per tutto il tempo in cui sei connesso, quindi aprire Statistiche durante una trasmissione mostra i minuti precedenti invece di partire da zero.
    I contatori dei fotogrammi provengono da OBS e si azzerano quando OBS o l'uscita di trasmissione vengono riavviati.

help-group-personalise-title = Personalizzazione
help-group-personalise-description = Aspetto, lingua e posizione dei file.

help-appearance-title = Temi e aspetto
help-appearance-subtitle = Compreso un aspetto coordinato con OBS
help-appearance-body =
    Lo schema colori segue per impostazione predefinita la preferenza chiara o scura del desktop, ma puoi forzare una delle due modalità.
    I temi sono famiglie compatibili con chiaro e scuro: scegline uno e verrà applicata la variante corrispondente allo schema colori corrente. La famiglia OBS riproduce l'aspetto di OBS Studio, così la superficie di controllo non stona con l'applicazione che gestisce.
    Il CSS personalizzato usa file chiari e scuri separati, così anche un aspetto personalizzato segue lo schema colori. Ricarica CSS personalizzato applica le modifiche senza riavviare.
    Movimento controlla quanto viene animata l'interfaccia: impostalo su Ridotto o Disattivato se il movimento distrae o il computer è sotto carico.

help-files-title = Dove SceneDeck conserva i dati
help-files-subtitle = Configurazione, registro e password di OBS
help-files-body =
    config.json salva impostazioni e connessioni in $XDG_CONFIG_HOME/scenedeck (di solito ~/.config/scenedeck). show-workspaces.json salva gli spazi; il vecchio registry.json viene conservato e adottato una sola volta.
    bookmarks.json salva le note. session-history.json opzionale si trova in $XDG_STATE_HOME/scenedeck (di solito ~/.local/state/scenedeck).
    Le password restano in Secret Service, separate per connessione. Gli export JSON/YAML non includono credenziali. Salva i file necessari e trasferisci le password separatamente.

help-shortcuts-title = Scorciatoie da tastiera
help-shortcuts-subtitle = L'elenco completo
help-shortcuts-body =
    F1 — apri questa guida.
    Ctrl+R — riconnettiti a OBS.
    Ctrl+, — apri Impostazioni.
    Ctrl+Q — esci da SceneDeck.
    Ctrl+1 … Ctrl+0 in Live — attiva la scena assegnata a quella posizione dell’Inventario, anche se filtrata. Configura la combinazione in Impostazioni → Scorciatoie scene.
    { palette-title }

## ui/window.rs — finestra di benvenuto al primo avvio
welcome-dialog-heading = Benvenuto in SceneDeck
welcome-dialog-body = Sembra che questo sia il primo avvio. La pagina Guida spiega come connettersi a OBS —anche quando OBS è su un altro computer— e come limitare la pagina Diretta alle scene che usi davvero. Bastano un paio di minuti e si evitano alcuni errori.
welcome-dialog-later = Non ora
welcome-dialog-open = Leggi la guida
window-help-tooltip = Guida e introduzione

# OBS controls and accessible alternatives
window-stop-disconnect-label = Ferma diretta/registrazione e disconnetti
window-stop-stream-disconnect-label = Ferma diretta e disconnetti
window-stop-record-disconnect-label = Ferma registrazione e disconnetti
disconnect-confirm-heading = Fermare diretta e registrazione prima di disconnettersi?
disconnect-confirm-body = Diretta: { $stream }. Registrazione: { $recording }. Le dirette e le registrazioni attive verranno fermate prima di disconnettere SceneDeck. Questa azione non ferma il buffer replay né la videocamera virtuale.
live-scenes-obs-empty = Questa raccolta OBS non contiene scene. Crea una scena in OBS, poi aggiorna SceneDeck.
live-organize-scenes = Organizza le scene
mixer-secondary-options = Raggruppamento e ricerca
inventory-move-up = Sposta { $scene } in alto
inventory-move-down = Sposta { $scene } in basso
graph-relationship-list = Relazioni tra scene — elenco leggibile
graph-no-dependencies = Nessuna dipendenza da scene annidate
audio-source-mute = Disattiva audio di { $source }
audio-source-lock = Blocca il fader locale di { $source }
audio-source-volume = Volume di { $source }
audio-source-meter = Indicatore di picco audio di { $source }
audio-meter-no-level = Nessun livello visibile
audio-meter-no-observation = Nessuna lettura del livello audio

# Session activity

workflow-activity-title = Attività recente
workflow-activity-detail = Solo sessione corrente, ultime 30 voci. Richieste, osservazioni ed errori sono separati. Le disconnessioni lasciano lacune; la coincidenza temporale non prova un nesso causale.
workflow-check-item = Visibilità della sorgente
workflow-activity-requested = Richiesto
workflow-activity-observed = Osservato
workflow-activity-failed = Non riuscito
workflow-activity-connection = Connessione


# Native OBS inspector
control-refresh = Aggiorna osservazioni OBS
control-inspection-note = Esamina qualsiasi scena senza cambiare Programma. Le modifiche alla visibilità riguardano la scena o il gruppo proprietario.
control-disconnected = Connettiti a OBS per esaminare le sorgenti.
control-loading = Lettura dello stato di OBS…
control-observed = Ultima osservazione: { $seconds } s fa. Aggiorna prima di basarti su questo stato.
control-scene-selector = Scena da esaminare — non cambia Programma
control-request-hint = Aggiorna per caricare i controlli nativi OBS e la visibilità delle sorgenti.
control-resume = Riprendi registrazione
control-pause = Metti in pausa registrazione
control-split = Dividi file di registrazione
control-replay-stop = Ferma buffer replay
control-replay-start = Avvia buffer replay
control-replay-save = Salva replay
control-camera-stop = Ferma videocamera virtuale
control-camera-start = Avvia videocamera virtuale
control-chapter-name = Nome del capitolo di registrazione
control-chapter = Crea capitolo
control-host-path = Percorso dell’ultimo replay sull’host OBS: { $path }
control-output-controls = Registrazione, replay e videocamera virtuale
control-source-search = Cerca sorgenti o percorsi proprietari
control-inspect-source = Esamina impostazioni e utilizzo della sorgente
control-hidden-parent = Un elemento antenato è nascosto. Abilitare solo questo elemento potrebbe non renderlo visibile.
control-sources-empty = Nessun elemento scena in questa osservazione.
control-shared-source = Audio, filtri, media e testo modificano questa sorgente ovunque sia usata. La visibilità riguarda il suo contenitore proprietario.
control-used-in = Usata in — percorsi strutturali
control-image-refresh = Acquisisci immagine fissa
control-image-alt = Immagine fissa di una scena OBS acquisita su richiesta
control-image-error = Impossibile decodificare l’immagine OBS.
control-image-note = Immagine fissa dell’ultima acquisizione. Non è un video in diretta.
control-studio-enable = Abilita modalità Studio
control-program = Programma: { $scene }
control-preview = Anteprima: { $scene }
control-stage = Prepara la scena esaminata
control-take = Transizione
control-transition = Transizione scena corrente
control-duration = Durata della transizione in millisecondi
control-apply = Applica
control-studio = Studio e transizione
control-scene-mode-changed = La modalità Studio è cambiata; aggiorna prima di attivare la scena. L’azione non è stata ripetuta.
control-filters = Filtri della sorgente
control-text-file = Questa sorgente di testo legge un file. Modifica il file sull’host OBS.
control-text-save = Salva testo in OBS
control-text = Sorgente di testo integrata
control-media-play = Riproduci
control-media-pause = Pausa
control-media-stop = Ferma
control-media-restart = Riavvia
control-media-previous = Precedente
control-media-next = Successivo
control-media-seconds = Posizione del media in secondi
control-media-seek = Vai alla posizione
control-media = Riproduzione media
control-monitor-host = Il monitoraggio audio riproduce sull’host OBS, non tramite SceneDeck.
control-monitor-off = Monitoraggio disattivato
control-monitor-only = Solo monitoraggio — uscita silenziata
control-monitor-output = Monitoraggio e uscita
control-audio-sync = Offset sincronizzazione audio (millisecondi)
control-audio-balance = Bilanciamento (0 = sinistra, 0,5 = centro, 1 = destra)
control-audio-track = Traccia audio { $track }
control-audio-tracks = Instradamento tracce audio
control-advanced-audio = Audio avanzato della sorgente


# Operating workspace
workspace-action-acknowledged = Richiesta confermata; stato aggiornato da OBS
workspace-replay-observed = OBS ha segnalato un replay salvato: { $path }
workspace-disk-space = Spazio disco di registrazione OBS: { $value } MiB
control-stale = Ultimo stato noto. Aggiorna le osservazioni OBS per abilitare le modifiche.
control-no-matches = Nessuna sorgente corrisponde alla ricerca. Cancella la ricerca per mostrare tutti gli elementi.


# OBS recording volume and session activity
stats-recording-disk = Spazio libero sul volume di registrazione OBS
stats-recording-disk-value = { $value } MiB
stats-activity-title = Attività recente e contesto dei grafici
stats-activity-detail = Ultime 15 voci della sessione. Seleziona una voce per segnarne l’ora sui grafici. Interruzioni di connessione, campioni mancanti e azzeramenti dei contatori limitano l’interpretazione. La coincidenza temporale non prova un nesso causale.
stats-activity-clear-marker = Rimuovi indicatore dal grafico
stats-activity-marker-visible = Le linee tratteggiate segnano l’evento selezionato tra i campioni osservati. I grafici usano la spaziatura dei campioni.
stats-activity-marker-unavailable = Nessun indicatore: l’evento è fuori dai campioni conservati, in una lacuna di campionamento o influenzato da una modifica dell’orologio.
stats-activity-empty = Nessuna operazione ancora osservata in questa sessione.
stats-activity-requested = Richiesto
stats-activity-observed = Osservato
stats-activity-failed = Non riuscito
stats-activity-connection = Connessione
stats-activity-entry = { $time } · { $origin } · { $description }
scene-card-stage-hint = Prepara questa scena in Anteprima. Usa il comando di passaggio per cambiare Programma.
scene-card-preview-label = Anteprima
hotkey-hint-stage-take = { $binding } · Prepara in Anteprima; usa il passaggio per cambiare Programma.


header-obs-setup = Configurazione OBS
header-obs-unknown = Sconosciuto
header-obs-context = Profilo: { $profile } · Raccolta scene: { $collection }

status-bar-connection-detail = { $state } · OBS { $version }


doctor-structural-title = Struttura e ruoli delle scene


control-image-caption = Immagine di { $scene }, acquisita { $seconds } s fa. Aggiorna l’immagine per acquisirla di nuovo.

output-results-acknowledged = Confermato

output-results-copy-path = Copia percorso dell’host OBS

output-results-host-path = Percorso sull’host OBS: { $path }

output-results-record-stop = Ferma registrazione

output-results-unconfirmed = Non confermato

output-results-title = Risultati delle uscite

output-results-detail = Le richieste e le osservazioni OBS sono separate. Un evento di salvataggio può provenire da un altro controller. I percorsi appartengono all’host OBS. Per questa sessione vengono conservati gli ultimi 24 risultati.

record-elapsed-tooltip = Tempo trascorso da quando OBS ha segnalato l’inizio della registrazione, comprese le pause. Non è la durata del file registrato.

window-open-navigation = Apri navigazione
sources-title = Sorgenti
sources-back = Torna alle sorgenti
sources-select-hint = Seleziona una sorgente per esaminarne le impostazioni.
sources-text-conflict = Questo testo è cambiato in OBS. Annulla per caricare il testo attuale prima di salvare.
inventory-name-column = Scena
inventory-role-column = Ruolo
inventory-actions-column = Opzioni
inventory-scene-options = Opzioni per { $scene }
inventory-columns-button = Larghezza colonne
inventory-name-width = Larghezza scena
inventory-role-width = Larghezza ruolo
inventory-reset-widths = Ripristina larghezza colonne
settings-scene-previews = Anteprime delle scene
settings-scene-previews-description = Mostra miniature ombreggiate su schede Live più grandi.
live-audio-expand = Espandi audio
live-audio-collapse = Comprimi audio
studio-mode-label = Studio
scene-card-image-pending = Miniatura non disponibile
scene-card-program-label = Programma
output-options = Altri controlli di uscita
output-replay-label = Replay · { $state }
output-camera-label = Videocamera virtuale · { $state }

output-camera-name = Cam. virtuale
output-replay-name = Replay

# Hardening: save completion and safe custom-theme dependencies.
app-persistence-drain-timeout = Pending saves have not finished. Retry waits again; Cancel keeps the application open.
app-persistence-drain-worker-failed = A background operation failed. Changes may not have reached disk. Cancel keeps the application open so you can check and save again. Retry waits for later saves but cannot recover the failed operation. Close Anyway acknowledges that its changes may be lost; other pending saves will still be completed.
app-persistence-close-anyway = Close Anyway (Changes May Be Lost)
theme-custom-css-dependencies-forbidden = Custom CSS must be self-contained: @import, image url() dependencies, and malformed or excessively nested rules are not allowed. Inline the styles and use gradients or colors instead of external images.

# Topology readiness: no structural verdict before a complete observation.
topology-missing-title = Scene topology not observed
topology-missing-description = Connect to OBS and refresh observations before relying on scene relationships or structural checks.
topology-loading-title = Reading scene topology…
topology-loading-description = Waiting for a complete observation from OBS. Structural checks are not available yet.
topology-failed-title = Scene topology could not be read
topology-failed-description = Structural checks are unavailable. Refresh to retry. OBS reported: { $error }
topology-empty-title = No scenes observed
topology-empty-description = OBS successfully reported no scenes in this collection. Create a scene in OBS, then refresh.
doctor-refresh-topology-tooltip = Refresh scene topology from OBS and run structural checks again.
media-state-none = Nessun media
media-state-playing = In riproduzione
media-state-opening = Apertura
media-state-buffering = Buffering
media-state-paused = In pausa
media-state-stopped = Fermato
media-state-ended = Terminato
media-state-error = Errore
media-state-unknown = Sconosciuto
workspace-warning-identity = Ricerca delle identità degli input non riuscita: { $detail }
workspace-warning-truncated = Inventario delle fonti troncato: budget di ispezione esaurito
workspace-warning-container = { $path }: { $detail }
workspace-warning-visibility = Visibilità di { $owner } / { $item } non disponibile: { $detail }
workspace-warning-recursion = Scansione delle scene annidate interrotta in { $path }
workspace-warning-usage-partial = Ricerca degli utilizzi parziale: limitata a { $scenes } scene e { $items } elementi ispezionati
workspace-warning-omitted = { $count ->
    [one] 1 altro avviso sulle fonti omesso
   *[other] { $count } altri avvisi sulle fonti omessi
}
controller-workspace-superseded = Richiesta dell'area di lavoro superata prima della modifica
controller-workspace-admission-full = Operazione dell'area di lavoro rifiutata: coda piena o terminata
controller-workspace-text-changed = Il testo è cambiato in OBS. Aggiorna prima di salvare
controller-workspace-action-cancelled = Azione dell'area di lavoro annullata prima della modifica
controller-endpoint-changed = L'endpoint OBS è cambiato; riconnetti e aggiorna
controller-collection-changed = La raccolta OBS è cambiata; aggiorna prima di continuare
controller-collection-changed-during-read = La raccolta OBS è cambiata durante la lettura; risultati scartati
controller-audio-changed = L'audio è cambiato durante l'osservazione; aggiornamento necessario
controller-outputs-still-active = Le uscite di OBS restano attive dopo l'arresto
controller-shutdown-unconfirmed = Disconnesso; arresto delle uscite non confermato: { $detail }; i comandi non sono stati ripetuti
controller-stop-deadline = scadenza dell'arresto ordinato superata
controller-connection-deadline = Scadenza della connessione a OBS superata
controller-publication-backlog = Il backlog delle notifiche di OBS ha superato il limite; riconnetti e aggiorna; la cronologia delle uscite potrebbe essere incompleta
controller-event-connection-ended = Connessione agli eventi di OBS terminata; aggiornamento necessario

# Scene identity changes
registry-scene-rename-conflict = La scena «{ $old }» è stata rinominata in «{ $new }», ma quel nome ha già metadati locali. Entrambe le voci sono state conservate; controlla l’Inventario.
registry-scene-rename-backlog = Sono arrivati troppi aggiornamenti del registro durante l’importazione. Controlla l’Inventario al termine.

# Collection-specific curation
workspace-curation-title = Spazio di lavoro dello spettacolo
workspace-curation-loading = Caricamento delle impostazioni di questa raccolta…
workspace-curation-unavailable = Connettiti a OBS e attendi la raccolta di scene.
workspace-curation-failed = Impossibile caricare questo spazio di lavoro: { $error }
workspace-curation-retry = Riprova

# Saved OBS connections
settings-connections-description = Salva una configurazione per ogni connessione OBS. La selezione non riconnette OBS; usa Connetti configurazione salvata quando sei pronto.
settings-connections-picker = Connessione salvata
settings-connection-name = Nome della connessione
settings-connection-add = Aggiungi connessione
settings-connection-connect = Connetti configurazione salvata
settings-connection-new-name = Nuova connessione { $number }
settings-connection-invalid = Inserisci un nome, un host e una porta da 1 a 65535.
settings-obs-connected-profile = Connesso a { $name } ({ $endpoint }), OBS { $version }

inventory-retained-identity = { $role } · Conservato dopo un conflitto di nome o identità; non applicato a una scena.

inventory-scene-favorite = Preferita
inventory-scene-tags = Tag (separati da virgole)
inventory-apply-tags = Applica tag

deck-search-placeholder = Cerca scene e tag
deck-favorites-only = Solo preferite
deck-reset = Mostra tutte
deck-no-results = Nessuna scena corrisponde a questa vista. Cancella i filtri per mostrare tutte le scene.
deck-reorder-hint = Cancella i filtri per riordinare le scene.

deck-menu = Viste
deck-none = Nessuna vista salvata.
deck-remove = Rimuovi la vista «{ $name }»
deck-save-title = Salva vista
deck-save-help = Scegli un nome univoco. La ricerca attuale e il filtro dei preferiti verranno salvati per questo spazio di lavoro.
deck-cancel = Annulla
deck-save = Salva
deck-name = Nome della vista

## Command palette
palette-title = Tavolozza dei comandi (Ctrl+K)
palette-search = Cerca scene e azioni…
palette-open-page = Apri { $page }
palette-stage-scene = Prepara { $name }
palette-switch-scene = Passa a { $name }
palette-open-source = Apri sorgente { $name }
palette-unmute = Riattiva audio di { $name }
palette-restart = Riavvia { $name }
palette-disable-filter = Disattiva { $filter } · { $name }
palette-enable-filter = Attiva { $filter } · { $name }
palette-scope-changed = Le azioni OBS non sono disponibili in questa tavolozza. Riaprila dopo la connessione o il cambio di spazio di lavoro.
palette-empty = Nessuna azione corrispondente.
palette-hint = Usa ↑ / ↓ per selezionare, Invio per eseguire, Esc per chiudere. Le azioni salvate per media e filtri funzionano in tutte le pagine.
palette-unavailable = Non disponibile: controlla la connessione, aggiorna lo stato di OBS o verifica la sorgente dell’azione.

## Custom action shortcuts
binding-title = Scorciatoie delle azioni
binding-description = Le associazioni appartengono a questo spazio e funzionano in ogni pagina tranne durante la digitazione. I conflitti restano inattivi. Le associazioni importate sono inizialmente disattivate.
binding-manage = Gestisci scorciatoie delle azioni
binding-invalid = Usa Ctrl, Alt o Super con un carattere oppure un tasto funzione F2–F24.
binding-reserved = Questa scorciatoia è riservata all’applicazione.
binding-slots-conflict = Questa scorciatoia è in conflitto con le impostazioni delle posizioni delle scene.
binding-duplicate = Un’altra azione attiva usa già questa scorciatoia.
binding-empty = Nessuna scorciatoia per le azioni.
binding-enabled = Attiva
binding-edit = Modifica scorciatoia
binding-remove = Rimuovi scorciatoia
binding-add = Aggiungi scorciatoia
binding-record-help = Scegli un’azione e registra una combinazione di tasti. Escape annulla la registrazione. Gli ingressi condivisi influenzano tutte le scene che li usano.
binding-action = Azione
binding-record = Registra scorciatoia
binding-recording = Premi una combinazione di tasti…
binding-unavailable = Azione non disponibile. Lo stato OBS viene aggiornato; controllalo e premi di nuovo la scorciatoia.

action-inspect-input = Esamina contenuti multimediali e filtri di una sorgente
action-inspect-none = Scegli una sorgente…
action-inspect-help = Le azioni disponibili appaiono sotto. La selezione in Sorgenti viene mantenuta.

quick-title = Azioni rapide
quick-manage = Personalizza azioni rapide
quick-add = Fissa azione
quick-edit = Cambia azione
quick-remove = Rimuovi azione fissata
quick-up = Sposta azione su
quick-down = Sposta azione giù
quick-empty = Fissa i controlli usati spesso accanto alle scene.
quick-description = I controlli fissati appartengono a questo spazio di lavoro. Le sorgenti condivise influiscono su tutte le scene che le usano.
quick-disabled = Disattivata: verifica e attiva in Personalizza.
quick-observed = Stato osservato: { $state }
quick-ready = Pronta
quick-muted = Audio disattivato
quick-unmuted = Audio attivato
quick-filter-enabled = Filtro attivato
quick-filter-disabled = Filtro disattivato

quick-toggle-mute = Attiva/disattiva audio · { $name }
quick-toggle-filter = Attiva/disattiva filtro · { $name } / { $filter }

# Readiness review
preflight-title = Controllo preliminare
preflight-description = Verifica la preparazione prima di iniziare. Parla nel microfono selezionato: il segnale dopo il fader sopra −50 dBFS viene ricordato per 10 secondi finché sono disponibili livelli recenti. Le letture mancanti o obsolete restano sconosciute.
preflight-close = Chiudi
preflight-passed = Superato
preflight-failed = Richiede attenzione
preflight-unknown = Sconosciuto
preflight-skipped = Saltato
preflight-refresh = Aggiorna osservazioni OBS
preflight-reset = Reimposta elenco
preflight-ready = Pronto: tutti i controlli configurati sono superati.
preflight-attention = Esamina i controlli seguenti.
preflight-collection-result = Raccolta: { $status } — prevista «{ $expected }», osservata «{ $actual }»
preflight-scene-result = Scena «{ $name }»: { $status }
preflight-no-scenes = Scene richieste: nessuna configurata.
preflight-mic-result = Microfono «{ $name }»: presente { $presence }; non silenziato { $unmuted }; segnale recente { $signal }
preflight-disk-result = Spazio di registrazione (minimo { $minimum } MiB): { $status }

preflight-configure = Configura controllo preliminare
preflight-expected = Raccolta di scene prevista
preflight-targets-description = Cambiare la previsione reimposta le scelte di scene e microfono. Le opzioni disponibili provengono dalla raccolta osservata; gli elementi salvati mancanti restano visibili. Per scegliere un’altra raccolta, selezionala in OBS e riapri Configura.
preflight-scenes = Scene richieste
preflight-check-mic = Controlla presenza, silenziamento e segnale del microfono
preflight-no-scene-choices = Nessuna scena disponibile per questa previsione.
preflight-no-microphone = Nessun microfono selezionato
preflight-check-disk = Controlla spazio di registrazione
preflight-minimum-disk = Spazio libero minimo (MiB)
preflight-manual-items = Elenco manuale (una voce univoca per riga)
preflight-invalid-plan = Inserisci una raccolta prevista, una soglia di spazio intera positiva e voci univoche.
preflight-saving = Salvataggio…
preflight-save-failed = Impossibile salvare: { $error }. Le modifiche restano qui; riprova a salvare.
preflight-editor-retired = La connessione, la raccolta o il piano salvato sono cambiati. Chiudi questo editor e riaprilo.

preflight-continue = Continua per avviare { $output }…
preflight-continue-description = Puoi continuare con controlli irrisolti. Restano valide le normali impostazioni di conferma. Nulla si avvia automaticamente.
preflight-review-before-output = Mostra il controllo prima di avviare una diretta o registrazione

preflight-free-space = Spazio libero di registrazione osservato: { $space } MiB

# Sustained live health
health-title = Avvisi di stato in diretta
health-network = Fotogrammi persi in rete
health-render = Sovraccarico di rendering
health-encoder = Sovraccarico di codifica
health-disk = Spazio di registrazione insufficiente
health-network-help = OBS perde fotogrammi della diretta. Controlla la rete o riduci il bitrate.
health-render-help = OBS salta fotogrammi di rendering. Riduci la complessità delle scene o le impostazioni video.
health-encoder-help = OBS salta fotogrammi di codifica. Riduci il carico del codificatore o le impostazioni di diretta e registrazione.
health-disk-help = Durante la registrazione il volume ha meno di 1 GiB libero. Libera spazio o termina la registrazione prima che si esaurisca.
health-active = Richiede attenzione
health-recovering = Ripristino — le letture recenti sono normali
health-unobserved = Letture recenti non disponibili
health-caption = { $title } — { $phase }
health-more = { $summary } · altri { $count }
health-policy = Gli avvisi richiedono almeno l’1% di fotogrammi saltati negli intervalli recenti per 5 secondi. Anche lo spazio insufficiente deve durare 5 secondi. Gli avvisi sulle prestazioni scompaiono dopo 10 secondi di letture normali; dati mancanti o obsoleti non indicano ripristino.
health-open-stats = Apri statistiche
health-no-alerts = Nessun avviso di stato attuale.

health-connection-lost = Connessione OBS persa
health-stream-stopped = Diretta interrotta
health-record-stopped = Registrazione interrotta
health-connection-help = SceneDeck ha perso la connessione a OBS. Controlla OBS e la rete, poi riconnettiti. Lo stato delle uscite resta non confermato fino alla ripresa delle osservazioni.
health-stream-stopped-help = OBS ha segnalato la fine della diretta senza un recente comando di arresto da SceneDeck. Controlla OBS e la destinazione prima di riavviare.
health-record-stopped-help = OBS ha segnalato la fine della registrazione senza un recente comando di arresto da SceneDeck. Controlla lo spazio e OBS. Questo avviso non conferma il salvataggio di un file.
health-acknowledge = Conferma

health-event-policy = Gli avvisi di eventi appartengono alla connessione attuale. Il ripristino osservato o una nuova connessione li rimuove. La conferma rimuove solo l’avviso.

# Selected-input audio health
health-audio-silence = { $input }: silenzio
health-audio-clipping = { $input }: clipping
health-audio-silence-help = Audio disattivato o livello dopo il fader pari o inferiore a −50 dBFS per 15 secondi. Controlla microfono, silenziamento e instradamento. Disattiva questa regola per il silenzio previsto.
health-audio-clipping-help = Almeno un canale prima del fader è rimasto a fondo scala per un secondo. Controlla il guadagno in ingresso.
health-audio-disabled = Monitoraggio disattivato
health-audio-missing = Ingresso selezionato mancante o ambiguo
health-audio-monitoring = Monitoraggio delle letture recenti
health-audio-muted = Monitoraggio — ingresso silenziato
health-audio-settings = Monitoraggio audio
health-audio-select = Mantieni una regola per questo ingresso
health-audio-check-silence = Avvisa in caso di silenzio prolungato
health-audio-check-clipping = Avvisa in caso di clipping prolungato
health-audio-policy = Seleziona gli ingressi per aggiungere regole; deseleziona per rimuoverle. Attiva esplicitamente il monitoraggio. Le regole funzionano durante la connessione, anche prima di avviare le uscite. Silenzio: 15 secondi a −50 dBFS o meno dopo il fader, o audio disattivato. Clipping: un secondo a fondo scala prima del fader. Il ripristino richiede dieci secondi di letture normali. Letture assenti o obsolete indicano dati non disponibili, non silenzio. Aggiorna per trovare ingressi.
health-audio-invalid = Scegli almeno un avviso per ogni regola attivata e mantieni una sola regola per ingresso.
health-audio-saved = Regole salvate.
health-audio-unsaved = Modifiche non salvate.
health-audio-retired = Lo spazio di lavoro o le regole salvate sono cambiati. Chiudi e riapri questo editor.

# Observed media timing
media-progress-label = Avanzamento del contenuto
media-position-observed = Posizione osservata: { $time }
media-duration-observed = Durata: { $time }
media-time-remaining = Tempo rimanente: { $time }
media-time-unavailable = Non disponibile
media-duration-unavailable = Non disponibile o indefinita
media-progress-hint = I tempi riflettono l’ultima osservazione di OBS. Usa il campo della posizione qui sotto per spostarti.

# Local timestamp bookmarks
bookmarks-title = Segnalibri
bookmark-policy = Salvati localmente. Il tempo della sessione include le pause. Il tempo OBS è un campione recente dell’uscita, non una posizione nel file né un capitolo.
bookmark-name = Nome del segnalibro
bookmark-add = Aggiungi segnalibro
bookmark-retry = Riprova
bookmark-saving = Salvataggio…
bookmark-save-error = Segnalibri non disponibili o non salvati: { $error }
bookmark-scope-changed = Connettiti a OBS o riapri questa finestra dopo aver cambiato connessione o raccolta. Le note esistenti restano disponibili.
bookmark-saved = { $count } segnalibri salvati localmente
bookmark-default = Segnalibro { $number }
bookmark-invalid-name = Inserisci un nome di 1–120 caratteri senza caratteri di controllo.
bookmark-full = È stato raggiunto il limite di 2000 segnalibri. Rimuovi alcune note per aggiungerne altre.
bookmark-rename = Rinomina
bookmark-remove = Rimuovi
bookmark-record-unknown = Tempo di registrazione non disponibile
bookmark-record-inactive = La registrazione era inattiva
bookmark-record-time = Tempo di uscita OBS: { $time } · { $state } · Età del campione: { $age } s
bookmark-description = { $created }
    Sessione { $session }: { $elapsed } dalla connessione
    { $connection } · { $endpoint }
    Raccolta: { $collection } · Scena osservata: { $scene }
    { $recording }

# Session reports and explicit export
reports-title = Rapporti di sessione
reports-previous = Sessione precedente
reports-next = Sessione successiva
reports-refresh = Aggiorna istantanea
reports-export-json = Esporta JSON
reports-export-csv = Esporta CSV
reports-empty = Nessuna sessione acquisita. Connettiti a OBS per avviare un rapporto.
reports-current = Sessione attuale
reports-scene = Scena osservata: { $scene }
reports-ended = Sessione di osservazione terminata: { $reason }
reports-alert-cleared = Avviso rimosso; il ripristino non è confermato
reports-exporting = Esportazione…
reports-exported = Esportazione salvata: { $path }
reports-export-error = Esportazione non riuscita: { $error }
reports-policy = Le ultime otto sessioni di connessione restano in memoria. Qui è disponibile anche la cronologia salvata. Aggiorna per includere nuove osservazioni o modifiche ai segnalibri. L’esportazione conserva l’istantanea selezionata. Le conferme non dimostrano il salvataggio di un file; i percorsi appartengono al computer OBS.
reports-view-limit = Questa vista mostra gli ultimi 150 eventi e 50 segnalibri. Le esportazioni includono tutti i dati conservati. Campioni mancanti e azzeramenti dei contatori limitano l’interpretazione; i contatori dei fotogrammi sono cumulativi.
reports-summary = { $start } → { $end }
    { $connection } · { $endpoint }
    Eventi conservati: { $events } · Omessi: { $omitted } · Lacune di attività: { $gaps }
    Campioni osservati: { $samples } · Conservati: { $kept }
    CPU massima rilevata: { $cpu } · FPS minimi rilevati: { $fps }
    Segnalibri: { $bookmarks }

# Optional session history
history-title = Cronologia delle sessioni
history-enable = Salva automaticamente la cronologia
history-retain = Sessioni da conservare (1–32)
history-apply = Applica impostazioni cronologia
history-retry = Ricarica e riprova
history-clear = Cancella cronologia salvata
history-saving = Salvataggio o caricamento cronologia…
history-error = Cronologia non disponibile o non salvata: { $error }
history-status = { $mode } · Sessioni salvate: { $count }
history-on = Cronologia automatica attiva
history-off = Cronologia automatica disattivata
history-settings-changed = Le impostazioni sono cambiate altrove. Riapri questa finestra.
history-app-closed = SceneDeck chiuso
reports-interrupted = Esecuzione precedente; ora di fine sconosciuta
history-settings-description = Il salvataggio automatico è disattivato per impostazione predefinita. Gestisci qui conservazione, ricaricamento e salvataggi non riusciti.
history-policy = Attivare la cronologia salva il rapporto attuale, compresi gli eventi precedenti conservati, e le sessioni future. I salvataggi avvengono circa ogni dieci secondi e ai cambi di sessione. Disattivare interrompe i salvataggi futuri e conserva i rapporti esistenti. Un limite inferiore rimuove le sessioni salvate più vecchie. I segnalibri vengono salvati separatamente.
history-clear-confirm = Rimuovere tutta la cronologia salvata e non salvata? I rapporti recenti in memoria e i segnalibri restano disponibili. La connessione attuale non verrà salvata di nuovo; la cronologia automatica riprenderà alla prossima connessione, se attiva.

history-exit-requested = Richiesta di uscita da SceneDeck
history-exit-resumed = Uscita annullata o rinviata; raccolta del rapporto ripresa

# Selected-input audio presets
preset-title = Preimpostazioni audio
preset-open = Apri
preset-shared = Seleziona gli ingressi per salvare gli attuali valori di silenziamento e guadagno. Le modifiche a un ingresso interessano tutte le scene che lo usano. Le preimpostazioni appartengono a questa connessione e raccolta di scene.
preset-name = Nome della preimpostazione
preset-new = Nuova preimpostazione
preset-save = Salva ingressi selezionati
preset-replace = Sostituisci preimpostazione selezionata
preset-rename = Rinomina preimpostazione selezionata
preset-delete = Elimina preimpostazione selezionata
preset-preview = Anteprima dei valori salvati
preset-missing = Assente o non disponibile
preset-invalid = Scegli da 1 a 64 ingressi disponibili e un nome univoco di massimo 80 caratteri. Puoi salvare fino a 64 preimpostazioni.
preset-current = Attuale: { $value }
preset-saved = Salvato: { $value }
preset-none = Seleziona una preimpostazione salvata per esaminarne i valori.
preset-refresh-needed = Aggiorna i dati audio prima di acquisire una preimpostazione o esaminare le modifiche.
preset-retired = Lo spazio di lavoro o le preimpostazioni salvate sono cambiati. Chiudi questo editor e riaprilo.
preset-pending = In attesa
preset-running = Verifica e applicazione
preset-observed = Valori confermati da OBS
preset-unchanged = Già corrispondente
preset-changed = Valori cambiati; verifica di nuovo
preset-failed = Richiesta non riuscita
preset-unconfirmed = Risultato non completamente confermato
preset-cancelled = Modifiche rimanenti annullate
preset-acknowledgements = Conferme OBS: guadagno { $gain }, silenziamento { $mute }
preset-unavailable = Impossibile applicare: connessione non disponibile, altra preimpostazione in corso o verifica scaduta.
preset-review = Verifica modifiche
preset-apply = Applica agli ingressi disponibili
preset-cancel = Ferma modifiche rimanenti
preset-apply-policy = Verifica prima di applicare. Gli ingressi mancanti vengono saltati; gli altri possono riuscire indipendentemente. Una conferma non conferma la rilettura. Il guadagno viene verificato prima di riattivare il suono. Fermare impedisce altre modifiche senza annullare quelle completate. Aggiorna e verifica di nuovo se i valori cambiano.
preset-unsupported = Questa connessione OBS non dichiara tutte le richieste necessarie per applicare le preimpostazioni audio.
preset-reviewed = I valori verificati vengono mantenuti per un massimo di 30 secondi.

# Desktop portal shortcuts
global-title = Scorciatoie del desktop
global-description = Registra fino a 64 azioni abilitate per questo spazio di lavoro connesso. Il desktop sceglie i tasti e può chiedere il permesso. I tasti confermati sostituiscono quelli dell’applicazione fino all’arresto. La registrazione termina alla disconnessione, quando cambiano spazio o associazioni, o alla chiusura della finestra principale; riattivala quando sei pronto.
global-enable = Abilita scorciatoie del desktop
global-stop = Ferma scorciatoie del desktop
global-off = Le scorciatoie del desktop sono disattivate.
global-pending = In attesa del desktop…
global-active = Le scorciatoie del desktop sono attive. Tasti assegnati:
global-none = Il desktop non ha assegnato scorciatoie.
global-unavailable = Impossibile abilitare le scorciatoie del desktop. Verifica che il desktop offra il portale GlobalShortcuts e riprova.

help-controls-subtitle =
    Scorciatoie, azioni rapide e preimpostazioni audio

help-controls-body =
    Ctrl+K cerca scene e azioni supportate. Impostazioni → Scorciatoie azioni assegna tasti stabili; le azioni rapide di Live fissano singoli controlli. Gli ingressi condivisi influenzano ogni scena che li usa.
    Le scorciatoie desktop si abilitano esplicitamente nel gestore e richiedono un portale compatibile. Il desktop sceglie i tasti; cambiare connessione, spazio o associazioni oppure chiudere la finestra principale termina la registrazione.
    Mixer → Preimpostazioni audio salva guadagno e silenziamento degli ingressi scelti. Esamina modifiche e ingressi mancanti prima di applicare. I risultati distinguono conferme e valori osservati; il successo può essere parziale. Fermare annulla le modifiche non inviate; riconnettere non le ripete.

help-readiness-subtitle =
    Preparazione e avvisi persistenti durante la diretta

help-readiness-body =
    La verifica preliminare controlla raccolta prevista, scene necessarie, silenziamento/segnale del microfono scelto, spazio di registrazione e una lista manuale. Non avvia uscite. Dati sconosciuti o vecchi non equivalgono a un esito positivo.
    Gli avvisi restano visibili tra le pagine operative. Rilevano perdite recenti e persistenti di fotogrammi, sovraccarico di rendering/codifica, poco spazio, disconnessione e arresti inattesi. Le regole di silenzio/clipping richiedono ingressi scelti esplicitamente; i misuratori non provano cosa sente il pubblico.

help-session-subtitle =
    Segnalibri, contenuti multimediali e rapporti

help-session-body =
    Sorgenti mostra progresso e tempo rimanente quando OBS fornisce la durata, oltre alla ricerca supportata. I controlli di uscita includono pausa e ripresa della registrazione.
    I segnalibri salvano tempo locale della sessione e un campione recente del tempo di registrazione OBS. Pause e riconnessioni impediscono di considerarli posizioni esatte del file o capitoli.
    I rapporti conservano eventi, incidenti e prove delle uscite entro limiti, con export CSV/JSON. La cronologia automatica è disattivata per impostazione predefinita; abilitala e scegli la conservazione in Cronologia sessioni. Richieste e conferme restano distinte dai risultati osservati. I percorsi di registrazione appartengono al computer OBS.
