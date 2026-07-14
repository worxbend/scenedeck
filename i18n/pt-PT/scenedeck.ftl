## Cadeias de texto da interface do SceneDeck (Português Europeu).
##
## Agrupadas pelo módulo em que cada mensagem é utilizada. Os identificadores
## das mensagens têm o prefixo do nome do módulo para se manterem inequívocos
## neste ficheiro único partilhado.

## Interno — utilizado apenas pelo teste de regressão do carregador de i18n,
## não é apresentado na interface. Cada idioma tem de definir esta mensagem
## para que o teste de verificação confirme que o pacote do idioma foi
## carregado (e não apenas o valor de recurso `en`).
i18n-loader-smoke-test = Idioma carregado.

## infra/error.rs — representações destinadas ao utilizador de AppError.
## `detail` é texto original vindo do OBS ou do sistema operativo e nunca é
## traduzido.
error-connection = Falha na ligação ao OBS: { $detail }
error-request = Falha no pedido ao OBS: { $detail }
error-config = Erro de configuração: { $detail }
error-storage = Erro de armazenamento: { $detail }
error-notification-title = Erro do SceneDeck: { $message }

## domain/audio.rs — AudioSourceScope::label()
audio-scope-global = Global
audio-scope-active = Cena
audio-scope-nested = Aninhado
audio-scope-group = Grupo

## domain/graph.rs — EdgeStatus summary label/tooltip
edge-status-ok-label = OK
edge-status-warning-label = Aviso
edge-status-forbidden-label = Proibido
edge-status-ok-tooltip = Ligações que cumprem a política do grafo
edge-status-warning-tooltip = Ligações fora de uma lista de permissões
edge-status-forbidden-tooltip = Ligações proibidas pela política do grafo

## domain/output.rs — OutputRunState::label(), OutputStatus::summary()
output-state-inactive = Inativo
output-state-starting = A iniciar
output-state-active = Ativo
output-state-stopping = A parar
output-state-reconnecting = A religar
output-state-paused = Em pausa
output-state-unknown = Desconhecido
output-summary = { $name }: { $state }

## domain/role.rs — SceneRole::label()/description()
role-primary = Principal
role-secondary = Secundária
role-module = Módulo
role-raw = Bruta
role-debug = Depuração
role-archive = Arquivo
role-unassigned = Sem atribuição
role-primary-desc = Cena comutável em direto
role-secondary-desc = Cena válida, oculta do Direto por predefinição
role-module-desc = Cena aninhada reutilizável, não comutável diretamente
role-raw-desc = Cena invólucro de hardware ou de fonte
role-debug-desc = Cena de teste temporária
role-archive-desc = Preservada mas excluída de todos os fluxos de trabalho

## domain/mixer.rs — MixerMode / MixerGrouping labels
mixer-mode-active = Ativo
mixer-mode-selected = Selecionado
mixer-mode-pinned = Fixado
mixer-mode-active-desc = Seguir a cena de emissão do OBS.
mixer-mode-selected-desc = Inspecionar a cena selecionada sem seguir o OBS.
mixer-mode-pinned-desc = Manter a cena selecionada estável durante a operação.
mixer-grouping-scope = Âmbito
mixer-grouping-scene-path = Caminho da Cena
mixer-grouping-none = Nenhum

## domain/diagnostic.rs — DiagnosticSeverity label/count
diag-label-info = Informação
diag-label-warning = Avisos
diag-label-error = Erros
diag-count-info = { $count ->
    [one] { $count } item de informação
   *[other] { $count } itens de informação
}
diag-count-warning = { $count ->
    [one] { $count } aviso
   *[other] { $count } avisos
}
diag-count-error = { $count ->
    [one] { $count } erro
   *[other] { $count } erros
}

## ui/pages/inventory.rs
inventory-no-role-assigned = Sem função atribuída

## services/doctor_service.rs — diagnostic message/suggestion templates
doctor-no-role = A cena não tem função atribuída no registo local.
doctor-no-role-suggestion = Abra o Inventário e atribua uma função.
doctor-stale-entry = A entrada do registo refere-se a uma cena não encontrada no OBS.
doctor-stale-entry-suggestion = Remova a entrada do Inventário.
doctor-protected-switchable = A cena protegida está na função comutável '{ $role }'.
doctor-protected-switchable-suggestion = As cenas protegidas são normalmente blocos de construção; considere Módulo ou Bruta.
doctor-cycle = Referência circular de cenas envolvendo '{ $parent }' e '{ $child }'.
doctor-cycle-suggestion = Remova o ciclo de cenas aninhadas; o OBS não consegue processar ciclos.
doctor-edge-primary-debug = A cena Principal depende de uma cena de Depuração. (→ '{ $child }')
doctor-edge-primary-debug-suggestion = Remova a cena de Depuração do caminho em direto antes de entrar em direto.
doctor-edge-primary-raw = A cena Principal envolve diretamente uma fonte Bruta. (→ '{ $child }')
doctor-edge-primary-raw-suggestion = Envolva a fonte Bruta numa cena Módulo para reutilização e clareza.
doctor-edge-module-primary = O Módulo depende de uma cena Principal, invertendo a hierarquia. (→ '{ $child }')
doctor-edge-module-primary-suggestion = Os módulos devem ser blocos de construção, não consumidores de cenas Principais.
doctor-edge-raw-nests = A cena Bruta aninha outra cena. (→ '{ $child }')
doctor-edge-raw-nests-suggestion = As cenas Brutas devem ser invólucros de fonte finais, sem cenas aninhadas.
doctor-edge-forbidden = A dependência de cena é proibida pela política do grafo. (→ '{ $child }')
doctor-edge-outside-policy = A dependência de cena está fora da política do grafo configurada. (→ '{ $child }')
doctor-edge-adjust-suggestion = Ajuste a relação de cena aninhada ou atualize as regras do grafo no registo.

## controller/app_controller.rs
controller-not-connected = Sem ligação ao OBS

## controller/state.rs — Page titles and ObsStatus labels
page-live = Direto
page-mixer = Mesa de Mistura
page-graph = Grafo
page-inventory = Inventário
page-doctor = Diagnóstico
page-settings = Definições
obs-status-disconnected = Desligado
obs-status-connecting = A ligar…
obs-status-connected = Ligado
obs-status-error = Erro

## storage/config.rs — ConfigStartupNotice
config-first-launch = Ainda não existem definições guardadas. Foram carregadas as predefinições.
config-read-failed = Não foi possível ler as definições: { $detail }
config-parse-failed = Não foi possível processar as definições: { $detail }

## graph.rs

graph-empty-title = Sem Dependências
graph-empty-description = Nenhuma cena aninha outras cenas, ou o OBS não está ligado. Ligue-se e adicione fontes de cena aninhadas para ver o grafo de dependências.
graph-page-title = Dependências de Cenas
graph-reset-tooltip = Repor disposição do grafo
graph-edge-summary-count = { $label } { $count }

## mixer.rs

## Page chrome / empty state
mixer-empty-title = Sem Dados da Mesa de Mistura
mixer-empty-description = Ligue-se ao OBS para carregar cenas e fontes de áudio.
mixer-page-title = Mesa de Mistura
mixer-controls-title = Controlos da Mesa de Mistura
mixer-summary-title = Fonte Atual da Mesa de Mistura

## Control rows (ComboRow / EntryRow titles+subtitles)
mixer-mode-row-title = Modo
mixer-mode-row-subtitle = Ativo segue o OBS; Selecionado e Fixado mantêm a cena escolhida estável.
mixer-scene-row-title = Cena
mixer-scene-row-subtitle = Utilizado pelos modos Selecionado e Fixado.
mixer-grouping-row-title = Agrupar Por
mixer-grouping-row-subtitle = Controla a forma como as fontes de áudio são organizadas abaixo.
mixer-search-row-title = Pesquisar

## Scene-loading / no-scene placeholders
mixer-no-scene-title = Nenhuma Cena Selecionada
mixer-no-scene-description = Escolha uma cena para carregar o respetivo áudio da mesa de mistura.
mixer-loading-title = A Carregar Áudio da Mesa de Mistura
mixer-loading-description = A carregar fontes de áudio para { $scene }.

## Audio-source empty states
mixer-current-scene-fallback = A cena atual
mixer-no-audio-sources-title = Sem Fontes de Áudio
mixer-no-audio-sources-description = { $scene } não tem fontes de áudio configuradas no OBS correspondentes.
mixer-no-matching-title = Sem Fontes de Áudio Correspondentes
mixer-no-matching-description = Ajuste o filtro de pesquisa para mostrar as fontes de áudio disponíveis.

## Group titles
mixer-group-all-sources = Todas as Fontes
mixer-group-global-fallback = Global

## Error placeholder + retry
mixer-error-title = Áudio da Mesa de Mistura Indisponível
mixer-error-description = Não foi possível carregar as fontes de áudio para { $scene }: { $message }
mixer-retry-button-label = Repetir
mixer-retry-button-tooltip = Repetir o carregamento do áudio da mesa de mistura

## Current-source summary row (source_summary / scene_target_summary)
mixer-summary-following-active = A seguir a cena ativa do OBS: { $scene }
mixer-summary-no-scene-selected = Nenhuma cena selecionada
mixer-summary-selected-scene = Cena selecionada: { $scene }
mixer-summary-pinned-scene = Cena fixada: { $scene }
mixer-summary-selected-fallback = Cena selecionada não definida; a utilizar a cena ativa do OBS: { $scene }
mixer-summary-pinned-selected-fallback = Cena fixada não definida; a utilizar a cena selecionada: { $scene }
mixer-summary-pinned-active-fallback = Cenas fixada e selecionada não definidas; a utilizar a cena ativa do OBS: { $scene }

## doctor.rs — page chrome, empty state, and all-clear text for the Doctor
## page. Diagnostic finding messages themselves (`doctor-no-role`,
## `doctor-cycle*`, `doctor-edge-*`, etc.) already exist in
## src/services/doctor_service.rs and are NOT redefined here.
doctor-page-title = Diagnóstico
doctor-empty-state-title = Nada a Verificar
doctor-empty-state-description = Ligue-se ao OBS para executar diagnósticos de arquitetura.
doctor-summary-row-title = Diagnósticos
doctor-rerun-tooltip = Executar diagnósticos novamente
doctor-all-clear-title = Nenhum problema encontrado
doctor-all-clear-detail = A arquitetura de cenas satisfaz todas as verificações.

## inventory.rs — page chrome, group titles, YAML import/export UI, and
## status messages. Role names/descriptions themselves are NOT redefined
## here — they already exist in src/domain/role.rs (`role-*`,
## `role-*-desc`, `role-unassigned`) and are reused directly via
## SceneRole::label()/description()/unassigned_label(). The already-present
## `inventory-no-role-assigned` message is also left untouched.
inventory-page-title = Inventário
inventory-empty-state-title = Sem Cenas
inventory-empty-state-description = Ligue-se ao OBS para carregar a lista de cenas.
inventory-scenes-group-title = Cenas do OBS
inventory-scenes-group-description = Atribua funções para controlar quais as cenas apresentadas na página Direto.
inventory-stale-group-title = Entradas de Registo Desatualizadas
inventory-stale-group-description = Estas cenas estão no seu registo local mas já não existem no OBS.
inventory-remove-stale-tooltip = Remover entrada desatualizada
inventory-yaml-row-title = YAML do Registo de Cenas
inventory-yaml-row-subtitle = Exporte ou importe funções de cenas, etiquetas, sinalizadores de proteção e regras do grafo.
inventory-yaml-filter-name = Ficheiros YAML

# Shared between the row's Export button, the export FileChooserNative's
# accept label, and (for Cancel) both the export and import dialogs.
inventory-export-button-label = Exportar
inventory-export-tooltip = Exportar registo de cenas para YAML
inventory-import-button-label = Importar
inventory-import-tooltip = Importar registo de cenas a partir de YAML
inventory-dialog-cancel-label = Cancelar

inventory-export-dialog-title = Exportar Registo de Cenas
inventory-export-success = Registo de cenas exportado para { $path }.
inventory-export-error = Falha na exportação: { $error }
inventory-export-no-file = Falha na exportação: nenhum ficheiro foi selecionado.

inventory-import-dialog-title = Importar Registo de Cenas
inventory-import-error = Falha na importação: { $error }
inventory-import-no-file = Falha na importação: nenhum ficheiro foi selecionado.

## window.rs

window-stream-live-tooltip = Emissão em direto
window-about-tooltip = Acerca do SceneDeck
window-refresh-tooltip = Atualizar página atual

window-stream-status-line = Emissão: { $state }{ $elapsed }
window-record-status-line = Gravação: { $state }{ $elapsed }

window-status-connecting = A ligar ao OBS…
window-connect-btn-connecting = A ligar…
window-current-scene-none = Cena atual: —
window-status-connected = Ligado — OBS { $version }
window-connect-btn-disconnect = Desligar
window-status-disconnected = Desligado
window-connect-btn-connect = Ligar ao OBS
window-live-disconnected-hint = Ligue-se ao OBS para utilizar os controlos de Direto
window-current-scene = Cena atual: { $scene }
window-status-error = Erro: { $error }
window-connect-btn-retry = Repetir
window-obs-connection-failed = Falha na ligação ao OBS
window-toast-obs-error = Erro do OBS: { $error }

window-output-kind-stream = Emissão
window-output-kind-record = Gravação

window-sidebar-output-starting = A iniciar…
window-sidebar-output-stopping = A parar…
window-sidebar-output-reconnecting = A religar…
window-sidebar-output-working = A processar…

window-sidebar-start-stream = Iniciar Emissão
window-sidebar-stop-stream = Parar Emissão
window-sidebar-start-recording = Iniciar Gravação
window-sidebar-stop-recording = Parar Gravação

window-selector-profile-label = Perfil
window-selector-profile-tooltip = Mudar de perfil do OBS
window-selector-collection-label = Coleção
window-selector-collection-tooltip = Mudar de coleção de cenas do OBS

## live.rs

live-start-stream-label = Iniciar Emissão
live-stop-stream-label = Parar Emissão
live-start-record-label = Iniciar Gravação
live-stop-record-label = Parar Gravação
live-stream-toggle-tooltip = Iniciar ou parar a emissão
live-record-toggle-tooltip = Iniciar ou parar a gravação
live-stream-inactive-label = Emissão: Inativa
live-record-inactive-label = Gravação: Inativa
live-copy-last-recording-path-tooltip = Copiar caminho da última gravação
live-copied-recording-path-tooltip = Caminho da última gravação copiado
live-copy-recording-path-with-value-tooltip = Copiar caminho da gravação: { $path }
live-stream-card-title = Emissão
live-recording-card-title = Gravação
live-current-scene-placeholder = Cena atual: —
live-scenes-section-label = Cenas
live-scenes-connect-hint = Ligue-se ao OBS para carregar cenas.
live-audio-section-label = Áudio
live-disconnected-title = Ligue-se ao OBS para utilizar os controlos de Direto
live-disconnected-detail = Utilize o controlo de ligação na parte inferior da barra lateral.
live-stream-command-error-label = Falha no comando de emissão
live-recording-command-error-label = Falha no comando de gravação
live-last-recording-detail = Última gravação: { $path }
live-starting-stream = A iniciar emissão…
live-stopping-stream = A parar emissão…
live-reconnecting-stream = A religar emissão…
live-starting-recording = A iniciar gravação…
live-stopping-recording = A parar gravação…
live-reconnecting-recording = A religar gravação…
live-button-starting = A iniciar…
live-button-stopping = A parar…
live-button-reconnecting = A religar…
live-button-working = A processar…
live-output-kind-stream = Emissão
live-output-kind-record = Gravação
live-output-label = { $kind }: { $state }
live-output-label-with-elapsed = { $kind }: { $state } · { $elapsed }
live-scenes-no-primary-hint = Não foram encontradas cenas com função Principal. Atribua funções no Inventário.
live-audio-empty-hint = Nenhuma entrada de áudio configurada.
live-cancel-button-label = Cancelar
live-start-stream-confirm-heading = Iniciar Emissão?
live-start-stream-confirm-body = O OBS irá começar a enviar a emissão em direto.
live-stop-stream-confirm-heading = Parar Emissão?
live-stop-stream-confirm-body = O OBS irá parar de enviar a emissão em direto.
live-start-recording-confirm-heading = Iniciar Gravação?
live-start-recording-confirm-body = O OBS irá iniciar uma nova gravação.
live-start-recording-confirm-label = Iniciar Gravação
live-stop-recording-confirm-heading = Parar Gravação?
live-stop-recording-confirm-body = O OBS irá parar a gravação atual.
live-stop-recording-confirm-label = Parar Gravação

## audio_card.rs
audio-card-mute-tooltip = Silenciar entrada
audio-card-source-path-tooltip = { $scope }: { $path }
audio-card-fader-tooltip = Regulador de volume
audio-card-lock-tooltip = Bloquear regulador de volume
audio-card-fine-plus-tooltip = +1 dB
audio-card-fine-reset-tooltip = Repor a 0,0 dB
audio-card-fine-minus-tooltip = -1 dB

## scene_card.rs
scene-card-tooltip = { $status } ({ $role })
scene-card-role-suffix = Cena { $role }

## status_bar.rs
status-bar-stream-inactive = Emissão: Inativa
status-bar-record-inactive = Gravação: Inativa
status-bar-fps-placeholder = FPS —
status-bar-cpu-placeholder = CPU —
status-bar-bitrate-placeholder = Débito —
status-bar-fps = FPS { $value }
status-bar-cpu = CPU { $value }%
status-bar-bitrate = Débito { $value } kbps
status-bar-dropped = { $count } perdidas

## settings.rs

settings-page-title = Definições
settings-appearance-title = Aparência
settings-appearance-description = As aplicações GNOME devem seguir o estilo do sistema por predefinição.
settings-theme-mode-system = Sistema
settings-theme-mode-light = Claro
settings-theme-mode-dark = Escuro
settings-color-scheme-title = Esquema de Cores
settings-color-scheme-subtitle = Seguir a preferência do sistema ou forçar claro / escuro
settings-theme-title = Tema
settings-theme-status-title = Estado do Tema
settings-theme-status-initial = Tema carregado.
settings-failed-to-save = Falha ao guardar: { $err }
settings-custom-css-title = CSS Personalizado
settings-custom-css-subtitle = Carregar ficheiros CSS separados para os modos claro e escuro
settings-custom-light-css-title = Caminho do CSS Claro Personalizado
settings-custom-dark-css-title = Caminho do CSS Escuro Personalizado
settings-reload-css-title = Recarregar CSS Personalizado
settings-reload-css-subtitle = Reaplicar o tema selecionado e o ficheiro CSS personalizado claro/escuro correspondente.
settings-reload-button = Recarregar
settings-language-title = Idioma
settings-language-description = As alterações têm efeito após reiniciar o SceneDeck.
settings-display-language-title = Idioma de Apresentação
settings-display-language-subtitle = Escolha um idioma, ou siga a definição regional do sistema.
settings-language-status-title = Estado do Idioma
settings-language-status-initial = Reinicie para aplicar uma alteração de idioma.
settings-language-saved = Idioma guardado. Reinicie o SceneDeck para o aplicar.
settings-obs-connection-title = Ligação ao OBS
settings-obs-connection-description = Definições WebSocket para o OBS Studio (porta predefinida: 4455).
settings-host-title = Anfitrião
settings-port-title = Porta
settings-password-title = Palavra-passe (opcional)
settings-obs-status-title = Estado do OBS
settings-invalid-port = Número de porta inválido.
settings-saved = Definições guardadas.
settings-password-saved = Palavra-passe guardada no chaveiro.
settings-keyring-error = Erro no chaveiro: { $err }
settings-output-safety-title = Segurança de Saída
settings-output-safety-description = Confirmações opcionais para ações críticas de emissão e gravação.
settings-confirm-start-stream-title = Confirmar Início de Emissão
settings-confirm-start-stream-subtitle = Perguntar antes de iniciar a emissão em direto.
settings-confirm-stop-stream-title = Confirmar Paragem de Emissão
settings-confirm-stop-stream-subtitle = Perguntar antes de parar a emissão em direto.
settings-confirm-start-recording-title = Confirmar Início de Gravação
settings-confirm-start-recording-subtitle = Perguntar antes de iniciar uma gravação.
settings-confirm-stop-recording-title = Confirmar Paragem de Gravação
settings-confirm-stop-recording-subtitle = Perguntar antes de parar uma gravação.
settings-obs-not-connected = Sem ligação ao OBS.
settings-obs-connecting = A ligar ao OBS…
settings-obs-connected = Ligado — OBS { $version }
settings-obs-error = Erro: { $err }
settings-theme-subtitle = { $description } Amostras: { $swatches }
settings-theme-loaded = { $theme } carregado ({ $variant }).
settings-theme-loaded-with-warnings = Tema carregado com avisos.

## theme.rs

theme-adwaita-default-name = Adwaita Predefinido
theme-adwaita-default-desc = Estilo neutro que segue as predefinições do GNOME.
theme-scenedeck-dark-name = SceneDeck Escuro
theme-scenedeck-dark-desc = Um tema de consola escuro reservado para operação em direto.
theme-scenedeck-light-name = SceneDeck Claro
theme-scenedeck-light-desc = Um tema de consola claro e nítido com contraste contido.
theme-obsidian-name = Obsidiana
theme-obsidian-desc = Superfícies de grafite de alta legibilidade com acentos frios.
theme-nord-name = Nord
theme-nord-desc = Superfícies azul-acinzentadas frias com acentos em tons gelados.
theme-dracula-inspired-name = Inspirado em Dracula
theme-dracula-inspired-desc = Uma paleta escura e expressiva com CSS original.
theme-solarized-dark-name = Solarized Escuro
theme-solarized-dark-desc = Contraste de baixo brilho com acentos turquesa e âmbar.
theme-high-contrast-name = Alto Contraste
theme-high-contrast-desc = Contornos e contraste mais fortes para controlos críticos.
theme-stream-red-name = Vermelho Emissão
theme-stream-red-desc = Acentos vermelhos orientados para transmissão em estados de direto.
theme-studio-purple-name = Roxo Estúdio
theme-studio-purple-desc = Acentos roxos controlados sem sobrecarregar as superfícies.
theme-ubuntu-violet-name = Violeta Ubuntu
theme-ubuntu-violet-desc = Superfícies violeta inspiradas no Ubuntu com um acento quente para o direto.
theme-custom-css-read-failed = Não foi possível ler o CSS personalizado a partir de { $path }: { $err }
theme-custom-css-no-matching-file = O CSS personalizado está ativado mas não existe ficheiro claro/escuro correspondente definido.
theme-css-no-display = { $label } não foi carregado porque não existe nenhum ecrã GTK disponível.
theme-css-parse-error = Erro ao processar CSS de { $label }: { $message }
