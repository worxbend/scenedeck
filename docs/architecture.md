# Architecture Overview

SceneDeck separates GTK UI code, command orchestration, OBS WebSocket access,
domain models, services, and local storage.

## Runtime Model

`src/app.rs` creates a Tokio runtime before the GTK application starts. The
runtime handle is passed into the controller so OBS work can run asynchronously
while GTK remains responsive.

The runtime outlives the GTK main loop:

```text
main()
  -> logging::init()
  -> scenedeck::run()
     -> create Tokio runtime
     -> create adw::Application
     -> build window on activate
     -> app.run()
     -> drop runtime
```

## State Ownership

`AppState` lives on the GTK thread in `Rc<RefCell<AppState>>`. It is the UI
source of truth for:

- Current page.
- Theme mode.
- OBS connection status.
- Scene inventory and graph.
- OBS profiles and scene collections.
- Stream and record status.
- Active audio inputs.
- Diagnostics.

GTK widgets are updated from events in `src/ui/window.rs`.

## Command Flow

User actions dispatch `AppCommand` through `NavigationContext`.

```text
GTK signal
  -> NavigationContext::dispatch(AppCommand)
  -> AppController::handle(command)
  -> optional Tokio task
  -> ObsClient request
  -> AppEvent sent through std::sync::mpsc
  -> GTK polling timer applies event
```

The GTK side polls the event receiver every 50 ms. This keeps all widget
mutation on the GTK main thread.

## OBS Session Lifecycle

`AppController` owns the active OBS session task. Reconnect aborts the previous
session, clears the shared client slot, and starts a new connection task.

The active OBS client is stored as:

```text
Arc<Mutex<Option<ObsClient>>>
```

The mutex is only held long enough to clone the cheap `ObsClient` handle. It
must not be held across `.await`.

After connection:

1. SceneDeck verifies the OBS version.
2. It publishes the client into the shared slot.
3. It refreshes profiles and scene collections.
4. It refreshes stream and record status.
5. It refreshes scene inventory, graph, and active scene audio.
6. It enters the OBS event loop.

On disconnect or event-stream end, the client slot is cleared and a
`Disconnected` event is sent.

## Boundaries

`src/obs/` is the only boundary that imports `obws`. OBS response types are
mapped to domain types before crossing into the controller or UI.

`src/controller/` owns orchestration. It translates commands into async work and
events into GTK-side updates.

`src/domain/` contains stable app concepts such as scenes, roles, audio inputs,
output status, graph, diagnostics, and OBS named lists.

`src/services/` contains pure or mostly pure higher-level logic, such as Doctor
checks and graph edge classification.

`src/storage/` owns local persistence: config JSON, scene registry JSON, XDG
paths, and keyring integration.

`src/ui/` owns GTK widget construction, CSS loading, page layout, navigation,
and action registration.

## Live Data Refresh

Scene inventory refresh also triggers:

- Active scene audio refresh.
- Scene dependency graph refresh.

Audio refresh for the current scene includes:

- OBS global audio sources first.
- Enabled input scene items from the current scene.
- Enabled inputs inside nested scenes and groups.

Input mute and volume events update the matching audio card without rebuilding
the whole mixer.

## Error Handling

Operational errors are converted into `AppError` and sent as `AppEvent::Error`.
The UI updates sidebar status, disables live output controls, shows the
disconnected Live view, and displays a toast.

Warnings that do not need user interruption use `tracing`.
