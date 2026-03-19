# FrieRay

## English

FrieRay is a desktop V2Ray/Xray client for macOS built with Tauri 2, React, and Rust.

It focuses on a clean desktop workflow for everyday use: subscription import, quick server selection, full-traffic TUN mode, built-in diagnostics, and a switchable interface style.

### Features

- VLESS / VMess / Trojan subscription import
- Server selection from a unified list
- Full-traffic TUN mode on macOS
- System proxy mode
- Per-server ping scan
- Per-server speed scan
- Live logs and connection diagnostics
- Light and dark themes
- Two interface styles: fantasy and classic

### Downloads

Prebuilt macOS builds are available in GitHub Releases.

Release file:

- `FrieRay_0.1.0_aarch64.dmg`

### Current Status

Implemented and available in the app:

- subscription management
- connect / disconnect flow
- Xray launch and shutdown
- TUN helper installation on macOS
- automatic TUN and proxy cleanup on startup and exit
- dashboard quick TUN toggle
- per-server ping and speed checks
- classic mode without fantasy/anime visuals

Currently still closer to prototype/UI status:

- Split Tunnel
- Routing editor

### Technology

- Tauri 2
- React 19
- Vite 7
- Rust
- Xray-core

### Development

Requirements:

- macOS
- Node.js 20+
- npm
- Rust toolchain
- Xcode Command Line Tools

Run locally:

```bash
npm install
npm run tauri dev
```

Build desktop app:

```bash
npm run tauri build
```

Build artifacts:

```text
src-tauri/target/release/bundle/macos/
src-tauri/target/release/bundle/dmg/
```

### TUN Mode

When TUN mode is enabled for the first time, FrieRay installs a small privileged helper on macOS to manage routes.

macOS may request an administrator password once during this step. After installation, the helper is reused.

### Notes

- `xray` is bundled from `src-tauri/binaries/`
- runtime settings are stored outside the repository in the system application data directory
- on exit, the app attempts to clean up TUN routes, Xray state, and system proxy settings automatically

## Русский

FrieRay — это десктопный V2Ray/Xray-клиент для macOS, написанный на Tauri 2, React и Rust.

Приложение рассчитано на повседневное использование: импорт подписок, быстрый выбор сервера, TUN-режим для всего трафика, встроенная диагностика и переключаемый стиль интерфейса.

### Возможности

- импорт подписок VLESS / VMess / Trojan
- выбор сервера из общего списка
- TUN-режим для всего трафика в macOS
- режим системного прокси
- массовая проверка ping по серверам
- массовая проверка скорости по серверам
- логи и диагностика подключения
- светлая и тёмная темы
- два стиля интерфейса: fantasy и classic

### Загрузка

Готовые сборки для macOS доступны во вкладке GitHub Releases.

Файл для скачивания:

- `FrieRay_0.1.0_aarch64.dmg`

### Текущий статус

Уже реализовано и доступно в приложении:

- управление подписками
- подключение и отключение
- запуск и остановка Xray
- установка TUN helper на macOS
- автоматическая очистка TUN и прокси при старте и выходе
- быстрый переключатель TUN на главной странице
- проверка ping и скорости для серверов
- классический режим без fantasy/anime-оформления

Пока ещё ближе к прототипу или UI-слою:

- Split Tunnel
- редактор маршрутизации

### Технологии

- Tauri 2
- React 19
- Vite 7
- Rust
- Xray-core

### Разработка

Требования:

- macOS
- Node.js 20+
- npm
- Rust toolchain
- Xcode Command Line Tools

Локальный запуск:

```bash
npm install
npm run tauri dev
```

Сборка приложения:

```bash
npm run tauri build
```

Артефакты сборки:

```text
src-tauri/target/release/bundle/macos/
src-tauri/target/release/bundle/dmg/
```

### TUN-режим

При первом включении TUN режима FrieRay устанавливает небольшой привилегированный helper для управления маршрутами в macOS.

Во время этой установки macOS может один раз запросить пароль администратора. После установки helper переиспользуется.

### Примечания

- `xray` поставляется вместе с приложением из `src-tauri/binaries/`
- пользовательские настройки и runtime-данные хранятся вне репозитория, в системной директории приложения
- при выходе приложение пытается автоматически очистить TUN-маршруты, состояние Xray и системный прокси

## License / Лицензия

No license file is included yet.

Файл лицензии пока не добавлен.
