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
    SceneDeck ніколи не перейменовує, не видаляє й не змінює порядок нічого в OBS. Програма читає ваші сцени через з'єднання OBS WebSocket і зберігає власні нотатки про них.
    «Роль» — одна з таких нотаток: ваша власна мітка призначення сцени — сцена для виходу в ефір, багаторазова накладка або залишена тестова сцена.
    Ці нотатки містяться у файлі registry.json поруч із файлом конфігурації, тож не зникають після перезапуску; їх можна експортувати й перенести на інший комп'ютер зі сторінки «Інвентар».
    Оскільки нотатки локальні, двоє людей можуть користуватися одним налаштуванням OBS і мати різні сторінки «Наживо».

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
    Експорт записує ролі, порядок, акцентні кольори, піктограми, теги й правила графа в один файл YAML — звичайний текстовий формат, який можна читати й зберігати в системі керування версіями.
    Імпорт замінює локальний реєстр вмістом такого файлу. Використовуйте його, щоб перенести готове налаштування на другий комп'ютер або повернутися до стану перед експериментом.
    Назви сцен пов'язують файл з OBS, тому перейменована в OBS сцена повернеться як застарілий запис. Інвентар показує застарілі записи й дає змогу їх видалити.

help-group-operate-title = Проведення ефіру
help-group-operate-description = Сторінки, якими ви користуєтеся під час трансляції.

help-live-title = Сторінка «Наживо»
help-live-subtitle = Картки сцен, звук і програмна сцена
help-live-body =
    «Наживо» — це операторське подання: поточна програмна сцена вгорі, картки сцен з одного боку й компактні картки звуку з іншого. Перетягніть розділювач, щоб надати більше місця потрібній частині.
    Натискання картки сцени перемикає OBS на цю сцену. Поточну позначено як Активна, решту — як Готові.
    Після підключення немає карток? Жодна сцена ще не має ролі Основна — дивіться вище «Приховування сцен, на які ви ніколи не перемикаєтеся».
    Рядок стану внизу видно на кожній сторінці; він показує стан з'єднання, стан трансляції та запису з тривалістю, а також поточні FPS, втрачені кадри, використання процесора й бітрейт.

help-hotkeys-title = Перемикання сцен із клавіатури
help-hotkeys-subtitle = Типово Ctrl+1 … Ctrl+0, із можливістю налаштування
help-hotkeys-body =
    Кожна з перших десяти карток «Наживо» має маленьку позначку з цифрою: перша картка — 1, дев'ята — 9, десята — 0. Підпис поруч із заголовком «Сцени» завжди показує поточне призначення.
    Номери позицій відповідають порядку в Інвентарі, тому зміна порядку карток також змінює порядок скорочень.
    Налаштування → Гарячі клавіші сцен визначають спосіб натискання цифри. Модифікатор із цифрою (типово Ctrl) не спрацює випадково під час введення тексту. Сама цифра — найшвидший варіант. Стиль із лідером працює як у vim: натисніть клавішу-лідер, відпустіть її, а потім натисніть цифру; під час очікування підпис показує «Лідер активний».
    Скорочення діють лише на сторінці «Наживо», а стилі без модифікатора не працюють, поки фокус у текстовому полі. Якщо цифрі не відповідає сцена, підпис повідомить про це замість перемикання.

help-audio-title = Звук: сторінка «Мікшер» та індикатори
help-audio-subtitle = Що показують кольорові смуги
help-audio-body =
    Картки звуку з'являються на сторінці «Наживо» та — з більшим простором і більшою кількістю елементів керування — на сторінці «Мікшер». Спочатку йдуть глобальні аудіопристрої OBS, потім джерела з підтримкою звуку в поточній сцені, включно з джерелами у вкладених сценах і групах.
    Режими Мікшера визначають, звук якої сцени ви переглядаєте. Активний стежить за програмною сценою OBS. Вибраний завантажує одну сцену й залишається на ній. Закріплений утримує вибрану сцену як постійну ціль, поки OBS перемикається далі.
    Індикатор поруч із кожним фейдером має шкалу від -60 дБ унизу до 0 дБ угорі й використовує пороги OBS: зелений нижче -20 дБ для музики та фону, жовтий від -20 до -9 дБ для мовлення, червоний вище -9 дБ, де починається кліпінг. Жоден сигнал не має постійно залишатися в червоній зоні.
    Один стовпчик означає монофонічне джерело; два — стерео, спочатку лівий, потім правий канал. Якщо рухається лише лівий стовпчик, половина глядачів нічого не почує з цього джерела.
    Лінія над заповненням показує найвищий пік за останні двадцять секунд — так найшвидше можна помітити пропущений кліпінг. Квадрат унизу показує рівень, що надходить із пристрою до фейдера; якщо він завеликий, рух фейдера цього не виправить.
    Кнопка блокування на картці лише заморожує повзунок SceneDeck. Вона нічого не блокує в OBS.

help-outputs-title = Початок і зупинка трансляції та запису
help-outputs-subtitle = І підтвердження, що запобігають випадковим діям
help-outputs-body =
    Кнопки Почати/Зупинити трансляцію та Почати/Зупинити запис розташовані внизу бічної панелі й доступні з кожної сторінки. Рядок стану показує стан і тривалість роботи.
    Налаштування → Безпека виходів визначають, які з чотирьох дій спочатку просять підтвердження. Типово зупинка будь-якого виходу потребує підтвердження, а початок — ні: ранній початок зазвичай не шкодить, тоді як рання зупинка завершує ефір.
    Зміни стану, виконані безпосередньо в OBS, також з'являються тут: SceneDeck стежить за подіями OBS, а не припускає, що натискання власної кнопки спрацювало.

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
    Налаштування, включно з хостом і портом OBS, зберігаються в $XDG_CONFIG_HOME/scenedeck/config.json — зазвичай ~/.config/scenedeck/config.json.
    Ролі сцен, порядок, акценти й піктограми зберігаються у файлі registry.json у тій самій теці.
    Пароля OBS немає в жодному з цих файлів. Він зберігається у в'язці ключів Secret Service вашого робочого столу — у тому самому сховищі, яким користується браузер.
    Створіть резервну копію обох файлів JSON, щоб перенести повне налаштування на інший комп'ютер, або скористайтеся експортом YAML в Інвентарі лише для сцен.

help-shortcuts-title = Клавіатурні скорочення
help-shortcuts-subtitle = Повний список
help-shortcuts-body =
    F1 — відкрити цей посібник.
    Ctrl+R — повторно підключитися до OBS.
    Ctrl+, — відкрити Налаштування.
    Ctrl+Q — завершити роботу SceneDeck.
    Ctrl+1 … Ctrl+0 на сторінці «Наживо» — перемкнутися на одну з перших десяти карток сцен. Комбінацію можна змінити в Налаштування → Гарячі клавіші сцен.

welcome-dialog-heading = Ласкаво просимо до SceneDeck
welcome-dialog-body = Схоже, це перший запуск. Сторінка «Допомога» пояснює підключення до OBS — зокрема OBS на іншому комп'ютері — і показує, як залишити на сторінці «Наживо» лише сцени, між якими ви справді перемикаєтеся. Це займе кілька хвилин і допоможе уникнути помилок.
welcome-dialog-later = Не зараз
welcome-dialog-open = Прочитати посібник
window-help-tooltip = Допомога й посібник із початку роботи
