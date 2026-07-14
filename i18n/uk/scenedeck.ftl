## SceneDeck UI strings (English, source locale).
##
## Grouped by the module each message is used from. Message ids are prefixed
## with the module name to keep them unambiguous in this single shared file.

## Internal — used only by the i18n loader's own regression test, not shown
## in the UI. Every locale must define this so the smoke test can confirm the
## locale's bundle loaded (not just the `en` fallback).
i18n-loader-smoke-test = Локалізацію завантажено.

## infra/error.rs — user-facing renderings of AppError. `detail` is raw
## upstream text (often from OBS or the OS) and is never translated.
error-connection = Помилка підключення до OBS: { $detail }
error-request = Помилка запиту до OBS: { $detail }
error-config = Помилка конфігурації: { $detail }
error-storage = Помилка сховища: { $detail }
error-notification-title = Помилка SceneDeck: { $message }

## domain/audio.rs — AudioSourceScope::label()
audio-scope-global = Глобальний
audio-scope-active = Сцена
audio-scope-nested = Вкладений
audio-scope-group = Група

## domain/graph.rs — EdgeStatus summary label/tooltip
edge-status-ok-label = Гаразд
edge-status-warning-label = Попередження
edge-status-forbidden-label = Заборонено
edge-status-ok-tooltip = Зв'язки, що відповідають політиці графа
edge-status-warning-tooltip = Зв'язки поза списком дозволених
edge-status-forbidden-tooltip = Зв'язки, заборонені політикою графа

## domain/output.rs — OutputRunState::label(), OutputStatus::summary()
output-state-inactive = Неактивно
output-state-starting = Запускається
output-state-active = Активно
output-state-stopping = Зупиняється
output-state-reconnecting = Перепідключення
output-state-paused = Призупинено
output-state-unknown = Невідомо
output-summary = { $name }: { $state }

## domain/role.rs — SceneRole::label()/description()
role-primary = Основна
role-secondary = Другорядна
role-module = Модуль
role-raw = Необроблена
role-debug = Налагодження
role-archive = Архів
role-unassigned = Не призначено
role-primary-desc = Сцена, доступна для перемикання в прямому ефірі
role-secondary-desc = Дійсна сцена, за замовчуванням прихована зі сторінки Ефір
role-module-desc = Багаторазова вкладена сцена, не перемикається напряму
role-raw-desc = Сцена-обгортка для обладнання або джерела
role-debug-desc = Тимчасова тестова сцена
role-archive-desc = Збережена, але виключена з усіх робочих процесів

## domain/mixer.rs — MixerMode / MixerGrouping labels
mixer-mode-active = Активний
mixer-mode-selected = Вибраний
mixer-mode-pinned = Закріплений
mixer-mode-active-desc = Слідувати за програмною сценою OBS.
mixer-mode-selected-desc = Переглядати вибрану сцену без слідування за OBS.
mixer-mode-pinned-desc = Утримувати вибрану сцену незмінною під час роботи.
mixer-grouping-scope = Область
mixer-grouping-scene-path = Шлях сцени
mixer-grouping-none = Немає

## domain/diagnostic.rs — DiagnosticSeverity label/count
diag-label-info = Інформація
diag-label-warning = Попередження
diag-label-error = Помилки
diag-count-info = { $count ->
    [one] { $count } інформаційний запис
    [few] { $count } інформаційні записи
    [many] { $count } інформаційних записів
   *[other] { $count } інформаційних записів
}
diag-count-warning = { $count ->
    [one] { $count } попередження
    [few] { $count } попередження
    [many] { $count } попереджень
   *[other] { $count } попереджень
}
diag-count-error = { $count ->
    [one] { $count } помилка
    [few] { $count } помилки
    [many] { $count } помилок
   *[other] { $count } помилок
}

## ui/pages/inventory.rs
inventory-no-role-assigned = Роль не призначено

## services/doctor_service.rs — diagnostic message/suggestion templates
doctor-no-role = Сцені не призначено роль у локальному реєстрі.
doctor-no-role-suggestion = Відкрийте Інвентар і призначте роль.
doctor-stale-entry = Запис реєстру посилається на сцену, яку не знайдено в OBS.
doctor-stale-entry-suggestion = Видаліть запис з Інвентаря.
doctor-protected-switchable = Захищена сцена має роль, що перемикається, '{ $role }'.
doctor-protected-switchable-suggestion = Захищені сцени зазвичай є будівельними блоками; розгляньте роль Модуль або Необроблена.
doctor-cycle = Циклічне посилання між сценами '{ $parent }' і '{ $child }'.
doctor-cycle-suggestion = Усуньте цикл вкладених сцен; OBS не може відображати цикли.
doctor-edge-primary-debug = Основна сцена залежить від сцени з роллю Налагодження. (→ '{ $child }')
doctor-edge-primary-debug-suggestion = Видаліть сцену з роллю Налагодження з робочого шляху перед виходом в ефір.
doctor-edge-primary-raw = Основна сцена напряму обгортає джерело з роллю Необроблена. (→ '{ $child }')
doctor-edge-primary-raw-suggestion = Обгорніть необроблене джерело в сцену з роллю Модуль для повторного використання та ясності.
doctor-edge-module-primary = Модуль залежить від основної сцени, що інвертує ієрархію. (→ '{ $child }')
doctor-edge-module-primary-suggestion = Модулі мають бути будівельними блоками, а не споживачами основних сцен.
doctor-edge-raw-nests = Сцена з роллю Необроблена вкладає іншу сцену. (→ '{ $child }')
doctor-edge-raw-nests-suggestion = Сцени з роллю Необроблена мають бути кінцевими обгортками джерел без вкладених сцен.
doctor-edge-forbidden = Залежність сцени заборонена політикою графа. (→ '{ $child }')
doctor-edge-outside-policy = Залежність сцени виходить за межі налаштованої політики графа. (→ '{ $child }')
doctor-edge-adjust-suggestion = Скоригуйте зв'язок вкладеної сцени або оновіть правила графа в реєстрі.

## controller/app_controller.rs
controller-not-connected = Немає підключення до OBS

## controller/state.rs — Page titles and ObsStatus labels
page-live = Ефір
page-mixer = Мікшер
page-graph = Граф
page-inventory = Інвентар
page-doctor = Діагностика
page-settings = Налаштування
obs-status-disconnected = Відключено
obs-status-connecting = Підключення…
obs-status-connected = Підключено
obs-status-error = Помилка

## storage/config.rs — ConfigStartupNotice
config-first-launch = Збережених налаштувань ще немає. Завантажено типові значення.
config-read-failed = Не вдалося прочитати налаштування: { $detail }
config-parse-failed = Не вдалося розібрати налаштування: { $detail }

## graph.rs

graph-empty-title = Немає залежностей
graph-empty-description = Жодна сцена не вкладає інші сцени, або немає підключення до OBS. Підключіться та додайте вкладені джерела сцен, щоб побачити граф залежностей.
graph-page-title = Залежності сцен
graph-reset-tooltip = Скинути розташування графа
graph-edge-summary-count = { $label } { $count }

## mixer.rs

## Page chrome / empty state
mixer-empty-title = Немає даних мікшера
mixer-empty-description = Підключіться до OBS, щоб завантажити сцени та аудіоджерела.
mixer-page-title = Мікшер
mixer-controls-title = Керування мікшером
mixer-summary-title = Поточне джерело мікшера

## Control rows (ComboRow / EntryRow titles+subtitles)
mixer-mode-row-title = Режим
mixer-mode-row-subtitle = Активний слідує за OBS; Вибраний і Закріплений утримують обрану сцену незмінною.
mixer-scene-row-title = Сцена
mixer-scene-row-subtitle = Використовується режимами Вибраний і Закріплений.
mixer-grouping-row-title = Групувати за
mixer-grouping-row-subtitle = Визначає, як аудіоджерела впорядковано нижче.
mixer-search-row-title = Пошук

## Scene-loading / no-scene placeholders
mixer-no-scene-title = Сцену не вибрано
mixer-no-scene-description = Виберіть сцену, щоб завантажити її аудіо мікшера.
mixer-loading-title = Завантаження аудіо мікшера
mixer-loading-description = Завантаження аудіоджерел для { $scene }.

## Audio-source empty states
mixer-current-scene-fallback = Поточна сцена
mixer-no-audio-sources-title = Немає аудіоджерел
mixer-no-audio-sources-description = { $scene } не має відповідних налаштованих аудіоджерел OBS.
mixer-no-matching-title = Немає відповідних аудіоджерел
mixer-no-matching-description = Змініть фільтр пошуку, щоб показати доступні аудіоджерела.

## Group titles
mixer-group-all-sources = Усі джерела
mixer-group-global-fallback = Глобальні

## Error placeholder + retry
mixer-error-title = Аудіо мікшера недоступне
mixer-error-description = Не вдалося завантажити аудіоджерела для { $scene }: { $message }
mixer-retry-button-label = Повторити
mixer-retry-button-tooltip = Повторити завантаження аудіо мікшера

## Current-source summary row (source_summary / scene_target_summary)
mixer-summary-following-active = Слідування за активною сценою OBS: { $scene }
mixer-summary-no-scene-selected = Сцену не вибрано
mixer-summary-selected-scene = Вибрана сцена: { $scene }
mixer-summary-pinned-scene = Закріплена сцена: { $scene }
mixer-summary-selected-fallback = Вибрану сцену не встановлено; використовується активна сцена OBS: { $scene }
mixer-summary-pinned-selected-fallback = Закріплену сцену не встановлено; використовується вибрана сцена: { $scene }
mixer-summary-pinned-active-fallback = Закріплену та вибрану сцени не встановлено; використовується активна сцена OBS: { $scene }

## doctor.rs — page chrome, empty state, and all-clear text for the Doctor
## page. Diagnostic finding messages themselves (`doctor-no-role`,
## `doctor-cycle*`, `doctor-edge-*`, etc.) already exist in
## src/services/doctor_service.rs and are NOT redefined here.
doctor-page-title = Діагностика
doctor-empty-state-title = Нічого перевіряти
doctor-empty-state-description = Підключіться до OBS, щоб запустити діагностику архітектури.
doctor-summary-row-title = Діагностика
doctor-rerun-tooltip = Запустити діагностику знову
doctor-all-clear-title = Проблем не знайдено
doctor-all-clear-detail = Архітектура сцен відповідає всім перевіркам.

## inventory.rs — page chrome, group titles, YAML import/export UI, and
## status messages. Role names/descriptions themselves are NOT redefined
## here — they already exist in src/domain/role.rs (`role-*`,
## `role-*-desc`, `role-unassigned`) and are reused directly via
## SceneRole::label()/description()/unassigned_label(). The already-present
## `inventory-no-role-assigned` message is also left untouched.
inventory-page-title = Інвентар
inventory-empty-state-title = Немає сцен
inventory-empty-state-description = Підключіться до OBS, щоб завантажити список сцен.
inventory-scenes-group-title = Сцени OBS
inventory-scenes-group-description = Призначайте ролі, щоб керувати тим, які сцени з'являються на сторінці Ефір.
inventory-stale-group-title = Застарілі записи реєстру
inventory-stale-group-description = Ці сцени є у вашому локальному реєстрі, але більше не існують в OBS.
inventory-remove-stale-tooltip = Видалити застарілий запис
inventory-yaml-row-title = YAML реєстру сцен
inventory-yaml-row-subtitle = Експортуйте або імпортуйте ролі сцен, теги, позначки захисту та правила графа.
inventory-yaml-filter-name = Файли YAML

# Shared between the row's Export button, the export FileChooserNative's
# accept label, and (for Cancel) both the export and import dialogs.
inventory-export-button-label = Експортувати
inventory-export-tooltip = Експортувати реєстр сцен у YAML
inventory-import-button-label = Імпортувати
inventory-import-tooltip = Імпортувати реєстр сцен із YAML
inventory-dialog-cancel-label = Скасувати

inventory-export-dialog-title = Експорт реєстру сцен
inventory-export-success = Реєстр сцен експортовано до { $path }.
inventory-export-error = Помилка експорту: { $error }
inventory-export-no-file = Помилка експорту: файл не вибрано.

inventory-import-dialog-title = Імпорт реєстру сцен
inventory-import-error = Помилка імпорту: { $error }
inventory-import-no-file = Помилка імпорту: файл не вибрано.

## window.rs

window-stream-live-tooltip = Триває пряма трансляція
window-about-tooltip = Про SceneDeck
window-refresh-tooltip = Оновити поточну сторінку

window-stream-status-line = Трансляція: { $state }{ $elapsed }
window-record-status-line = Запис: { $state }{ $elapsed }

window-status-connecting = Підключення до OBS…
window-connect-btn-connecting = Підключення…
window-current-scene-none = Поточна сцена: —
window-status-connected = Підключено — OBS { $version }
window-connect-btn-disconnect = Відключитися
window-status-disconnected = Відключено
window-connect-btn-connect = Підключитися до OBS
window-live-disconnected-hint = Підключіться до OBS, щоб використовувати керування Ефіром
window-current-scene = Поточна сцена: { $scene }
window-status-error = Помилка: { $error }
window-connect-btn-retry = Повторити
window-obs-connection-failed = Не вдалося підключитися до OBS
window-toast-obs-error = Помилка OBS: { $error }

window-output-kind-stream = Трансляція
window-output-kind-record = Запис

window-sidebar-output-starting = Запуск…
window-sidebar-output-stopping = Зупинка…
window-sidebar-output-reconnecting = Перепідключення…
window-sidebar-output-working = Виконання…

window-sidebar-start-stream = Почати трансляцію
window-sidebar-stop-stream = Зупинити трансляцію
window-sidebar-start-recording = Почати запис
window-sidebar-stop-recording = Зупинити запис

window-selector-profile-label = Профіль
window-selector-profile-tooltip = Перемкнути профіль OBS
window-selector-collection-label = Колекція
window-selector-collection-tooltip = Перемкнути колекцію сцен OBS

## live.rs

live-start-stream-label = Почати трансляцію
live-stop-stream-label = Зупинити трансляцію
live-start-record-label = Почати запис
live-stop-record-label = Зупинити запис
live-stream-toggle-tooltip = Почати або зупинити трансляцію
live-record-toggle-tooltip = Почати або зупинити запис
live-stream-inactive-label = Трансляція: Неактивно
live-record-inactive-label = Запис: Неактивно
live-copy-last-recording-path-tooltip = Копіювати шлях останнього запису
live-copied-recording-path-tooltip = Шлях останнього запису скопійовано
live-copy-recording-path-with-value-tooltip = Копіювати шлях запису: { $path }
live-stream-card-title = Трансляція
live-recording-card-title = Запис
live-current-scene-placeholder = Поточна сцена: —
live-scenes-section-label = Сцени
live-scenes-connect-hint = Підключіться до OBS, щоб завантажити сцени.
live-audio-section-label = Аудіо
live-disconnected-title = Підключіться до OBS, щоб використовувати керування Ефіром
live-disconnected-detail = Скористайтеся елементом підключення внизу бічної панелі.
live-stream-command-error-label = Помилка команди трансляції
live-recording-command-error-label = Помилка команди запису
live-last-recording-detail = Останній запис: { $path }
live-starting-stream = Запуск трансляції…
live-stopping-stream = Зупинка трансляції…
live-reconnecting-stream = Перепідключення трансляції…
live-starting-recording = Запуск запису…
live-stopping-recording = Зупинка запису…
live-reconnecting-recording = Перепідключення запису…
live-button-starting = Запуск…
live-button-stopping = Зупинка…
live-button-reconnecting = Перепідключення…
live-button-working = Виконання…
live-output-kind-stream = Трансляція
live-output-kind-record = Запис
live-output-label = { $kind }: { $state }
live-output-label-with-elapsed = { $kind }: { $state } · { $elapsed }
live-scenes-no-primary-hint = Сцен з роллю Основна не знайдено. Призначте ролі в Інвентарі.
live-audio-empty-hint = Аудіовходи не налаштовано.
live-cancel-button-label = Скасувати
live-start-stream-confirm-heading = Почати трансляцію?
live-start-stream-confirm-body = OBS почне надсилання прямої трансляції.
live-stop-stream-confirm-heading = Зупинити трансляцію?
live-stop-stream-confirm-body = OBS припинить надсилання прямої трансляції.
live-start-recording-confirm-heading = Почати запис?
live-start-recording-confirm-body = OBS почне новий запис.
live-start-recording-confirm-label = Почати запис
live-stop-recording-confirm-heading = Зупинити запис?
live-stop-recording-confirm-body = OBS зупинить поточний запис.
live-stop-recording-confirm-label = Зупинити запис

## audio_card.rs
audio-card-mute-tooltip = Вимкнути звук входу
audio-card-source-path-tooltip = { $scope }: { $path }
audio-card-fader-tooltip = Регулятор гучності
audio-card-lock-tooltip = Заблокувати повзунок гучності
audio-card-fine-plus-tooltip = +1 dB
audio-card-fine-reset-tooltip = Скинути до 0.0 dB
audio-card-fine-minus-tooltip = -1 dB

## scene_card.rs
scene-card-tooltip = { $status } ({ $role })
scene-card-role-suffix = { $role } сцена

## status_bar.rs
status-bar-stream-inactive = Трансляція: Неактивно
status-bar-record-inactive = Запис: Неактивно
status-bar-fps-placeholder = FPS —
status-bar-cpu-placeholder = CPU —
status-bar-bitrate-placeholder = Бітрейт —
status-bar-fps = FPS { $value }
status-bar-cpu = CPU { $value }%
status-bar-bitrate = Бітрейт { $value } kbps
status-bar-dropped = { $count } втрачено

## settings.rs

settings-page-title = Налаштування
settings-appearance-title = Вигляд
settings-appearance-description = Додатки GNOME мають за замовчуванням слідувати системному стилю.
settings-theme-mode-system = Системна
settings-theme-mode-light = Світла
settings-theme-mode-dark = Темна
settings-color-scheme-title = Кольорова схема
settings-color-scheme-subtitle = Слідувати системним налаштуванням або примусово встановити світлу / темну
settings-theme-title = Тема
settings-theme-status-title = Стан теми
settings-theme-status-initial = Тему завантажено.
settings-failed-to-save = Не вдалося зберегти: { $err }
settings-custom-css-title = Власний CSS
settings-custom-css-subtitle = Завантажувати окремі файли CSS користувача для світлого й темного режиму
settings-custom-light-css-title = Шлях до власного CSS світлої теми
settings-custom-dark-css-title = Шлях до власного CSS темної теми
settings-reload-css-title = Перезавантажити власний CSS
settings-reload-css-subtitle = Повторно застосувати вибрану тему та відповідний файл власного CSS для світлого/темного режиму.
settings-reload-button = Перезавантажити
settings-language-title = Мова
settings-language-description = Зміни набувають чинності після перезапуску SceneDeck.
settings-display-language-title = Мова інтерфейсу
settings-display-language-subtitle = Виберіть мову або слідуйте системній локалі.
settings-language-status-title = Стан мови
settings-language-status-initial = Перезапустіть, щоб застосувати змінену мову.
settings-language-saved = Мову збережено. Перезапустіть SceneDeck, щоб застосувати її.
settings-obs-connection-title = Підключення до OBS
settings-obs-connection-description = Налаштування WebSocket для OBS Studio (типовий порт: 4455).
settings-host-title = Хост
settings-port-title = Порт
settings-password-title = Пароль (необов'язково)
settings-obs-status-title = Стан OBS
settings-invalid-port = Некоректний номер порту.
settings-saved = Налаштування збережено.
settings-password-saved = Пароль збережено у сховищі ключів.
settings-keyring-error = Помилка сховища ключів: { $err }
settings-output-safety-title = Безпека виведення
settings-output-safety-description = Необов'язкові підтвердження для критичних дій трансляції та запису.
settings-confirm-start-stream-title = Підтверджувати початок трансляції
settings-confirm-start-stream-subtitle = Запитувати перед початком прямої трансляції.
settings-confirm-stop-stream-title = Підтверджувати зупинку трансляції
settings-confirm-stop-stream-subtitle = Запитувати перед зупинкою прямої трансляції.
settings-confirm-start-recording-title = Підтверджувати початок запису
settings-confirm-start-recording-subtitle = Запитувати перед початком запису.
settings-confirm-stop-recording-title = Підтверджувати зупинку запису
settings-confirm-stop-recording-subtitle = Запитувати перед зупинкою запису.
settings-obs-not-connected = Немає підключення до OBS.
settings-obs-connecting = Підключення до OBS…
settings-obs-connected = Підключено — OBS { $version }
settings-obs-error = Помилка: { $err }
settings-theme-subtitle = { $description } Зразки кольорів: { $swatches }
settings-theme-loaded = Завантажено { $theme } ({ $variant }).
settings-theme-loaded-with-warnings = Тему завантажено з попередженнями.

## theme.rs

theme-adwaita-default-name = Типова Adwaita
theme-adwaita-default-desc = Нейтральний стиль, що відповідає типовим налаштуванням GNOME.
theme-scenedeck-dark-name = SceneDeck темна
theme-scenedeck-dark-desc = Стримана темна консольна тема для роботи в прямому ефірі.
theme-scenedeck-light-name = SceneDeck світла
theme-scenedeck-light-desc = Чітка світла консольна тема зі стриманим контрастом.
theme-obsidian-name = Обсидіан
theme-obsidian-desc = Графітові поверхні з високою читабельністю та холодними акцентами.
theme-nord-name = Nord
theme-nord-desc = Холодні синьо-сірі поверхні з інеїстими акцентами.
theme-dracula-inspired-name = У стилі Dracula
theme-dracula-inspired-desc = Темна виразна палітра на основі оригінального CSS.
theme-solarized-dark-name = Solarized темна
theme-solarized-dark-desc = М'який контраст без відблисків із бірюзовими та бурштиновими акцентами.
theme-high-contrast-name = Висока контрастність
theme-high-contrast-desc = Виразніші контури та контраст для критично важливих елементів керування.
theme-stream-red-name = Трансляційний червоний
theme-stream-red-desc = Червоні акценти для трансляції, орієнтовані на стани прямого ефіру.
theme-studio-purple-name = Студійний фіолетовий
theme-studio-purple-desc = Стримані фіолетові акценти без надмірного домінування на поверхнях.
theme-ubuntu-violet-name = Ubuntu фіолетовий
theme-ubuntu-violet-desc = Фіолетові поверхні в стилі Ubuntu з теплим акцентом для прямого ефіру.
theme-custom-css-read-failed = Не вдалося прочитати власний CSS із { $path }: { $err }
theme-custom-css-no-matching-file = Власний CSS увімкнено, але не встановлено відповідний файл для світлого/темного режиму.
theme-css-no-display = { $label } не завантажено, оскільки немає доступного дисплея GTK.
theme-css-parse-error = Помилка розбору CSS { $label }: { $message }
