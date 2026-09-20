## Cadenas de la interfaz de SceneDeck (español).
##
## Agrupadas por el módulo desde el que se usa cada mensaje. Los ids de los
## mensajes llevan el prefijo del módulo para evitar ambigüedades en este
## único archivo compartido.

## Interno — usado solo por la prueba de regresión del propio cargador de
## i18n, no se muestra en la interfaz. Todo idioma debe definir este mensaje
## para que la prueba pueda confirmar que el paquete del idioma se cargó (y
## no solo el idioma `en` de reserva).
i18n-loader-smoke-test = Localización cargada.

## infra/error.rs — representaciones de AppError orientadas al usuario.
## `detail` es texto sin procesar proveniente de OBS o del sistema operativo
## y nunca se traduce.
error-connection = Fallo de conexión con OBS: { $detail }
error-request = Fallo en la solicitud a OBS: { $detail }
error-config = Error de configuración: { $detail }
error-storage = Error de almacenamiento: { $detail }
error-notification-title = Error de SceneDeck: { $message }

## domain/audio.rs — AudioSourceScope::label()
audio-scope-global = Global
audio-scope-active = Escena
audio-scope-nested = Anidado
audio-scope-group = Grupo

## domain/graph.rs — EdgeStatus summary label/tooltip
edge-status-ok-label = OK
edge-status-warning-label = Advertencia
edge-status-forbidden-label = Prohibido
edge-status-ok-tooltip = Conexiones que cumplen la política del grafo
edge-status-warning-tooltip = Conexiones fuera de una lista de permitidos
edge-status-forbidden-tooltip = Conexiones prohibidas por la política del grafo

## domain/output.rs — OutputRunState::label(), OutputStatus::summary()
output-state-inactive = Inactivo
output-state-starting = Iniciando
output-state-active = Activo
output-state-stopping = Deteniendo
output-state-reconnecting = Reconectando
output-state-paused = Pausado
output-state-unknown = Desconocido
output-summary = { $name }: { $state }

## domain/role.rs — SceneRole::label()/description()
role-primary = Principal
role-secondary = Secundaria
role-module = Módulo
role-raw = Sin procesar
role-debug = Depuración
role-archive = Archivada
role-unassigned = Sin asignar
role-primary-desc = Escena conmutable en directo
role-secondary-desc = Escena válida, oculta de Directo de forma predeterminada
role-module-desc = Escena anidada reutilizable, no conmutable directamente
role-raw-desc = Escena contenedora de hardware o fuente
role-debug-desc = Escena de prueba temporal
role-archive-desc = Conservada, pero excluida de todos los flujos de trabajo

## domain/mixer.rs — MixerMode / MixerGrouping labels
mixer-mode-active = Activo
mixer-mode-selected = Seleccionado
mixer-mode-pinned = Fijado
mixer-mode-active-desc = Sigue la escena de programa de OBS.
mixer-mode-selected-desc = Inspecciona la escena seleccionada sin seguir a OBS.
mixer-mode-pinned-desc = Mantiene estable la escena seleccionada mientras se opera.
mixer-grouping-scope = Ámbito
mixer-grouping-scene-path = Ruta de la escena
mixer-grouping-none = Ninguno

## domain/diagnostic.rs — DiagnosticSeverity label/count
diag-label-info = Información
diag-label-warning = Advertencias
diag-label-error = Errores
diag-count-info = { $count ->
    [one] { $count } elemento de información
   *[other] { $count } elementos de información
}
diag-count-warning = { $count ->
    [one] { $count } advertencia
   *[other] { $count } advertencias
}
diag-count-error = { $count ->
    [one] { $count } error
   *[other] { $count } errores
}

## ui/pages/inventory.rs
inventory-no-role-assigned = Sin rol asignado

## services/doctor_service.rs — diagnostic message/suggestion templates
doctor-no-role = La escena no tiene ningún rol asignado en el registro local.
doctor-no-role-suggestion = Abra Inventario y asigne un rol.
doctor-stale-entry = La entrada del registro hace referencia a una escena que no se encuentra en OBS.
doctor-stale-entry-suggestion = Elimine la entrada de Inventario.
doctor-protected-switchable = La escena protegida está en el rol conmutable «{ $role }».
doctor-protected-switchable-suggestion = Las escenas protegidas suelen ser bloques de construcción; considere Módulo o Sin procesar.
doctor-cycle = Referencia circular de escenas entre «{ $parent }» y «{ $child }».
doctor-cycle-suggestion = Elimine el bucle de escenas anidadas; OBS no puede representar ciclos.
doctor-edge-primary-debug = La escena Principal depende de una escena de Depuración. (→ «{ $child }»)
doctor-edge-primary-debug-suggestion = Elimine la escena de Depuración de la ruta en directo antes de salir al aire.
doctor-edge-primary-raw = La escena Principal envuelve directamente una fuente Sin procesar. (→ «{ $child }»)
doctor-edge-primary-raw-suggestion = Envuelva la fuente Sin procesar en una escena Módulo para facilitar la reutilización y la claridad.
doctor-edge-module-primary = El Módulo depende de una escena Principal, invirtiendo la jerarquía. (→ «{ $child }»)
doctor-edge-module-primary-suggestion = Los módulos deben ser bloques de construcción, no consumidores de escenas Principales.
doctor-edge-raw-nests = La escena Sin procesar anida otra escena. (→ «{ $child }»)
doctor-edge-raw-nests-suggestion = Las escenas Sin procesar deben ser contenedores de fuente terminales, sin escenas anidadas.
doctor-edge-forbidden = La dependencia de escena está prohibida por la política del grafo. (→ «{ $child }»)
doctor-edge-outside-policy = La dependencia de escena está fuera de la política del grafo configurada. (→ «{ $child }»)
doctor-edge-adjust-suggestion = Ajuste la relación de escena anidada o actualice las reglas del grafo del registro.

## controller/app_controller.rs
controller-not-connected = No conectado a OBS

## controller/state.rs — Page titles and ObsStatus labels
page-live = Directo
page-stats = Estadísticas
page-mixer = Mezclador
page-graph = Grafo
page-inventory = Inventario
page-doctor = Diagnóstico
page-settings = Preferencias
obs-status-disconnected = Desconectado
obs-status-connecting = Conectando…
obs-status-connected = Conectado
obs-status-error = Error

## storage/config.rs — ConfigStartupNotice
config-first-launch = Aún no hay ajustes guardados. Se cargan los valores predeterminados.
config-read-failed = No se pudieron leer los ajustes: { $detail }
config-parse-failed = No se pudieron analizar los ajustes: { $detail }

## graph.rs

graph-empty-title = Sin dependencias
graph-empty-description = Ninguna escena anida otras escenas, o OBS no está conectado. Conecte y agregue fuentes de escenas anidadas para ver el grafo de dependencias.
graph-page-title = Dependencias de escenas
graph-reset-tooltip = Restablecer la disposición del grafo
graph-edge-summary-count = { $label } { $count }

## mixer.rs

## Page chrome / empty state
mixer-empty-title = Sin datos del mezclador
mixer-empty-description = Conéctese a OBS para cargar las escenas y las fuentes de audio.
mixer-page-title = Mezclador
mixer-controls-title = Controles del mezclador
mixer-summary-title = Fuente actual del mezclador

## Control rows (ComboRow / EntryRow titles+subtitles)
mixer-mode-row-title = Modo
mixer-mode-row-subtitle = Activo sigue a OBS; Seleccionado y Fijado mantienen estable la escena elegida.
mixer-scene-row-title = Escena
mixer-scene-row-subtitle = Usado por los modos Seleccionado y Fijado.
mixer-grouping-row-title = Agrupar por
mixer-grouping-row-subtitle = Controla cómo se organizan a continuación las fuentes de audio.
mixer-search-row-title = Buscar

## Scene-loading / no-scene placeholders
mixer-no-scene-title = Ninguna escena seleccionada
mixer-no-scene-description = Elija una escena para cargar su audio del mezclador.
mixer-loading-title = Cargando audio del mezclador
mixer-loading-description = Cargando fuentes de audio de { $scene }.

## Audio-source empty states
mixer-current-scene-fallback = La escena actual
mixer-no-audio-sources-title = Sin fuentes de audio
mixer-no-audio-sources-description = { $scene } no tiene fuentes de audio de OBS configuradas que coincidan.
mixer-no-matching-title = Ninguna fuente de audio coincidente
mixer-no-matching-description = Ajuste el filtro de búsqueda para mostrar las fuentes de audio disponibles.

## Group titles
mixer-group-all-sources = Todas las fuentes
mixer-group-global-fallback = Global

## Error placeholder + retry
mixer-error-title = Audio del mezclador no disponible
mixer-error-description = No se pudieron cargar las fuentes de audio de { $scene }: { $message }
mixer-retry-button-label = Reintentar
mixer-retry-button-tooltip = Reintentar la carga del audio del mezclador

## Current-source summary row (source_summary / scene_target_summary)
mixer-summary-following-active = Siguiendo la escena activa de OBS: { $scene }
mixer-summary-no-scene-selected = Ninguna escena seleccionada
mixer-summary-selected-scene = Escena seleccionada: { $scene }
mixer-summary-pinned-scene = Escena fijada: { $scene }
mixer-summary-selected-fallback = No se ha definido una escena seleccionada; se usa la escena activa de OBS: { $scene }
mixer-summary-pinned-selected-fallback = No se ha definido una escena fijada; se usa la escena seleccionada: { $scene }
mixer-summary-pinned-active-fallback = No se han definido ni la escena fijada ni la seleccionada; se usa la escena activa de OBS: { $scene }

## doctor.rs — page chrome, empty state, and all-clear text for the Doctor
## page. Diagnostic finding messages themselves (`doctor-no-role`,
## `doctor-cycle*`, `doctor-edge-*`, etc.) already exist in
## src/services/doctor_service.rs and are NOT redefined here.
doctor-page-title = Diagnóstico
doctor-empty-state-title = Nada que comprobar
doctor-empty-state-description = Conéctese a OBS para ejecutar los diagnósticos de arquitectura.
doctor-summary-row-title = Diagnósticos
doctor-rerun-tooltip = Ejecutar los diagnósticos de nuevo
doctor-all-clear-title = No se encontraron problemas
doctor-all-clear-detail = La arquitectura de escenas cumple todas las comprobaciones.

## inventory.rs — page chrome, group titles, YAML import/export UI, and
## status messages. Role names/descriptions themselves are NOT redefined
## here — they already exist in src/domain/role.rs (`role-*`,
## `role-*-desc`, `role-unassigned`) and are reused directly via
## SceneRole::label()/description()/unassigned_label(). The already-present
## `inventory-no-role-assigned` message is also left untouched.
inventory-page-title = Inventario
inventory-empty-state-title = Sin escenas
inventory-empty-state-description = Conéctese a OBS para cargar la lista de escenas.
inventory-scenes-group-title = Escenas de OBS
inventory-scenes-group-description = Asigne roles para controlar qué escenas aparecen en la página Directo.
inventory-stale-group-title = Entradas de registro obsoletas
inventory-stale-group-description = Estas escenas están en su registro local pero ya no existen en OBS.
inventory-remove-stale-tooltip = Eliminar entrada obsoleta
inventory-yaml-row-title = YAML del registro de escenas
inventory-yaml-row-subtitle = Exporte o importe roles de escenas, etiquetas, marcas de protección y reglas del grafo.
inventory-yaml-filter-name = Archivos YAML

# Shared between the row's Export button, the export FileChooserNative's
# accept label, and (for Cancel) both the export and import dialogs.
inventory-export-button-label = Exportar
inventory-export-tooltip = Exportar el registro de escenas a YAML
inventory-import-button-label = Importar
inventory-import-tooltip = Importar el registro de escenas desde YAML
inventory-dialog-cancel-label = Cancelar

inventory-export-dialog-title = Exportar registro de escenas
inventory-export-success = Registro de escenas exportado a { $path }.
inventory-export-error = Error al exportar: { $error }
inventory-export-no-file = Error al exportar: no se seleccionó ningún archivo.

inventory-import-dialog-title = Importar registro de escenas
inventory-import-error = Error al importar: { $error }
inventory-import-no-file = Error al importar: no se seleccionó ningún archivo.

## window.rs

window-stream-live-tooltip = Transmitiendo en directo
window-about-tooltip = Acerca de SceneDeck
window-refresh-tooltip = Actualizar la página actual

window-stream-status-line = Transmisión: { $state }{ $elapsed }
window-record-status-line = Grabación: { $state }{ $elapsed }

window-status-connecting = Conectando con OBS…
window-connect-btn-connecting = Conectando…
window-current-scene-none = Escena actual: —
window-status-connected = Conectado — OBS { $version }
window-connect-btn-disconnect = Desconectar
window-status-disconnected = Desconectado
window-connect-btn-connect = Conectar a OBS
window-live-disconnected-hint = Conéctese a OBS para usar los controles de Directo
window-current-scene = Escena actual: { $scene }
window-status-error = Error: { $error }
window-connect-btn-retry = Reintentar
window-obs-connection-failed = Fallo de conexión con OBS
window-toast-obs-error = Error de OBS: { $error }

window-output-kind-stream = Transmisión
window-output-kind-record = Grabación

window-sidebar-output-starting = Iniciando…
window-sidebar-output-stopping = Deteniendo…
window-sidebar-output-reconnecting = Reconectando…
window-sidebar-output-working = Procesando…

window-sidebar-start-stream = Iniciar transmisión
window-sidebar-stop-stream = Detener transmisión
window-sidebar-start-recording = Iniciar grabación
window-sidebar-stop-recording = Detener grabación

window-selector-profile-label = Perfil
window-selector-profile-tooltip = Cambiar de perfil de OBS
window-selector-collection-label = Colección
window-selector-collection-tooltip = Cambiar de colección de escenas de OBS

## live.rs

live-start-stream-label = Iniciar transmisión
live-stop-stream-label = Detener transmisión
live-start-record-label = Iniciar grabación
live-stop-record-label = Detener grabación
live-stream-toggle-tooltip = Iniciar o detener la transmisión
live-record-toggle-tooltip = Iniciar o detener la grabación
live-stream-inactive-label = Transmisión: Inactiva
live-record-inactive-label = Grabación: Inactiva
live-copy-last-recording-path-tooltip = Copiar la ruta de la última grabación
live-copied-recording-path-tooltip = Ruta de la última grabación copiada
live-copy-recording-path-with-value-tooltip = Copiar ruta de grabación: { $path }
live-stream-card-title = Transmisión
live-recording-card-title = Grabación
live-current-scene-placeholder = Escena actual: —
live-scenes-section-label = Escenas
live-scenes-connect-hint = Conéctese a OBS para cargar las escenas.
live-audio-section-label = Audio
live-disconnected-title = Conéctese a OBS para usar los controles de Directo
live-disconnected-detail = Use el control de conexión en la parte inferior de la barra lateral.
live-stream-command-error-label = Error en el comando de transmisión
live-recording-command-error-label = Error en el comando de grabación
live-last-recording-detail = Última grabación: { $path }
live-starting-stream = Iniciando transmisión…
live-stopping-stream = Deteniendo transmisión…
live-reconnecting-stream = Reconectando transmisión…
live-starting-recording = Iniciando grabación…
live-stopping-recording = Deteniendo grabación…
live-reconnecting-recording = Reconectando grabación…
live-button-starting = Iniciando…
live-button-stopping = Deteniendo…
live-button-reconnecting = Reconectando…
live-button-working = Procesando…
live-output-kind-stream = Transmisión
live-output-kind-record = Grabación
live-output-label = { $kind }: { $state }
live-output-label-with-elapsed = { $kind }: { $state } · { $elapsed }
live-scenes-no-primary-hint = No se encontraron escenas con rol Principal. Asigne roles en Inventario.
live-audio-empty-hint = No hay entradas de audio configuradas.
live-cancel-button-label = Cancelar
live-start-stream-confirm-heading = ¿Iniciar transmisión?
live-start-stream-confirm-body = OBS comenzará a enviar la transmisión en directo.
live-stop-stream-confirm-heading = ¿Detener transmisión?
live-stop-stream-confirm-body = OBS dejará de enviar la transmisión en directo.
live-start-recording-confirm-heading = ¿Iniciar grabación?
live-start-recording-confirm-body = OBS iniciará una nueva grabación.
live-start-recording-confirm-label = Iniciar grabación
live-stop-recording-confirm-heading = ¿Detener grabación?
live-stop-recording-confirm-body = OBS detendrá la grabación actual.
live-stop-recording-confirm-label = Detener grabación

## audio_card.rs
audio-card-mute-tooltip = Silenciar entrada
audio-card-source-path-tooltip = { $scope }: { $path }
audio-card-fader-tooltip = Control deslizante de volumen
audio-card-lock-tooltip = Bloquear el control deslizante de volumen
audio-card-fine-plus-tooltip = +1 dB
audio-card-fine-reset-tooltip = Restablecer a 0.0 dB
audio-card-fine-minus-tooltip = -1 dB
audio-card-meter-tooltip-title = Medidor de volumen: { $channels }
audio-card-meter-tooltip-zones = Verde por debajo de -20 dB · Amarillo hasta -9 dB · Rojo por encima, cerca del recorte
audio-card-meter-tooltip-indicators = Barra: nivel de pico con caída · Línea: pico más alto en 20 s · Punto: sonoridad · Base: nivel que llega del dispositivo
audio-card-meter-tooltip-waiting = Medidor de volumen: esperando niveles de OBS

## icon.rs
icon-none = Sin icono
icon-camera = Cámara
icon-desktop = Escritorio
icon-game = Juego
icon-film = Película
icon-images = Imágenes
icon-television = Televisión
icon-browser = Navegador
icon-terminal = Terminal
icon-code = Código
icon-chat = Chat
icon-guests = Invitados
icon-star = Estrella
icon-alert = Aviso
icon-break = Descanso
icon-countdown = Cuenta atrás
icon-start = Inicio
icon-pause = Pausa
icon-stop = Parada
icon-settings = Ajustes
icon-layers = Capas
icon-microphone = Micrófono
icon-headset = Auriculares con micrófono
icon-headphones = Auriculares
icon-speaker = Altavoz
icon-volume = Volumen
icon-music = Música
icon-instrument = Instrumento
icon-radio = Radio
icon-call = Llamada
icon-waveform = Forma de onda
inventory-scene-icon-tooltip = Elegir un icono para esta escena
mixer-input-icon-tooltip = Elegir un icono para esta fuente de audio

## meter.rs
audio-meter-zone-nominal = Verde
audio-meter-zone-warning = Amarillo
audio-meter-zone-error = Rojo
audio-meter-channel-mono = Mono
audio-meter-channel-left = Izquierdo
audio-meter-channel-right = Derecho
audio-meter-channel-front-left = Frontal izquierdo
audio-meter-channel-front-right = Frontal derecho
audio-meter-channel-front-center = Frontal central
audio-meter-channel-lfe = LFE
audio-meter-channel-rear-left = Trasero izquierdo
audio-meter-channel-rear-right = Trasero derecho
audio-meter-channel-side-left = Lateral izquierdo
audio-meter-channel-side-right = Lateral derecho
audio-meter-channel-numbered = Canal { $index }

## hotkey.rs
hotkey-modifier-ctrl = Ctrl+
hotkey-modifier-alt = Alt+
hotkey-modifier-shift = Mayús+
hotkey-modifier-super = Súper+
hotkey-modifier-ctrl-alt = Ctrl+Alt+
hotkey-modifier-ctrl-shift = Ctrl+Mayús+
hotkey-style-plain = Solo el dígito
hotkey-style-ctrl = Ctrl + dígito
hotkey-style-alt = Alt + dígito
hotkey-style-shift = Mayús + dígito
hotkey-style-super = Súper + dígito
hotkey-style-ctrl-alt = Ctrl+Alt + dígito
hotkey-style-ctrl-shift = Ctrl+Mayús + dígito
hotkey-style-leader = Tecla líder y luego dígito
hotkey-leader-space = Espacio
hotkey-leader-comma = Coma
hotkey-leader-semicolon = Punto y coma
hotkey-leader-backslash = Barra invertida
hotkey-leader-grave = Acento grave
hotkey-shortcut-leader = { $leader } y luego { $digit }
hotkey-hint-plain = Pulsa 1 … 0
hotkey-hint-modifier = Pulsa { $modifier }1 … 0
hotkey-hint-leader = Pulsa { $leader } y luego 1 … 0
hotkey-hint-leader-armed = Líder activo — pulsa 1 … 0
hotkey-hint-empty-slot = No hay escena en la posición { $slot }

## scene_card.rs
scene-card-tooltip = { $status } ({ $role })
scene-card-tooltip-with-hotkey = { $status } ({ $role }) · { $hotkey }
scene-card-role-suffix = Escena { $role }

## status_bar.rs
status-bar-stream-inactive = Transmisión: Inactiva
status-bar-record-inactive = Grabación: Inactiva
status-bar-fps-placeholder = FPS —
status-bar-cpu-placeholder = CPU —
status-bar-bitrate-placeholder = Tasa de bits —
status-bar-fps = FPS { $value }
status-bar-cpu = CPU { $value }%
status-bar-bitrate = Tasa de bits { $value } kbps
status-bar-dropped = { $count } descartados
status-bar-dropped-placeholder = Descartados —

## settings.rs

settings-page-title = Preferencias
settings-appearance-title = Apariencia
settings-appearance-description = Las aplicaciones de GNOME deberían seguir el estilo del sistema de forma predeterminada.
settings-theme-mode-system = Sistema
settings-theme-mode-light = Claro
settings-theme-mode-dark = Oscuro
settings-color-scheme-title = Esquema de color
settings-color-scheme-subtitle = Seguir la preferencia del sistema o forzar claro / oscuro
settings-motion-title = Movimiento
settings-motion-subtitle = Cuánto se anima la interfaz. Los indicadores en directo siguen siendo legibles en todos los niveles.
settings-motion-full = Completo
settings-motion-reduced = Reducido
settings-motion-off = Desactivado
settings-theme-title = Tema
settings-theme-status-title = Estado del tema
settings-theme-status-initial = Tema cargado.
settings-failed-to-save = Error al guardar: { $err }
settings-custom-css-title = CSS personalizado
settings-custom-css-subtitle = Cargar archivos CSS de usuario independientes para el modo claro y el oscuro
settings-custom-light-css-title = Ruta del CSS claro personalizado
settings-custom-dark-css-title = Ruta del CSS oscuro personalizado
settings-reload-css-title = Recargar CSS personalizado
settings-reload-css-subtitle = Reaplicar el tema seleccionado y el archivo CSS personalizado claro/oscuro correspondiente.
settings-reload-button = Recargar
settings-language-title = Idioma
settings-language-description = Los cambios surten efecto después de reiniciar SceneDeck.
settings-display-language-title = Idioma de la interfaz
settings-display-language-subtitle = Elija un idioma o siga la configuración regional del sistema.
settings-language-status-title = Estado del idioma
settings-language-status-initial = Reinicie para aplicar un cambio de idioma.
settings-language-saved = Idioma guardado. Reinicie SceneDeck para aplicarlo.
settings-obs-connection-title = Conexión con OBS
settings-obs-connection-description = Ajustes de WebSocket para OBS Studio (puerto predeterminado: 4455).
settings-host-title = Host
settings-port-title = Puerto
settings-password-title = Contraseña (opcional)
settings-obs-status-title = Estado de OBS
settings-invalid-port = Número de puerto no válido.
settings-saved = Preferencias guardadas.
settings-password-saved = Contraseña guardada en el llavero.
settings-keyring-error = Error del llavero: { $err }
settings-output-safety-title = Seguridad de salida
settings-output-safety-description = Confirmaciones opcionales para acciones críticas de transmisión y grabación.
settings-confirm-start-stream-title = Confirmar inicio de transmisión
settings-confirm-start-stream-subtitle = Preguntar antes de iniciar la transmisión en directo.
settings-confirm-stop-stream-title = Confirmar detención de transmisión
settings-confirm-stop-stream-subtitle = Preguntar antes de detener la transmisión en directo.
settings-confirm-start-recording-title = Confirmar inicio de grabación
settings-confirm-start-recording-subtitle = Preguntar antes de iniciar una grabación.
settings-confirm-stop-recording-title = Confirmar detención de grabación
settings-confirm-stop-recording-subtitle = Preguntar antes de detener una grabación.
settings-hotkeys-title = Atajos de escena
settings-hotkeys-description = Cambia de escena en Directo desde el teclado. Las posiciones siguen el orden de escenas definido en Inventario.
settings-hotkeys-enabled-title = Activar atajos de escena
settings-hotkeys-enabled-subtitle = Las teclas numéricas cambian las tarjetas de escena de Directo, en el orden en que aparecen.
settings-hotkeys-style-title = Combinación de teclas
settings-hotkeys-style-subtitle = Qué teclas cambian de escena. Los dígitos sueltos se desactivan mientras un campo de texto tiene el foco.
settings-hotkeys-leader-title = Tecla líder
settings-hotkeys-leader-subtitle = Primera tecla del atajo de dos pulsaciones, al estilo de vim.
settings-hotkeys-timeout-title = Tiempo de espera del líder
settings-hotkeys-timeout-subtitle = Cuánto espera el líder a su dígito, en milisegundos.
settings-hotkeys-preview-title = Asignaciones actuales
settings-hotkeys-preview-subtitle = { $first } … { $last } cambian las primeras { $count } escenas de Directo.
settings-hotkeys-preview-disabled = Los atajos de escena están desactivados.
settings-obs-not-connected = No conectado a OBS.
settings-obs-connecting = Conectando con OBS…
settings-obs-connected = Conectado — OBS { $version }
settings-obs-error = Error: { $err }
settings-theme-subtitle = { $description } Muestras: { $swatches }
settings-theme-loaded = { $theme } cargado ({ $variant }).
settings-theme-loaded-with-warnings = Tema cargado con advertencias.

## theme.rs

theme-adwaita-default-name = Adwaita predeterminado
theme-adwaita-default-desc = Estilo neutro que sigue los valores predeterminados de GNOME.
theme-scenedeck-dark-name = SceneDeck Oscuro
theme-scenedeck-dark-desc = Un tema de consola oscuro reservado para la operación en directo.
theme-scenedeck-light-name = SceneDeck Claro
theme-scenedeck-light-desc = Un tema de consola claro y nítido con contraste moderado.
theme-obs-name = OBS
theme-obs-desc = El aspecto predeterminado de OBS Studio, interpretado con libadwaita.
theme-obsidian-name = Obsidiana
theme-obsidian-desc = Superficies de grafito de alta legibilidad con acentos fríos.
theme-nord-name = Nord
theme-nord-desc = Superficies frías azul-grisáceas con acentos de tono escarcha.
theme-dracula-inspired-name = Inspirado en Dracula
theme-dracula-inspired-desc = Una paleta oscura y expresiva que usa CSS original.
theme-solarized-dark-name = Solarized Oscuro
theme-solarized-dark-desc = Contraste de bajo resplandor con acentos turquesa y ámbar.
theme-high-contrast-name = Alto contraste
theme-high-contrast-desc = Contornos y contraste más marcados para controles críticos.
theme-stream-red-name = Rojo transmisión
theme-stream-red-desc = Acentos rojos orientados a la retransmisión para los estados en directo.
theme-studio-purple-name = Púrpura estudio
theme-studio-purple-desc = Acentos púrpura controlados sin dominar las superficies.
theme-ubuntu-violet-name = Violeta Ubuntu
theme-ubuntu-violet-desc = Superficies violeta inspiradas en Ubuntu con un acento cálido en directo.
theme-custom-css-read-failed = No se pudo leer el CSS personalizado desde { $path }: { $err }
theme-custom-css-no-matching-file = El CSS personalizado está activado, pero no hay ningún archivo claro/oscuro correspondiente configurado.
theme-css-no-display = { $label } no se cargó porque no hay ninguna pantalla de GTK disponible.
theme-css-parse-error = Error al analizar el CSS de { $label }: { $message }
## stats.rs — live streaming telemetry
stats-page-title = Estadísticas de emisión
stats-page-subtitle = Telemetría en directo consultada a OBS una vez por segundo mientras hay conexión.
stats-gauge-fps = FPS
stats-gauge-frame-time = Tiempo por fotograma (ms)
stats-gauge-dropped = Fotogramas perdidos
stats-gauge-congestion = Congestión
stats-chart-fps = Fotogramas por segundo
stats-chart-frame-time = Tiempo medio de renderizado por fotograma (ms)
stats-chart-output-skipped = Fotogramas de salida omitidos por muestra
stats-chart-render-skipped = Fotogramas de renderizado perdidos por muestra
stats-card-render-frames = Fotogramas de renderizado perdidos
stats-card-output-frames = Fotogramas de salida omitidos
stats-card-stream-frames = Fotogramas perdidos en la emisión
stats-card-frame-time = Tiempo medio de renderizado por fotograma
stats-card-cpu = Uso de CPU de OBS
stats-card-memory = Uso de memoria de OBS
stats-card-bitrate = Tasa de bits de la emisión
stats-value-placeholder = —
stats-value-frames = { $skipped } de { $total } ({ $percent } %)
stats-value-ms = { $value } ms
stats-value-percent = { $value } %
stats-value-mb = { $value } MB
stats-value-kbps = { $value } kb/s

## Ayuda, incorporación y mensajes añadidos en la versión 0.4.
page-help = Ayuda
config-parse-failed-backed-up = No se pudieron analizar los ajustes: { $detail } — el archivo se conservó en { $path } y se cargaron los valores predeterminados.
stats-popout-button = Abrir las estadísticas en una ventana aparte

## ui/pages/help.rs — guía de incorporación. Los cuerpos tienen varias líneas
## a propósito: cada línea se muestra por separado dentro de un tema desplegable.
help-page-title = Ayuda e introducción
help-hero-title = Bienvenido a SceneDeck
help-hero-description = Una superficie de control nativa para Linux destinada a OBS Studio. Esta guía explica la primera conexión, cómo limitar la página Directo a las escenas que realmente usa y para qué sirve cada página de la barra lateral. Despliegue un tema para leerlo.
help-expand-hint = Pulse un tema para desplegarlo.

help-open-settings = Abrir Preferencias
help-open-inventory = Abrir Inventario
help-open-doctor = Abrir Diagnóstico
help-open-live = Abrir Directo
help-open-mixer = Abrir Mezclador
help-open-graph = Abrir Grafo
help-open-stats = Abrir Estadísticas

help-group-start-title = Primeros pasos
help-group-start-description = El camino más corto desde una instalación nueva hasta el primer cambio de escena.

help-quickstart-title = Cinco pasos para cambiar de escena por primera vez
help-quickstart-subtitle = Sígalos en orden la primera vez
help-quickstart-body =
    1. En OBS Studio, abra Herramientas → Ajustes del servidor WebSocket y marque «Activar servidor WebSocket». Deje OBS abierto.
    2. En ese mismo diálogo de OBS, pulse «Mostrar información de conexión» y anote el puerto del servidor (4455 de forma predeterminada) y la contraseña del servidor.
    3. En SceneDeck, abra Preferencias e introduzca el anfitrión, el puerto y la contraseña. Mantenga 127.0.0.1 como anfitrión cuando OBS se ejecute en este mismo equipo.
    4. Pulse Conectar en la parte inferior de la barra lateral. La línea de estado situada encima del botón se volverá verde y mostrará «Conectado».
    5. Abra Inventario y asigne el rol «Principal» a las escenas que quiera usar durante una emisión. Esas escenas —y solo esas— se convertirán en tarjetas de la página Directo.

help-concepts-title = Cómo entiende SceneDeck su configuración
help-concepts-subtitle = Los roles, el registro y lo que nunca modifica OBS
help-concepts-body =
    SceneDeck nunca cambia el nombre, elimina ni reordena nada dentro de OBS. Lee sus escenas mediante la conexión WebSocket de OBS y guarda sus propias notas sobre ellas.
    Un «rol» es una de esas notas: una etiqueta propia que indica para qué sirve una escena, ya sea la que muestra en directo, una superposición reutilizable o una escena de prueba antigua.
    Esas notas se guardan en registry.json junto al archivo de configuración, por lo que sobreviven a los reinicios y se pueden exportar y trasladar a otro equipo desde la página Inventario.
    Como las notas son locales, dos personas pueden compartir una configuración de OBS y mantener cada una una página Directo distinta.

help-group-connect-title = Conexión con OBS
help-group-connect-description = En este equipo o al otro lado de la sala.

help-connect-local-title = Conexión con OBS en este equipo
help-connect-local-subtitle = El caso predeterminado: anfitrión 127.0.0.1, puerto 4455
help-connect-local-body =
    127.0.0.1 es la dirección que usa un equipo para comunicarse consigo mismo, por lo que es el anfitrión correcto cuando OBS y SceneDeck se ejecutan en el mismo equipo.
    El puerto debe coincidir con el puerto del servidor de los ajustes del servidor WebSocket de OBS. OBS usa 4455 salvo que lo haya cambiado.
    Si «Activar autenticación» está marcado en OBS, pegue la contraseña en Preferencias → Contraseña. SceneDeck la guarda en el llavero del escritorio (el mismo lugar donde el navegador guarda los inicios de sesión), nunca en el archivo de configuración de texto sin formato.
    Pulse Conectar en la barra lateral o pulse Ctrl+R en cualquier momento para volver a conectarse.

help-connect-remote-title = Conexión con OBS en otro equipo
help-connect-remote-subtitle = El PC de emisión en una esquina y el control desde su portátil
help-connect-remote-body =
    Esta es la configuración con dos equipos: OBS se ejecuta en el equipo que captura y codifica, y SceneDeck en el equipo que tiene delante. Ambos deben estar en la misma red.
    En el equipo con OBS, vaya a Herramientas → Ajustes del servidor WebSocket, marque «Activar servidor WebSocket» y «Activar autenticación», y establezca una contraseña que pueda escribir. No desactive la autenticación: de lo contrario, cualquiera en la red que pueda acceder al puerto podría iniciar y detener la emisión.
    Busque la dirección del equipo con OBS en ese mismo equipo. En Linux, ejecute `ip addr` y busque una dirección como 192.168.1.42; en Windows, ejecute `ipconfig` y consulte la dirección IPv4; en macOS, aparece en Ajustes del Sistema → Red. Es la dirección del equipo, no la de OBS.
    Permita el puerto en el cortafuegos del equipo con OBS. En Linux con ufw, use `sudo ufw allow 4455/tcp`; en Windows, permita OBS en el Firewall de Windows Defender para redes privadas.
    En las Preferencias de SceneDeck, introduzca esa dirección como anfitrión (por ejemplo, 192.168.1.42), mantenga el puerto 4455 y pegue la contraseña. Pulse Conectar.
    Es preferible conectar por cable el equipo con OBS. Los cambios de escena por Wi-Fi funcionan, pero un paquete perdido retrasa el corte.
    Un consejo que puede salvar una emisión: reserve una dirección fija para el equipo con OBS en los ajustes DHCP del enrutador, para que el anfitrión guardado siga funcionando después de reiniciar.

help-connect-share-title = Permitir que un copresentador o moderador controle el panel
help-connect-share-subtitle = Un segundo SceneDeck conectado al mismo equipo
help-connect-share-body =
    El servidor WebSocket de OBS admite varios clientes a la vez, por lo que otra persona en un segundo equipo puede ejecutar su propia copia de SceneDeck contra la misma configuración: es el mismo proceso del tema anterior, realizado dos veces.
    No hay inicios de sesión por persona: quien tenga el anfitrión, el puerto y la contraseña en sus Preferencias tendrá exactamente el mismo control que usted, incluido iniciar y detener la emisión. Entrégueselos solo a alguien a quien confiaría su sesión de OBS.
    Limite primero lo que verá. Los roles y las escenas ocultas se guardan en el Inventario local, no en OBS; configúrelos en su equipo y pida a su colaborador que exporte o vuelva a crear el YAML del registro de escenas, en lugar de empezar con una lista completa sin filtrar.
    No redirija el puerto WebSocket a Internet para dar acceso a un colaborador remoto. Conecten ambos equipos a la misma VPN o red de Tailscale, o encaminen la conexión mediante SSH y establezca como anfitrión la dirección del túnel.
    Todas las personas conectadas ven el mismo estado en directo: un cambio de escena o un movimiento de regulador desde cualquiera de los equipos aparece de inmediato en ambos, igual que si compartieran el mismo teclado.

help-connect-trouble-title = Cuando Conectar no funciona
help-connect-trouble-subtitle = Lea el error y revise esta lista en orden
help-connect-trouble-body =
    «Conexión rechazada» casi siempre significa que el servidor WebSocket no está activado en OBS o que el puerto no coincide. Compruebe ambos en Herramientas → Ajustes del servidor WebSocket.
    Si la conexión se queda esperando y después agota el tiempo, normalmente un cortafuegos está descartando el tráfico o la dirección del anfitrión pertenece a otro equipo.
    «Error de autenticación» significa que la contraseña es incorrecta. Vuelva a introducirla en Preferencias; el campo es de solo escritura, por lo que parece vacío aunque haya una contraseña guardada.
    ¿No aparece nada en el estado de la barra lateral? Confirme que OBS se está ejecutando y que no está bloqueado por uno de sus propios diálogos modales.
    SceneDeck se vuelve a conectar automáticamente tras perder el enlace, y Ctrl+R fuerza un intento inmediato.

help-group-scenes-title = Organización de las escenas
help-group-scenes-description = La página Directo debe mostrar lo que usa y nada más.

help-scenes-hide-title = Ocultar escenas que nunca muestra en directo
help-scenes-hide-subtitle = La opción más útil para configurar primero
help-scenes-hide-body =
    Una configuración de OBS en uso acumula escenas que solo existen para anidarse en otras, o que se crearon para una prueba y nunca se eliminaron. Mostrarlas en una superficie de control en directo facilita un corte equivocado.
    SceneDeck solo muestra una tarjeta en Directo para las escenas cuyo rol es Principal. Los demás roles se ocultan, así que «ocultar una escena» consiste simplemente en asignarle cualquier rol distinto de Principal.
    Abra Inventario. Cada escena de OBS tiene una fila con un selector de rol a la derecha.
    Asigne Principal al pequeño grupo de escenas que realmente muestra durante una emisión. Para el resto, elija: Secundaria (una escena real que puede necesitar, pero no quiere en Directo), Módulo (una superposición o rótulo que solo se anida en otra escena), Sin procesar (una cámara o captura básica), Depuración (una escena de prueba) o Archivada (guardada para más adelante y apartada).
    Las escenas sin rol tampoco aparecen en Directo, por lo que dejarlas Sin asignar también las oculta. Aun así, es mejor asignar un rol a propósito: la página Diagnóstico marca las escenas Sin asignar para que detecte las nuevas.
    El cambio se aplica de inmediato: vuelva a Directo y la tarjeta habrá desaparecido. Nada ha cambiado en OBS.

help-scenes-order-title = Orden, colores e iconos
help-scenes-order-subtitle = Haga que la tarjeta correcta sea la más evidente
help-scenes-order-body =
    Arrastre una escena desde el tirador situado a la izquierda de su fila en Inventario para definir el orden. Las tarjetas de Directo y los atajos numéricos siguen ese orden, por lo que la tarjeta que activa con 1 es la primera.
    El selector de color de acento tiñe la tarjeta de esa escena en Directo. Reserve un color intenso para las escenas importantes —la escena «estamos en directo» o la lectura del patrocinador— para encontrarlas de un vistazo.
    El selector de iconos situado a la izquierda de cada fila añade un símbolo a la tarjeta de Directo. Hay treinta iconos disponibles, además de una opción «sin icono» para quitarlo.
    El orden, los colores y los iconos se guardan en el registro local, no en OBS.

help-scenes-registry-title = Copia de seguridad y traslado de la configuración
help-scenes-registry-subtitle = La fila YAML del registro de escenas en Inventario
help-scenes-registry-body =
    Exportar guarda los roles, el orden, los colores de acento, los iconos, las etiquetas y las reglas del grafo en un único archivo YAML, un formato de texto sencillo que puede leer y mantener bajo control de versiones.
    Importar sustituye el registro local por el contenido de ese archivo. Úselo para trasladar una configuración terminada a otro equipo o para restaurarla después de un experimento.
    Los nombres de escena vinculan el archivo con OBS, por lo que una escena cuyo nombre cambie en OBS reaparecerá como una entrada obsoleta. Inventario muestra esas entradas y permite eliminarlas.

help-group-operate-title = Durante una emisión
help-group-operate-description = Las páginas que realmente usa mientras emite.

help-live-title = La página Directo
help-live-subtitle = Tarjetas de escena, audio y escena de programa
help-live-body =
    Directo es la vista de operación: la escena de programa actual arriba, las tarjetas de escena a un lado y las tarjetas de audio compactas al otro. Arrastre el separador para dar más espacio a la mitad que necesite.
    Al pulsar una tarjeta, OBS cambia a esa escena. La actual se marca como Activa y las demás como Preparadas.
    ¿No hay tarjetas después de conectarse? Ninguna escena tiene aún el rol Principal; consulte «Ocultar escenas que nunca muestra en directo» más arriba.
    La barra de estado inferior permanece visible en todas las páginas y muestra el estado de la conexión, la emisión y la grabación con el tiempo transcurrido, además de los FPS, fotogramas perdidos, CPU y tasa de bits en directo.

help-hotkeys-title = Cambiar de escena con el teclado
help-hotkeys-subtitle = Ctrl+1 … Ctrl+0 de forma predeterminada y configurable
help-hotkeys-body =
    Cada una de las diez primeras tarjetas de Directo muestra una pequeña insignia con su dígito: la primera es 1, la novena es 9 y la décima es 0. El texto junto al encabezado Escenas siempre indica la combinación actual.
    Los números de posición siguen el orden de Inventario, así que reordenar allí las tarjetas reordena también los atajos.
    Preferencias → Atajos de escena permite elegir cómo se pulsa el dígito. Un modificador más el dígito (Ctrl de forma predeterminada) evita activaciones accidentales al escribir. El dígito solo es la opción más rápida. El estilo de tecla líder funciona como en vim: pulse la tecla líder, suéltela y pulse el dígito; el texto mostrará «Líder activo» mientras espera.
    Los atajos solo funcionan en la página Directo y los estilos sin modificador se desactivan cuando un campo de texto tiene el foco. Si no hay una escena asignada a un dígito, el texto lo indicará sin cambiar de escena.

help-audio-title = Audio: la página Mezclador y los medidores
help-audio-subtitle = Qué indican las barras de colores
help-audio-body =
    Las tarjetas de audio aparecen en Directo y, con más espacio y controles, en la página Mezclador. Primero se muestran los dispositivos de audio globales de OBS y después las fuentes con audio de la escena actual, incluidas las fuentes de escenas anidadas y grupos.
    Los modos del Mezclador determinan el audio de qué escena está viendo. Activo sigue la escena de programa de OBS. Seleccionado carga una escena y permanece en ella. Fijado mantiene una escena elegida como objetivo permanente mientras OBS avanza.
    El medidor junto a cada regulador va de -60 dB abajo a 0 dB arriba y usa los umbrales de OBS: verde por debajo de -20 dB para música y fondo, amarillo de -20 a -9 dB para voz y rojo por encima de -9 dB, donde empieza la saturación. Nada debe permanecer en rojo.
    Una columna indica una fuente mono; dos indican estéreo, izquierda y después derecha. Si solo se mueve la columna izquierda, la mitad de sus espectadores no oirá esa fuente.
    La línea sobre el relleno muestra el pico más alto de los últimos veinte segundos, la forma más rápida de detectar una saturación que se le haya escapado. El cuadrado de la parte inferior muestra el nivel que llega del dispositivo antes del regulador; si es demasiado alto, mover el regulador no lo corregirá.
    El botón de bloqueo de una tarjeta solo inmoviliza el control deslizante de SceneDeck. No bloquea nada en OBS.

help-outputs-title = Iniciar y detener la emisión y la grabación
help-outputs-subtitle = Y las confirmaciones que evitan accidentes
help-outputs-body =
    Los botones Iniciar/Detener emisión e Iniciar/Detener grabación están en la parte inferior de la barra lateral y se puede acceder a ellos desde cualquier página. La barra de estado muestra el estado y cuánto tiempo llevan activos.
    Preferencias → Seguridad de salida determina cuáles de las cuatro acciones piden confirmación. De forma predeterminada, se confirma la detención de cualquiera de las salidas, pero no su inicio: se asume que empezar antes de tiempo cuesta poco y detenerse antes de tiempo termina la emisión.
    Los cambios realizados en OBS también aparecen aquí: SceneDeck sigue los eventos de OBS en lugar de dar por hecho que su botón ha funcionado.

help-group-inspect-title = Comprobación de la configuración
help-group-inspect-description = Detecte la sorpresa antes de que ocurra en directo.

help-doctor-title = Diagnóstico
help-doctor-subtitle = Problemas estructurales ordenados por gravedad
help-doctor-body =
    Diagnóstico lee la lista de escenas, las asignaciones de roles y el anidamiento, y clasifica lo que parece incorrecto como Errores, Advertencias e Información.
    Entre los hallazgos habituales están las escenas sin rol, las escenas recordadas que ya no existen en OBS, los anidamientos circulares y las escenas anidadas en una dirección que sus roles no permiten.
    Se vuelve a ejecutar cada vez que abre la página, y el botón Volver a ejecutar lo fuerza.
    Conviene revisarlo después de cualquier cambio en la configuración de OBS y una vez más antes de emitir.

help-graph-title = Grafo
help-graph-subtitle = Qué escenas contiene cada escena
help-graph-body =
    Anidar una escena dentro de otra permite crear superposiciones y diseños compartidos en OBS, pero también puede hacer que una escena dependa de algo que había olvidado.
    Grafo enumera cada escena principal y su contenido, y compara cada relación con las reglas de sus roles: correcta, dudosa o prohibida.
    Úselo para responder «¿qué se romperá si cambio esta escena?» antes de modificarla.

help-stats-title = Estadísticas
help-stats-subtitle = Si el equipo mantiene el ritmo
help-stats-body =
    Los indicadores de FPS, tiempo de renderizado por fotograma, fotogramas perdidos y congestión de red se vuelven ámbar y después rojos a medida que empeora cada valor: los fotogramas perdidos avisan al 1 % y la congestión al 30 %.
    Los gráficos de tendencias conservan aproximadamente los últimos dos minutos y los gráficos de barras muestran cuándo se perdieron fotogramas, no un total acumulado, para distinguir un mal momento aislado de una tendencia.
    Las muestras se recogen durante todo el tiempo que permanece conectado, por lo que abrir Estadísticas a mitad de una emisión muestra los minutos anteriores en vez de empezar vacío.
    Los contadores de fotogramas proceden de OBS y se restablecen cuando se reinicia OBS o la salida de emisión.

help-group-personalise-title = Personalización
help-group-personalise-description = Apariencia, idioma y ubicación de sus archivos.

help-appearance-title = Temas y apariencia
help-appearance-subtitle = Incluido un aspecto acorde con el propio OBS
help-appearance-body =
    El esquema de color sigue de forma predeterminada la preferencia clara u oscura del escritorio, aunque puede forzar una de ellas.
    Los temas son familias adaptadas a los modos claro y oscuro: elija una y se aplicará la variante correspondiente al esquema de color actual. La familia OBS imita el aspecto de OBS Studio para que la superficie de control no desentone con la aplicación que controla.
    El CSS personalizado admite archivos claros y oscuros separados, por lo que también sigue el esquema de color. Recargar CSS personalizado aplica los cambios sin reiniciar.
    Movimiento controla cuánto se anima la interfaz; establézcalo en Reducido o Desactivado si el movimiento distrae o el equipo está ocupado.

help-files-title = Dónde guarda SceneDeck sus datos
help-files-subtitle = Configuración, registro y contraseña de OBS
help-files-body =
    Los ajustes, incluidos el anfitrión y el puerto de OBS, se guardan en $XDG_CONFIG_HOME/scenedeck/config.json, normalmente ~/.config/scenedeck/config.json.
    Los roles, el orden, los acentos y los iconos de las escenas se guardan en registry.json en la misma carpeta.
    La contraseña de OBS no aparece en ninguno de esos archivos. Se almacena en el depósito de claves Secret Service del escritorio, la misma caja fuerte que usa el navegador.
    Haga una copia de seguridad de ambos archivos JSON para trasladar una configuración completa a otro equipo, o use la exportación YAML de Inventario solo para la parte de las escenas.

help-shortcuts-title = Atajos de teclado
help-shortcuts-subtitle = La lista completa
help-shortcuts-body =
    F1 — abrir esta guía.
    Ctrl+R — volver a conectarse a OBS.
    Ctrl+, — abrir Preferencias.
    Ctrl+Q — salir de SceneDeck.
    Ctrl+1 … Ctrl+0 en la página Directo — cambiar a una de las diez primeras tarjetas de escena. La combinación se puede configurar en Preferencias → Atajos de escena.

## ui/window.rs — diálogo de bienvenida del primer inicio
welcome-dialog-heading = Bienvenido a SceneDeck
welcome-dialog-body = Parece que esta es la primera vez que inicia la aplicación. La página Ayuda explica cómo conectarse a OBS —también si OBS está en otro equipo— y cómo limitar la página Directo a las escenas que realmente usa. Solo lleva un par de minutos y evita algunos errores.
welcome-dialog-later = Ahora no
welcome-dialog-open = Leer la guía
window-help-tooltip = Guía de ayuda e introducción
