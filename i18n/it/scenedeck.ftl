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
inventory-stale-group-description = Queste scene sono presenti nel registro locale ma non esistono più in OBS.
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
scene-card-tooltip-active = Scena di programma attuale
scene-card-tooltip-previous = Scena attiva in precedenza
scene-card-tooltip-ready = Passa a questa scena
scene-card-status-active = Attiva
scene-card-status-previous = Precedente
scene-card-status-ready = Pronta
scene-card-marker-active = On
scene-card-marker-previous = Ultima

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
settings-theme-subtitle = { $description } Campioni: { $swatches }
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
    SceneDeck non rinomina, elimina o riordina mai nulla all'interno di OBS. Legge le scene tramite la connessione WebSocket di OBS e conserva annotazioni proprie su di esse.
    Un «ruolo» è una di queste annotazioni: un'etichetta personale che indica lo scopo di una scena, come la scena da mandare in onda, una sovrimpressione riutilizzabile o una vecchia scena di prova.
    Queste annotazioni si trovano in registry.json accanto al file di configurazione, quindi sopravvivono ai riavvii e possono essere esportate e trasferite su un altro computer dalla pagina Inventario.
    Poiché le annotazioni sono locali, due persone possono condividere la stessa configurazione OBS e mantenere pagine Diretta diverse.

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
    Esporta scrive ruoli, ordine, colori in evidenza, icone, tag e regole del grafo in un unico file YAML, un formato di testo semplice che puoi leggere e conservare nel controllo versione.
    Importa sostituisce il registro locale con il contenuto di quel file. Usalo per trasferire una configurazione completa su un secondo computer o per ripristinarla dopo un esperimento.
    I nomi delle scene collegano il file a OBS, quindi una scena rinominata in OBS riappare come voce obsoleta. Inventario elenca queste voci e consente di rimuoverle.

help-group-operate-title = Durante una trasmissione
help-group-operate-description = Le pagine che usi davvero mentre sei in onda.

help-live-title = La pagina Diretta
help-live-subtitle = Schede delle scene, audio e scena di programma
help-live-body =
    Diretta è la vista operativa: la scena di programma corrente in alto, le schede delle scene da un lato e le schede audio compatte dall'altro. Trascina il divisore per dare più spazio alla metà che ti serve.
    Facendo clic su una scheda, OBS passa a quella scena. Quella corrente è contrassegnata come Attiva, le altre come Pronte.
    Nessuna scheda dopo la connessione? Nessuna scena ha ancora il ruolo Primaria: consulta «Nascondere le scene che non mandi mai in onda» qui sopra.
    La barra di stato in basso rimane visibile in ogni pagina e mostra lo stato della connessione, della trasmissione e della registrazione con il tempo trascorso, oltre a FPS, fotogrammi persi, CPU e bitrate in tempo reale.

help-hotkeys-title = Cambiare scena dalla tastiera
help-hotkeys-subtitle = Ctrl+1 … Ctrl+0 per impostazione predefinita e configurabili
help-hotkeys-body =
    Ognuna delle prime dieci schede di Diretta mostra un piccolo contrassegno con la propria cifra: la prima è 1, la nona è 9 e la decima è 0. La didascalia accanto al titolo Scene indica sempre la combinazione corrente.
    I numeri delle posizioni seguono l'ordine di Inventario, quindi riordinare le schede lì riordina anche le scorciatoie.
    Impostazioni → Scorciatoie scene consente di scegliere come premere la cifra. Un modificatore più la cifra (Ctrl per impostazione predefinita) evita attivazioni accidentali mentre digiti. La sola cifra è l'opzione più rapida. Lo stile con tasto leader funziona come in vim: premi il tasto leader, rilascialo e poi premi la cifra; mentre attende, la didascalia mostra «Leader attivo».
    Le scorciatoie funzionano solo nella pagina Diretta e gli stili senza modificatore vengono disattivati quando un campo di testo ha lo stato attivo. Se a una cifra non corrisponde alcuna scena, la didascalia lo segnala senza cambiare scena.

help-audio-title = Audio: la pagina Mixer e gli indicatori
help-audio-subtitle = Cosa indicano le barre colorate
help-audio-body =
    Le schede audio compaiono in Diretta e, con più spazio e controlli, nella pagina Mixer. Prima vengono mostrati i dispositivi audio globali di OBS, poi le sorgenti audio della scena corrente, incluse quelle nelle scene annidate e nei gruppi.
    Le modalità del Mixer stabiliscono l'audio di quale scena stai osservando. Attiva segue la scena di programma OBS. Selezionata carica una scena e resta su di essa. Fissata mantiene una scena scelta come destinazione permanente mentre OBS prosegue.
    L'indicatore accanto a ogni fader va da -60 dB in basso a 0 dB in alto e usa le soglie di OBS: verde sotto -20 dB per musica e sottofondo, giallo da -20 a -9 dB per la voce, rosso sopra -9 dB dove inizia la distorsione. Nessun segnale dovrebbe restare sul rosso.
    Una colonna indica una sorgente mono; due indicano stereo, sinistra e poi destra. Se si muove solo la colonna sinistra, metà degli spettatori non sentirà quella sorgente.
    La linea sopra il riempimento mostra il picco più alto degli ultimi venti secondi, il modo più rapido per individuare una distorsione sfuggita. Il quadrato in basso mostra il livello in arrivo dal dispositivo prima del fader: se è troppo alto, spostare il fader non lo correggerà.
    Il pulsante di blocco su una scheda immobilizza soltanto il cursore di SceneDeck. Non blocca nulla in OBS.

help-outputs-title = Avvio e arresto di trasmissione e registrazione
help-outputs-subtitle = E le conferme che evitano un incidente
help-outputs-body =
    I pulsanti Avvia/Arresta trasmissione e Avvia/Arresta registrazione si trovano in fondo alla barra laterale e sono accessibili da ogni pagina. La barra di stato mostra lo stato e da quanto tempo sono in esecuzione.
    Impostazioni → Sicurezza uscite stabilisce quali delle quattro azioni richiedono una conferma. Per impostazione predefinita, l'arresto di entrambe le uscite chiede conferma mentre l'avvio no: si presume che iniziare in anticipo costi poco e fermarsi in anticipo termini la trasmissione.
    Anche le modifiche apportate direttamente in OBS compaiono qui: SceneDeck segue gli eventi di OBS invece di presumere che la pressione del proprio pulsante abbia funzionato.

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
    Le impostazioni, inclusi l'host e la porta di OBS, si trovano in $XDG_CONFIG_HOME/scenedeck/config.json, in genere ~/.config/scenedeck/config.json.
    Ruoli, ordine, colori in evidenza e icone delle scene si trovano in registry.json nella stessa cartella.
    La password di OBS non è presente in nessuno dei due file. È salvata nel portachiavi Secret Service del desktop, la stessa cassaforte usata dal browser.
    Esegui il backup di entrambi i file JSON per trasferire un'intera configurazione su un altro computer, oppure usa l'esportazione YAML di Inventario solo per la parte relativa alle scene.

help-shortcuts-title = Scorciatoie da tastiera
help-shortcuts-subtitle = L'elenco completo
help-shortcuts-body =
    F1 — apri questa guida.
    Ctrl+R — riconnettiti a OBS.
    Ctrl+, — apri Impostazioni.
    Ctrl+Q — esci da SceneDeck.
    Ctrl+1 … Ctrl+0 nella pagina Diretta — passa a una delle prime dieci schede delle scene. La combinazione è configurabile in Impostazioni → Scorciatoie scene.

## ui/window.rs — finestra di benvenuto al primo avvio
welcome-dialog-heading = Benvenuto in SceneDeck
welcome-dialog-body = Sembra che questo sia il primo avvio. La pagina Guida spiega come connettersi a OBS —anche quando OBS è su un altro computer— e come limitare la pagina Diretta alle scene che usi davvero. Bastano un paio di minuti e si evitano alcuni errori.
welcome-dialog-later = Non ora
welcome-dialog-open = Leggi la guida
window-help-tooltip = Guida e introduzione
