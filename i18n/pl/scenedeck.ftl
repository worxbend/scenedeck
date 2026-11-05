## SceneDeck UI strings (Polish translation).
##
## Grouped by the module each message is used from. Message ids are prefixed
## with the module name to keep them unambiguous in this single shared file.

## Internal — used only by the i18n loader's own regression test, not shown
## in the UI. Every locale must define this so the smoke test can confirm the
## locale's bundle loaded (not just the `en` fallback).
i18n-loader-smoke-test = Lokalizacja wczytana.

## infra/error.rs — user-facing renderings of AppError. `detail` is raw
## upstream text (often from OBS or the OS) and is never translated.
error-connection = Połączenie z OBS nie powiodło się: { $detail }
error-request = Żądanie do OBS nie powiodło się: { $detail }
error-config = Błąd konfiguracji: { $detail }
error-storage = Błąd magazynu danych: { $detail }
error-notification-title = Błąd SceneDeck: { $message }
events-resync-warning = Pominięto powiadomienia OBS. SceneDeck ponownie się synchronizuje; historia wyjść może być niepełna.

## domain/audio.rs — AudioSourceScope::label()
audio-scope-global = Globalny
audio-scope-active = Scena
audio-scope-nested = Zagnieżdżony
audio-scope-group = Grupa

## domain/graph.rs — EdgeStatus summary label/tooltip
edge-status-ok-label = OK
edge-status-warning-label = Ostrzeżenie
edge-status-forbidden-label = Zabronione
edge-status-ok-tooltip = Połączenia zgodne z polityką grafu
edge-status-warning-tooltip = Połączenia spoza listy dozwolonych
edge-status-forbidden-tooltip = Połączenia zabronione przez politykę grafu

## domain/output.rs — OutputRunState::label(), OutputStatus::summary()
output-state-inactive = Nieaktywny
output-state-starting = Uruchamianie
output-state-active = Aktywny
output-state-stopping = Zatrzymywanie
output-state-reconnecting = Ponowne łączenie
output-state-paused = Wstrzymany
output-state-unknown = Nieznany
output-summary = { $name }: { $state }

## domain/role.rs — SceneRole::label()/description()
role-primary = Główna
role-secondary = Dodatkowa
role-module = Moduł
role-raw = Surowa
role-debug = Testowa
role-archive = Archiwum
role-unassigned = Nieprzypisana
role-primary-desc = Scena przełączalna na żywo
role-secondary-desc = Prawidłowa scena, domyślnie ukryta na stronie Na żywo
role-module-desc = Wielokrotnego użytku scena zagnieżdżona, nieprzełączalna bezpośrednio
role-raw-desc = Scena opakowująca sprzęt lub źródło
role-debug-desc = Tymczasowa scena testowa
role-archive-desc = Zachowana, ale wykluczona ze wszystkich przepływów pracy

## domain/mixer.rs — MixerMode / MixerGrouping labels
mixer-mode-active = Aktywny
mixer-mode-selected = Wybrany
mixer-mode-pinned = Przypięty
mixer-mode-active-desc = Podąża za sceną programu OBS.
mixer-mode-selected-desc = Sprawdź wybraną scenę bez podążania za OBS.
mixer-mode-pinned-desc = Utrzymuj wybraną scenę bez zmian podczas pracy.
mixer-grouping-scope = Zakres
mixer-grouping-scene-path = Ścieżka sceny
mixer-grouping-none = Brak

## domain/diagnostic.rs — DiagnosticSeverity label/count
diag-label-info = Informacje
diag-label-warning = Ostrzeżenia
diag-label-error = Błędy
diag-count-info = { $count ->
    [one] { $count } informacja
    [few] { $count } informacje
    [many] { $count } informacji
   *[other] { $count } informacji
}
diag-count-warning = { $count ->
    [one] { $count } ostrzeżenie
    [few] { $count } ostrzeżenia
    [many] { $count } ostrzeżeń
   *[other] { $count } ostrzeżeń
}
diag-count-error = { $count ->
    [one] { $count } błąd
    [few] { $count } błędy
    [many] { $count } błędów
   *[other] { $count } błędów
}

## ui/pages/inventory.rs
inventory-no-role-assigned = Brak przypisanej roli

## services/doctor_service.rs — diagnostic message/suggestion templates
doctor-no-role = Scena nie ma przypisanej roli w lokalnym rejestrze.
doctor-no-role-suggestion = Otwórz Inwentarz i przypisz rolę.
doctor-stale-entry = Wpis w rejestrze odwołuje się do sceny, której nie znaleziono w OBS.
doctor-stale-entry-suggestion = Usuń wpis z Inwentarza.
doctor-protected-switchable = Chroniona scena znajduje się w przełączalnej roli „{ $role }”.
doctor-protected-switchable-suggestion = Chronione sceny zwykle pełnią rolę elementów składowych; rozważ rolę Moduł lub Surowa.
doctor-cycle = Cykliczne odwołanie między scenami „{ $parent }” i „{ $child }”.
doctor-cycle-suggestion = Usuń pętlę zagnieżdżonych scen; OBS nie potrafi renderować cykli.
doctor-edge-primary-debug = Scena Główna zależy od sceny Testowej. (→ „{ $child }”)
doctor-edge-primary-debug-suggestion = Usuń scenę testową ze ścieżki na żywo przed rozpoczęciem transmisji.
doctor-edge-primary-raw = Scena Główna bezpośrednio opakowuje źródło typu Surowa. (→ „{ $child }”)
doctor-edge-primary-raw-suggestion = Opakuj źródło typu Surowa w scenę Moduł, aby zwiększyć czytelność i możliwość ponownego użycia.
doctor-edge-module-primary = Moduł zależy od sceny Głównej, co odwraca hierarchię. (→ „{ $child }”)
doctor-edge-module-primary-suggestion = Moduły powinny być elementami składowymi, a nie konsumentami scen Głównych.
doctor-edge-raw-nests = Scena Surowa zagnieżdża inną scenę. (→ „{ $child }”)
doctor-edge-raw-nests-suggestion = Sceny Surowe powinny być końcowymi opakowaniami źródeł, bez zagnieżdżonych scen.
doctor-edge-forbidden = Zależność między scenami jest zabroniona przez politykę grafu. (→ „{ $child }”)
doctor-edge-outside-policy = Zależność między scenami wykracza poza skonfigurowaną politykę grafu. (→ „{ $child }”)
doctor-edge-adjust-suggestion = Dostosuj relację zagnieżdżonych scen lub zaktualizuj reguły grafu w rejestrze.

## controller/app_controller.rs
controller-not-connected = Brak połączenia z OBS

## controller/state.rs — Page titles and ObsStatus labels
page-live = Na żywo
page-stats = Statystyki
page-mixer = Mikser
page-graph = Graf
page-inventory = Inwentarz
page-doctor = Diagnostyka
page-settings = Ustawienia
obs-status-disconnected = Rozłączono
obs-status-connecting = Łączenie…
obs-status-connected = Połączono
obs-status-error = Błąd

## storage/config.rs — ConfigStartupNotice
config-first-launch = Brak zapisanych ustawień. Wczytano wartości domyślne.
config-read-failed = Nie udało się odczytać ustawień: { $detail }
config-parse-failed = Nie udało się przetworzyć ustawień: { $detail }

## graph.rs

graph-empty-title = Brak zależności
graph-empty-description = Żadna scena nie zagnieżdża innych scen lub brak połączenia z OBS. Połącz się i dodaj zagnieżdżone źródła scen, aby zobaczyć graf zależności.
graph-page-title = Zależności scen
graph-reset-tooltip = Zresetuj układ grafu
graph-edge-summary-count = { $label } { $count }

## mixer.rs

## Page chrome / empty state
mixer-empty-title = Brak danych miksera
mixer-empty-description = Połącz się z OBS, aby wczytać sceny i źródła audio.
mixer-page-title = Mikser
mixer-controls-title = Sterowanie mikserem
mixer-summary-title = Bieżące źródło miksera

## Control rows (ComboRow / EntryRow titles+subtitles)
mixer-mode-row-title = Tryb
mixer-mode-row-subtitle = Aktywny podąża za OBS; Wybrany i Przypięty utrzymują wybraną scenę bez zmian.
mixer-scene-row-title = Scena
mixer-scene-row-subtitle = Używane przez tryby Wybrany i Przypięty.
mixer-grouping-row-title = Grupuj według
mixer-grouping-row-subtitle = Określa sposób układu źródeł audio poniżej.
mixer-search-row-title = Szukaj

## Scene-loading / no-scene placeholders
mixer-no-scene-title = Nie wybrano sceny
mixer-no-scene-description = Wybierz scenę, aby wczytać jej audio w mikserze.
mixer-loading-title = Wczytywanie audio miksera
mixer-loading-description = Wczytywanie źródeł audio dla { $scene }.

## Audio-source empty states
mixer-current-scene-fallback = Bieżąca scena
mixer-no-audio-sources-title = Brak źródeł audio
mixer-no-audio-sources-description = { $scene } nie ma pasujących skonfigurowanych źródeł audio OBS.
mixer-no-matching-title = Brak pasujących źródeł audio
mixer-no-matching-description = Dostosuj filtr wyszukiwania, aby wyświetlić dostępne źródła audio.

## Group titles
mixer-group-all-sources = Wszystkie źródła
mixer-group-global-fallback = Globalne

## Error placeholder + retry
mixer-error-title = Audio miksera niedostępne
mixer-error-description = Nie udało się wczytać źródeł audio dla { $scene }: { $message }
mixer-retry-button-label = Ponów
mixer-retry-button-tooltip = Ponów wczytywanie audio miksera

## Current-source summary row (source_summary / scene_target_summary)
mixer-summary-following-active = Podążanie za aktywną sceną OBS: { $scene }
mixer-summary-no-scene-selected = Nie wybrano sceny
mixer-summary-selected-scene = Wybrana scena: { $scene }
mixer-summary-pinned-scene = Przypięta scena: { $scene }
mixer-summary-selected-fallback = Nie ustawiono wybranej sceny; używana jest aktywna scena OBS: { $scene }
mixer-summary-pinned-selected-fallback = Nie ustawiono przypiętej sceny; używana jest wybrana scena: { $scene }
mixer-summary-pinned-active-fallback = Nie ustawiono przypiętej ani wybranej sceny; używana jest aktywna scena OBS: { $scene }

## doctor.rs — page chrome, empty state, and all-clear text for the Doctor
## page. Diagnostic finding messages themselves (`doctor-no-role`,
## `doctor-cycle*`, `doctor-edge-*`, etc.) already exist in
## src/services/doctor_service.rs and are NOT redefined here.
doctor-page-title = Diagnostyka
doctor-empty-state-title = Nic do sprawdzenia
doctor-empty-state-description = Połącz się z OBS, aby uruchomić diagnostykę architektury.
doctor-summary-row-title = Diagnostyka
doctor-rerun-tooltip = Uruchom diagnostykę ponownie
doctor-all-clear-title = Nie znaleziono problemów
doctor-all-clear-detail = Architektura scen spełnia wszystkie kontrole.

## inventory.rs — page chrome, group titles, YAML import/export UI, and
## status messages. Role names/descriptions themselves are NOT redefined
## here — they already exist in src/domain/role.rs (`role-*`,
## `role-*-desc`, `role-unassigned`) and are reused directly via
## SceneRole::label()/description()/unassigned_label(). The already-present
## `inventory-no-role-assigned` message is also left untouched.
inventory-page-title = Inwentarz
inventory-empty-state-title = Brak scen
inventory-empty-state-description = Połącz się z OBS, aby wczytać listę scen.
inventory-scenes-group-title = Sceny OBS
inventory-scenes-group-description = Przypisz role, aby kontrolować, które sceny pojawiają się na stronie Na żywo.
inventory-stale-group-title = Nieaktualne wpisy rejestru
inventory-stale-group-description = Sprawdź wpisy brakujących scen i konflikty tożsamości. Zachowane wpisy nie są stosowane do bieżących scen.
inventory-remove-stale-tooltip = Usuń nieaktualny wpis
inventory-yaml-row-title = YAML rejestru scen
inventory-yaml-row-subtitle = Eksportuj lub importuj role scen, tagi, flagi ochrony i reguły grafu.
inventory-yaml-filter-name = Pliki YAML
inventory-accent-clear-tooltip = Wyczyść kolor akcentu sceny
inventory-drag-tooltip = Przeciągnij, aby zmienić kolejność scen
inventory-accent-dialog-title = Kolor akcentu sceny
inventory-accent-choose-tooltip = Wybierz kolor akcentu sceny

# Shared between the row's Export button, the export FileChooserNative's
# accept label, and (for Cancel) both the export and import dialogs.
inventory-export-button-label = Eksportuj
inventory-export-tooltip = Eksportuj rejestr scen do YAML
inventory-import-button-label = Importuj
inventory-import-tooltip = Importuj rejestr scen z YAML
inventory-dialog-cancel-label = Anuluj

inventory-export-dialog-title = Eksportuj rejestr scen
inventory-export-success = Wyeksportowano rejestr scen do { $path }.
inventory-export-error = Eksport nie powiódł się: { $error }
inventory-export-no-file = Eksport nie powiódł się: nie wybrano pliku.

inventory-import-dialog-title = Importuj rejestr scen
inventory-import-error = Import nie powiódł się: { $error }
inventory-import-no-file = Import nie powiódł się: nie wybrano pliku.

## window.rs

window-stream-live-tooltip = Transmisja na żywo
window-about-tooltip = O programie SceneDeck
window-refresh-tooltip = Odśwież bieżącą stronę

window-stream-status-line = Transmisja: { $state }{ $elapsed }
window-record-status-line = Nagrywanie: { $state }{ $elapsed }

window-status-connecting = Łączenie z OBS…
window-connect-btn-connecting = Łączenie…
window-current-scene-none = Bieżąca scena: —
window-status-connected = Połączono — OBS { $version }
window-connect-btn-disconnect = Rozłącz
window-status-disconnected = Rozłączono
window-connect-btn-connect = Połącz z OBS
window-live-disconnected-hint = Połącz się z OBS, aby korzystać ze sterowania Na żywo
window-current-scene = Bieżąca scena: { $scene }
window-status-error = Błąd: { $error }
window-connect-btn-retry = Ponów
window-obs-connection-failed = Połączenie z OBS nie powiodło się
window-toast-obs-error = Błąd OBS: { $error }

window-output-kind-stream = Transmisja
window-output-kind-record = Nagrywanie

window-sidebar-output-starting = Uruchamianie…
window-sidebar-output-stopping = Zatrzymywanie…
window-sidebar-output-reconnecting = Ponowne łączenie…
window-sidebar-output-working = Przetwarzanie…

window-sidebar-start-stream = Rozpocznij transmisję
window-sidebar-stop-stream = Zatrzymaj transmisję
window-sidebar-start-recording = Rozpocznij nagrywanie
window-sidebar-stop-recording = Zatrzymaj nagrywanie

window-selector-profile-label = Profil
window-selector-profile-tooltip = Przełącz profil OBS
window-selector-collection-label = Kolekcja
window-selector-collection-tooltip = Przełącz kolekcję scen OBS

## live.rs

live-start-stream-label = Rozpocznij transmisję
live-stop-stream-label = Zatrzymaj transmisję
live-start-record-label = Rozpocznij nagrywanie
live-stop-record-label = Zatrzymaj nagrywanie
live-stream-toggle-tooltip = Rozpocznij lub zatrzymaj transmisję
live-record-toggle-tooltip = Rozpocznij lub zatrzymaj nagrywanie
live-stream-inactive-label = Transmisja: Nieaktywna
live-record-inactive-label = Nagrywanie: Nieaktywne
live-copy-last-recording-path-tooltip = Skopiuj ścieżkę ostatniego nagrania
live-copied-recording-path-tooltip = Skopiowano ścieżkę ostatniego nagrania
live-copy-recording-path-with-value-tooltip = Skopiuj ścieżkę nagrania: { $path }
live-stream-card-title = Transmisja
live-recording-card-title = Nagrywanie
live-current-scene-placeholder = Bieżąca scena: —
live-scenes-section-label = Sceny
live-scenes-connect-hint = Połącz się z OBS, aby wczytać sceny.
live-audio-section-label = Audio
live-disconnected-title = Połącz się z OBS, aby korzystać ze sterowania Na żywo
live-disconnected-detail = Użyj elementu połączenia u dołu paska bocznego.
live-stream-command-error-label = Polecenie transmisji nie powiodło się
live-recording-command-error-label = Polecenie nagrywania nie powiodło się
live-last-recording-detail = Ostatnie nagranie: { $path }
live-starting-stream = Uruchamianie transmisji…
live-stopping-stream = Zatrzymywanie transmisji…
live-reconnecting-stream = Ponowne łączenie transmisji…
live-starting-recording = Uruchamianie nagrywania…
live-stopping-recording = Zatrzymywanie nagrywania…
live-reconnecting-recording = Ponowne łączenie nagrywania…
live-button-starting = Uruchamianie…
live-button-stopping = Zatrzymywanie…
live-button-reconnecting = Ponowne łączenie…
live-button-working = Przetwarzanie…
live-output-kind-stream = Transmisja
live-output-kind-record = Nagrywanie
live-output-label = { $kind }: { $state }
live-output-label-with-elapsed = { $kind }: { $state } · { $elapsed }
live-scenes-no-primary-hint = Nie znaleziono scen z rolą Główna. Przypisz role w Inwentarzu.
live-audio-empty-hint = Nie skonfigurowano żadnych wejść audio.
live-cancel-button-label = Anuluj
live-start-stream-confirm-heading = Rozpocząć transmisję?
live-start-stream-confirm-body = OBS rozpocznie wysyłanie transmisji na żywo.
live-stop-stream-confirm-heading = Zatrzymać transmisję?
live-stop-stream-confirm-body = OBS zatrzyma wysyłanie transmisji na żywo.
live-start-recording-confirm-heading = Rozpocząć nagrywanie?
live-start-recording-confirm-body = OBS rozpocznie nowe nagranie.
live-start-recording-confirm-label = Rozpocznij nagrywanie
live-stop-recording-confirm-heading = Zatrzymać nagrywanie?
live-stop-recording-confirm-body = OBS zatrzyma bieżące nagranie.
live-stop-recording-confirm-label = Zatrzymaj nagrywanie

## audio_card.rs
audio-card-mute-tooltip = Wycisz wejście
audio-card-source-path-tooltip = { $scope }: { $path }
audio-card-fader-tooltip = Suwak głośności
audio-card-lock-tooltip = Zablokuj suwak głośności
audio-card-fine-plus-tooltip = +1 dB
audio-card-fine-reset-tooltip = Zresetuj do 0,0 dB
audio-card-fine-minus-tooltip = -1 dB
audio-card-meter-tooltip-title = Wskaźnik głośności: { $channels }
audio-card-meter-tooltip-zones = Zielony poniżej -20 dB · Żółty do -9 dB · Czerwony powyżej, blisko przesterowania
audio-card-meter-tooltip-indicators = Słupek: poziom szczytowy z opadaniem · Linia: najgłośniejszy szczyt w 20 s · Kropka: głośność · Podstawa: poziom z urządzenia
audio-card-meter-tooltip-waiting = Wskaźnik głośności: oczekiwanie na poziomy z OBS

## icon.rs
icon-none = Brak ikony
icon-camera = Kamera
icon-desktop = Pulpit
icon-game = Gra
icon-film = Film
icon-images = Obrazy
icon-television = Telewizja
icon-browser = Przeglądarka
icon-terminal = Terminal
icon-code = Kod
icon-chat = Czat
icon-guests = Goście
icon-star = Gwiazda
icon-alert = Alert
icon-break = Przerwa
icon-countdown = Odliczanie
icon-start = Start
icon-pause = Pauza
icon-stop = Stop
icon-settings = Ustawienia
icon-layers = Warstwy
icon-microphone = Mikrofon
icon-headset = Zestaw słuchawkowy
icon-headphones = Słuchawki
icon-speaker = Głośnik
icon-volume = Głośność
icon-music = Muzyka
icon-instrument = Instrument
icon-radio = Radio
icon-call = Połączenie
icon-waveform = Przebieg
inventory-scene-icon-tooltip = Wybierz ikonę dla tej sceny
mixer-input-icon-tooltip = Wybierz ikonę dla tego źródła dźwięku

## meter.rs
audio-meter-zone-nominal = Zielony
audio-meter-zone-warning = Żółty
audio-meter-zone-error = Czerwony
audio-meter-channel-mono = Mono
audio-meter-channel-left = Lewy
audio-meter-channel-right = Prawy
audio-meter-channel-front-left = Przedni lewy
audio-meter-channel-front-right = Przedni prawy
audio-meter-channel-front-center = Przedni środkowy
audio-meter-channel-lfe = LFE
audio-meter-channel-rear-left = Tylny lewy
audio-meter-channel-rear-right = Tylny prawy
audio-meter-channel-side-left = Boczny lewy
audio-meter-channel-side-right = Boczny prawy
audio-meter-channel-numbered = Kanał { $index }

## hotkey.rs
hotkey-modifier-ctrl = Ctrl+
hotkey-modifier-alt = Alt+
hotkey-modifier-shift = Shift+
hotkey-modifier-super = Super+
hotkey-modifier-ctrl-alt = Ctrl+Alt+
hotkey-modifier-ctrl-shift = Ctrl+Shift+
hotkey-style-plain = Sama cyfra
hotkey-style-ctrl = Ctrl + cyfra
hotkey-style-alt = Alt + cyfra
hotkey-style-shift = Shift + cyfra
hotkey-style-super = Super + cyfra
hotkey-style-ctrl-alt = Ctrl+Alt + cyfra
hotkey-style-ctrl-shift = Ctrl+Shift + cyfra
hotkey-style-leader = Klawisz leader, potem cyfra
hotkey-leader-space = Spacja
hotkey-leader-comma = Przecinek
hotkey-leader-semicolon = Średnik
hotkey-leader-backslash = Backslash
hotkey-leader-grave = Backtick
hotkey-shortcut-leader = { $leader }, potem { $digit }
hotkey-hint-plain = Naciśnij 1 … 0
hotkey-hint-modifier = Naciśnij { $modifier }1 … 0
hotkey-hint-leader = Naciśnij { $leader }, potem 1 … 0
hotkey-hint-leader-armed = Leader aktywny — naciśnij 1 … 0
hotkey-hint-empty-slot = Brak sceny na pozycji { $slot }

## scene_card.rs
scene-card-tooltip = { $status } ({ $role })
scene-card-tooltip-with-hotkey = { $status } ({ $role }) · { $hotkey }
scene-card-role-suffix = Scena: { $role }
scene-card-tooltip-ready = Przełącz na tę scenę

## status_bar.rs
status-bar-stream-inactive = Transmisja: Nieaktywna
status-bar-record-inactive = Nagrywanie: Nieaktywne
status-bar-fps-placeholder = FPS —
status-bar-cpu-placeholder = CPU —
status-bar-bitrate-placeholder = Przepływność —
status-bar-fps = FPS { $value }
status-bar-cpu = CPU { $value }%
status-bar-bitrate = Przepływność { $value } kbps
status-bar-dropped = Utracone: { $count }
status-bar-dropped-placeholder = Utracone —

## settings.rs

settings-page-title = Ustawienia
settings-appearance-title = Wygląd
settings-appearance-description = Aplikacje GNOME powinny domyślnie podążać za stylem systemu.
settings-theme-mode-system = Systemowy
settings-theme-mode-light = Jasny
settings-theme-mode-dark = Ciemny
settings-color-scheme-title = Schemat kolorów
settings-color-scheme-subtitle = Podążaj za preferencją systemu lub wymuś tryb jasny / ciemny
settings-motion-title = Ruch
settings-motion-subtitle = Jak mocno animuje się interfejs. Wskaźniki na żywo pozostają czytelne na każdym poziomie.
settings-motion-full = Pełny
settings-motion-reduced = Ograniczony
settings-motion-off = Wyłączony
settings-theme-title = Motyw
settings-theme-status-title = Stan motywu
settings-theme-status-initial = Motyw wczytany.
settings-failed-to-save = Zapis nie powiódł się: { $err }
settings-custom-css-title = Niestandardowy CSS
settings-custom-css-subtitle = Wczytaj osobne pliki CSS użytkownika dla trybu jasnego i ciemnego
settings-custom-light-css-title = Ścieżka niestandardowego CSS (jasny)
settings-custom-dark-css-title = Ścieżka niestandardowego CSS (ciemny)
settings-reload-css-title = Wczytaj ponownie niestandardowy CSS
settings-reload-css-subtitle = Zastosuj ponownie wybrany motyw oraz odpowiadający mu plik CSS jasny/ciemny.
settings-reload-button = Wczytaj ponownie
settings-language-title = Język
settings-language-description = Zmiany zaczną obowiązywać po ponownym uruchomieniu SceneDeck.
settings-display-language-title = Język interfejsu
settings-display-language-subtitle = Wybierz język lub podążaj za ustawieniami regionalnymi systemu.
language-system-default = Domyślny język systemu
settings-language-status-title = Stan języka
settings-language-status-initial = Uruchom ponownie, aby zastosować zmianę języka.
settings-language-saved = Zapisano język. Uruchom ponownie SceneDeck, aby go zastosować.
settings-obs-connection-title = Połączenie z OBS
settings-obs-connection-description = Ustawienia WebSocket dla OBS Studio (domyślny port: 4455).
settings-host-title = Host
settings-port-title = Port
settings-password-title = Hasło (opcjonalne)
settings-obs-status-title = Stan OBS
settings-invalid-port = Nieprawidłowy numer portu.
settings-saved = Zapisano ustawienia.
settings-password-saved = Hasło zapisano w bazie kluczy (keyring).
settings-keyring-error = Błąd bazy kluczy (keyring): { $err }
settings-output-safety-title = Bezpieczeństwo wyjść
settings-output-safety-description = Opcjonalne potwierdzenia krytycznych działań transmisji i nagrywania.
settings-confirm-start-stream-title = Potwierdź rozpoczęcie transmisji
settings-confirm-start-stream-subtitle = Pytaj przed rozpoczęciem transmisji na żywo.
settings-confirm-stop-stream-title = Potwierdź zatrzymanie transmisji
settings-confirm-stop-stream-subtitle = Pytaj przed zatrzymaniem transmisji na żywo.
settings-confirm-start-recording-title = Potwierdź rozpoczęcie nagrywania
settings-confirm-start-recording-subtitle = Pytaj przed rozpoczęciem nagrywania.
settings-confirm-stop-recording-title = Potwierdź zatrzymanie nagrywania
settings-confirm-stop-recording-subtitle = Pytaj przed zatrzymaniem nagrywania.
settings-hotkeys-title = Skróty scen
settings-hotkeys-description = Przełączaj sceny na stronie Na żywo z klawiatury. Numery pozycji wynikają z kolejności scen ustawionej w Inwentarzu.
settings-hotkeys-enabled-title = Włącz skróty scen
settings-hotkeys-enabled-subtitle = Klawisze cyfr przełączają karty scen na stronie Na żywo, w kolejności kart.
settings-hotkeys-style-title = Kombinacja klawiszy
settings-hotkeys-style-subtitle = Które klawisze przełączają scenę. Same cyfry nie działają, gdy fokus ma pole tekstowe.
settings-hotkeys-leader-title = Klawisz leader
settings-hotkeys-leader-subtitle = Pierwszy klawisz dwuetapowego skrótu, w stylu vima.
settings-hotkeys-timeout-title = Limit czasu leadera
settings-hotkeys-timeout-subtitle = Jak długo leader czeka na cyfrę, w milisekundach.
settings-hotkeys-preview-title = Bieżące przypisania
settings-hotkeys-preview-subtitle = { $first } … { $last } przełączają pierwsze { $count } scen na stronie Na żywo.
settings-hotkeys-preview-disabled = Skróty scen są wyłączone.
settings-obs-not-connected = Brak połączenia z OBS.
settings-obs-connecting = Łączenie z OBS…
settings-obs-connected = Połączono — OBS { $version }
settings-obs-error = Błąd: { $err }
settings-theme-loaded = Wczytano { $theme } ({ $variant }).
settings-theme-loaded-with-warnings = Motyw wczytany z ostrzeżeniami.
theme-variant-light = Jasny
theme-variant-dark = Ciemny

## theme.rs

theme-adwaita-default-name = Adwaita Default
theme-adwaita-default-desc = Neutralny styl zgodny z domyślnymi ustawieniami GNOME.
theme-scenedeck-dark-name = SceneDeck Dark
theme-scenedeck-dark-desc = Stonowany ciemny motyw konsolowy do pracy na żywo.
theme-scenedeck-light-name = SceneDeck Light
theme-scenedeck-light-desc = Wyrazisty jasny motyw konsolowy o stonowanym kontraście.
theme-obs-name = OBS
theme-obs-desc = Domyślny wygląd OBS Studio w interpretacji libadwaity.
theme-obsidian-name = Obsidian
theme-obsidian-desc = Bardzo czytelne grafitowe powierzchnie z chłodnymi akcentami.
theme-nord-name = Nord
theme-nord-desc = Chłodne niebiesko-szare powierzchnie z lodowymi akcentami.
theme-dracula-inspired-name = Dracula Inspired
theme-dracula-inspired-desc = Ciemna, ekspresyjna paleta z oryginalnym kodem CSS.
theme-solarized-dark-name = Solarized Dark
theme-solarized-dark-desc = Kontrast bez efektu olśnienia, z akcentami w kolorze morskim i bursztynowym.
theme-high-contrast-name = High Contrast
theme-high-contrast-desc = Wyraźniejsze obrysy i kontrast dla kluczowych elementów sterujących.
theme-stream-red-name = Stream Red
theme-stream-red-desc = Czerwone akcenty w duchu nadawczym dla stanów na żywo.
theme-studio-purple-name = Studio Purple
theme-studio-purple-desc = Stonowane fioletowe akcenty bez dominowania nad powierzchniami.
theme-ubuntu-violet-name = Ubuntu Violet
theme-ubuntu-violet-desc = Fioletowe powierzchnie inspirowane Ubuntu z ciepłym akcentem na żywo.
theme-custom-css-read-failed = Nie udało się odczytać niestandardowego CSS z { $path }: { $err }
theme-custom-css-no-matching-file = Niestandardowy CSS jest włączony, ale nie ustawiono odpowiedniego pliku jasny/ciemny.
theme-css-no-display = { $label } nie został wczytany, ponieważ brak dostępnego wyświetlacza GTK.
theme-css-parse-error = Błąd analizy CSS w { $label }: { $message }
## stats.rs — live streaming telemetry
stats-page-title = Statystyki transmisji
stats-page-subtitle = Dane na żywo pobierane z OBS raz na sekundę, gdy połączenie jest aktywne.
stats-gauge-fps = FPS
stats-gauge-frame-time = Czas klatki (ms)
stats-gauge-dropped = Zgubione klatki
stats-gauge-congestion = Przeciążenie
stats-chart-fps = Klatki na sekundę
stats-chart-frame-time = Średni czas renderowania klatki (ms)
stats-chart-output-skipped = Pominięte klatki wyjściowe na próbkę
stats-chart-render-skipped = Utracone klatki renderowania na próbkę
stats-card-render-frames = Utracone klatki renderowania
stats-card-output-frames = Pominięte klatki wyjściowe
stats-card-stream-frames = Zgubione klatki transmisji
stats-card-frame-time = Średni czas renderowania klatki
stats-card-cpu = Użycie procesora przez OBS
stats-card-memory = Użycie pamięci przez OBS
stats-card-bitrate = Bitrate transmisji
stats-value-placeholder = —
stats-value-frames = { $skipped } z { $total } ({ $percent }%)
stats-value-ms = { $value } ms
stats-value-percent = { $value }%
stats-value-mb = { $value } MB
stats-value-kbps = { $value } kb/s

## Help and onboarding
page-help = Pomoc
config-parse-failed-backed-up = Nie udało się przeanalizować ustawień: { $detail } — plik zachowano pod adresem { $path }, a wczytano ustawienia domyślne.
stats-popout-button = Otwórz statystyki w osobnym oknie

help-page-title = Pomoc i wprowadzenie
help-hero-title = Witamy w SceneDeck
help-hero-description = Natywny dla Linuksa panel sterowania OBS Studio. Ten przewodnik przeprowadzi Cię przez pierwsze połączenie, pokaże, jak ograniczyć stronę Na żywo do faktycznie przełączanych scen, i objaśni każdą stronę na pasku bocznym. Rozwiń temat, aby go przeczytać.
help-expand-hint = Naciśnij temat, aby go rozwinąć.

help-open-settings = Otwórz Ustawienia
help-open-inventory = Otwórz Inwentarz
help-open-doctor = Otwórz Diagnostykę
help-open-live = Otwórz Na żywo
help-open-mixer = Otwórz Mikser
help-open-graph = Otwórz Graf
help-open-stats = Otwórz Statystyki

help-group-start-title = Pierwsze kroki
help-group-start-description = Najkrótsza droga od świeżej instalacji do przełączania scen.

help-quickstart-title = Pięć kroków do pierwszego przełączenia sceny
help-quickstart-subtitle = Za pierwszym razem wykonaj je po kolei
help-quickstart-body =
    1. W OBS Studio otwórz Narzędzia → Ustawienia serwera WebSocket i zaznacz „Włącz serwer WebSocket”. Pozostaw OBS uruchomiony.
    2. W tym samym oknie OBS naciśnij „Pokaż informacje o połączeniu” i zanotuj Port serwera (domyślnie 4455) oraz Hasło serwera.
    3. W SceneDeck otwórz Ustawienia i uzupełnij Host, Port oraz Hasło. Jeśli OBS działa na tym samym komputerze, Host pozostaje ustawiony na 127.0.0.1.
    4. Naciśnij Połącz na dole paska bocznego. Wiersz stanu nad przyciskiem zmieni kolor na zielony i wyświetli „Połączono”.
    5. Otwórz Inwentarz i nadaj rolę „Główna” scenom, które chcesz przełączać podczas programu. Te — i tylko te — staną się kartami na stronie Na żywo.

help-concepts-title = Jak SceneDeck rozumie Twoją konfigurację
help-concepts-subtitle = Role, rejestr i to, czego SceneDeck nigdy nie zmienia w OBS
help-concepts-body =
    Zapisane połączenia mają oddzielne dane dostępu. Każde połączenie, adres i kolekcja scen ma własny obszar roboczy.
    Role, tagi, ulubione, kolejność, kolory, ikony i nazwane widoki to lokalna organizacja; nie edytują scen OBS. Wyszukiwanie i zapisane widoki nie zmieniają przypisanych akcji.
    Znane UUID zachowują metadane po zmianie nazwy. Zastępstwo z innym UUID nie dziedziczy tej tożsamości.

help-group-connect-title = Łączenie z OBS
help-group-connect-description = Na tym komputerze lub po drugiej stronie pomieszczenia.

help-connect-local-title = Łączenie z OBS na tym komputerze
help-connect-local-subtitle = Typowy przypadek — host 127.0.0.1, port 4455
help-connect-local-body =
    127.0.0.1 to adres, którego komputer używa do komunikacji z samym sobą, dlatego jest właściwym Hostem, gdy OBS i SceneDeck działają obok siebie.
    Port musi odpowiadać Portowi serwera w Ustawieniach serwera WebSocket OBS. OBS używa portu 4455, o ile go nie zmieniono.
    Jeśli w OBS zaznaczono „Włącz uwierzytelnianie”, wklej hasło w Ustawienia → Hasło. SceneDeck przechowuje je w pęku kluczy pulpitu (tam, gdzie przeglądarka przechowuje zapisane loginy), nigdy w zwykłym pliku tekstowym konfiguracji.
    Naciśnij Połącz na pasku bocznym albo w dowolnej chwili naciśnij Ctrl+R, aby połączyć się ponownie.

help-connect-remote-title = Łączenie z OBS na innym komputerze
help-connect-remote-subtitle = Komputer do transmisji w rogu, sterowanie z laptopa
help-connect-remote-body =
    To konfiguracja z dwoma komputerami: OBS działa na maszynie przechwytującej i kodującej obraz, a SceneDeck na komputerze przed Tobą. Oba muszą znajdować się w tej samej sieci.
    Na komputerze z OBS, w Narzędzia → Ustawienia serwera WebSocket: zaznacz „Włącz serwer WebSocket”, zaznacz „Włącz uwierzytelnianie” i ustaw hasło, które możesz wpisać. Nie wyłączaj uwierzytelniania — w przeciwnym razie każda osoba w sieci mająca dostęp do portu mogłaby rozpocząć lub zatrzymać transmisję.
    Adres komputera z OBS sprawdź na nim samym. W Linuksie uruchom `ip addr` i znajdź adres podobny do 192.168.1.42; w Windows uruchom `ipconfig` i odczytaj Adres IPv4; w macOS znajdziesz go w Ustawienia systemowe → Sieć. Jest to adres komputera, nie programu OBS.
    Zezwól na ruch przez ten port w zaporze komputera z OBS. W Linuksie z ufw użyj `sudo ufw allow 4455/tcp`; w Windows zezwól aplikacji OBS na dostęp do sieci prywatnych w Zaporze Windows Defender.
    W Ustawieniach SceneDeck wpisz ten adres w polu Host (na przykład 192.168.1.42), pozostaw Port 4455 i wklej hasło. Naciśnij Połącz.
    Dla komputera z OBS preferuj połączenie przewodowe. Przełączanie scen przez Wi-Fi działa, ale utracony pakiet oznacza opóźnione cięcie.
    Wskazówka, która może uratować program: zarezerwuj stały adres komputera z OBS w ustawieniach DHCP routera, aby zapisany Host nadal działał po ponownym uruchomieniu.

help-connect-share-title = Udostępnianie panelu współprowadzącemu lub moderatorowi
help-connect-share-subtitle = Drugi SceneDeck skierowany na ten sam zestaw
help-connect-share-body =
    Serwer WebSocket OBS przyjmuje jednocześnie więcej niż jednego klienta, więc druga osoba na drugim komputerze może uruchomić własną kopię SceneDeck połączoną z tym samym zestawem — to konfiguracja z poprzedniego tematu wykonana dwukrotnie.
    Nie ma osobnych kont: osoba, która ma Host, Port i hasło w Ustawieniach, ma dokładnie takie same uprawnienia jak Ty, łącznie z rozpoczynaniem i zatrzymywaniem transmisji. Udostępniaj je tylko osobie, której można powierzyć Twoją sesję OBS.
    Najpierw ogranicz jej widok. Role i ukryte sceny znajdują się w lokalnym Inwentarzu, a nie w OBS, więc skonfiguruj je na swoim komputerze i poproś współpracownika o zaimportowanie Rejestru scen YAML albo odtworzenie go, zamiast zaczynać od pełnej, niefiltrowanej listy scen.
    Nie przekierowuj portu WebSocket do Internetu, aby udostępnić go zdalnemu współpracownikowi. Połącz oba komputery z tą samą siecią VPN lub Tailscale albo tuneluj połączenie przez SSH, a jako Host podaj adres tunelu.
    Każda połączona osoba widzi ten sam stan na żywo — przełączenie sceny lub ruch suwaka z dowolnej strony pojawia się natychmiast po obu stronach, tak samo jak przy wspólnej klawiaturze.

help-connect-trouble-title = Gdy przycisk Połącz nie działa
help-connect-trouble-subtitle = Przeczytaj błąd, a następnie sprawdź kolejno tę listę
help-connect-trouble-body =
    „Odmowa połączenia” niemal zawsze oznacza, że serwer WebSocket nie jest włączony w OBS albo port się nie zgadza. Sprawdź oba ustawienia w Narzędzia → Ustawienia serwera WebSocket.
    Połączenie, które zawiesza się, a potem przekracza limit czasu, zwykle oznacza, że zapora odrzuca ruch albo adres Host należy do innego komputera.
    „Uwierzytelnianie nie powiodło się” oznacza błędne hasło. Wpisz je ponownie w Ustawieniach; pole służy tylko do zapisu, więc wygląda na puste nawet po zapisaniu hasła.
    Brak jakiegokolwiek stanu na pasku bocznym? Upewnij się, że OBS rzeczywiście działa i nie czeka na zamknięcie własnego okna modalnego.
    SceneDeck automatycznie łączy się ponownie po zerwaniu połączenia, a Ctrl+R wymusza natychmiastową próbę.

help-group-scenes-title = Porządkowanie scen
help-group-scenes-description = Strona Na żywo powinna pokazywać tylko sceny, na które się przełączasz.

help-scenes-hide-title = Ukrywanie scen, na które nigdy nie przełączasz
help-scenes-hide-subtitle = Najbardziej użyteczne ustawienie na początek
help-scenes-hide-body =
    Działająca konfiguracja OBS gromadzi sceny, które służą wyłącznie do zagnieżdżania w innych scenach, lub zostały utworzone do jednego testu i nigdy ich nie usunięto. Umieszczenie ich na panelu sterowania transmisją grozi błędnym cięciem.
    SceneDeck pokazuje kartę Na żywo tylko dla scen z rolą Główna. Każda inna rola jest ukryta na stronie Na żywo, więc „ukrycie sceny” oznacza po prostu nadanie jej roli innej niż Główna.
    Otwórz Inwentarz. Każda scena OBS ma wiersz z selektorem roli po prawej stronie.
    Nadaj rolę Główna kilku scenom, na które faktycznie przełączasz się na antenie. Pozostałym przypisz: Dodatkowa (prawdziwa scena, której czasem potrzebujesz, ale nie chcesz jej na stronie Na żywo), Moduł (nakładka lub belka, zawsze zagnieżdżana w innej scenie), Surowa (sama kamera lub opakowanie źródła przechwytywania), Debugowanie (scena testowa) albo Archiwum (zachowana na później, poza głównym widokiem).
    Sceny bez przypisanej roli również nie pojawiają się na stronie Na żywo, więc pozostawienie roli Nieprzypisana także je ukrywa. Celowe nadanie roli jest jednak lepsze: strona Diagnostyka oznacza Nieprzypisane sceny, dzięki czemu zauważysz nowe.
    Zmiana działa natychmiast — wróć na stronę Na żywo, a karty już nie będzie. W OBS nic się nie zmieniło.

help-scenes-order-title = Kolejność, kolory i ikony
help-scenes-order-subtitle = Niech właściwa karta od razu rzuca się w oczy
help-scenes-order-body =
    Przeciągnij scenę za uchwyt po lewej stronie jej wiersza w Inwentarzu, aby ustawić kolejność. Karty Na żywo i skróty numeryczne używają tej samej kolejności, więc karta pod klawiszem 1 znajduje się na górze.
    Selektor koloru akcentu zabarwia kartę Na żywo danej sceny. Mocny kolor zachowaj dla scen o istotnych konsekwencjach — sceny „jesteśmy na żywo” lub materiału sponsora — aby wzrok od razu na nie trafiał.
    Selektor ikony po lewej stronie każdego wiersza umieszcza symbol na karcie Na żywo. Dostępnych jest trzydzieści ikon oraz pozycja „bez ikony”, która ją usuwa.
    Kolejność, kolory i ikony są zapisane w lokalnym rejestrze, a nie w OBS.

help-scenes-registry-title = Tworzenie kopii zapasowej i przenoszenie konfiguracji
help-scenes-registry-subtitle = Wiersz Rejestr scen YAML w Inwentarzu
help-scenes-registry-body =
    Inwentarz eksportuje rejestr bieżącego obszaru do YAML, wraz z widokami, akcjami, regułami kontroli i ustawieniami audio. Import zastępuje organizację tego obszaru, nie innych obszarów ani danych dostępu OBS.
    Importowane skróty i szybkie akcje są początkowo wyłączone. Sprawdź tożsamości przed włączeniem. UUID śledzą zmiany nazw; ta sama nazwa z innym UUID oznacza inny cel. Nieaktualne wpisy można usunąć w Inwentarzu.

help-group-operate-title = Prowadzenie programu
help-group-operate-description = Strony używane podczas trwającej transmisji.

help-live-title = Strona Na żywo
help-live-subtitle = Ściana scen, Studio, dźwięk i wyjścia
help-live-body =
    Live to ściana scen. Miniatury scen w Ustawieniach dodają przyciemnione obrazy i większe karty; wyłączenie przywraca kompaktowe kafelki.
    W trybie bezpośrednim karty i skróty zmieniają Program. Przełącznik Studio na górnym pasku odzwierciedla OBS: karty przygotowują Podgląd, a Przejście przenosi go do Programu. Przejście i obsługiwany czas są widoczne tylko w trybie Studio.
    Rozwiń pasek dźwięku pod scenami, aby uzyskać więcej elementów sterowania, lub otwórz Mikser. Transmisja, nagrywanie, bufor powtórek i kamera wirtualna mają jeden pasek wyjść.
    Źródła na górnym pasku otwierają widoczność i szczegółowe ustawienia. Sprawdzanie tam sceny nigdy nie zmienia Programu.
    Brak kart po połączeniu? W Inwentarzu przypisz rolę Główna scenom, które mają być w Live.

help-hotkeys-title = Przełączanie scen z klawiatury
help-hotkeys-subtitle = Domyślnie Ctrl+1 … Ctrl+0, z możliwością konfiguracji
help-hotkeys-body =
    Pierwszych dziesięć przełączalnych scen w zapisanej kolejności Inwentarza otrzymuje cyfry 1–9, a następnie 0. Każda widoczna karta pokazuje przypisaną cyfrę, a wskazówka sceny podaje skonfigurowany skrót.
    Wyszukiwanie, zapisane zestawy i wyświetlanie ulubionych na początku nie zmieniają numerów. Skrót może aktywować scenę ukrytą przez filtr. Zmiana kolejności w Inwentarzu bez filtrów zmienia przypisania.
    Ustawienia → Skróty scen określają sposób naciskania cyfry. Modyfikator z cyfrą (domyślnie Ctrl) nie uruchomi skrótu przypadkiem podczas pisania. Sama cyfra jest najszybsza. Styl z liderem działa jak w vimie: naciśnij klawisz lidera, zwolnij go, a potem naciśnij cyfrę; podczas oczekiwania podpis pokazuje „Lider aktywny”.
    Skróty scen działają tylko na stronie Na żywo i są wstrzymywane, gdy pole tekstowe ma fokus. Cyfra bez przypisanej sceny zgłasza puste miejsce i niczego nie przełącza.

help-audio-title = Dźwięk: strona Mikser i mierniki
help-audio-subtitle = Co oznaczają kolorowe paski
help-audio-body =
    Kompaktowy pasek dźwięku w Live można rozwinąć do szybkich zmian; Mikser udostępnia pełne suwaki. Najpierw są globalne urządzenia audio OBS, a potem źródła dźwięku bieżącej sceny, również sceny zagnieżdżone i grupy.
    Tryby Miksera określają, dźwięk której sceny oglądasz. Aktywny podąża za sceną programową OBS. Wybrany wczytuje jedną scenę i pozostaje przy niej. Przypięty zachowuje wybraną scenę jako stały cel, gdy OBS przełącza się dalej.
    Miernik obok każdego suwaka obejmuje zakres od -60 dB u dołu do 0 dB u góry i używa progów OBS: zielony poniżej -20 dB dla muzyki i tła, żółty od -20 do -9 dB dla mowy, czerwony powyżej -9 dB, gdzie zaczyna się przesterowanie. Nic nie powinno stale pozostawać w czerwonym zakresie.
    Jedna kolumna oznacza źródło mono; dwie oznaczają stereo — najpierw lewy, potem prawy kanał. Jeśli porusza się tylko lewa kolumna, połowa odbiorców nic z niego nie usłyszy.
    Linia nad wypełnieniem pokazuje najwyższy szczyt z ostatnich dwudziestu sekund — to najszybszy sposób na zauważenie pominiętego przesterowania. Kwadrat na samym dole pokazuje poziom docierający z urządzenia przed suwakiem; jeśli jest zbyt wysoki, ruch suwaka tego nie naprawi.
    Przycisk blokady na karcie zamraża jedynie suwak w SceneDeck. Nie blokuje niczego w OBS.

help-outputs-title = Transmisja, nagrywanie, powtórki i kamera wirtualna
help-outputs-subtitle = I potwierdzenia zapobiegające pomyłkom
help-outputs-body =
    Pasek wyjść w Live zawiera transmisję, nagrywanie, bufor powtórek i kamerę wirtualną. Więcej opcji wyjścia otwiera pauzę/wznowienie, podział pliku/rozdziały i ostatnie wyniki.
    Ustawienia → Bezpieczeństwo wyjść sterują potwierdzeniami uruchamiania i zatrzymywania transmisji oraz nagrywania. Zatrzymanie domyślnie wymaga potwierdzenia.
    Zdarzenia i odświeżone obserwacje OBS określają wyświetlany stan. Sama zaakceptowana prośba nie dowodzi uruchomienia wyjścia ani zapisania pliku. Ścieżki należą do komputera z OBS.

help-group-inspect-title = Sprawdzanie konfiguracji
help-group-inspect-description = Znajdź niespodziankę, zanim pojawi się na antenie.

help-doctor-title = Diagnostyka
help-doctor-subtitle = Problemy strukturalne uporządkowane według ważności
help-doctor-body =
    Diagnostyka odczytuje listę scen, przypisane role i zagnieżdżenia między scenami, a następnie zgłasza podejrzane elementy jako Błędy, Ostrzeżenia i Informacje.
    Typowe wyniki: sceny bez przypisanej roli, zapamiętane sceny, których nie ma już w OBS, cykliczne zagnieżdżenia oraz scena zagnieżdżona w innej w kierunku niedozwolonym przez reguły ról.
    Diagnostyka uruchamia się ponownie po każdym otwarciu strony, a przycisk Uruchom ponownie wymusza sprawdzenie.
    Warto zajrzeć po każdej zmianie konfiguracji OBS i jeszcze raz przed rozpoczęciem transmisji.

help-graph-title = Graf
help-graph-subtitle = Które sceny znajdują się wewnątrz innych
help-graph-body =
    Zagnieżdżanie sceny w innej służy w OBS do budowania nakładek i wspólnych układów, ale może też uzależnić scenę od zapomnianego elementu.
    Graf pokazuje każdą scenę nadrzędną i jej zawartość oraz oznacza każdą relację względem reguł ról jako poprawną, wątpliwą lub zabronioną.
    Przed wprowadzeniem zmiany użyj go, aby odpowiedzieć na pytanie „co przestanie działać, jeśli zmienię tę scenę?”.

help-stats-title = Statystyki
help-stats-subtitle = Czy komputer nadąża
help-stats-body =
    Wskaźniki FPS, czasu renderowania klatki, utraconych klatek i przeciążenia sieci zmieniają kolor na bursztynowy, a potem czerwony wraz z pogarszaniem się wartości — ostrzeżenie o utraconych klatkach pojawia się przy 1%, a o przeciążeniu przy 30%.
    Wykresy trendów przechowują mniej więcej ostatnie dwie minuty, a wykresy słupkowe pokazują moment utraty klatek zamiast sumy narastającej. Dzięki temu wiadomo, czy zacięcie było pojedynczym zdarzeniem, czy trendem.
    Próbki są zbierane przez cały czas połączenia, więc otwarcie Statystyk w trakcie transmisji pokazuje poprzednie minuty, zamiast zaczynać od pustego wykresu.
    Liczniki klatek pochodzą z OBS i zerują się po ponownym uruchomieniu OBS lub wyjścia transmisji.

help-group-personalise-title = Dostosowanie do własnych potrzeb
help-group-personalise-description = Wygląd, język i miejsce przechowywania plików.

help-appearance-title = Motywy i wygląd
help-appearance-subtitle = W tym wygląd pasujący do samego OBS
help-appearance-body =
    Schemat kolorów domyślnie podąża za jasnymi lub ciemnymi ustawieniami pulpitu, ale można wymusić jeden z nich.
    Motywy to rodziny obsługujące tryb jasny i ciemny: wybierz jedną, a zostanie użyty wariant odpowiadający bieżącemu schematowi kolorów. Rodzina OBS odwzorowuje wygląd OBS Studio, dzięki czemu panel sterowania nie kłóci się wizualnie ze sterowaną aplikacją.
    Własny CSS przyjmuje osobne pliki dla trybu jasnego i ciemnego, dzięki czemu własny wygląd również podąża za schematem kolorów. Przycisk Wczytaj własny CSS ponownie zastosuje zmiany bez ponownego uruchamiania.
    Ustawienie Ruch określa intensywność animacji interfejsu — wybierz Ograniczony lub Wyłączony, jeśli ruch rozprasza albo komputer jest obciążony.

help-files-title = Gdzie SceneDeck przechowuje dane
help-files-subtitle = Konfiguracja, rejestr i hasło OBS
help-files-body =
    config.json zapisuje ustawienia i połączenia w $XDG_CONFIG_HOME/scenedeck (zwykle ~/.config/scenedeck). show-workspaces.json zapisuje obszary; stary registry.json jest zachowany i przejmowany jednokrotnie.
    bookmarks.json zapisuje notatki. Opcjonalny session-history.json znajduje się w $XDG_STATE_HOME/scenedeck (zwykle ~/.local/state/scenedeck).
    Hasła pozostają w Secret Service, oddzielnie dla każdego połączenia. Eksport JSON/YAML nie zawiera danych dostępu. Skopiuj potrzebne pliki i przenieś hasła osobno.

help-shortcuts-title = Skróty klawiaturowe
help-shortcuts-subtitle = Pełna lista
help-shortcuts-body =
    F1 — otwórz ten przewodnik.
    Ctrl+R — połącz ponownie z OBS.
    Ctrl+, — otwórz Ustawienia.
    Ctrl+Q — zakończ SceneDeck.
    Ctrl+1 … Ctrl+0 na stronie Na żywo — aktywuj scenę przypisaną do tej pozycji Inwentarza, nawet ukrytą przez filtr. Kombinację można zmienić w Ustawieniach → Skróty scen.
    { palette-title }

welcome-dialog-heading = Witamy w SceneDeck
welcome-dialog-body = Wygląda na to, że uruchamiasz program po raz pierwszy. Strona Pomoc opisuje łączenie z OBS — także na innym komputerze — i pokazuje, jak ograniczyć stronę Na żywo do faktycznie przełączanych scen. Zajmie to kilka minut i pomoże uniknąć błędów.
welcome-dialog-later = Nie teraz
welcome-dialog-open = Przeczytaj przewodnik
window-help-tooltip = Pomoc i przewodnik wprowadzający

# OBS controls and accessible alternatives
window-stop-disconnect-label = Zatrzymaj transmisję/nagrywanie i rozłącz
window-stop-stream-disconnect-label = Zatrzymaj transmisję i rozłącz
window-stop-record-disconnect-label = Zatrzymaj nagrywanie i rozłącz
disconnect-confirm-heading = Zatrzymać transmisję i nagrywanie przed rozłączeniem?
disconnect-confirm-body = Transmisja: { $stream }. Nagrywanie: { $recording }. Aktywna transmisja i nagrywanie zostaną zatrzymane przed rozłączeniem SceneDeck. Ta czynność nie zatrzymuje bufora powtórek ani kamery wirtualnej.
live-scenes-obs-empty = Ta kolekcja OBS nie zawiera scen. Utwórz scenę w OBS, a następnie odśwież SceneDeck.
live-organize-scenes = Uporządkuj sceny
mixer-secondary-options = Grupowanie i wyszukiwanie
inventory-move-up = Przenieś { $scene } w górę
inventory-move-down = Przenieś { $scene } w dół
graph-relationship-list = Relacje między scenami — lista tekstowa
graph-no-dependencies = Brak zależności od zagnieżdżonych scen
audio-source-mute = Wycisz { $source }
audio-source-lock = Zablokuj lokalny suwak { $source }
audio-source-volume = Głośność { $source }
audio-source-meter = Wskaźnik szczytowego poziomu dźwięku { $source }
audio-meter-no-level = Brak widocznego poziomu
audio-meter-no-observation = Brak odczytu poziomu dźwięku

# Session activity

workflow-activity-title = Ostatnia aktywność
workflow-activity-detail = Tylko bieżąca sesja, 30 najnowszych wpisów. Żądania, obserwacje i błędy są rozdzielone. Rozłączenia tworzą luki; zbieżność w czasie nie dowodzi przyczyny.
workflow-check-item = Widoczność źródła
workflow-activity-requested = Zażądano
workflow-activity-observed = Zaobserwowano
workflow-activity-failed = Niepowodzenie
workflow-activity-connection = Połączenie


# Native OBS inspector
control-refresh = Odśwież odczyty OBS
control-inspection-note = Przeglądaj dowolną scenę bez zmiany Programu. Zmiany widoczności dotyczą sceny lub grupy zawierającej element.
control-disconnected = Połącz się z OBS, aby sprawdzać źródła.
control-loading = Odczytywanie stanu OBS…
control-observed = Ostatni odczyt: { $seconds } s temu. Odśwież stan, zanim na nim polegniesz.
control-scene-selector = Scena do przeglądania — nie zmienia Programu
control-request-hint = Odśwież, aby wczytać natywne sterowanie OBS i widoczność źródeł.
control-resume = Wznów nagrywanie
control-pause = Wstrzymaj nagrywanie
control-split = Podziel plik nagrania
control-replay-stop = Zatrzymaj bufor powtórek
control-replay-start = Uruchom bufor powtórek
control-replay-save = Zapisz powtórkę
control-camera-stop = Zatrzymaj kamerę wirtualną
control-camera-start = Uruchom kamerę wirtualną
control-chapter-name = Nazwa rozdziału nagrania
control-chapter = Utwórz rozdział
control-host-path = Ścieżka ostatniej powtórki na komputerze OBS: { $path }
control-output-controls = Nagrywanie, powtórki i kamera wirtualna
control-source-search = Szukaj źródeł lub ścieżek nadrzędnych
control-inspect-source = Przeglądaj ustawienia i użycie źródła
control-hidden-parent = Element nadrzędny jest ukryty. Włączenie tylko tego elementu może nie zapewnić jego widoczności.
control-sources-empty = Brak elementów sceny w tym odczycie.
control-shared-source = Dźwięk, filtry, multimedia i tekst wpływają na to źródło wszędzie, gdzie jest używane. Widoczność dotyczy jego kontenera nadrzędnego.
control-used-in = Używane w — ścieżki struktury
control-image-refresh = Przechwyć obraz nieruchomy
control-image-alt = Obraz sceny OBS przechwycony na żądanie
control-image-error = Nie można zdekodować obrazu OBS.
control-image-note = Obraz z ostatniego przechwycenia. To nie jest podgląd wideo na żywo.
control-studio-enable = Włącz tryb studia
control-program = Program: { $scene }
control-preview = Podgląd: { $scene }
control-stage = Przygotuj przeglądaną scenę
control-take = Przejście
control-transition = Bieżące przejście sceny
control-duration = Czas przejścia w milisekundach
control-apply = Zastosuj
control-studio = Studio i przejście
control-scene-mode-changed = Tryb studia uległ zmianie. Odśwież przed aktywowaniem sceny. Operacja nie została ponowiona.
control-filters = Filtry źródła
control-text-file = To źródło tekstowe odczytuje plik. Edytuj plik na komputerze OBS.
control-text-save = Zapisz tekst w OBS
control-text = Wbudowane źródło tekstowe
control-media-play = Odtwórz
control-media-pause = Pauza
control-media-stop = Zatrzymaj
control-media-restart = Odtwórz od początku
control-media-previous = Poprzedni
control-media-next = Następny
control-media-seconds = Pozycja multimediów w sekundach
control-media-seek = Przejdź do pozycji
control-media = Sterowanie multimediami
control-monitor-host = Odsłuch jest odtwarzany na komputerze OBS, a nie przez SceneDeck.
control-monitor-off = Odsłuch wyłączony
control-monitor-only = Tylko odsłuch — wycisz wyjście
control-monitor-output = Odsłuch i wyjście
control-audio-sync = Przesunięcie synchronizacji dźwięku (milisekundy)
control-audio-balance = Balans (0 = lewy, 0,5 = środek, 1 = prawy)
control-audio-track = Ścieżka dźwiękowa { $track }
control-audio-tracks = Przypisanie ścieżek dźwiękowych
control-advanced-audio = Zaawansowany dźwięk źródła


# Operating workspace
workspace-action-acknowledged = Żądanie potwierdzone; stan odświeżono z OBS
workspace-replay-observed = OBS zgłosił zapisaną powtórkę: { $path }
workspace-disk-space = Miejsce na nagrywanie OBS: { $value } MiB
control-stale = Ostatni znany stan. Odśwież odczyty OBS, aby włączyć zmiany.
control-no-matches = Żadne źródło nie pasuje do wyszukiwania. Wyczyść wyszukiwanie, aby pokazać wszystkie elementy.


# OBS recording volume and session activity
stats-recording-disk = Wolne miejsce na dysku nagrywania OBS
stats-recording-disk-value = { $value } MiB
stats-activity-title = Ostatnia aktywność i kontekst wykresów
stats-activity-detail = 15 najnowszych wpisów sesji. Wybierz wpis, aby zaznaczyć jego czas na wykresach próbek. Przerwy połączenia, brakujące próbki i zerowanie liczników ograniczają interpretację. Zbieżność w czasie nie dowodzi przyczyny.
stats-activity-clear-marker = Usuń znacznik wykresu
stats-activity-marker-visible = Przerywane linie oznaczają wybrane zdarzenie między odczytanymi próbkami. Wykresy używają odstępów między próbkami.
stats-activity-marker-unavailable = Brak znacznika: zdarzenie jest poza zachowanymi próbkami, w luce pomiarowej lub obejmuje zmianę zegara.
stats-activity-empty = W tej sesji nie zaobserwowano jeszcze żadnych operacji.
stats-activity-requested = Zażądano
stats-activity-observed = Zaobserwowano
stats-activity-failed = Niepowodzenie
stats-activity-connection = Połączenie
stats-activity-entry = { $time } · { $origin } · { $description }
scene-card-stage-hint = Przygotuj tę scenę w Podglądzie. Użyj przejścia, aby zmienić Program.
scene-card-preview-label = Podgląd
hotkey-hint-stage-take = { $binding } · Przygotuj w Podglądzie; użyj przejścia, aby zmienić Program.


header-obs-setup = Konfiguracja OBS
header-obs-unknown = Nieznane
header-obs-context = Profil: { $profile } · Kolekcja scen: { $collection }

status-bar-connection-detail = { $state } · OBS { $version }


doctor-structural-title = Struktura i role scen


control-image-caption = Obraz sceny { $scene }, przechwycony { $seconds } s temu. Odśwież obraz, aby przechwycić ponownie.

output-results-acknowledged = Potwierdzone

output-results-copy-path = Kopiuj ścieżkę na komputerze OBS

output-results-host-path = Ścieżka na komputerze OBS: { $path }

output-results-record-stop = Zatrzymaj nagrywanie

output-results-unconfirmed = Niepotwierdzone

output-results-title = Wyniki wyjść

output-results-detail = Żądania i obserwacje OBS są rozdzielone. Zdarzenie zapisu może pochodzić z innego kontrolera. Ścieżki dotyczą komputera OBS. W tej sesji zachowywane są 24 najnowsze wyniki.

record-elapsed-tooltip = Czas od zgłoszenia rozpoczęcia nagrywania przez OBS, łącznie z przerwami. Nie jest to czas trwania nagranego pliku.

window-open-navigation = Otwórz nawigację
sources-title = Źródła
sources-back = Powrót do źródeł
sources-select-hint = Wybierz źródło, aby sprawdzić jego ustawienia.
sources-text-conflict = Ten tekst zmienił się w OBS. Anuluj, aby wczytać bieżący tekst przed zapisaniem.
inventory-name-column = Scena
inventory-role-column = Rola
inventory-actions-column = Opcje
inventory-scene-options = Opcje sceny { $scene }
inventory-columns-button = Szerokość kolumn
inventory-name-width = Szerokość sceny
inventory-role-width = Szerokość roli
inventory-reset-widths = Przywróć szerokości kolumn
settings-scene-previews = Miniatury scen
settings-scene-previews-description = Pokazuj przyciemnione miniatury na większych kartach Live.
live-audio-expand = Rozwiń dźwięk
live-audio-collapse = Zwiń dźwięk
studio-mode-label = Studio
scene-card-image-pending = Miniatura niedostępna
scene-card-program-label = Program
output-options = Więcej opcji wyjścia
output-replay-label = Replay · { $state }
output-camera-label = Kamera wirtualna · { $state }

output-camera-name = Kamera wirt.
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
media-state-none = Brak multimediów
media-state-playing = Odtwarzanie
media-state-opening = Otwieranie
media-state-buffering = Buforowanie
media-state-paused = Wstrzymano
media-state-stopped = Zatrzymano
media-state-ended = Zakończono
media-state-error = Błąd
media-state-unknown = Nieznany
workspace-warning-identity = Nie udało się ustalić tożsamości wejść: { $detail }
workspace-warning-truncated = Lista źródeł skrócona: wyczerpano limit sprawdzania
workspace-warning-container = { $path }: { $detail }
workspace-warning-visibility = Widoczność { $owner } / { $item } niedostępna: { $detail }
workspace-warning-recursion = Skanowanie zagnieżdżonych scen zatrzymano na { $path }
workspace-warning-usage-partial = Wyszukiwanie użyć niepełne: ograniczone do { $scenes } scen i { $items } sprawdzonych elementów
workspace-warning-omitted = { $count ->
    [one] Pominięto 1 dodatkowe ostrzeżenie o źródłach
   *[other] Pominięto { $count } dodatkowych ostrzeżeń o źródłach
}
controller-workspace-superseded = Żądanie obszaru roboczego zastąpione przed zmianą
controller-workspace-admission-full = Operacja obszaru roboczego odrzucona: kolejka pełna lub wycofana
controller-workspace-text-changed = Tekst zmienił się w OBS. Odśwież przed zapisaniem
controller-workspace-action-cancelled = Akcja obszaru roboczego anulowana przed zmianą
controller-endpoint-changed = Punkt końcowy OBS się zmienił; połącz ponownie i odśwież
controller-collection-changed = Kolekcja OBS się zmieniła; odśwież przed kontynuowaniem
controller-collection-changed-during-read = Kolekcja OBS zmieniła się podczas odczytu; wyniki odrzucono
controller-audio-changed = Dźwięk zmienił się podczas obserwacji; wymagane odświeżenie
controller-outputs-still-active = Wyjścia OBS pozostają aktywne po zatrzymaniu
controller-shutdown-unconfirmed = Rozłączono; zatrzymanie wyjść niepotwierdzone: { $detail }; polecenia nie zostały ponowione
controller-stop-deadline = przekroczono limit czasu łagodnego zatrzymania
controller-connection-deadline = Przekroczono limit czasu połączenia z OBS
controller-publication-backlog = Kolejka powiadomień OBS przekroczyła limit; połącz ponownie i odśwież; historia wyjść może być niepełna
controller-event-connection-ended = Połączenie zdarzeń OBS zakończone; wymagane odświeżenie

# Scene identity changes
registry-scene-rename-conflict = Nazwa sceny „{ $old }” została zmieniona na „{ $new }”, ale ta nazwa ma już lokalne metadane. Zachowano oba wpisy; sprawdź Inwentarz.
registry-scene-rename-backlog = Podczas importu otrzymano zbyt wiele aktualizacji rejestru. Sprawdź Inwentarz po zakończeniu importu.

# Collection-specific curation
workspace-curation-title = Obszar roboczy programu
workspace-curation-loading = Wczytywanie ustawień tej kolekcji…
workspace-curation-unavailable = Połącz się z OBS i poczekaj na kolekcję scen.
workspace-curation-failed = Nie udało się wczytać obszaru roboczego: { $error }
workspace-curation-retry = Ponów

# Saved OBS connections
settings-connections-description = Zapisz ustawienia dla każdego połączenia OBS. Wybór nie łączy ponownie; użyj przycisku Połącz zapisane ustawienia.
settings-connections-picker = Zapisane połączenie
settings-connection-name = Nazwa połączenia
settings-connection-add = Dodaj połączenie
settings-connection-connect = Połącz zapisane ustawienia
settings-connection-new-name = Nowe połączenie { $number }
settings-connection-invalid = Podaj nazwę, host i port od 1 do 65535.
settings-obs-connected-profile = Połączono z { $name } ({ $endpoint }), OBS { $version }

inventory-retained-identity = { $role } · Zachowano po konflikcie nazwy lub tożsamości; nie przypisano do sceny.

inventory-scene-favorite = Ulubiona
inventory-scene-tags = Tagi (oddzielone przecinkami)
inventory-apply-tags = Zastosuj tagi

deck-search-placeholder = Szukaj scen i tagów
deck-favorites-only = Tylko ulubione
deck-reset = Pokaż wszystkie
deck-no-results = Żadna scena nie pasuje do tego widoku. Wyczyść filtry, aby pokazać wszystkie sceny.
deck-reorder-hint = Wyczyść filtry, aby zmienić kolejność scen.

deck-menu = Zestawy
deck-none = Brak zapisanych zestawów.
deck-remove = Usuń zestaw „{ $name }”
deck-save-title = Zapisz widok jako zestaw
deck-save-help = Wybierz unikalną nazwę. Bieżące wyszukiwanie i filtr ulubionych zostaną zapisane w tym obszarze roboczym.
deck-cancel = Anuluj
deck-save = Zapisz
deck-name = Nazwa zestawu

## Command palette
palette-title = Paleta poleceń (Ctrl+K)
palette-search = Szukaj scen i działań…
palette-open-page = Otwórz { $page }
palette-stage-scene = Przygotuj { $name }
palette-switch-scene = Przełącz na { $name }
palette-open-source = Otwórz źródło { $name }
palette-unmute = Wyłącz wyciszenie { $name }
palette-restart = Uruchom ponownie { $name }
palette-disable-filter = Wyłącz { $filter } · { $name }
palette-enable-filter = Włącz { $filter } · { $name }
palette-scope-changed = Działania OBS są niedostępne w tej palecie. Otwórz ją ponownie po połączeniu lub zmianie obszaru roboczego.
palette-empty = Brak pasujących działań.
palette-hint = ↑ / ↓ wybiera, Enter wykonuje, Esc zamyka. Zapisane działania multimediów i filtrów działają na wszystkich stronach.
palette-unavailable = Niedostępne: sprawdź połączenie, odśwież stan OBS lub sprawdź źródło działania.

## Custom action shortcuts
binding-title = Skróty działań
binding-description = Skróty należą do tego obszaru roboczego i działają na każdej stronie poza wpisywaniem tekstu. Konfliktowe skróty pozostają nieaktywne. Importowane skróty są początkowo wyłączone.
binding-manage = Zarządzaj skrótami działań
binding-invalid = Użyj Ctrl, Alt lub Super ze znakiem albo klawisza funkcyjnego F2–F24.
binding-reserved = Ten skrót jest zarezerwowany przez aplikację.
binding-slots-conflict = Ten skrót koliduje z ustawieniami pozycji scen.
binding-duplicate = Inne włączone działanie już używa tego skrótu.
binding-empty = Brak skrótów działań.
binding-enabled = Włączony
binding-edit = Edytuj skrót
binding-remove = Usuń skrót
binding-add = Dodaj skrót
binding-record-help = Wybierz działanie i zarejestruj kombinację klawiszy. Escape anuluje rejestrowanie. Współdzielone wejścia wpływają na wszystkie sceny, które ich używają.
binding-action = Działanie
binding-record = Zarejestruj skrót
binding-recording = Naciśnij kombinację klawiszy…
binding-unavailable = Działanie niedostępne. Stan OBS jest odświeżany; sprawdź go i ponownie naciśnij skrót.

action-inspect-input = Sprawdź multimedia i filtry źródła
action-inspect-none = Wybierz źródło…
action-inspect-help = Dostępne działania pojawią się poniżej. Wybór na stronie Źródła pozostanie bez zmian.

quick-title = Szybkie działania
quick-manage = Dostosuj szybkie działania
quick-add = Przypnij działanie
quick-edit = Zmień działanie
quick-remove = Usuń przypięte działanie
quick-up = Przenieś działanie w górę
quick-down = Przenieś działanie w dół
quick-empty = Przypnij często używane elementy sterowania obok scen.
quick-description = Przypięte działania należą do tego obszaru roboczego. Wspólne źródła wpływają na każdą scenę, która ich używa.
quick-disabled = Wyłączone — sprawdź i włącz w ustawieniach szybkich działań.
quick-observed = Zaobserwowany stan: { $state }
quick-ready = Gotowe
quick-muted = Wyciszone
quick-unmuted = Niewyciszone
quick-filter-enabled = Filtr włączony
quick-filter-disabled = Filtr wyłączony

quick-toggle-mute = Przełącz wyciszenie · { $name }
quick-toggle-filter = Przełącz filtr · { $name } / { $filter }

# Readiness review
preflight-title = Kontrola przed rozpoczęciem
preflight-description = Sprawdź gotowość przed rozpoczęciem. Mów do wybranego mikrofonu: sygnał za suwakiem powyżej −50 dBFS jest pamiętany przez 10 sekund, dopóki dostępne są świeże poziomy. Brakujące lub nieaktualne odczyty pozostają nieznane.
preflight-close = Zamknij
preflight-passed = Zaliczono
preflight-failed = Wymaga uwagi
preflight-unknown = Nieznane
preflight-skipped = Pominięto
preflight-refresh = Odśwież odczyty OBS
preflight-reset = Wyzeruj listę
preflight-ready = Gotowe: wszystkie skonfigurowane kontrole zaliczono.
preflight-attention = Sprawdź poniższe wyniki.
preflight-collection-result = Kolekcja: { $status } — oczekiwana „{ $expected }”, odczytana „{ $actual }”
preflight-scene-result = Scena „{ $name }”: { $status }
preflight-no-scenes = Wymagane sceny: nie skonfigurowano.
preflight-mic-result = Mikrofon „{ $name }”: obecny { $presence }; niewyciszony { $unmuted }; ostatni sygnał { $signal }
preflight-disk-result = Miejsce na nagranie (minimum { $minimum } MiB): { $status }

preflight-configure = Konfiguruj kontrolę
preflight-expected = Oczekiwana kolekcja scen
preflight-targets-description = Zmiana oczekiwania zeruje wybór scen i mikrofonu. Dostępne opcje pochodzą z odczytanej kolekcji; zapisane brakujące obiekty pozostają widoczne. Aby wybrać inną kolekcję, przełącz na nią OBS i otwórz konfigurację ponownie.
preflight-scenes = Wymagane sceny
preflight-check-mic = Sprawdzaj obecność, wyciszenie i sygnał mikrofonu
preflight-no-scene-choices = Brak dostępnych scen dla tego oczekiwania.
preflight-no-microphone = Nie wybrano mikrofonu
preflight-check-disk = Sprawdzaj miejsce na nagranie
preflight-minimum-disk = Minimalne wolne miejsce (MiB)
preflight-manual-items = Lista ręczna (jeden unikalny punkt w wierszu)
preflight-invalid-plan = Podaj oczekiwaną kolekcję, dodatni całkowity próg miejsca i unikalne punkty listy.
preflight-saving = Zapisywanie…
preflight-save-failed = Nie udało się zapisać: { $error }. Zmiany pozostają tutaj; spróbuj zapisać ponownie.
preflight-editor-retired = Połączenie, kolekcja lub zapisany plan zmieniły się. Zamknij edytor i otwórz go ponownie.

preflight-continue = Kontynuuj uruchamianie: { $output }…
preflight-continue-description = Możesz kontynuować z nierozstrzygniętymi kontrolami. Zwykłe ustawienia potwierdzania pozostają w mocy. Nic nie uruchamia się automatycznie.
preflight-review-before-output = Wyświetl kontrolę przed uruchomieniem transmisji lub nagrywania

preflight-free-space = Odczytane wolne miejsce na nagranie: { $space } MiB

# Sustained live health
health-title = Alerty stanu na żywo
health-network = Utrata klatek w sieci
health-render = Przeciążenie renderowania
health-encoder = Przeciążenie kodowania
health-disk = Mało miejsca na nagranie
health-network-help = OBS traci klatki transmisji. Sprawdź połączenie sieciowe lub zmniejsz bitrate.
health-render-help = OBS pomija klatki renderowania. Zmniejsz złożoność scen lub ustawienia wideo.
health-encoder-help = OBS pomija klatki kodowania. Zmniejsz obciążenie kodera lub ustawienia nagrywania i transmisji.
health-disk-help = Podczas nagrywania na woluminie nagrania pozostaje mniej niż 1 GiB. Zwolnij miejsce lub zakończ nagranie, zanim się wyczerpie.
health-active = Wymaga uwagi
health-recovering = Powrót do normy — ostatnie odczyty są prawidłowe
health-unobserved = Najnowsze odczyty niedostępne
health-caption = { $title } — { $phase }
health-more = { $summary } · jeszcze { $count }
health-policy = Alerty klatek wymagają co najmniej 1% pominiętych klatek w ostatnich interwałach przez 5 sekund. Mała ilość miejsca również musi trwać 5 sekund. Ostrzeżenia wydajności znikają po 10 sekundach prawidłowych odczytów; brakujące lub nieaktualne dane nie oznaczają poprawy.
health-open-stats = Otwórz statystyki
health-no-alerts = Brak bieżących alertów stanu.

health-connection-lost = Utracono połączenie z OBS
health-stream-stopped = Transmisja zatrzymana
health-record-stopped = Nagrywanie zatrzymane
health-connection-help = SceneDeck utracił połączenie z OBS. Sprawdź OBS i sieć, a następnie połącz ponownie. Stan wyjść pozostaje niepotwierdzony do wznowienia odczytów.
health-stream-stopped-help = OBS zgłosił zatrzymanie transmisji bez niedawnego polecenia zatrzymania z SceneDeck. Sprawdź OBS i miejsce docelowe przed ponownym uruchomieniem.
health-record-stopped-help = OBS zgłosił zatrzymanie nagrywania bez niedawnego polecenia zatrzymania z SceneDeck. Sprawdź miejsce na nagranie i OBS. Ten alert nie potwierdza zapisania pliku.
health-acknowledge = Potwierdź

health-event-policy = Alerty zdarzeń dotyczą bieżącego połączenia. Zaobserwowane przywrócenie lub nowe połączenie je usuwa. Potwierdzenie tylko usuwa powiadomienie.

# Selected-input audio health
health-audio-silence = { $input }: cisza
health-audio-clipping = { $input }: przesterowanie
health-audio-silence-help = Wyciszenie lub poziom po suwaku nieprzekraczający −50 dBFS przez 15 sekund. Sprawdź mikrofon, wyciszenie i routing. Wyłącz regułę na czas planowanej ciszy.
health-audio-clipping-help = Co najmniej jeden kanał przed suwakiem utrzymuje pełną skalę przez sekundę. Sprawdź wzmocnienie wejścia.
health-audio-disabled = Monitorowanie wyłączone
health-audio-missing = Wybrane wejście jest niedostępne lub niejednoznaczne
health-audio-monitoring = Monitorowanie aktualnych odczytów
health-audio-muted = Monitorowanie — wejście wyciszone
health-audio-settings = Monitorowanie dźwięku
health-audio-select = Zachowaj regułę dla tego wejścia
health-audio-check-silence = Ostrzegaj o długotrwałej ciszy
health-audio-check-clipping = Ostrzegaj o długotrwałym przesterowaniu
health-audio-policy = Wybierz wejścia, aby dodać reguły; usuń zaznaczenie, aby usunąć regułę. Włącz monitorowanie świadomie. Reguły działają podczas połączenia, także przed uruchomieniem wyjść. Cisza: 15 sekund na poziomie najwyżej −50 dBFS po suwaku lub wyciszenie. Przesterowanie: sekunda przy pełnej skali przed suwakiem. Powrót do normy wymaga dziesięciu sekund poprawnych odczytów. Brak lub nieaktualność odczytów oznacza niedostępność, nie ciszę. Odśwież, aby znaleźć wejścia.
health-audio-invalid = Wybierz co najmniej jedno ostrzeżenie dla każdej włączonej reguły i pozostaw jedną regułę na wejście.
health-audio-saved = Reguły zapisane.
health-audio-unsaved = Niezapisane zmiany.
health-audio-retired = Obszar roboczy lub zapisane reguły uległy zmianie. Zamknij i otwórz edytor ponownie.

# Observed media timing
media-progress-label = Postęp odtwarzania
media-position-observed = Zaobserwowana pozycja: { $time }
media-duration-observed = Czas trwania: { $time }
media-time-remaining = Pozostało: { $time }
media-time-unavailable = Niedostępne
media-duration-unavailable = Niedostępny lub nieokreślony
media-progress-hint = Czasy odzwierciedlają ostatni odczyt z OBS. Aby przewinąć, użyj pola pozycji poniżej.

# Local timestamp bookmarks
bookmarks-title = Zakładki
bookmark-policy = Zapisywane lokalnie. Czas sesji obejmuje pauzy. Czas OBS to niedawny odczyt czasu wyjścia, a nie pozycja w pliku ani rozdział.
bookmark-name = Nazwa zakładki
bookmark-add = Dodaj zakładkę
bookmark-retry = Ponów
bookmark-saving = Zapisywanie…
bookmark-save-error = Zakładki niedostępne lub niezapisane: { $error }
bookmark-scope-changed = Połącz z OBS lub otwórz ponownie to okno po zmianie połączenia lub kolekcji. Istniejące notatki pozostają dostępne.
bookmark-saved = { $count } zakładek zapisanych lokalnie
bookmark-default = Zakładka { $number }
bookmark-invalid-name = Wpisz nazwę o długości 1–120 znaków bez znaków sterujących.
bookmark-full = Osiągnięto limit 2000 zakładek. Usuń notatki, aby dodać kolejne.
bookmark-rename = Zmień nazwę
bookmark-remove = Usuń
bookmark-record-unknown = Czas nagrywania niedostępny
bookmark-record-inactive = Nagrywanie było nieaktywne
bookmark-record-time = Czas wyjścia OBS: { $time } · { $state } · Wiek próbki: { $age } s
bookmark-description = { $created }
    Sesja { $session }: { $elapsed } od połączenia
    { $connection } · { $endpoint }
    Kolekcja: { $collection } · Zaobserwowana scena: { $scene }
    { $recording }

# Session reports and explicit export
reports-title = Raporty sesji
reports-previous = Poprzednia sesja
reports-next = Następna sesja
reports-refresh = Odśwież migawkę
reports-export-json = Eksportuj JSON
reports-export-csv = Eksportuj CSV
reports-empty = Nie zarejestrowano sesji. Połącz z OBS, aby rozpocząć raport.
reports-current = Bieżąca sesja
reports-scene = Zaobserwowana scena: { $scene }
reports-ended = Sesja obserwacji zakończona: { $reason }
reports-alert-cleared = Alert usunięty; nie potwierdza to poprawy
reports-exporting = Eksportowanie…
reports-exported = Eksport zapisany: { $path }
reports-export-error = Błąd eksportu: { $error }
reports-policy = Ostatnie osiem sesji połączenia pozostaje w pamięci. Zapisana historia jest również dostępna tutaj. Odśwież, aby dodać nowe obserwacje lub zmiany zakładek. Eksport zachowuje wybraną migawkę. Potwierdzenia nie dowodzą zapisania pliku; ścieżki dotyczą komputera OBS.
reports-view-limit = Widok pokazuje ostatnie 150 zdarzeń i 50 zakładek. Eksport zawiera wszystkie zachowane rekordy. Brakujące próbki i zerowanie liczników ograniczają interpretację; liczniki klatek są skumulowane.
reports-summary = { $start } → { $end }
    { $connection } · { $endpoint }
    Zachowane zdarzenia: { $events } · Pominięte: { $omitted } · Luki aktywności: { $gaps }
    Zaobserwowane próbki: { $samples } · Zachowane: { $kept }
    Najwyższe zmierzone CPU: { $cpu } · Najniższe zmierzone FPS: { $fps }
    Zakładki: { $bookmarks }

# Optional session history
history-title = Historia sesji
history-enable = Automatycznie zapisuj historię sesji
history-retain = Liczba zachowanych sesji (1–32)
history-apply = Zastosuj ustawienia historii
history-retry = Wczytaj ponownie i ponów
history-clear = Wyczyść zapisaną historię
history-saving = Zapisywanie lub wczytywanie historii…
history-error = Historia niedostępna lub niezapisana: { $error }
history-status = { $mode } · Zapisane sesje: { $count }
history-on = Automatyczna historia włączona
history-off = Automatyczna historia wyłączona
history-settings-changed = Ustawienia zmieniono w innym miejscu. Otwórz to okno ponownie.
history-app-closed = SceneDeck zamknięty
reports-interrupted = Poprzednie uruchomienie; czas zakończenia nieznany
history-settings-description = Automatyczny zapis jest domyślnie wyłączony. Tutaj zarządzaj retencją, ponownym wczytywaniem i błędami zapisu.
history-policy = Włączenie zapisuje bieżący raport wraz z zachowanymi wcześniejszymi zdarzeniami oraz przyszłe sesje. Zapis odbywa się mniej więcej co dziesięć sekund i przy zmianie sesji. Wyłączenie zatrzymuje dalsze zapisy i zachowuje istniejące raporty. Mniejszy limit usuwa najstarsze zapisane sesje. Zakładki są zapisywane oddzielnie.
history-clear-confirm = Usunąć całą zapisaną i niezapisaną historię? Ostatnie raporty w pamięci oraz zakładki pozostaną dostępne. Bieżące połączenie nie będzie ponownie zapisane; automatyczna historia wznowi się przy następnym połączeniu, jeśli jest włączona.

history-exit-requested = Zażądano zamknięcia SceneDeck
history-exit-resumed = Zamknięcie anulowane lub opóźnione; wznowiono raportowanie

# Selected-input audio presets
preset-title = Ustawienia wstępne dźwięku
preset-open = Otwórz
preset-shared = Wybierz wejścia, aby zapisać ich bieżące wyciszenie i wzmocnienie. Zmiany wejścia dotyczą każdej sceny, która go używa. Ustawienia należą do tego połączenia i kolekcji scen.
preset-name = Nazwa ustawienia
preset-new = Nowe ustawienie
preset-save = Zapisz wybrane wejścia
preset-replace = Zastąp wybrane ustawienie
preset-rename = Zmień nazwę wybranego ustawienia
preset-delete = Usuń wybrane ustawienie
preset-preview = Podgląd zapisanych wartości
preset-missing = Brak lub niedostępne
preset-invalid = Wybierz 1–64 dostępnych wejść i unikalną nazwę do 80 znaków. Można zapisać do 64 ustawień.
preset-current = Bieżące: { $value }
preset-saved = Zapisane: { $value }
preset-none = Wybierz zapisane ustawienie, aby sprawdzić jego wartości.
preset-refresh-needed = Odśwież dane audio przed zapisaniem ustawienia lub sprawdzeniem zmian.
preset-retired = Obszar roboczy lub zapisane ustawienia zmieniły się. Zamknij ten edytor i otwórz go ponownie.
preset-pending = Oczekiwanie
preset-running = Sprawdzanie i stosowanie
preset-observed = Wartości potwierdzone przez OBS
preset-unchanged = Już zgodne
preset-changed = Wartości zmienione; sprawdź ponownie
preset-failed = Żądanie nie powiodło się
preset-unconfirmed = Wynik nie w pełni potwierdzony
preset-cancelled = Pozostałe zmiany anulowane
preset-acknowledgements = Potwierdzenia OBS: wzmocnienie { $gain }, wyciszenie { $mute }
preset-unavailable = Nie można zastosować: brak połączenia, inne ustawienie w toku lub podgląd wygasł.
preset-review = Sprawdź zmiany
preset-apply = Zastosuj do dostępnych wejść
preset-cancel = Zatrzymaj pozostałe zmiany
preset-apply-policy = Sprawdź przed zastosowaniem. Brakujące wejścia są pomijane; pozostałe mogą zakończyć się niezależnie. Potwierdzenie żądania nie potwierdza odczytu wartości. Wzmocnienie jest sprawdzane przed włączeniem dźwięku. Zatrzymanie blokuje dalsze zmiany, lecz nie cofa ukończonych. Odśwież i sprawdź ponownie po zmianie wartości.
preset-unsupported = To połączenie OBS nie udostępnia wszystkich żądań potrzebnych do zastosowania ustawień dźwięku.
preset-reviewed = Sprawdzony zestaw wartości jest zachowywany przez maksymalnie 30 sekund.

# Desktop portal shortcuts
global-title = Skróty pulpitu
global-description = Zarejestruj do 64 włączonych powiązań akcji dla tego połączonego obszaru roboczego. Pulpit wybiera klawisze i może poprosić o zgodę. Potwierdzone skróty zastępują odpowiadające im skróty aplikacji do zatrzymania. Rejestracja kończy się po rozłączeniu, zmianie obszaru lub powiązań albo zamknięciu głównego okna; włącz ją ponownie, gdy będziesz gotowy.
global-enable = Włącz skróty pulpitu
global-stop = Zatrzymaj skróty pulpitu
global-off = Skróty pulpitu są wyłączone.
global-pending = Oczekiwanie na pulpit…
global-active = Skróty pulpitu są aktywne. Przypisane klawisze:
global-none = Pulpit nie przypisał żadnych skrótów.
global-unavailable = Nie udało się włączyć skrótów pulpitu. Sprawdź, czy pulpit udostępnia portal GlobalShortcuts, i spróbuj ponownie.

help-controls-subtitle =
    Skróty, szybkie akcje i ustawienia audio

help-controls-body =
    Ctrl+K wyszukuje sceny i obsługiwane akcje. Ustawienia → Skróty akcji przypisuje stałe klawisze; szybkie akcje Live przypinają pojedyncze elementy sterowania. Wspólne wejścia wpływają na wszystkie używające ich sceny.
    Skróty pulpitu włącza się jawnie w menedżerze; wymagają obsługi portalu. Pulpit wybiera klawisze; zmiana połączenia, obszaru lub powiązań albo zamknięcie głównego okna kończy rejestrację.
    Mixer → Ustawienia audio zapisuje wzmocnienie i wyciszenie wybranych wejść. Przejrzyj zmiany i brakujące wejścia przed zastosowaniem. Wyniki rozróżniają potwierdzenia i zaobserwowane wartości; możliwy jest częściowy sukces. Zatrzymanie anuluje niewysłane zmiany; ponowne połączenie ich nie odtwarza.

help-readiness-subtitle =
    Przygotowanie i trwałe ostrzeżenia na żywo

help-readiness-body =
    Kontrola wstępna sprawdza oczekiwaną kolekcję, wymagane sceny, wyciszenie/sygnał wybranego mikrofonu, miejsce na nagranie i ręczną listę. Nie uruchamia wyjścia. Nieznane lub stare dane nie oznaczają powodzenia.
    Ostrzeżenia pozostają widoczne między stronami. Wykrywają utrzymujące się bieżące utraty klatek, przeciążenie renderowania/kodera, mało miejsca, rozłączenie i nieoczekiwane zatrzymania. Reguły ciszy/przesterowania wymagają jawnie wybranych wejść; mierniki nie dowodzą, co słyszy widownia.

help-session-subtitle =
    Zakładki, multimedia i raporty sesji

help-session-body =
    Źródła pokazują postęp i pozostały czas, gdy OBS udostępnia długość, oraz obsługiwane przewijanie. Sterowanie wyjściami obejmuje pauzę i wznowienie nagrywania.
    Zakładki zapisują lokalny czas sesji i ostatnią próbkę czasu nagrywania OBS. Przerwy i ponowne połączenia oznaczają, że nie są to dokładne pozycje pliku ani rozdziały.
    Raporty przechowują ograniczone zdarzenia, incydenty i dowody wyjść z eksportem CSV/JSON. Automatyczna historia jest domyślnie wyłączona; włącz ją i wybierz retencję w Historii sesji. Żądania i potwierdzenia pozostają oddzielone od zaobserwowanych wyników. Ścieżki nagrań należą do komputera OBS.
