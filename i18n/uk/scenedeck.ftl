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
events-resync-warning = Пропущено сповіщення OBS. SceneDeck повторно синхронізується; історія виходів може бути неповною.

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
page-stats = Статистика
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
inventory-stale-group-description = Записи відсутніх сцен або записи з конфліктами ідентичності потребують перевірки. Збережені записи не застосовано до поточних сцен.
inventory-remove-stale-tooltip = Видалити застарілий запис
inventory-yaml-row-title = YAML реєстру сцен
inventory-yaml-row-subtitle = Експортуйте або імпортуйте ролі сцен, теги, позначки захисту та правила графа.
inventory-yaml-filter-name = Файли YAML
inventory-accent-clear-tooltip = Скинути колір акцента сцени
inventory-drag-tooltip = Перетягніть, щоб змінити порядок сцен
inventory-accent-dialog-title = Колір акцента сцени
inventory-accent-choose-tooltip = Виберіть колір акцента сцени

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
audio-card-meter-tooltip-title = Індикатор гучності: { $channels }
audio-card-meter-tooltip-zones = Зелений нижче -20 dB · Жовтий до -9 dB · Червоний вище, близько до кліпінгу
audio-card-meter-tooltip-indicators = Смуга: піковий рівень зі спадом · Лінія: найгучніший пік за 20 с · Крапка: гучність · Основа: рівень, що надходить із пристрою
audio-card-meter-tooltip-waiting = Індикатор гучності: очікування рівнів від OBS

## icon.rs
icon-none = Без піктограми
icon-camera = Камера
icon-desktop = Робочий стіл
icon-game = Гра
icon-film = Фільм
icon-images = Зображення
icon-television = Телебачення
icon-browser = Браузер
icon-terminal = Термінал
icon-code = Код
icon-chat = Чат
icon-guests = Гості
icon-star = Зірка
icon-alert = Сповіщення
icon-break = Перерва
icon-countdown = Зворотний відлік
icon-start = Початок
icon-pause = Пауза
icon-stop = Зупинка
icon-settings = Налаштування
icon-layers = Шари
icon-microphone = Мікрофон
icon-headset = Гарнітура
icon-headphones = Навушники
icon-speaker = Динамік
icon-volume = Гучність
icon-music = Музика
icon-instrument = Інструмент
icon-radio = Радіо
icon-call = Виклик
icon-waveform = Осцилограма
inventory-scene-icon-tooltip = Виберіть піктограму для цієї сцени
mixer-input-icon-tooltip = Виберіть піктограму для цього джерела звуку

## meter.rs
audio-meter-zone-nominal = Зелений
audio-meter-zone-warning = Жовтий
audio-meter-zone-error = Червоний
audio-meter-channel-mono = Моно
audio-meter-channel-left = Лівий
audio-meter-channel-right = Правий
audio-meter-channel-front-left = Передній лівий
audio-meter-channel-front-right = Передній правий
audio-meter-channel-front-center = Передній центральний
audio-meter-channel-lfe = LFE
audio-meter-channel-rear-left = Задній лівий
audio-meter-channel-rear-right = Задній правий
audio-meter-channel-side-left = Бічний лівий
audio-meter-channel-side-right = Бічний правий
audio-meter-channel-numbered = Канал { $index }

## hotkey.rs
hotkey-modifier-ctrl = Ctrl+
hotkey-modifier-alt = Alt+
hotkey-modifier-shift = Shift+
hotkey-modifier-super = Super+
hotkey-modifier-ctrl-alt = Ctrl+Alt+
hotkey-modifier-ctrl-shift = Ctrl+Shift+
hotkey-style-plain = Лише цифра
hotkey-style-ctrl = Ctrl + цифра
hotkey-style-alt = Alt + цифра
hotkey-style-shift = Shift + цифра
hotkey-style-super = Super + цифра
hotkey-style-ctrl-alt = Ctrl+Alt + цифра
hotkey-style-ctrl-shift = Ctrl+Shift + цифра
hotkey-style-leader = Клавіша-лідер, потім цифра
hotkey-leader-space = Пробіл
hotkey-leader-comma = Кома
hotkey-leader-semicolon = Крапка з комою
hotkey-leader-backslash = Зворотна скісна риска
hotkey-leader-grave = Зворотний апостроф
hotkey-shortcut-leader = { $leader }, потім { $digit }
hotkey-hint-plain = Натисніть 1 … 0
hotkey-hint-modifier = Натисніть { $modifier }1 … 0
hotkey-hint-leader = Натисніть { $leader }, потім 1 … 0
hotkey-hint-leader-armed = Лідер активний — натисніть 1 … 0
hotkey-hint-empty-slot = Немає сцени на позиції { $slot }

## scene_card.rs
scene-card-tooltip = { $status } ({ $role })
scene-card-tooltip-with-hotkey = { $status } ({ $role }) · { $hotkey }
scene-card-role-suffix = { $role } сцена
scene-card-tooltip-ready = Перемкнутися на цю сцену

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
status-bar-dropped-placeholder = Втрачено —

## settings.rs

settings-page-title = Налаштування
settings-appearance-title = Вигляд
settings-appearance-description = Додатки GNOME мають за замовчуванням слідувати системному стилю.
settings-theme-mode-system = Системна
settings-theme-mode-light = Світла
settings-theme-mode-dark = Темна
settings-color-scheme-title = Кольорова схема
settings-color-scheme-subtitle = Слідувати системним налаштуванням або примусово встановити світлу / темну
settings-motion-title = Рух
settings-motion-subtitle = Наскільки анімований інтерфейс. Індикатори наживо лишаються розбірливими на будь-якому рівні.
settings-motion-full = Повний
settings-motion-reduced = Обмежений
settings-motion-off = Вимкнено
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
language-system-default = Типова мова системи
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
settings-hotkeys-title = Гарячі клавіші сцен
settings-hotkeys-description = Перемикайте сцени на сторінці «Наживо» з клавіатури. Номери позицій відповідають порядку сцен, заданому в Інвентарі.
settings-hotkeys-enabled-title = Увімкнути гарячі клавіші сцен
settings-hotkeys-enabled-subtitle = Цифрові клавіші перемикають картки сцен на сторінці «Наживо» в порядку карток.
settings-hotkeys-style-title = Комбінація клавіш
settings-hotkeys-style-subtitle = Які клавіші перемикають сцену. Самі цифри не спрацьовують, поки фокус у текстовому полі.
settings-hotkeys-leader-title = Клавіша-лідер
settings-hotkeys-leader-subtitle = Перша клавіша двокрокового скорочення, у стилі vim.
settings-hotkeys-timeout-title = Час очікування лідера
settings-hotkeys-timeout-subtitle = Скільки лідер чекає на цифру, у мілісекундах.
settings-hotkeys-preview-title = Поточні призначення
settings-hotkeys-preview-subtitle = { $first } … { $last } перемикають перші { $count } сцен на сторінці «Наживо».
settings-hotkeys-preview-disabled = Гарячі клавіші сцен вимкнено.
settings-obs-not-connected = Немає підключення до OBS.
settings-obs-connecting = Підключення до OBS…
settings-obs-connected = Підключено — OBS { $version }
settings-obs-error = Помилка: { $err }
settings-theme-loaded = Завантажено { $theme } ({ $variant }).
settings-theme-loaded-with-warnings = Тему завантажено з попередженнями.
theme-variant-light = Світла
theme-variant-dark = Темна

## theme.rs

theme-adwaita-default-name = Типова Adwaita
theme-adwaita-default-desc = Нейтральний стиль, що відповідає типовим налаштуванням GNOME.
theme-scenedeck-dark-name = SceneDeck темна
theme-scenedeck-dark-desc = Стримана темна консольна тема для роботи в прямому ефірі.
theme-scenedeck-light-name = SceneDeck світла
theme-scenedeck-light-desc = Чітка світла консольна тема зі стриманим контрастом.
theme-obs-name = OBS
theme-obs-desc = Типовий вигляд OBS Studio, відтворений засобами libadwaita.
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
## stats.rs — live streaming telemetry
stats-page-title = Статистика трансляції
stats-page-subtitle = Телеметрія в реальному часі, яку SceneDeck запитує в OBS щосекунди, поки є з'єднання.
stats-gauge-fps = Кадри/с
stats-gauge-frame-time = Час кадру (мс)
stats-gauge-dropped = Втрачені кадри
stats-gauge-congestion = Перевантаження
stats-chart-fps = Кадрів на секунду
stats-chart-frame-time = Середній час рендерингу кадру (мс)
stats-chart-output-skipped = Пропущені вихідні кадри на вибірку
stats-chart-render-skipped = Втрачені кадри рендерингу на вибірку
stats-card-render-frames = Втрачені кадри рендерингу
stats-card-output-frames = Пропущені вихідні кадри
stats-card-stream-frames = Втрачені кадри трансляції
stats-card-frame-time = Середній час рендерингу кадру
stats-card-cpu = Використання ЦП OBS
stats-card-memory = Використання пам'яті OBS
stats-card-bitrate = Бітрейт трансляції
stats-value-placeholder = —
stats-value-frames = { $skipped } з { $total } ({ $percent } %)
stats-value-ms = { $value } мс
stats-value-percent = { $value } %
stats-value-mb = { $value } МБ
stats-value-kbps = { $value } кбіт/с

## Help and onboarding
page-help = Допомога
config-parse-failed-backed-up = Не вдалося розібрати налаштування: { $detail } — файл збережено за адресою { $path }, завантажено типові налаштування.
stats-popout-button = Відкрити статистику в окремому вікні

help-page-title = Допомога й початок роботи
help-hero-title = Ласкаво просимо до SceneDeck
help-hero-description = Нативна панель керування OBS Studio для Linux. Цей посібник допоможе вперше підключитися, покаже, як залишити на сторінці «Наживо» лише ті сцени, між якими ви справді перемикаєтеся, та пояснить кожну сторінку бічної панелі. Розгорніть тему, щоб прочитати її.
help-expand-hint = Натисніть тему, щоб розгорнути її.

help-open-settings = Відкрити Налаштування
help-open-inventory = Відкрити Інвентар
help-open-doctor = Відкрити Діагностику
help-open-live = Відкрити Наживо
help-open-mixer = Відкрити Мікшер
help-open-graph = Відкрити Граф
help-open-stats = Відкрити Статистику

help-group-start-title = Початок роботи
help-group-start-description = Найкоротший шлях від свіжого встановлення до перемикання сцен.

help-quickstart-title = П'ять кроків до першого перемикання сцени
help-quickstart-subtitle = Уперше виконайте їх по черзі
help-quickstart-body =
    1. В OBS Studio відкрийте Інструменти → Налаштування сервера WebSocket і позначте «Увімкнути сервер WebSocket». Не закривайте OBS.
    2. У тому самому вікні OBS натисніть «Показати відомості про підключення» й занотуйте Порт сервера (типово 4455) та Пароль сервера.
    3. У SceneDeck відкрийте Налаштування й заповніть поля Хост, Порт і Пароль. Якщо OBS працює на цьому самому комп'ютері, для Хоста залиште 127.0.0.1.
    4. Натисніть Підключитися внизу бічної панелі. Рядок стану над кнопкою стане зеленим і покаже «Підключено».
    5. Відкрийте Інвентар і призначте роль «Основна» сценам, між якими хочете перемикатися під час ефіру. Саме вони — і лише вони — стануть картками на сторінці «Наживо».

help-concepts-title = Як SceneDeck розуміє ваше налаштування
help-concepts-subtitle = Ролі, реєстр і те, чого SceneDeck ніколи не змінює в OBS
help-concepts-body =
    Збережені підключення мають окремі облікові дані. Кожне підключення, адреса й колекція сцен має власний робочий простір шоу.
    Ролі, теги, обране, порядок, кольори, значки й іменовані набори — локальні налаштування; вони не змінюють сцени OBS. Пошук і збережені подання не перепризначають прив’язані дії.
    Відомі UUID сцен зберігають метадані після перейменування. Заміна з іншим UUID не успадковує цю ідентичність.

help-group-connect-title = Підключення до OBS
help-group-connect-description = На цьому комп'ютері або через усю кімнату.

help-connect-local-title = Підключення до OBS на цьому комп'ютері
help-connect-local-subtitle = Типовий випадок — хост 127.0.0.1, порт 4455
help-connect-local-body =
    127.0.0.1 — це адреса, яку комп'ютер використовує для зв'язку із самим собою, тож вона є правильним Хостом, коли OBS і SceneDeck працюють поруч.
    Порт має збігатися з Портом сервера в Налаштуваннях сервера WebSocket OBS. OBS використовує 4455, якщо ви його не змінили.
    Якщо в OBS позначено «Увімкнути автентифікацію», вставте пароль у Налаштування → Пароль. SceneDeck зберігає його у в'язці ключів робочого столу (там само, де браузер зберігає облікові дані), а не у звичайному текстовому файлі конфігурації.
    Натисніть Підключитися на бічній панелі або будь-коли натисніть Ctrl+R, щоб підключитися знову.

help-connect-remote-title = Підключення до OBS на іншому комп'ютері
help-connect-remote-subtitle = Комп'ютер для трансляції в кутку, керування з ноутбука
help-connect-remote-body =
    Це налаштування з двома комп'ютерами: OBS працює на машині, що захоплює й кодує відео, а SceneDeck — на комп'ютері перед вами. Обидва мають бути в одній мережі.
    На комп'ютері з OBS у меню Інструменти → Налаштування сервера WebSocket: позначте «Увімкнути сервер WebSocket», позначте «Увімкнути автентифікацію» й задайте пароль, який зможете ввести. Не вимикайте автентифікацію — інакше будь-хто в мережі з доступом до порту зможе почати або зупинити вашу трансляцію.
    Дізнайтеся адресу комп'ютера з OBS на ньому самому. У Linux виконайте `ip addr` і знайдіть адресу на кшталт 192.168.1.42; у Windows виконайте `ipconfig` і знайдіть IPv4-адресу; у macOS вона міститься в Системні параметри → Мережа. Це адреса комп'ютера, а не програми OBS.
    Дозвольте порт у брандмауері комп'ютера з OBS. У Linux з ufw виконайте `sudo ufw allow 4455/tcp`; у Windows дозвольте OBS у Брандмауері Windows Defender для приватних мереж.
    У Налаштуваннях SceneDeck введіть цю адресу в поле Хост (наприклад, 192.168.1.42), залиште Порт 4455 і вставте пароль. Натисніть Підключитися.
    Для комп'ютера з OBS краще використовувати дротове з'єднання. Перемикання сцен через Wi-Fi працює, але втрачений пакет означає запізніле перемикання.
    Порада, що може врятувати ефір: зарезервуйте постійну адресу комп'ютера з OBS у налаштуваннях DHCP маршрутизатора, щоб збережений Хост працював і після перезапуску.

help-connect-share-title = Як дозволити співведучому або модератору керувати панеллю
help-connect-share-subtitle = Другий SceneDeck, підключений до тієї самої системи
help-connect-share-body =
    Сервер WebSocket OBS приймає кілька клієнтів одночасно, тому друга людина на іншому комп'ютері може запустити власну копію SceneDeck для тієї самої системи — це налаштування з попередньої теми, виконане двічі.
    Окремих облікових записів немає: той, хто має Хост, Порт і пароль у Налаштуваннях, отримує ті самі права, що й ви, включно з початком і зупинкою трансляції. Передавайте їх лише людині, якій довірили б свою сесію OBS.
    Спочатку обмежте її подання. Ролі й приховані сцени зберігаються в локальному Інвентарі, а не в OBS, тому налаштуйте їх на своєму комп'ютері й попросіть помічника імпортувати або відтворити Реєстр сцен YAML замість того, щоб починати з повного невідфільтрованого списку сцен.
    Не перенаправляйте порт WebSocket в Інтернет заради доступу віддаленого помічника. Підключіть обох до однієї мережі VPN чи Tailscale або створіть тунель через SSH і вкажіть адресу тунелю як Хост.
    Усі підключені бачать однаковий стан наживо — перемикання сцени або рух фейдера з будь-якого боку одразу відображається для обох, ніби ви сидите за однією клавіатурою.

help-connect-trouble-title = Якщо кнопка Підключитися не працює
help-connect-trouble-subtitle = Прочитайте помилку, а потім послідовно перевірте цей список
help-connect-trouble-body =
    «У підключенні відмовлено» майже завжди означає, що сервер WebSocket не ввімкнено в OBS або порт не збігається. Перевірте обидва параметри в Інструменти → Налаштування сервера WebSocket.
    Якщо підключення зависає, а потім перевищує час очікування, зазвичай брандмауер відкидає трафік або адреса Хоста належить іншому комп'ютеру.
    «Помилка автентифікації» означає, що пароль неправильний. Введіть його знову в Налаштуваннях; поле призначене лише для запису, тому виглядає порожнім навіть зі збереженим паролем.
    На бічній панелі взагалі немає стану? Переконайтеся, що OBS справді працює й не очікує закриття власного модального вікна.
    SceneDeck автоматично підключається знову після розриву зв'язку, а Ctrl+R негайно запускає нову спробу.

help-group-scenes-title = Упорядкування сцен
help-group-scenes-description = Сторінка «Наживо» має показувати лише сцени, на які ви перемикаєтеся.

help-scenes-hide-title = Приховування сцен, на які ви ніколи не перемикаєтеся
help-scenes-hide-subtitle = Найкорисніше початкове налаштування
help-scenes-hide-body =
    Робоче налаштування OBS накопичує сцени, які існують лише для вкладення в інші сцени, або були створені для одного тесту й не видалені. Якщо розмістити їх на панелі керування прямим ефіром, легко перемкнутися не туди.
    SceneDeck показує картку «Наживо» лише для сцен із роллю Основна. Усі інші ролі приховано зі сторінки «Наживо», тож «приховати сцену» означає просто призначити їй будь-яку роль, крім Основної.
    Відкрийте Інвентар. Кожна сцена OBS має рядок із засобом вибору ролі праворуч.
    Призначте роль Основна кільком сценам, на які справді перемикаєтеся в ефірі. Усім іншим призначте одну з ролей: Другорядна (справжня сцена, яка іноді потрібна, але не має бути на сторінці «Наживо»), Модуль (накладка або нижня третина, яку завжди вкладають в іншу сцену), Необроблена (камера або оболонка джерела захоплення), Налагодження (тестова сцена) чи Архів (збережена на потім і прибрана з дороги).
    Сцени без призначеної ролі також не показуються на сторінці «Наживо», тому роль Не призначено теж приховує сцену. Проте краще призначати роль свідомо: сторінка «Діагностика» позначає непризначені сцени, щоб ви помічали нові.
    Зміна набуває чинності одразу — поверніться на сторінку «Наживо», і картка зникне. В OBS нічого не змінилося.

help-scenes-order-title = Порядок, кольори та піктограми
help-scenes-order-subtitle = Нехай потрібна картка одразу впадає в око
help-scenes-order-body =
    Перетягніть сцену за ручку ліворуч у її рядку Інвентарю, щоб установити порядок. Картки «Наживо» та цифрові скорочення відповідають цьому порядку, тож картка для клавіші 1 буде вгорі.
    Засіб вибору акцентного кольору забарвлює картку сцени «Наживо». Залиште насичений колір для сцен із важливими наслідками — сцени «ми в ефірі» або рекламної інтеграції — щоб погляд одразу знаходив їх.
    Засіб вибору піктограми ліворуч у кожному рядку додає символ на картку сцени «Наживо». Доступно тридцять піктограм і пункт «без піктограми», що прибирає її.
    Порядок, кольори та піктограми зберігаються в локальному реєстрі, а не в OBS.

help-scenes-registry-title = Резервне копіювання та перенесення налаштування
help-scenes-registry-subtitle = Рядок Реєстр сцен YAML в Інвентарі
help-scenes-registry-body =
    Інвентар експортує реєстр поточного простору в YAML, зокрема набори, дії, правила перевірки й аудіопресети. Імпорт замінює налаштування лише цього простору, а не інших просторів чи облікових даних OBS.
    Імпортовані прив’язки та швидкі дії спочатку вимкнені. Перевірте ідентичності перед увімкненням. UUID відстежують перейменування; та сама назва з іншим UUID означає іншу ціль. Застарілі записи можна видалити в Інвентарі.

help-group-operate-title = Проведення ефіру
help-group-operate-description = Сторінки, якими ви користуєтеся під час трансляції.

help-live-title = Сторінка «Наживо»
help-live-subtitle = Стіна сцен, Studio, аудіо та виходи
help-live-body =
    Live — це стіна сцен. Мініатюри сцен у Налаштуваннях додають затемнені зображення та більші картки; після вимкнення повертаються компактні плитки.
    У прямому режимі картки та клавіатурні скорочення перемикають Ефір. Перемикач Studio у верхній панелі синхронізується з OBS: картки вибирають Перегляд, а Перехід надсилає його в Ефір. Вибір переходу й підтримувана тривалість з’являються лише в режимі Studio.
    Розгорніть аудіосмугу під сценами для додаткових регуляторів або відкрийте Мікшер. Трансляція, запис, буфер повтору та віртуальна камера мають спільну панель виходів.
    Джерела у верхній панелі відкривають видимість та детальні налаштування. Перегляд сцени там ніколи не перемикає Ефір.
    Після з’єднання немає карток? Призначте потрібним сценам роль Основна в Інвентарі.

help-hotkeys-title = Перемикання сцен із клавіатури
help-hotkeys-subtitle = Типово Ctrl+1 … Ctrl+0, із можливістю налаштування
help-hotkeys-body =
    Перші десять доступних для перемикання сцен у збереженому порядку Інвентарю отримують цифри 1–9, потім 0. Кожна видима картка показує призначену цифру; підказка сцени вказує налаштовану комбінацію.
    Пошук, збережені набори й показ обраних першими не змінюють номери. Комбінація може активувати приховану фільтром сцену. Зміна порядку в Інвентарі без фільтрів змінює призначення.
    Налаштування → Гарячі клавіші сцен визначають спосіб натискання цифри. Модифікатор із цифрою (типово Ctrl) не спрацює випадково під час введення тексту. Сама цифра — найшвидший варіант. Стиль із лідером працює як у vim: натисніть клавішу-лідер, відпустіть її, а потім натисніть цифру; під час очікування підпис показує «Лідер активний».
    Комбінації сцен працюють лише на сторінці Наживо й не діють, коли фокус у текстовому полі. Цифра без призначеної сцени повідомляє про порожній слот і нічого не перемикає.

help-audio-title = Звук: сторінка «Мікшер» та індикатори
help-audio-subtitle = Що показують кольорові смуги
help-audio-body =
    Компактна аудіосмуга Live розгортається для швидких змін, а Мікшер надає повні фейдери. Спочатку показано глобальні аудіопристрої OBS, потім джерела поточної сцени, зокрема вкладені сцени та групи.
    Режими Мікшера визначають, звук якої сцени ви переглядаєте. Активний стежить за програмною сценою OBS. Вибраний завантажує одну сцену й залишається на ній. Закріплений утримує вибрану сцену як постійну ціль, поки OBS перемикається далі.
    Індикатор поруч із кожним фейдером має шкалу від -60 дБ унизу до 0 дБ угорі й використовує пороги OBS: зелений нижче -20 дБ для музики та фону, жовтий від -20 до -9 дБ для мовлення, червоний вище -9 дБ, де починається кліпінг. Жоден сигнал не має постійно залишатися в червоній зоні.
    Один стовпчик означає монофонічне джерело; два — стерео, спочатку лівий, потім правий канал. Якщо рухається лише лівий стовпчик, половина глядачів нічого не почує з цього джерела.
    Лінія над заповненням показує найвищий пік за останні двадцять секунд — так найшвидше можна помітити пропущений кліпінг. Квадрат унизу показує рівень, що надходить із пристрою до фейдера; якщо він завеликий, рух фейдера цього не виправить.
    Кнопка блокування на картці лише заморожує повзунок SceneDeck. Вона нічого не блокує в OBS.

help-outputs-title = Трансляція, запис, повтор і віртуальна камера
help-outputs-subtitle = І підтвердження, що запобігають випадковим діям
help-outputs-body =
    Панель виходів у Live містить трансляцію, запис, буфер повтору та віртуальну камеру. Інші елементи керування виходами відкривають паузу/відновлення запису, поділ файлу/розділи та останні результати.
    Налаштування → Безпека виходів керує підтвердженнями запуску та зупинки трансляції й запису. За замовчуванням зупинку потрібно підтвердити.
    Події та оновлені спостереження OBS визначають показаний стан. Саме підтвердження запиту не доводить, що вихід запущено або файл збережено. Шляхи належать комп’ютеру з OBS.

help-group-inspect-title = Перевірка налаштування
help-group-inspect-description = Знайдіть несподіванку, перш ніж вона потрапить в ефір.

help-doctor-title = Діагностика
help-doctor-subtitle = Структурні проблеми за рівнем серйозності
help-doctor-body =
    Діагностика читає список сцен, призначені ролі та вкладення між сценами, а потім показує підозрілі місця як Помилки, Попередження та Інформацію.
    Типові знахідки: сцени без призначеної ролі, збережені сцени, яких більше немає в OBS, циклічні вкладення та сцена, вкладена в іншу в напрямку, забороненому правилами ролей.
    Перевірка запускається щоразу, коли ви відкриваєте сторінку, а кнопка Запустити знову примусово повторює її.
    Варто перевіряти після кожної зміни налаштування OBS і ще раз перед виходом в ефір.

help-graph-title = Граф
help-graph-subtitle = Які сцени вкладено в інші
help-graph-body =
    Вкладення однієї сцени в іншу дає змогу створювати в OBS накладки та спільні макети, але через нього сцена також може залежати від чогось забутого.
    Граф показує кожну батьківську сцену та її вміст і оцінює кожен зв'язок за правилами ролей: правильний, сумнівний або заборонений.
    Скористайтеся ним, щоб перед зміною відповісти на запитання «що зламається, якщо я зміню цю сцену?».

help-stats-title = Статистика
help-stats-subtitle = Чи встигає комп'ютер
help-stats-body =
    Індикатори FPS, часу рендерингу кадру, втрачених кадрів і перевантаження мережі стають бурштиновими, а потім червоними з погіршенням значень — попередження про втрачені кадри з'являється на 1%, про перевантаження — на 30%.
    Графіки тенденцій зберігають приблизно останні дві хвилини, а стовпчасті діаграми показують момент втрати кадрів замість накопичувального підсумку. Так можна зрозуміти, чи було зависання одноразовим, чи стало тенденцією.
    Зразки збираються весь час, поки є з'єднання, тому відкриття Статистики посеред трансляції покаже попередні хвилини, а не порожній графік.
    Лічильники кадрів надходять з OBS і скидаються після перезапуску OBS або виходу трансляції.

help-group-personalise-title = Налаштування під себе
help-group-personalise-description = Вигляд, мова та місце зберігання файлів.

help-appearance-title = Теми та вигляд
help-appearance-subtitle = Зокрема вигляд, що пасує до самого OBS
help-appearance-body =
    Колірна схема типово відповідає світлим або темним налаштуванням робочого столу, але можна примусово вибрати один режим.
    Теми — це родини зі світлим і темним варіантами: виберіть родину, і буде застосовано варіант для поточної колірної схеми. Родина OBS відтворює вигляд OBS Studio, тож панель керування не суперечить програмі, якою керує.
    Власний CSS використовує окремі файли для світлого й темного режимів, тому власний вигляд також відповідає колірній схемі. Кнопка Перезавантажити власний CSS застосовує зміни без перезапуску.
    Параметр Рух визначає інтенсивність анімацій інтерфейсу — виберіть Зменшений або Вимкнений, якщо рух відволікає чи комп'ютер перевантажений.

help-files-title = Де SceneDeck зберігає дані
help-files-subtitle = Конфігурація, реєстр і пароль OBS
help-files-body =
    config.json зберігає налаштування й підключення в $XDG_CONFIG_HOME/scenedeck (зазвичай ~/.config/scenedeck). show-workspaces.json зберігає простори; старий registry.json залишається й переноситься одноразово.
    bookmarks.json зберігає нотатки. Необов’язковий session-history.json міститься в $XDG_STATE_HOME/scenedeck (зазвичай ~/.local/state/scenedeck).
    Паролі залишаються в Secret Service окремо для кожного підключення. Експорт JSON/YAML не містить облікових даних. Копіюйте потрібні файли й переносьте паролі окремо.

help-shortcuts-title = Клавіатурні скорочення
help-shortcuts-subtitle = Повний список
help-shortcuts-body =
    F1 — відкрити цей посібник.
    Ctrl+R — повторно підключитися до OBS.
    Ctrl+, — відкрити Налаштування.
    Ctrl+Q — завершити роботу SceneDeck.
    Ctrl+1 … Ctrl+0 на сторінці Наживо — активувати сцену відповідного слота Інвентарю, навіть якщо її приховано фільтром. Комбінацію можна змінити в Налаштуваннях → Гарячі клавіші сцен.
    { palette-title }

welcome-dialog-heading = Ласкаво просимо до SceneDeck
welcome-dialog-body = Схоже, це перший запуск. Сторінка «Допомога» пояснює підключення до OBS — зокрема OBS на іншому комп'ютері — і показує, як залишити на сторінці «Наживо» лише сцени, між якими ви справді перемикаєтеся. Це займе кілька хвилин і допоможе уникнути помилок.
welcome-dialog-later = Не зараз
welcome-dialog-open = Прочитати посібник
window-help-tooltip = Допомога й посібник із початку роботи

# OBS controls and accessible alternatives
window-stop-disconnect-label = Зупинити трансляцію/запис і від’єднатися
window-stop-stream-disconnect-label = Зупинити трансляцію і від’єднатися
window-stop-record-disconnect-label = Зупинити запис і від’єднатися
disconnect-confirm-heading = Зупинити трансляцію та запис перед від’єднанням?
disconnect-confirm-body = Трансляція: { $stream }. Запис: { $recording }. Активні трансляцію та запис буде зупинено перед від’єднанням SceneDeck. Ця дія не зупиняє буфер повторів і віртуальну камеру.
live-scenes-obs-empty = У цій колекції OBS немає сцен. Створіть сцену в OBS, а потім оновіть SceneDeck.
live-organize-scenes = Упорядкувати сцени
mixer-secondary-options = Групування та пошук
inventory-move-up = Перемістити { $scene } вгору
inventory-move-down = Перемістити { $scene } вниз
graph-relationship-list = Зв’язки сцен — текстовий список
graph-no-dependencies = Немає залежностей від вкладених сцен
audio-source-mute = Вимкнути звук { $source }
audio-source-lock = Заблокувати локальний повзунок { $source }
audio-source-volume = Гучність { $source }
audio-source-meter = Індикатор пікового рівня звуку { $source }
audio-meter-no-level = Немає видимого рівня
audio-meter-no-observation = Немає вимірювання рівня звуку

# Session activity

workflow-activity-title = Останні дії
workflow-activity-detail = Лише поточний сеанс, останні 30 записів. Запити, спостереження та помилки розділено. Від’єднання залишають прогалини; збіг у часі не доводить причинного зв’язку.
workflow-check-item = Видимість джерела
workflow-activity-requested = Запитано
workflow-activity-observed = Спостережено
workflow-activity-failed = Помилка
workflow-activity-connection = З’єднання


# Native OBS inspector
control-refresh = Оновити спостереження OBS
control-inspection-note = Переглядайте будь-яку сцену без зміни Програми. Зміни видимості впливають на сцену або групу, що містить елемент.
control-disconnected = Під’єднайтеся до OBS, щоб переглядати джерела.
control-loading = Зчитування стану OBS…
control-observed = Останнє спостереження: { $seconds } с тому. Оновіть дані, перш ніж покладатися на цей стан.
control-scene-selector = Сцена для перегляду — не змінює Програму
control-request-hint = Оновіть дані, щоб завантажити власні засоби керування OBS і видимість джерел.
control-resume = Продовжити запис
control-pause = Призупинити запис
control-split = Розділити файл запису
control-replay-stop = Зупинити буфер повторів
control-replay-start = Запустити буфер повторів
control-replay-save = Зберегти повтор
control-camera-stop = Зупинити віртуальну камеру
control-camera-start = Запустити віртуальну камеру
control-chapter-name = Назва розділу запису
control-chapter = Створити розділ
control-host-path = Шлях останнього повтору на комп’ютері OBS: { $path }
control-output-controls = Запис, повтори та віртуальна камера
control-source-search = Пошук джерел або шляхів контейнерів
control-inspect-source = Переглянути налаштування й використання джерела
control-hidden-parent = Батьківський елемент прихований. Увімкнення лише цього елемента може не зробити його видимим.
control-sources-empty = У цьому спостереженні немає елементів сцени.
control-shared-source = Звук, фільтри, медіа й текст впливають на це джерело всюди, де воно використовується. Видимість стосується контейнера елемента.
control-used-in = Використовується в — структурні шляхи
control-image-refresh = Зробити знімок
control-image-alt = Знімок сцени OBS, зроблений на запит
control-image-error = Не вдалося декодувати зображення OBS.
control-image-note = Знімок з останнього захоплення. Це не відео наживо.
control-studio-enable = Увімкнути режим студії
control-program = Програма: { $scene }
control-preview = Перегляд: { $scene }
control-stage = Підготувати сцену, що переглядається
control-take = Перехід
control-transition = Поточний перехід сцени
control-duration = Тривалість переходу в мілісекундах
control-apply = Застосувати
control-studio = Студія та перехід
control-scene-mode-changed = Режим студії змінено. Оновіть дані перед активацією сцени. Дію не було повторено.
control-filters = Фільтри джерела
control-text-file = Це текстове джерело читає файл. Відредагуйте файл на комп’ютері OBS.
control-text-save = Зберегти текст в OBS
control-text = Вбудоване текстове джерело
control-media-play = Відтворити
control-media-pause = Пауза
control-media-stop = Зупинити
control-media-restart = Відтворити спочатку
control-media-previous = Попереднє
control-media-next = Наступне
control-media-seconds = Позиція медіа в секундах
control-media-seek = Перейти до позиції
control-media = Керування медіа
control-monitor-host = Моніторинг звуку відтворюється на комп’ютері OBS, а не через SceneDeck.
control-monitor-off = Моніторинг вимкнено
control-monitor-only = Лише моніторинг — вимкнути звук на виході
control-monitor-output = Моніторинг і вихід
control-audio-sync = Зсув синхронізації звуку (мілісекунди)
control-audio-balance = Баланс (0 = ліворуч, 0,5 = центр, 1 = праворуч)
control-audio-track = Звукова доріжка { $track }
control-audio-tracks = Маршрутизація звукових доріжок
control-advanced-audio = Розширені налаштування звуку джерела


# Operating workspace
workspace-action-acknowledged = Запит підтверджено; стан оновлено з OBS
workspace-replay-observed = OBS повідомив про збережений повтор: { $path }
workspace-disk-space = Вільний простір для запису OBS: { $value } МіБ
control-stale = Останній відомий стан. Оновіть спостереження OBS, щоб увімкнути зміни.
control-no-matches = Жодне джерело не відповідає пошуку. Очистьте пошук, щоб показати всі елементи.


# OBS recording volume and session activity
stats-recording-disk = Вільний простір на диску запису OBS
stats-recording-disk-value = { $value } МіБ
stats-activity-title = Останні дії та контекст графіків
stats-activity-detail = Останні 15 записів сеансу. Виберіть запис, щоб позначити його час на графіках. Розриви з’єднання, відсутні вимірювання та скидання лічильників обмежують тлумачення. Збіг у часі не доводить причинного зв’язку.
stats-activity-clear-marker = Прибрати позначку з графіка
stats-activity-marker-visible = Пунктирні лінії позначають вибрану подію між вимірюваннями. Графіки використовують інтервали між вимірюваннями.
stats-activity-marker-unavailable = Немає позначки: подія поза збереженими вимірюваннями, у прогалині вимірювань або на неї вплинула зміна годинника.
stats-activity-empty = У цьому сеансі ще не спостерігалося жодних операцій.
stats-activity-requested = Запитано
stats-activity-observed = Спостережено
stats-activity-failed = Помилка
stats-activity-connection = З’єднання
stats-activity-entry = { $time } · { $origin } · { $description }
scene-card-stage-hint = Підготуйте цю сцену в Перегляді. Виконайте перехід, щоб змінити Програму.
scene-card-preview-label = Перегляд
hotkey-hint-stage-take = { $binding } · Підготувати в Перегляді; виконайте перехід, щоб змінити Програму.


header-obs-setup = Налаштування OBS
header-obs-unknown = Невідомо
header-obs-context = Профіль: { $profile } · Колекція сцен: { $collection }

status-bar-connection-detail = { $state } · OBS { $version }


doctor-structural-title = Структура та ролі сцен


control-image-caption = Знімок сцени { $scene }, зроблений { $seconds } с тому. Оновіть зображення, щоб зробити новий знімок.

output-results-acknowledged = Підтверджено

output-results-copy-path = Копіювати шлях на комп’ютері OBS

output-results-host-path = Шлях на комп’ютері OBS: { $path }

output-results-record-stop = Зупинити запис

output-results-unconfirmed = Не підтверджено

output-results-title = Результати виходів

output-results-detail = Запити та спостереження OBS розділено. Подія збереження може походити від іншого контролера. Шляхи стосуються комп’ютера OBS. У цьому сеансі зберігаються останні 24 результати.

record-elapsed-tooltip = Час від повідомленого OBS початку запису, включно з паузами. Це не тривалість записаного файлу.

window-open-navigation = Відкрити навігацію
sources-title = Джерела
sources-back = Назад до джерел
sources-select-hint = Виберіть джерело, щоб переглянути його налаштування.
sources-text-conflict = Цей текст змінено в OBS. Скасуйте редагування, щоб завантажити поточний текст перед збереженням.
inventory-name-column = Сцена
inventory-role-column = Роль
inventory-actions-column = Параметри
inventory-scene-options = Параметри сцени { $scene }
inventory-columns-button = Ширина стовпців
inventory-name-width = Ширина сцени
inventory-role-width = Ширина ролі
inventory-reset-widths = Скинути ширину стовпців
settings-scene-previews = Мініатюри сцен
settings-scene-previews-description = Показувати затемнені мініатюри на більших картках Live.
live-audio-expand = Розгорнути аудіо
live-audio-collapse = Згорнути аудіо
studio-mode-label = Studio
scene-card-image-pending = Мініатюра недоступна
scene-card-program-label = Ефір
output-options = Інші елементи керування виходами
output-replay-label = Replay · { $state }
output-camera-label = Віртуальна камера · { $state }

output-camera-name = Вірт. камера
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
media-state-none = Немає медіа
media-state-playing = Відтворення
media-state-opening = Відкриття
media-state-buffering = Буферизація
media-state-paused = Призупинено
media-state-stopped = Зупинено
media-state-ended = Завершено
media-state-error = Помилка
media-state-unknown = Невідомо
workspace-warning-identity = Не вдалося визначити входи: { $detail }
workspace-warning-truncated = Список джерел скорочено: вичерпано ліміт перевірки
workspace-warning-container = { $path }: { $detail }
workspace-warning-visibility = Видимість { $owner } / { $item } недоступна: { $detail }
workspace-warning-recursion = Сканування вкладених сцен зупинено на { $path }
workspace-warning-usage-partial = Пошук використань неповний: обмежено { $scenes } сценами та { $items } перевіреними елементами
workspace-warning-omitted = { $count ->
    [one] Пропущено ще 1 попередження про джерела
   *[other] Пропущено ще { $count } попереджень про джерела
}
controller-workspace-superseded = Запит робочої області застарів до зміни
controller-workspace-admission-full = Операцію робочої області відхилено: черга заповнена або завершена
controller-workspace-text-changed = Текст змінено в OBS. Оновіть перед збереженням
controller-workspace-action-cancelled = Дію робочої області скасовано до зміни
controller-endpoint-changed = Кінцеву точку OBS змінено; перепідключіться й оновіть
controller-collection-changed = Колекцію OBS змінено; оновіть перед продовженням
controller-collection-changed-during-read = Колекцію OBS змінено під час читання; результати відкинуто
controller-audio-changed = Аудіо змінилося під час спостереження; потрібне оновлення
controller-outputs-still-active = Виходи OBS залишаються активними після зупинки
controller-shutdown-unconfirmed = Від’єднано; зупинку виходів не підтверджено: { $detail }; команди не повторювалися
controller-stop-deadline = перевищено час на коректну зупинку
controller-connection-deadline = Перевищено час підключення до OBS
controller-publication-backlog = Черга сповіщень OBS перевищила ліміт; перепідключіться й оновіть; історія виходів може бути неповною
controller-event-connection-ended = З’єднання подій OBS завершено; потрібне оновлення

# Scene identity changes
registry-scene-rename-conflict = Сцену «{ $old }» перейменовано на «{ $new }», але ця назва вже має локальні метадані. Обидва записи збережено; перевірте Інвентар.
registry-scene-rename-backlog = Під час імпорту надійшло забагато оновлень реєстру. Перевірте Інвентар після завершення імпорту.

# Collection-specific curation
workspace-curation-title = Робочий простір шоу
workspace-curation-loading = Завантаження налаштувань цієї колекції…
workspace-curation-unavailable = Під’єднайтеся до OBS і зачекайте на колекцію сцен.
workspace-curation-failed = Не вдалося завантажити робочий простір: { $error }
workspace-curation-retry = Повторити

# Saved OBS connections
settings-connections-description = Збережіть налаштування для кожного підключення OBS. Вибір не перепідключає OBS; натисніть «Підключити збережене», коли будете готові.
settings-connections-picker = Збережене підключення
settings-connection-name = Назва підключення
settings-connection-add = Додати підключення
settings-connection-connect = Підключити збережене
settings-connection-new-name = Нове підключення { $number }
settings-connection-invalid = Введіть назву, адресу та порт від 1 до 65535.
settings-obs-connected-profile = Підключено до { $name } ({ $endpoint }), OBS { $version }

inventory-retained-identity = { $role } · Збережено після конфлікту назви або ідентичності; не застосовано до сцени.

inventory-scene-favorite = Обране
inventory-scene-tags = Теги (через кому)
inventory-apply-tags = Застосувати теги

deck-search-placeholder = Пошук сцен і тегів
deck-favorites-only = Лише обрані
deck-reset = Показати всі
deck-no-results = Жодна сцена не відповідає цьому поданню. Очистьте фільтри, щоб показати всі сцени.
deck-reorder-hint = Очистьте фільтри, щоб змінити порядок сцен.

deck-menu = Набори
deck-none = Збережених наборів ще немає.
deck-remove = Видалити набір «{ $name }»
deck-save-title = Зберегти подання як набір
deck-save-help = Виберіть унікальну назву. Поточний пошук і фільтр обраного буде збережено для цього робочого простору.
deck-cancel = Скасувати
deck-save = Зберегти
deck-name = Назва набору

## Command palette
palette-title = Палітра команд (Ctrl+K)
palette-search = Пошук сцен і дій…
palette-open-page = Відкрити { $page }
palette-stage-scene = Підготувати { $name }
palette-switch-scene = Перемкнути на { $name }
palette-open-source = Відкрити джерело { $name }
palette-unmute = Увімкнути звук { $name }
palette-restart = Перезапустити { $name }
palette-disable-filter = Вимкнути { $filter } · { $name }
palette-enable-filter = Увімкнути { $filter } · { $name }
palette-scope-changed = Дії OBS у цій палітрі недоступні. Відкрийте її знову після підключення або зміни робочого простору.
palette-empty = Немає відповідних дій.
palette-hint = Виберіть дію клавішами ↑ / ↓, Enter — виконати, Esc — закрити. Збережені дії з медіа та фільтрами працюють на всіх сторінках.
palette-unavailable = Недоступно: перевірте з’єднання, оновіть стан OBS або перевірте джерело дії.

## Custom action shortcuts
binding-title = Комбінації клавіш для дій
binding-description = Прив’язки належать цьому робочому простору й діють на всіх сторінках, крім введення тексту. Конфліктні прив’язки неактивні. Імпортовані прив’язки спочатку вимкнені.
binding-manage = Керувати комбінаціями для дій
binding-invalid = Використайте Ctrl, Alt або Super із символом чи функціональну клавішу F2–F24.
binding-reserved = Ця комбінація зарезервована застосунком.
binding-slots-conflict = Ця комбінація конфліктує з налаштуваннями позицій сцен.
binding-duplicate = Інша ввімкнена дія вже використовує цю комбінацію.
binding-empty = Комбінацій для дій ще немає.
binding-enabled = Увімкнено
binding-edit = Редагувати комбінацію
binding-remove = Видалити комбінацію
binding-add = Додати комбінацію
binding-record-help = Оберіть дію та запишіть комбінацію клавіш. Escape скасовує запис. Спільні входи впливають на всі сцени, що їх використовують.
binding-action = Дія
binding-record = Записати комбінацію
binding-recording = Натисніть комбінацію клавіш…
binding-unavailable = Дія недоступна. Стан OBS оновлюється; перевірте його й натисніть комбінацію знову.

action-inspect-input = Перевірити медіа та фільтри джерела
action-inspect-none = Виберіть джерело…
action-inspect-help = Доступні дії з’являться нижче. Вибір на сторінці «Джерела» збережеться.

quick-title = Швидкі дії
quick-manage = Налаштувати швидкі дії
quick-add = Закріпити дію
quick-edit = Змінити дію
quick-remove = Видалити закріплену дію
quick-up = Перемістити дію вгору
quick-down = Перемістити дію вниз
quick-empty = Закріпіть часто вживані елементи керування поруч зі сценами.
quick-description = Закріплені дії належать цьому робочому простору. Зміни спільних джерел впливають на всі сцени, що їх використовують.
quick-disabled = Вимкнено — перевірте й увімкніть у налаштуванні швидких дій.
quick-observed = Спостережений стан: { $state }
quick-ready = Готово
quick-muted = Звук вимкнено
quick-unmuted = Звук увімкнено
quick-filter-enabled = Фільтр увімкнено
quick-filter-disabled = Фільтр вимкнено

quick-toggle-mute = Перемкнути звук · { $name }
quick-toggle-filter = Перемкнути фільтр · { $name } / { $filter }

# Readiness review
preflight-title = Перевірка перед початком
preflight-description = Перевірте готовність перед початком. Говоріть у вибраний мікрофон: сигнал після фейдера вище −50 dBFS запам’ятовується на 10 секунд, доки надходять свіжі рівні. Недоступні або застарілі показники залишаються невідомими.
preflight-close = Закрити
preflight-passed = Пройдено
preflight-failed = Потребує уваги
preflight-unknown = Невідомо
preflight-skipped = Пропущено
preflight-refresh = Оновити спостереження OBS
preflight-reset = Скинути контрольний список
preflight-ready = Готово: усі налаштовані перевірки пройдено.
preflight-attention = Перегляньте перевірки нижче.
preflight-collection-result = Колекція: { $status } — очікувана «{ $expected }», спостережена «{ $actual }»
preflight-scene-result = Сцена «{ $name }»: { $status }
preflight-no-scenes = Обов’язкові сцени: не налаштовано.
preflight-mic-result = Мікрофон «{ $name }»: наявність { $presence }; звук увімкнено { $unmuted }; недавній сигнал { $signal }
preflight-disk-result = Місце для запису (мінімум { $minimum } MiB): { $status }

preflight-configure = Налаштувати перевірку
preflight-expected = Очікувана колекція сцен
preflight-targets-description = Зміна очікування скидає вибір сцен і мікрофона. Доступні варіанти походять зі спостереженої колекції; збережені відсутні об’єкти залишаються видимими. Для вибору з іншої колекції перемкніть OBS на неї та знову відкрийте налаштування.
preflight-scenes = Обов’язкові сцени
preflight-check-mic = Перевіряти наявність, вимкнення звуку та сигнал мікрофона
preflight-no-scene-choices = Немає доступних сцен для цього очікування.
preflight-no-microphone = Мікрофон не вибрано
preflight-check-disk = Перевіряти місце для запису
preflight-minimum-disk = Мінімум вільного місця (MiB)
preflight-manual-items = Ручний контрольний список (один унікальний пункт на рядок)
preflight-invalid-plan = Укажіть очікувану колекцію, додатне ціле число для порога місця та унікальні пункти списку.
preflight-saving = Збереження…
preflight-save-failed = Не вдалося зберегти: { $error }. Зміни залишилися тут; спробуйте зберегти знову.
preflight-editor-retired = З’єднання, колекція або збережений план змінилися. Закрийте цей редактор і відкрийте знову.

preflight-continue = Продовжити запуск: { $output }…
preflight-continue-description = Можна продовжити з невирішеними перевірками. Звичайні налаштування підтвердження запуску зберігаються. Нічого не запускається автоматично.
preflight-review-before-output = Показувати перевірку перед початком трансляції або запису

preflight-free-space = Спостережене вільне місце для запису: { $space } MiB

# Sustained live health
health-title = Сповіщення про стан ефіру
health-network = Втрати кадрів у мережі
health-render = Перевантаження рендерингу
health-encoder = Перевантаження кодування
health-disk = Мало місця для запису
health-network-help = OBS втрачає кадри трансляції. Перевірте мережеве з’єднання або зменште бітрейт.
health-render-help = OBS пропускає кадри рендерингу. Зменште складність сцени або параметри відео.
health-encoder-help = OBS пропускає кадри кодування. Зменште навантаження на кодер або параметри запису й трансляції.
health-disk-help = Під час запису на томі для запису залишилося менше 1 GiB. Звільніть місце або завершіть запис, перш ніж воно закінчиться.
health-active = Потребує уваги
health-recovering = Відновлення — останні показники нормальні
health-unobserved = Нові показники недоступні
health-caption = { $title } — { $phase }
health-more = { $summary } · ще { $count }
health-policy = Сповіщення про кадри вимагають щонайменше 1% пропущених кадрів у недавніх інтервалах протягом 5 секунд. Нестача місця також має тривати 5 секунд. Попередження про продуктивність зникають після 10 секунд нормальних показників; відсутні або застарілі дані не означають відновлення.
health-open-stats = Відкрити статистику
health-no-alerts = Немає поточних сповіщень про стан.

health-connection-lost = З’єднання з OBS втрачено
health-stream-stopped = Трансляція зупинилася
health-record-stopped = Запис зупинився
health-connection-help = SceneDeck втратив з’єднання з OBS. Перевірте OBS і мережу та під’єднайтеся знову. Стан виходів не підтверджено, доки спостереження не відновляться.
health-stream-stopped-help = OBS повідомив про зупинку трансляції без недавньої команди зупинки від SceneDeck. Перевірте OBS і призначення перед повторним запуском.
health-record-stopped-help = OBS повідомив про зупинку запису без недавньої команди зупинки від SceneDeck. Перевірте місце для запису й OBS. Це сповіщення не підтверджує збереження файлу.
health-acknowledge = Підтвердити

health-event-policy = Сповіщення про події належать поточному з’єднанню. Спостережене відновлення або нове з’єднання прибирає їх. Підтвердження лише прибирає повідомлення.

# Selected-input audio health
health-audio-silence = { $input }: тиша
health-audio-clipping = { $input }: кліпінг
health-audio-silence-help = Звук вимкнено або рівень після фейдера не перевищує −50 dBFS протягом 15 секунд. Перевірте мікрофон, вимкнення звуку та маршрутизацію. Вимкніть це правило для запланованої тиші.
health-audio-clipping-help = Принаймні один канал до фейдера перебуває на повній шкалі протягом секунди. Перевірте підсилення входу.
health-audio-disabled = Моніторинг вимкнено
health-audio-missing = Вибраний вхід відсутній або неоднозначний
health-audio-monitoring = Моніторинг свіжих показників
health-audio-muted = Моніторинг — звук входу вимкнено
health-audio-settings = Моніторинг звуку
health-audio-select = Зберігати правило для цього входу
health-audio-check-silence = Попереджати про тривалу тишу
health-audio-check-clipping = Попереджати про тривалий кліпінг
health-audio-policy = Виберіть входи, щоб додати правила; зніміть вибір, щоб видалити правило. Увімкніть моніторинг явно. Правила діють під час підключення, навіть до запуску виходів. Тиша: 15 секунд на рівні не вище −50 dBFS після фейдера або з вимкненим звуком. Кліпінг: секунда на повній шкалі до фейдера. Для відновлення потрібно десять секунд нормальних показників. Відсутні чи застарілі показники означають недоступність, а не тишу. Оновіть список входів.
health-audio-invalid = Виберіть хоча б одне попередження для кожного ввімкненого правила та залиште лише одне правило на вхід.
health-audio-saved = Правила збережено.
health-audio-unsaved = Незбережені зміни.
health-audio-retired = Робочий простір або збережені правила змінилися. Закрийте й знову відкрийте редактор.

# Observed media timing
media-progress-label = Перебіг відтворення
media-position-observed = Спостережена позиція: { $time }
media-duration-observed = Тривалість: { $time }
media-time-remaining = Залишилося: { $time }
media-time-unavailable = Недоступно
media-duration-unavailable = Недоступна або невизначена
media-progress-hint = Час відповідає останньому спостереженню OBS. Для переходу використовуйте поле позиції нижче.

# Local timestamp bookmarks
bookmarks-title = Закладки
bookmark-policy = Зберігаються локально. Час сеансу враховує паузи. Час OBS — це нещодавній показник часу виходу, а не позиція у файлі чи розділ.
bookmark-name = Назва закладки
bookmark-add = Додати закладку
bookmark-retry = Повторити
bookmark-saving = Збереження…
bookmark-save-error = Закладки недоступні або не збережені: { $error }
bookmark-scope-changed = Підключіться до OBS або відкрийте це вікно знову після зміни підключення чи колекції. Наявні нотатки залишаються доступними.
bookmark-saved = Локально збережено закладок: { $count }
bookmark-default = Закладка { $number }
bookmark-invalid-name = Введіть назву від 1 до 120 символів без керівних символів.
bookmark-full = Досягнуто межі у 2000 закладок. Видаліть нотатки, щоб додати нові.
bookmark-rename = Перейменувати
bookmark-remove = Видалити
bookmark-record-unknown = Час запису недоступний
bookmark-record-inactive = Запис був неактивним
bookmark-record-time = Час виходу OBS: { $time } · { $state } · Вік показника: { $age } с
bookmark-description = { $created }
    Сеанс { $session }: { $elapsed } від підключення
    { $connection } · { $endpoint }
    Колекція: { $collection } · Спостережена сцена: { $scene }
    { $recording }

# Session reports and explicit export
reports-title = Звіти сеансів
reports-previous = Попередній сеанс
reports-next = Наступний сеанс
reports-refresh = Оновити знімок
reports-export-json = Експортувати JSON
reports-export-csv = Експортувати CSV
reports-empty = Сеансів ще не зафіксовано. Під’єднайтеся до OBS, щоб розпочати звіт.
reports-current = Поточний сеанс
reports-scene = Спостережена сцена: { $scene }
reports-ended = Сеанс спостереження завершено: { $reason }
reports-alert-cleared = Сповіщення прибрано; це не підтверджує відновлення
reports-exporting = Експорт…
reports-exported = Експорт збережено: { $path }
reports-export-error = Помилка експорту: { $error }
reports-policy = Останні вісім сеансів підключення залишаються в пам’яті. Збережена історія також доступна тут. Оновіть знімок, щоб додати нові спостереження або зміни закладок. Експорт зберігає вибраний знімок. Підтвердження не доводять збереження файлу; шляхи належать комп’ютеру OBS.
reports-view-limit = Тут показано останні 150 подій і 50 закладок. Експорт містить усі збережені записи. Пропущені вимірювання та скидання лічильників обмежують тлумачення; лічильники кадрів накопичувальні.
reports-summary = { $start } → { $end }
    { $connection } · { $endpoint }
    Збережені події: { $events } · Пропущено: { $omitted } · Прогалини активності: { $gaps }
    Отримані вимірювання: { $samples } · Збережено: { $kept }
    Найвище виміряне CPU: { $cpu } · Найнижчий виміряний FPS: { $fps }
    Закладки: { $bookmarks }

# Optional session history
history-title = Історія сеансів
history-enable = Автоматично зберігати історію сеансів
history-retain = Кількість збережених сеансів (1–32)
history-apply = Застосувати налаштування історії
history-retry = Перезавантажити й повторити
history-clear = Очистити збережену історію
history-saving = Збереження або завантаження історії…
history-error = Історія недоступна або не збережена: { $error }
history-status = { $mode } · Збережені сеанси: { $count }
history-on = Автоматичну історію ввімкнено
history-off = Автоматичну історію вимкнено
history-settings-changed = Налаштування змінено в іншому вікні. Відкрийте це вікно знову.
history-app-closed = SceneDeck закрито
reports-interrupted = Попередній запуск; час завершення невідомий
history-settings-description = Автоматичне збереження типово вимкнено. Тут можна налаштувати зберігання, перезавантажити історію та повторити невдалі записи.
history-policy = Увімкнення зберігає поточний звіт разом зі збереженими попередніми подіями та майбутні сеанси. Запис відбувається приблизно кожні десять секунд і при зміні сеансу. Вимкнення припиняє нові записи та залишає збережені звіти. Зменшення ліміту видаляє найстаріші збережені сеанси. Закладки зберігаються окремо.
history-clear-confirm = Видалити всю збережену та незбережену історію? Останні звіти в пам’яті та закладки залишаться доступними. Поточне підключення більше не записуватиметься; автоматична історія відновиться з наступного підключення, якщо її ввімкнено.

history-exit-requested = Запит на вихід зі SceneDeck
history-exit-resumed = Вихід скасовано або відкладено; запис звіту відновлено

# Selected-input audio presets
preset-title = Аудіопресети
preset-open = Відкрити
preset-shared = Виберіть входи, щоб зберегти їхні поточні стани вимкнення звуку та підсилення. Зміни входу впливають на всі сцени, які його використовують. Пресети належать цьому підключенню та колекції сцен.
preset-name = Назва пресета
preset-new = Новий пресет
preset-save = Зберегти вибрані входи
preset-replace = Замінити вибраний пресет
preset-rename = Перейменувати вибраний пресет
preset-delete = Видалити вибраний пресет
preset-preview = Перегляд збережених значень
preset-missing = Відсутній або недоступний
preset-invalid = Виберіть 1–64 доступні входи й унікальну назву до 80 символів. Можна зберегти до 64 пресетів.
preset-current = Поточне: { $value }
preset-saved = Збережене: { $value }
preset-none = Виберіть збережений пресет, щоб переглянути його значення.
preset-refresh-needed = Оновіть аудіодані перед збереженням пресета або переглядом змін.
preset-retired = Робочий простір або збережені пресети змінилися. Закрийте цей редактор і відкрийте його знову.
preset-pending = Очікування
preset-running = Перевірка та застосування
preset-observed = Значення підтверджено OBS
preset-unchanged = Уже збігається
preset-changed = Значення змінилися; перегляньте знову
preset-failed = Помилка запиту
preset-unconfirmed = Результат підтверджено не повністю
preset-cancelled = Решту змін скасовано
preset-acknowledgements = Підтвердження OBS: підсилення { $gain }, вимкнення звуку { $mute }
preset-unavailable = Неможливо застосувати: підключення недоступне, інший пресет виконується або перегляд застарів.
preset-review = Переглянути зміни
preset-apply = Застосувати до доступних входів
preset-cancel = Зупинити решту змін
preset-apply-policy = Перегляньте зміни перед застосуванням. Відсутні входи пропускаються; інші можуть завершитися незалежно. Підтвердження запиту не підтверджує прочитані значення. Підсилення перевіряється перед увімкненням звуку. Зупинка запобігає подальшим змінам, але не скасовує завершених. Якщо значення змінилися, оновіть і перегляньте їх знову.
preset-unsupported = Це підключення OBS не оголошує всі запити, потрібні для застосування аудіопресетів.
preset-reviewed = Знімок перегляду зберігається до 30 секунд.

# Desktop portal shortcuts
global-title = Глобальні сполучення клавіш
global-description = Зареєструйте до 64 увімкнених дій для цього підключеного робочого простору. Середовище робочого столу обирає клавіші та може запитати дозвіл. Підтверджені глобальні клавіші замінюють відповідні клавіші програми до зупинки. Реєстрація завершується після від’єднання, зміни простору чи прив’язок або закриття головного вікна; увімкніть її знову, коли будете готові.
global-enable = Увімкнути глобальні сполучення
global-stop = Зупинити глобальні сполучення
global-off = Глобальні сполучення вимкнено.
global-pending = Очікування середовища робочого столу…
global-active = Глобальні сполучення активні. Призначені клавіші:
global-none = Середовище робочого столу не призначило жодного сполучення.
global-unavailable = Не вдалося увімкнути глобальні сполучення. Перевірте, чи середовище робочого столу надає портал GlobalShortcuts, і повторіть спробу.

help-controls-subtitle =
    Прив’язки, швидкі дії та аудіопресети

help-controls-body =
    Ctrl+K шукає сцени й підтримувані дії. Налаштування → Сполучення дій призначає сталі клавіші; швидкі дії на Live закріплюють окремі елементи керування. Спільні входи впливають на всі сцени, що їх використовують.
    Глобальні сполучення вмикаються явно в менеджері та потребують сумісного порталу. Середовище робочого столу обирає клавіші; зміна підключення, простору чи прив’язок або закриття головного вікна завершує реєстрацію.
    Mixer → Аудіопресети зберігає підсилення й вимкнення звуку вибраних входів. Перегляньте зміни та відсутні входи перед застосуванням. Результати розрізняють підтвердження й спостережені значення; можливий частковий успіх. Зупинка скасовує ненадіслані зміни; повторне підключення їх не відтворює.

help-readiness-subtitle =
    Підготовка й постійні попередження під час ефіру

help-readiness-body =
    Передефірна перевірка контролює очікувану колекцію, потрібні сцени, вимкнення звуку/сигнал вибраного мікрофона, місце для запису й ручний список. Вона не запускає виходи. Невідомі або застарілі дані не означають успіху.
    Попередження залишаються видимими між сторінками керування. Вони виявляють тривалі поточні втрати кадрів, перевантаження рендерингу/кодувальника, брак місця, втрату підключення й неочікувані зупинки. Правила тиші/перевантаження звуку потребують явно вибраних входів; індикатори не доводять, що чує аудиторія.

help-session-subtitle =
    Закладки, медіа та звіти сеансів

help-session-body =
    Джерела показують прогрес і залишок часу, коли OBS надає тривалість, а також підтримуване перемотування. Керування виходами має паузу й відновлення запису.
    Закладки зберігають локальний час сеансу й свіжий зразок часу запису OBS. Через паузи та повторні підключення це не точні позиції у файлі чи розділи.
    Звіти зберігають обмежену кількість подій, інцидентів і свідчень виходів з експортом CSV/JSON. Автоматична історія типово вимкнена; увімкніть її й оберіть зберігання в Історії сеансів. Запити та підтвердження відокремлені від спостережених результатів. Шляхи записів належать комп’ютеру OBS.
