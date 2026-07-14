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

## scene_card.rs
scene-card-tooltip = { $status } ({ $role })
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

## settings.rs

settings-page-title = Ustawienia
settings-appearance-title = Wygląd
settings-appearance-description = Aplikacje GNOME powinny domyślnie podążać za stylem systemu.
settings-theme-mode-system = Systemowy
settings-theme-mode-light = Jasny
settings-theme-mode-dark = Ciemny
settings-color-scheme-title = Schemat kolorów
settings-color-scheme-subtitle = Podążaj za preferencją systemu lub wymuś tryb jasny / ciemny
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
