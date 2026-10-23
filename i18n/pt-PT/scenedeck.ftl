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
events-resync-warning = Foram perdidas notificações do OBS. O SceneDeck está a ressincronizar; o histórico de saídas pode estar incompleto.

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
page-stats = Estatísticas
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
inventory-accent-clear-tooltip = Limpar a cor de destaque da cena
inventory-drag-tooltip = Arraste para reordenar a cena
inventory-accent-dialog-title = Cor de destaque da cena
inventory-accent-choose-tooltip = Escolha a cor de destaque da cena

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
audio-card-meter-tooltip-title = Medidor de volume: { $channels }
audio-card-meter-tooltip-zones = Verde abaixo de -20 dB · Amarelo até -9 dB · Vermelho acima, perto do corte
audio-card-meter-tooltip-indicators = Barra: nível de pico com queda · Linha: pico mais alto em 20 s · Ponto: intensidade sonora · Base: nível que chega do dispositivo
audio-card-meter-tooltip-waiting = Medidor de volume: à espera dos níveis do OBS

## icon.rs
icon-none = Sem ícone
icon-camera = Câmara
icon-desktop = Ambiente de trabalho
icon-game = Jogo
icon-film = Filme
icon-images = Imagens
icon-television = Televisão
icon-browser = Navegador
icon-terminal = Terminal
icon-code = Código
icon-chat = Conversa
icon-guests = Convidados
icon-star = Estrela
icon-alert = Aviso
icon-break = Pausa
icon-countdown = Contagem decrescente
icon-start = Início
icon-pause = Pausar
icon-stop = Paragem
icon-settings = Definições
icon-layers = Camadas
icon-microphone = Microfone
icon-headset = Auscultadores com microfone
icon-headphones = Auscultadores
icon-speaker = Altifalante
icon-volume = Volume
icon-music = Música
icon-instrument = Instrumento
icon-radio = Rádio
icon-call = Chamada
icon-waveform = Forma de onda
inventory-scene-icon-tooltip = Escolher um ícone para esta cena
mixer-input-icon-tooltip = Escolher um ícone para esta fonte de áudio

## meter.rs
audio-meter-zone-nominal = Verde
audio-meter-zone-warning = Amarelo
audio-meter-zone-error = Vermelho
audio-meter-channel-mono = Mono
audio-meter-channel-left = Esquerdo
audio-meter-channel-right = Direito
audio-meter-channel-front-left = Frontal esquerdo
audio-meter-channel-front-right = Frontal direito
audio-meter-channel-front-center = Frontal central
audio-meter-channel-lfe = LFE
audio-meter-channel-rear-left = Traseiro esquerdo
audio-meter-channel-rear-right = Traseiro direito
audio-meter-channel-side-left = Lateral esquerdo
audio-meter-channel-side-right = Lateral direito
audio-meter-channel-numbered = Canal { $index }

## hotkey.rs
hotkey-modifier-ctrl = Ctrl+
hotkey-modifier-alt = Alt+
hotkey-modifier-shift = Shift+
hotkey-modifier-super = Super+
hotkey-modifier-ctrl-alt = Ctrl+Alt+
hotkey-modifier-ctrl-shift = Ctrl+Shift+
hotkey-style-plain = Apenas o dígito
hotkey-style-ctrl = Ctrl + dígito
hotkey-style-alt = Alt + dígito
hotkey-style-shift = Shift + dígito
hotkey-style-super = Super + dígito
hotkey-style-ctrl-alt = Ctrl+Alt + dígito
hotkey-style-ctrl-shift = Ctrl+Shift + dígito
hotkey-style-leader = Tecla líder e depois dígito
hotkey-leader-space = Espaço
hotkey-leader-comma = Vírgula
hotkey-leader-semicolon = Ponto e vírgula
hotkey-leader-backslash = Barra invertida
hotkey-leader-grave = Acento grave
hotkey-shortcut-leader = { $leader } e depois { $digit }
hotkey-hint-plain = Prima 1 … 0
hotkey-hint-modifier = Prima { $modifier }1 … 0
hotkey-hint-leader = Prima { $leader } e depois 1 … 0
hotkey-hint-leader-armed = Líder ativo — prima 1 … 0
hotkey-hint-empty-slot = Sem cena na posição { $slot }

## scene_card.rs
scene-card-tooltip = { $status } ({ $role })
scene-card-tooltip-with-hotkey = { $status } ({ $role }) · { $hotkey }
scene-card-role-suffix = Cena { $role }
scene-card-tooltip-ready = Mudar para esta cena

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
status-bar-dropped-placeholder = Perdidas —

## settings.rs

settings-page-title = Definições
settings-appearance-title = Aparência
settings-appearance-description = As aplicações GNOME devem seguir o estilo do sistema por predefinição.
settings-theme-mode-system = Sistema
settings-theme-mode-light = Claro
settings-theme-mode-dark = Escuro
settings-color-scheme-title = Esquema de Cores
settings-color-scheme-subtitle = Seguir a preferência do sistema ou forçar claro / escuro
settings-motion-title = Movimento
settings-motion-subtitle = Quanto a interface anima. Os indicadores em direto continuam legíveis em todos os níveis.
settings-motion-full = Completo
settings-motion-reduced = Reduzido
settings-motion-off = Desligado
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
language-system-default = Predefinição do sistema
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
settings-hotkeys-title = Atalhos de Cena
settings-hotkeys-description = Mude as cenas do Direto pelo teclado. As posições seguem a ordem de cenas definida no Inventário.
settings-hotkeys-enabled-title = Ativar Atalhos de Cena
settings-hotkeys-enabled-subtitle = As teclas numéricas mudam os cartões de cena da página Direto, pela ordem dos cartões.
settings-hotkeys-style-title = Combinação de Teclas
settings-hotkeys-style-subtitle = Que teclas mudam de cena. Os dígitos isolados ficam inativos enquanto um campo de texto tem o foco.
settings-hotkeys-leader-title = Tecla Líder
settings-hotkeys-leader-subtitle = Primeira tecla do atalho de duas etapas, ao estilo do vim.
settings-hotkeys-timeout-title = Tempo Limite do Líder
settings-hotkeys-timeout-subtitle = Quanto tempo o líder espera pelo dígito, em milissegundos.
settings-hotkeys-preview-title = Atribuições Atuais
settings-hotkeys-preview-subtitle = { $first } … { $last } mudam as primeiras { $count } cenas do Direto.
settings-hotkeys-preview-disabled = Os atalhos de cena estão desativados.
settings-obs-not-connected = Sem ligação ao OBS.
settings-obs-connecting = A ligar ao OBS…
settings-obs-connected = Ligado — OBS { $version }
settings-obs-error = Erro: { $err }
settings-theme-loaded = { $theme } carregado ({ $variant }).
settings-theme-loaded-with-warnings = Tema carregado com avisos.
theme-variant-light = Claro
theme-variant-dark = Escuro

## theme.rs

theme-adwaita-default-name = Adwaita Predefinido
theme-adwaita-default-desc = Estilo neutro que segue as predefinições do GNOME.
theme-scenedeck-dark-name = SceneDeck Escuro
theme-scenedeck-dark-desc = Um tema de consola escuro reservado para operação em direto.
theme-scenedeck-light-name = SceneDeck Claro
theme-scenedeck-light-desc = Um tema de consola claro e nítido com contraste contido.
theme-obs-name = OBS
theme-obs-desc = O aspeto predefinido do OBS Studio, interpretado com libadwaita.
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
## stats.rs — live streaming telemetry
stats-page-title = Estatísticas da transmissão
stats-page-subtitle = Telemetria em direto consultada ao OBS uma vez por segundo enquanto há ligação.
stats-gauge-fps = FPS
stats-gauge-frame-time = Tempo por fotograma (ms)
stats-gauge-dropped = Fotogramas perdidos
stats-gauge-congestion = Congestão
stats-chart-fps = Fotogramas por segundo
stats-chart-frame-time = Tempo médio de renderização por fotograma (ms)
stats-chart-output-skipped = Fotogramas de saída ignorados por amostra
stats-chart-render-skipped = Fotogramas de renderização perdidos por amostra
stats-card-render-frames = Fotogramas de renderização perdidos
stats-card-output-frames = Fotogramas de saída ignorados
stats-card-stream-frames = Fotogramas perdidos na transmissão
stats-card-frame-time = Tempo médio de renderização por fotograma
stats-card-cpu = Utilização de CPU do OBS
stats-card-memory = Utilização de memória do OBS
stats-card-bitrate = Taxa de bits da transmissão
stats-value-placeholder = —
stats-value-frames = { $skipped } de { $total } ({ $percent } %)
stats-value-ms = { $value } ms
stats-value-percent = { $value } %
stats-value-mb = { $value } MB
stats-value-kbps = { $value } kb/s

## Ajuda, integração inicial e mensagens adicionadas na versão 0.4.
page-help = Ajuda
config-parse-failed-backed-up = Não foi possível processar as definições: { $detail } — o ficheiro foi mantido em { $path } e foram carregadas as predefinições.
stats-popout-button = Abrir as estatísticas numa janela separada

## ui/pages/help.rs — guia de integração inicial. Os corpos têm várias linhas
## de propósito: cada linha é apresentada separadamente num tópico expansível.
help-page-title = Ajuda e introdução
help-hero-title = Bem-vindo ao SceneDeck
help-hero-description = Uma superfície de controlo nativa para Linux destinada ao OBS Studio. Este guia explica a primeira ligação, mostra como limitar a página Direto às cenas que realmente utiliza e descreve todas as páginas da barra lateral. Expanda um tópico para o ler.
help-expand-hint = Selecione um tópico para o expandir.

help-open-settings = Abrir Definições
help-open-inventory = Abrir Inventário
help-open-doctor = Abrir Diagnóstico
help-open-live = Abrir Direto
help-open-mixer = Abrir Mesa de Mistura
help-open-graph = Abrir Grafo
help-open-stats = Abrir Estatísticas

help-group-start-title = Primeiros passos
help-group-start-description = O caminho mais curto desde uma instalação nova até à primeira mudança de cena.

help-quickstart-title = Cinco passos para a primeira mudança de cena
help-quickstart-subtitle = Siga-os por ordem na primeira vez
help-quickstart-body =
    1. No OBS Studio, abra Ferramentas → Definições do servidor WebSocket e selecione «Ativar servidor WebSocket». Deixe o OBS em execução.
    2. Na mesma janela do OBS, prima «Mostrar informações de ligação» e anote a porta do servidor (4455 por predefinição) e a palavra-passe do servidor.
    3. No SceneDeck, abra Definições e preencha Anfitrião, Porta e Palavra-passe. Mantenha 127.0.0.1 como Anfitrião quando o OBS for executado neste mesmo computador.
    4. Prima Ligar na parte inferior da barra lateral. A linha de estado acima do botão fica verde e apresenta «Ligado».
    5. Abra o Inventário e atribua a função «Principal» às cenas que pretende utilizar durante uma transmissão. Essas cenas —e apenas essas— tornam-se cartões na página Direto.

help-concepts-title = Como o SceneDeck interpreta a sua configuração
help-concepts-subtitle = Funções, o registo e aquilo em que nunca toca no OBS
help-concepts-body =
    O SceneDeck nunca muda o nome, elimina ou reordena nada dentro do OBS. Lê as cenas através da ligação WebSocket do OBS e guarda as suas próprias notas sobre elas.
    Uma «função» é uma dessas notas: uma etiqueta sua que indica para que serve uma cena, como a cena que coloca no ar, uma sobreposição reutilizável ou uma antiga cena de teste.
    Essas notas ficam em registry.json junto ao ficheiro de configuração, pelo que sobrevivem aos reinícios e podem ser exportadas e transferidas para outro computador a partir da página Inventário.
    Como as notas são locais, duas pessoas podem partilhar uma configuração do OBS e cada uma manter uma página Direto diferente.

help-group-connect-title = Ligação ao OBS
help-group-connect-description = Neste computador ou do outro lado da sala.

help-connect-local-title = Ligação ao OBS neste computador
help-connect-local-subtitle = O caso predefinido: anfitrião 127.0.0.1, porta 4455
help-connect-local-body =
    127.0.0.1 é o endereço que um computador utiliza para comunicar consigo próprio, pelo que é o Anfitrião correto quando o OBS e o SceneDeck são executados lado a lado.
    A porta tem de corresponder à porta do servidor nas Definições do servidor WebSocket do OBS. O OBS utiliza 4455, a menos que a tenha alterado.
    Se «Ativar autenticação» estiver selecionado no OBS, cole a palavra-passe em Definições → Palavra-passe. O SceneDeck guarda-a no chaveiro do ambiente de trabalho (o mesmo local onde o navegador guarda os inícios de sessão), nunca no ficheiro de configuração em texto simples.
    Prima Ligar na barra lateral ou prima Ctrl+R em qualquer momento para voltar a ligar.

help-connect-remote-title = Ligação ao OBS noutro computador
help-connect-remote-subtitle = PC de transmissão num canto, controlo a partir do portátil
help-connect-remote-body =
    Esta é a configuração com dois computadores: o OBS é executado no computador que captura e codifica, e o SceneDeck no computador à sua frente. Ambos têm de estar na mesma rede.
    No computador com o OBS, em Ferramentas → Definições do servidor WebSocket, selecione «Ativar servidor WebSocket» e «Ativar autenticação» e defina uma palavra-passe que possa escrever. Não desative a autenticação: caso contrário, qualquer pessoa na rede que consiga aceder à porta poderá iniciar e parar a transmissão.
    Encontre o endereço do computador com o OBS nesse mesmo computador. No Linux, execute `ip addr` e procure um endereço como 192.168.1.42; no Windows, execute `ipconfig` e leia o Endereço IPv4; no macOS, encontra-se em Definições do Sistema → Rede. É o endereço do computador, não do OBS.
    Permita a porta na firewall do computador com o OBS. No Linux com ufw, utilize `sudo ufw allow 4455/tcp`; no Windows, permita o OBS na Firewall do Windows Defender para redes privadas.
    Nas Definições do SceneDeck, introduza esse endereço como Anfitrião (por exemplo, 192.168.1.42), mantenha a Porta em 4455 e cole a palavra-passe. Prima Ligar.
    É preferível uma ligação com fios para o computador com o OBS. As mudanças de cena por Wi-Fi funcionam, mas um pacote perdido atrasa a mudança.
    Uma sugestão que pode salvar uma transmissão: reserve um endereço fixo para o computador com o OBS nas definições de DHCP do router, para que o Anfitrião guardado continue a funcionar depois de reiniciar.

help-connect-share-title = Permitir que um coapresentador ou moderador controle o painel
help-connect-share-subtitle = Um segundo SceneDeck ligado à mesma régie
help-connect-share-body =
    O servidor WebSocket do OBS aceita vários clientes em simultâneo, pelo que uma segunda pessoa noutro computador pode executar a sua própria cópia do SceneDeck com a mesma régie: é a configuração do tópico anterior feita duas vezes.
    Não existem inícios de sessão individuais: quem tiver o Anfitrião, a Porta e a palavra-passe nas suas Definições tem exatamente o mesmo controlo que o utilizador, incluindo iniciar e parar a transmissão. Partilhe-os apenas com alguém a quem confiaria a sua sessão do OBS.
    Limite primeiro aquilo que a outra pessoa verá. As funções e as cenas ocultas ficam no Inventário local, não no OBS; configure-as no seu computador e peça ao colaborador que exporte ou recrie o YAML do Registo de Cenas em vez de começar com uma lista completa e não filtrada.
    Não encaminhe a porta WebSocket para a Internet para dar acesso a um colaborador remoto. Coloquem ambos os computadores na mesma VPN ou rede Tailscale, ou criem um túnel SSH e indiquem como Anfitrião o endereço do túnel.
    Todas as pessoas ligadas veem o mesmo estado em direto: uma mudança de cena ou de regulador feita de qualquer lado aparece imediatamente em ambos, tal como se estivessem ao mesmo teclado.

help-connect-trouble-title = Quando Ligar não funciona
help-connect-trouble-subtitle = Leia o erro e percorra esta lista
help-connect-trouble-body =
    «Ligação recusada» significa quase sempre que o servidor WebSocket não está ativado no OBS ou que a porta não corresponde. Verifique ambos em Ferramentas → Definições do servidor WebSocket.
    Uma ligação que fica pendente e depois excede o tempo limite indica normalmente que uma firewall está a bloquear o tráfego ou que o endereço do Anfitrião pertence a outro computador.
    «Falha na autenticação» significa que a palavra-passe está errada. Introduza-a novamente nas Definições; o campo é apenas de escrita, pelo que parece vazio mesmo quando existe uma palavra-passe guardada.
    Não aparece nada no estado da barra lateral? Confirme que o OBS está realmente em execução e que não está bloqueado por uma janela de diálogo modal própria.
    O SceneDeck volta a ligar automaticamente depois de perder a ligação, e Ctrl+R força uma tentativa imediata.

help-group-scenes-title = Organização das cenas
help-group-scenes-description = A página Direto deve mostrar aquilo para que muda, e nada mais.

help-scenes-hide-title = Ocultar cenas que nunca coloca no ar
help-scenes-hide-subtitle = A definição mais útil para configurar primeiro
help-scenes-hide-body =
    Uma configuração do OBS em utilização acumula cenas que existem apenas para serem aninhadas noutras ou que foram criadas para um teste e nunca eliminadas. Apresentá-las numa superfície de controlo em direto facilita uma mudança errada.
    O SceneDeck só apresenta um cartão em Direto para cenas com a função Principal. Todas as outras funções ficam ocultas, pelo que «ocultar uma cena» significa simplesmente atribuir-lhe qualquer função diferente de Principal.
    Abra o Inventário. Cada cena do OBS tem uma linha com um seletor de função à direita.
    Defina Principal para as poucas cenas que realmente coloca no ar. Para as restantes, escolha: Secundária (uma cena real de que por vezes precisa, mas que não quer em Direto), Módulo (uma sobreposição ou rodapé apenas aninhado noutra cena), Bruta (uma câmara ou captura básica), Depuração (uma cena de teste) ou Arquivo (guardada para mais tarde e afastada).
    As cenas sem função também ficam fora de Direto, pelo que deixá-las Sem atribuição também as oculta. Ainda assim, é melhor atribuir uma função de propósito: a página Diagnóstico assinala as cenas Sem atribuição para que repare nas novas.
    A alteração tem efeito imediato: volte a Direto e o cartão terá desaparecido. Nada mudou no OBS.

help-scenes-order-title = Ordem, cores e ícones
help-scenes-order-subtitle = Torne evidente o cartão certo
help-scenes-order-body =
    Arraste uma cena pela pega à esquerda da respetiva linha no Inventário para definir a ordem. Os cartões de Direto e os atalhos numéricos seguem essa ordem, pelo que o cartão ativado com 1 é o primeiro.
    O seletor da cor de destaque tinge o cartão da cena em Direto. Reserve uma cor forte para as cenas importantes —a cena «estamos em direto» ou a leitura do patrocinador— para que sejam imediatamente visíveis.
    O seletor de ícones à esquerda de cada linha coloca um símbolo no cartão da cena em Direto. Estão disponíveis trinta ícones, além de uma opção «sem ícone» para o remover.
    A ordem, as cores e os ícones são guardados no registo local, não no OBS.

help-scenes-registry-title = Cópia de segurança e transferência da configuração
help-scenes-registry-subtitle = A linha YAML do Registo de Cenas no Inventário
help-scenes-registry-body =
    Exportar grava funções, ordem, cores de destaque, ícones, etiquetas e regras do grafo num único ficheiro YAML, um formato de texto simples que pode ler e manter sob controlo de versões.
    Importar substitui o registo local pelo conteúdo desse ficheiro. Utilize-o para transferir uma configuração concluída para outro computador ou para a repor depois de uma experiência.
    Os nomes das cenas ligam o ficheiro ao OBS, pelo que uma cena cujo nome seja alterado no OBS reaparece como uma entrada desatualizada. O Inventário apresenta essas entradas e permite removê-las.

help-group-operate-title = Durante uma transmissão
help-group-operate-description = As páginas que realmente utiliza enquanto está no ar.

help-live-title = A página Direto
help-live-subtitle = Parede de cenas, Studio, áudio e saídas
help-live-body =
    Live é uma parede de cenas. Miniaturas das cenas nas Definições adiciona imagens escurecidas e cartões maiores; ao desativar, regressam os mosaicos compactos.
    No modo direto, os cartões e atalhos mudam Programa. O interruptor Studio na barra superior acompanha o OBS: os cartões preparam Pré-visualização e Transição envia-a para Programa. A transição e a duração suportada só aparecem no modo Studio.
    Expanda a barra de áudio abaixo das cenas para mais controlos ou abra Misturador. Transmissão, gravação, buffer de replay e câmara virtual partilham uma barra de saída.
    Fontes na barra superior abre a visibilidade e as definições detalhadas. Inspecionar uma cena aí nunca muda Programa.
    Sem cartões após ligar? Atribua a função Principal no Inventário às cenas que pretende em Live.

help-hotkeys-title = Mudar de cena com o teclado
help-hotkeys-subtitle = Ctrl+1 … Ctrl+0 por predefinição e configurável
help-hotkeys-body =
    Cada um dos primeiros dez cartões de Direto apresenta um pequeno emblema com o respetivo algarismo: o primeiro é 1, o nono é 9 e o décimo é 0. A legenda junto ao título Cenas indica sempre a combinação atual.
    Os números das posições seguem a ordem do Inventário, pelo que reordenar os cartões nessa página também reordena os atalhos.
    Definições → Atalhos de Cena permite escolher como premir o algarismo. Um modificador mais o algarismo (Ctrl por predefinição) evita ativações acidentais enquanto escreve. O algarismo isolado é a opção mais rápida. O estilo de tecla líder funciona como no vim: prima a tecla líder, liberte-a e depois prima o algarismo; enquanto aguarda, a legenda apresenta «Líder ativo».
    Os atalhos funcionam apenas na página Direto e os estilos sem modificador ficam inativos quando um campo de texto tem o foco. Se não houver uma cena associada a um algarismo, a legenda indica-o sem mudar de cena.

help-audio-title = Áudio: a página Mesa de Mistura e os medidores
help-audio-subtitle = O que indicam as barras coloridas
help-audio-body =
    A barra compacta de áudio de Live expande-se para ajustes rápidos; Misturador oferece os faders completos. Primeiro surgem os dispositivos globais de áudio do OBS e depois as fontes da cena atual, incluindo cenas aninhadas e grupos.
    Os modos da Mesa de Mistura determinam o áudio da cena que está a observar. Ativo segue a cena de emissão do OBS. Selecionado carrega uma cena e permanece nela. Fixado mantém uma cena escolhida como destino permanente enquanto o OBS avança.
    O medidor junto de cada regulador vai de -60 dB em baixo a 0 dB em cima e utiliza os limiares do OBS: verde abaixo de -20 dB para música e fundo, amarelo de -20 a -9 dB para a voz e vermelho acima de -9 dB, onde começa a distorção. Nenhum sinal deve permanecer no vermelho.
    Uma coluna indica uma fonte mono; duas indicam estéreo, esquerda e depois direita. Se apenas a coluna esquerda se mover, metade dos espectadores não ouvirá essa fonte.
    A linha acima do preenchimento mostra o pico mais alto dos últimos vinte segundos, a forma mais rápida de detetar uma distorção que lhe tenha escapado. O quadrado no fundo mostra o nível que chega do dispositivo antes do regulador; se for demasiado alto, mover o regulador não o corrigirá.
    O botão de bloqueio de um cartão apenas imobiliza o controlo deslizante do SceneDeck. Não bloqueia nada no OBS.

help-outputs-title = Transmissão, gravação, replay e câmara virtual
help-outputs-subtitle = E as confirmações que evitam um acidente
help-outputs-body =
    A barra de saída de Live reúne transmissão, gravação, buffer de replay e câmara virtual. Mais controlos de saída abre pausa/retoma, divisão do ficheiro/capítulos e resultados recentes.
    Definições → Segurança de saída controla as confirmações para iniciar e parar transmissões e gravações. Parar exige confirmação por predefinição.
    Os eventos e as observações atualizadas do OBS determinam o estado apresentado. Um pedido aceite não prova, por si só, que uma saída arrancou ou que um ficheiro foi guardado. Os caminhos pertencem ao computador do OBS.

help-group-inspect-title = Verificação da configuração
help-group-inspect-description = Encontre a surpresa antes de ela acontecer no ar.

help-doctor-title = Diagnóstico
help-doctor-subtitle = Problemas estruturais ordenados por gravidade
help-doctor-body =
    O Diagnóstico lê a lista de cenas, as atribuições de funções e os aninhamentos, e comunica o que parece errado como Erros, Avisos e Informação.
    Entre os problemas típicos estão cenas sem função, cenas memorizadas que já não existem no OBS, aninhamentos circulares e cenas aninhadas numa direção que as respetivas funções não permitem.
    É executado novamente sempre que abre a página, e o botão Executar novamente força-o de imediato.
    Vale a pena consultá-lo depois de qualquer alteração à configuração do OBS e mais uma vez antes de entrar em direto.

help-graph-title = Grafo
help-graph-subtitle = Que cenas estão dentro de quais
help-graph-body =
    Aninhar uma cena noutra permite criar sobreposições e disposições partilhadas no OBS, mas também pode fazer com que uma cena dependa de algo de que se esqueceu.
    O Grafo apresenta cada cena principal e o que contém, e compara cada relação com as regras das suas funções: correta, questionável ou proibida.
    Utilize-o para responder a «o que deixa de funcionar se eu alterar esta cena?» antes de a modificar.

help-stats-title = Estatísticas
help-stats-subtitle = Se o computador está a acompanhar
help-stats-body =
    Os indicadores de FPS, tempo de renderização dos fotogramas, fotogramas perdidos e congestionamento da rede ficam primeiro âmbar e depois vermelhos à medida que cada valor piora: os fotogramas perdidos avisam a 1% e o congestionamento a 30%.
    Os gráficos de tendências guardam aproximadamente os últimos dois minutos e os gráficos de barras mostram quando se perderam fotogramas, em vez de um total acumulado, para distinguir um único mau momento de uma tendência.
    As amostras são recolhidas durante todo o tempo em que permanece ligado, pelo que abrir Estatísticas a meio de uma transmissão mostra os minutos anteriores em vez de começar vazio.
    Os contadores de fotogramas vêm do OBS e são repostos quando o OBS ou a saída de transmissão reinicia.

help-group-personalise-title = Personalização
help-group-personalise-description = Aspeto, idioma e localização dos ficheiros.

help-appearance-title = Temas e aspeto
help-appearance-subtitle = Incluindo um visual que combina com o próprio OBS
help-appearance-body =
    O esquema de cores segue por predefinição a preferência clara ou escura do ambiente de trabalho, mas pode forçar uma das opções.
    Os temas são famílias adaptadas aos modos claro e escuro: escolha uma e será aplicada a variante correspondente ao esquema de cores atual. A família OBS reproduz o aspeto do OBS Studio, para que a superfície de controlo não destoe da aplicação que controla.
    O CSS personalizado utiliza ficheiros claros e escuros separados, pelo que um visual personalizado também segue o esquema de cores. Recarregar CSS personalizado aplica as alterações sem reiniciar.
    Movimento controla quanto a interface é animada; defina-o como Reduzido ou Desligado se o movimento distrair ou o computador estiver ocupado.

help-files-title = Onde o SceneDeck guarda os dados
help-files-subtitle = Configuração, registo e palavra-passe do OBS
help-files-body =
    As definições, incluindo o anfitrião e a porta do OBS, ficam em $XDG_CONFIG_HOME/scenedeck/config.json, normalmente ~/.config/scenedeck/config.json.
    As funções, a ordem, as cores de destaque e os ícones das cenas ficam em registry.json na mesma pasta.
    A palavra-passe do OBS não consta de nenhum dos ficheiros. É guardada no chaveiro Secret Service do ambiente de trabalho, o mesmo cofre utilizado pelo navegador.
    Faça uma cópia de segurança dos dois ficheiros JSON para transferir uma configuração completa para outro computador, ou utilize a exportação YAML do Inventário apenas para a parte das cenas.

help-shortcuts-title = Atalhos de teclado
help-shortcuts-subtitle = A lista completa
help-shortcuts-body =
    F1 — abrir este guia.
    Ctrl+R — voltar a ligar ao OBS.
    Ctrl+, — abrir Definições.
    Ctrl+Q — sair do SceneDeck.
    Ctrl+1 … Ctrl+0 na página Direto — mudar para um dos primeiros dez cartões de cenas. A combinação pode ser configurada em Definições → Atalhos de Cenas.

## ui/window.rs — janela de boas-vindas da primeira execução
welcome-dialog-heading = Bem-vindo ao SceneDeck
welcome-dialog-body = Parece ser a primeira vez que executa a aplicação. A página Ajuda explica como ligar ao OBS —incluindo quando o OBS está noutro computador— e como limitar a página Direto às cenas que realmente utiliza. Demora apenas alguns minutos e evita alguns erros.
welcome-dialog-later = Agora não
welcome-dialog-open = Ler o guia
window-help-tooltip = Guia de ajuda e introdução

# OBS controls and accessible alternatives
window-stop-disconnect-label = Parar transmissão/gravação e desligar
window-stop-stream-disconnect-label = Parar transmissão e desligar
window-stop-record-disconnect-label = Parar gravação e desligar
disconnect-confirm-heading = Parar a transmissão e a gravação antes de desligar?
disconnect-confirm-body = Transmissão: { $stream }. Gravação: { $recording }. A transmissão e a gravação ativas serão paradas antes de desligar o SceneDeck. Esta ação não para o buffer de repetição nem a câmara virtual.
live-scenes-obs-empty = Esta coleção do OBS não tem cenas. Crie uma cena no OBS e atualize o SceneDeck.
live-organize-scenes = Organizar cenas
mixer-secondary-options = Agrupamento e pesquisa
inventory-move-up = Mover { $scene } para cima
inventory-move-down = Mover { $scene } para baixo
graph-relationship-list = Relações entre cenas — lista de texto
graph-no-dependencies = Sem dependências de cenas aninhadas
audio-source-mute = Silenciar { $source }
audio-source-lock = Bloquear o regulador local de { $source }
audio-source-volume = Volume de { $source }
audio-source-meter = Medidor de pico de áudio de { $source }
audio-meter-no-level = Sem nível visível
audio-meter-no-observation = Sem leitura do nível de áudio

# Session activity

workflow-activity-title = Atividade recente
workflow-activity-detail = Apenas a sessão atual, com as 30 entradas mais recentes. Pedidos, observações e falhas estão separados. As desconexões deixam lacunas; a proximidade temporal não prova causalidade.
workflow-check-item = Visibilidade da fonte
workflow-activity-requested = Solicitado
workflow-activity-observed = Observado
workflow-activity-failed = Falhou
workflow-activity-connection = Ligação


# Native OBS inspector
control-refresh = Atualizar observações do OBS
control-inspection-note = Inspecione qualquer cena sem alterar o Programa. As alterações de visibilidade afetam a cena ou o grupo que contém o elemento.
control-disconnected = Ligue ao OBS para inspecionar fontes.
control-loading = A ler o estado do OBS…
control-observed = Última observação: há { $seconds } s. Atualize antes de confiar neste estado.
control-scene-selector = Cena a inspecionar — não altera o Programa
control-request-hint = Atualize para carregar os controlos nativos do OBS e a visibilidade das fontes.
control-resume = Retomar gravação
control-pause = Pausar gravação
control-split = Dividir ficheiro de gravação
control-replay-stop = Parar buffer de repetição
control-replay-start = Iniciar buffer de repetição
control-replay-save = Guardar repetição
control-camera-stop = Parar câmara virtual
control-camera-start = Iniciar câmara virtual
control-chapter-name = Nome do capítulo da gravação
control-chapter = Criar capítulo
control-host-path = Caminho da última repetição no computador do OBS: { $path }
control-output-controls = Gravação, repetição e câmara virtual
control-source-search = Pesquisar fontes ou caminhos de origem
control-inspect-source = Inspecionar definições e utilização da fonte
control-hidden-parent = Um elemento ascendente está oculto. Ativar apenas este elemento pode não o tornar visível.
control-sources-empty = Não há elementos de cena nesta observação.
control-shared-source = Áudio, filtros, multimédia e texto afetam esta fonte em todos os locais onde é utilizada. A visibilidade é local ao seu contentor.
control-used-in = Utilizada em — caminhos estruturais
control-image-refresh = Capturar imagem fixa
control-image-alt = Imagem fixa de uma cena OBS capturada a pedido
control-image-error = Não foi possível descodificar a imagem do OBS.
control-image-note = Imagem fixa da última captura. Não é vídeo em direto.
control-studio-enable = Ativar modo de estúdio
control-program = Programa: { $scene }
control-preview = Pré-visualização: { $scene }
control-stage = Preparar a cena inspecionada
control-take = Transição
control-transition = Transição de cena atual
control-duration = Duração da transição em milissegundos
control-apply = Aplicar
control-studio = Estúdio e transição
control-scene-mode-changed = O modo de estúdio mudou; atualize antes de ativar a cena. A ação não foi repetida.
control-filters = Filtros da fonte
control-text-file = Esta fonte de texto lê um ficheiro. Edite o ficheiro no computador do OBS.
control-text-save = Guardar texto no OBS
control-text = Fonte de texto integrada
control-media-play = Reproduzir
control-media-pause = Pausar
control-media-stop = Parar
control-media-restart = Reiniciar
control-media-previous = Anterior
control-media-next = Seguinte
control-media-seconds = Posição do conteúdo em segundos
control-media-seek = Ir para a posição
control-media = Reprodução de multimédia
control-monitor-host = A monitorização de áudio é reproduzida no computador do OBS, não através do SceneDeck.
control-monitor-off = Monitorização desligada
control-monitor-only = Apenas monitorização — silenciar saída
control-monitor-output = Monitorização e saída
control-audio-sync = Desfasamento de sincronização de áudio (milissegundos)
control-audio-balance = Balanço (0 = esquerda, 0,5 = centro, 1 = direita)
control-audio-track = Faixa de áudio { $track }
control-audio-tracks = Encaminhamento de faixas de áudio
control-advanced-audio = Áudio avançado da fonte


# Operating workspace
workspace-action-acknowledged = Pedido confirmado; estado atualizado a partir do OBS
workspace-replay-observed = O OBS indicou uma repetição guardada: { $path }
workspace-disk-space = Espaço em disco de gravação do OBS: { $value } MiB
control-stale = Último estado conhecido. Atualize as observações do OBS para permitir alterações.
control-no-matches = Nenhuma fonte corresponde à pesquisa. Limpe a pesquisa para mostrar todos os elementos.


# OBS recording volume and session activity
stats-recording-disk = Espaço livre no volume de gravação do OBS
stats-recording-disk-value = { $value } MiB
stats-activity-title = Atividade recente e contexto dos gráficos
stats-activity-detail = As 15 entradas mais recentes da sessão. Selecione uma entrada para marcar o seu instante nos gráficos. Falhas de ligação, amostras em falta e reposições de contadores limitam a interpretação. A proximidade temporal não prova causalidade.
stats-activity-clear-marker = Limpar marcador do gráfico
stats-activity-marker-visible = As linhas tracejadas assinalam o evento selecionado entre as amostras observadas. Os gráficos usam o espaçamento entre amostras.
stats-activity-marker-unavailable = Sem marcador: este evento está fora das amostras retidas, numa lacuna de amostragem ou foi afetado por uma alteração do relógio.
stats-activity-empty = Ainda não foram observadas operações nesta sessão.
stats-activity-requested = Solicitado
stats-activity-observed = Observado
stats-activity-failed = Falhou
stats-activity-connection = Ligação
stats-activity-entry = { $time } · { $origin } · { $description }
scene-card-stage-hint = Prepare esta cena na Pré-visualização. Use a transição para alterar o Programa.
scene-card-preview-label = Pré-visualização
hotkey-hint-stage-take = { $binding } · Preparar na Pré-visualização; usar a transição para alterar o Programa.


header-obs-setup = Configuração do OBS
header-obs-unknown = Desconhecido
header-obs-context = Perfil: { $profile } · Coleção de cenas: { $collection }

status-bar-connection-detail = { $state } · OBS { $version }


doctor-structural-title = Estrutura e funções das cenas


control-image-caption = Imagem de { $scene }, capturada há { $seconds } s. Atualize a imagem para capturar novamente.

output-results-acknowledged = Confirmado

output-results-copy-path = Copiar caminho no computador do OBS

output-results-host-path = Caminho no computador do OBS: { $path }

output-results-record-stop = Parar gravação

output-results-unconfirmed = Não confirmado

output-results-title = Resultados das saídas

output-results-detail = Os pedidos e as observações do OBS estão separados. Um evento de gravação pode vir de outro controlador. Os caminhos pertencem ao computador do OBS. São mantidos os 24 resultados mais recentes desta sessão.

record-elapsed-tooltip = Tempo decorrido desde que o OBS comunicou o início da gravação, incluindo pausas. Não é a duração do ficheiro gravado.

window-open-navigation = Abrir navegação
sources-title = Fontes
sources-back = Voltar às fontes
sources-select-hint = Selecione uma fonte para consultar as definições.
sources-text-conflict = Este texto foi alterado no OBS. Cancele para carregar o texto atual antes de guardar.
inventory-name-column = Cena
inventory-role-column = Função
inventory-actions-column = Opções
inventory-scene-options = Opções de { $scene }
inventory-columns-button = Largura das colunas
inventory-name-width = Largura da cena
inventory-role-width = Largura da função
inventory-reset-widths = Repor larguras das colunas
settings-scene-previews = Miniaturas das cenas
settings-scene-previews-description = Mostrar miniaturas escurecidas em cartões Live maiores.
live-audio-expand = Expandir áudio
live-audio-collapse = Recolher áudio
studio-mode-label = Studio
scene-card-image-pending = Miniatura indisponível
scene-card-program-label = Programa
output-options = Mais controlos de saída
output-replay-label = Replay · { $state }
output-camera-label = Câmara virtual · { $state }

output-camera-name = Câm. virtual
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
media-state-none = Sem multimédia
media-state-playing = A reproduzir
media-state-opening = A abrir
media-state-buffering = A carregar
media-state-paused = Em pausa
media-state-stopped = Parado
media-state-ended = Terminado
media-state-error = Erro
media-state-unknown = Desconhecido
