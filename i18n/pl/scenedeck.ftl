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
inventory-stale-group-description = Te sceny znajdują się w lokalnym rejestrze, ale nie istnieją już w OBS.
inventory-remove-stale-tooltip = Usuń nieaktualny wpis
inventory-yaml-row-title = YAML rejestru scen
inventory-yaml-row-subtitle = Eksportuj lub importuj role scen, tagi, flagi ochrony i reguły grafu.
inventory-yaml-filter-name = Pliki YAML

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
settings-theme-subtitle = { $description } Próbki kolorów: { $swatches }
settings-theme-loaded = Wczytano { $theme } ({ $variant }).
settings-theme-loaded-with-warnings = Motyw wczytany z ostrzeżeniami.

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
    SceneDeck nigdy nie zmienia nazw, nie usuwa ani nie przestawia niczego w OBS. Odczytuje sceny przez połączenie OBS WebSocket i przechowuje własne notatki na ich temat.
    „Rola” to jedna z takich notatek: Twoja etykieta określająca przeznaczenie sceny — scena emitowana na antenie, nakładka wielokrotnego użytku albo pozostałość po teście.
    Notatki znajdują się w pliku registry.json obok pliku konfiguracji, więc zachowują się po ponownym uruchomieniu oraz można je wyeksportować i przenieść na inny komputer ze strony Inwentarz.
    Ponieważ notatki są lokalne, dwie osoby mogą korzystać z jednej konfiguracji OBS i każda może mieć inną stronę Na żywo.

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
    Eksport zapisuje role, kolejność, kolory akcentu, ikony, tagi i reguły grafu w jednym pliku YAML — zwykłym formacie tekstowym, który można czytać i przechowywać w systemie kontroli wersji.
    Import zastępuje lokalny rejestr zawartością takiego pliku. Użyj go, aby przenieść gotową konfigurację na drugi komputer albo wrócić do stanu sprzed eksperymentu.
    Nazwy scen łączą plik z OBS, więc scena przemianowana w OBS wraca jako nieaktualny wpis. Inwentarz pokazuje nieaktualne wpisy i pozwala je usunąć.

help-group-operate-title = Prowadzenie programu
help-group-operate-description = Strony używane podczas trwającej transmisji.

help-live-title = Strona Na żywo
help-live-subtitle = Karty scen, dźwięk i scena programowa
help-live-body =
    Na żywo to widok operatorski: bieżąca scena programowa u góry, karty scen po jednej stronie i kompaktowe karty dźwięku po drugiej. Przeciągnij separator, aby przeznaczyć więcej miejsca na potrzebną część.
    Kliknięcie karty sceny przełącza OBS na tę scenę. Bieżąca jest oznaczona jako Aktywna, a pozostałe jako Gotowe.
    Brak kart po połączeniu? Żadna scena nie ma jeszcze roli Główna — zobacz powyżej „Ukrywanie scen, na które nigdy nie przełączasz”.
    Pasek stanu u dołu jest widoczny na każdej stronie i pokazuje stan połączenia, stan transmisji i nagrywania wraz z czasem trwania oraz bieżące FPS, utracone klatki, użycie procesora i bitrate.

help-hotkeys-title = Przełączanie scen z klawiatury
help-hotkeys-subtitle = Domyślnie Ctrl+1 … Ctrl+0, z możliwością konfiguracji
help-hotkeys-body =
    Każda z pierwszych dziesięciu kart Na żywo ma małą plakietkę z cyfrą: pierwsza karta to 1, dziewiąta to 9, a dziesiąta to 0. Podpis obok nagłówka Sceny zawsze pokazuje bieżące przypisanie.
    Numery pozycji odpowiadają kolejności w Inwentarzu, więc zmiana kolejności kart zmienia też kolejność skrótów.
    Ustawienia → Skróty scen określają sposób naciskania cyfry. Modyfikator z cyfrą (domyślnie Ctrl) nie uruchomi skrótu przypadkiem podczas pisania. Sama cyfra jest najszybsza. Styl z liderem działa jak w vimie: naciśnij klawisz lidera, zwolnij go, a potem naciśnij cyfrę; podczas oczekiwania podpis pokazuje „Lider aktywny”.
    Skróty działają tylko na stronie Na żywo, a style bez modyfikatora są nieaktywne, gdy fokus znajduje się w polu tekstowym. Cyfra bez przypisanej sceny wyświetla tę informację w podpisie zamiast cokolwiek przełączać.

help-audio-title = Dźwięk: strona Mikser i mierniki
help-audio-subtitle = Co oznaczają kolorowe paski
help-audio-body =
    Karty dźwięku pojawiają się na stronie Na żywo oraz — z większą ilością miejsca i elementów sterujących — na stronie Mikser. Najpierw wyświetlane są globalne urządzenia audio OBS, a potem źródła obsługujące dźwięk w bieżącej scenie, również te w zagnieżdżonych scenach i grupach.
    Tryby Miksera określają, dźwięk której sceny oglądasz. Aktywny podąża za sceną programową OBS. Wybrany wczytuje jedną scenę i pozostaje przy niej. Przypięty zachowuje wybraną scenę jako stały cel, gdy OBS przełącza się dalej.
    Miernik obok każdego suwaka obejmuje zakres od -60 dB u dołu do 0 dB u góry i używa progów OBS: zielony poniżej -20 dB dla muzyki i tła, żółty od -20 do -9 dB dla mowy, czerwony powyżej -9 dB, gdzie zaczyna się przesterowanie. Nic nie powinno stale pozostawać w czerwonym zakresie.
    Jedna kolumna oznacza źródło mono; dwie oznaczają stereo — najpierw lewy, potem prawy kanał. Jeśli porusza się tylko lewa kolumna, połowa odbiorców nic z niego nie usłyszy.
    Linia nad wypełnieniem pokazuje najwyższy szczyt z ostatnich dwudziestu sekund — to najszybszy sposób na zauważenie pominiętego przesterowania. Kwadrat na samym dole pokazuje poziom docierający z urządzenia przed suwakiem; jeśli jest zbyt wysoki, ruch suwaka tego nie naprawi.
    Przycisk blokady na karcie zamraża jedynie suwak w SceneDeck. Nie blokuje niczego w OBS.

help-outputs-title = Rozpoczynanie i zatrzymywanie transmisji oraz nagrywania
help-outputs-subtitle = I potwierdzenia zapobiegające pomyłkom
help-outputs-body =
    Przyciski Rozpocznij/Zatrzymaj transmisję i Rozpocznij/Zatrzymaj nagrywanie znajdują się na dole paska bocznego i są dostępne z każdej strony. Pasek stanu pokazuje stan i czas działania.
    Ustawienia → Bezpieczeństwo wyjść określają, które z czterech działań wymagają potwierdzenia. Domyślnie zatrzymanie każdego wyjścia wymaga potwierdzenia, a rozpoczęcie nie — wcześniejsze rozpoczęcie zwykle niewiele kosztuje, a zbyt wczesne zatrzymanie kończy program.
    Zmiany stanu wykonane w samym OBS również są tutaj widoczne: SceneDeck śledzi zdarzenia OBS, zamiast zakładać, że naciśnięcie własnego przycisku zadziałało.

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
    Ustawienia, w tym host i port OBS, znajdują się w $XDG_CONFIG_HOME/scenedeck/config.json — zwykle ~/.config/scenedeck/config.json.
    Role scen, kolejność, akcenty i ikony znajdują się w pliku registry.json w tym samym folderze.
    Hasła OBS nie ma w żadnym z tych plików. Jest przechowywane w pęku kluczy Secret Service pulpitu — tym samym sejfie, którego używa przeglądarka.
    Aby przenieść pełną konfigurację na inny komputer, wykonaj kopię obu plików JSON; dla samych scen możesz też użyć eksportu YAML w Inwentarzu.

help-shortcuts-title = Skróty klawiaturowe
help-shortcuts-subtitle = Pełna lista
help-shortcuts-body =
    F1 — otwórz ten przewodnik.
    Ctrl+R — połącz ponownie z OBS.
    Ctrl+, — otwórz Ustawienia.
    Ctrl+Q — zakończ SceneDeck.
    Ctrl+1 … Ctrl+0 na stronie Na żywo — przełącz na jedną z pierwszych dziesięciu kart scen. Kombinację można skonfigurować w Ustawienia → Skróty scen.

welcome-dialog-heading = Witamy w SceneDeck
welcome-dialog-body = Wygląda na to, że uruchamiasz program po raz pierwszy. Strona Pomoc opisuje łączenie z OBS — także na innym komputerze — i pokazuje, jak ograniczyć stronę Na żywo do faktycznie przełączanych scen. Zajmie to kilka minut i pomoże uniknąć błędów.
welcome-dialog-later = Nie teraz
welcome-dialog-open = Przeczytaj przewodnik
window-help-tooltip = Pomoc i przewodnik wprowadzający
