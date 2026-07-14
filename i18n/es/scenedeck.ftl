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

## scene_card.rs
scene-card-tooltip = { $status } ({ $role })
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

## settings.rs

settings-page-title = Preferencias
settings-appearance-title = Apariencia
settings-appearance-description = Las aplicaciones de GNOME deberían seguir el estilo del sistema de forma predeterminada.
settings-theme-mode-system = Sistema
settings-theme-mode-light = Claro
settings-theme-mode-dark = Oscuro
settings-color-scheme-title = Esquema de color
settings-color-scheme-subtitle = Seguir la preferencia del sistema o forzar claro / oscuro
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
