# FrieRay

## English

FrieRay is a desktop V2Ray/Xray client for macOS built with Tauri 2, React, and Rust.

It focuses on a clean desktop workflow for everyday use: subscription import, quick server selection, full-traffic TUN mode, built-in diagnostics, menu bar control, and a switchable interface style.

### Features

- VLESS / VMess / Trojan / Shadowsocks subscription import
- Server selection from a unified list
- macOS menu bar / tray icon with popup
- Quick connect / disconnect from the tray
- Quick connect to the best server based on measured speed
- Progress feedback while the tray selects and tests the best server
- Launch at login and background tray mode on macOS
- Full-traffic TUN mode on macOS
- System proxy mode
- Per-server ping scan
- Per-server speed scan
- Live logs and connection diagnostics
- Light and dark themes
- Two interface styles: fantasy and classic

### Supported Input Formats

FrieRay currently accepts several common subscription and config formats on import:

- plain-text subscription lists with `vless://`, `vmess://`, `trojan://`, `ss://`
- the same lists when they are base64-encoded
- Xray / V2Ray JSON configs with supported `outbounds`
- some sing-box-style JSON outbound definitions using fields such as `type`, `server`, `server_port`, `uuid`, `tls`, and `transport`

Current import compatibility is focused on these outbound protocols:

- VLESS
- VMess
- Trojan
- Shadowsocks

Important:

- import support does not automatically mean every protocol/path is equally tested in the full connect flow
- the current desktop workflow is primarily tested around Xray-based VLESS setups
- formats such as Clash YAML, TUIC, Hysteria, WireGuard, or other unsupported protocol families are not guaranteed to work yet

### Downloads

Prebuilt macOS builds are available in GitHub Releases.

Release file:

- `FrieRay_0.2.1_aarch64.dmg`

### Current Status

Implemented and available in the app:

- subscription management
- connect / disconnect flow
- Xray launch and shutdown
- TUN helper installation on macOS
- automatic TUN and proxy cleanup on startup and exit
- macOS tray workflow with popup and quick actions
- best-server quick connect from the tray
- launch at login / background mode for macOS
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

Приложение рассчитано на повседневное использование: импорт подписок, быстрый выбор сервера, TUN-режим для всего трафика, встроенная диагностика, управление через menu bar и переключаемый стиль интерфейса.

### Возможности

- импорт подписок VLESS / VMess / Trojan / Shadowsocks
- выбор сервера из общего списка
- иконка в menu bar / tray с popup-окном
- быстрое подключение и отключение из трея
- быстрое подключение к лучшему серверу по измеренной скорости
- индикация прогресса, пока tray подбирает и тестирует лучший сервер
- запуск при входе в систему и фоновый режим на macOS
- TUN-режим для всего трафика в macOS
- режим системного прокси
- массовая проверка ping по серверам
- массовая проверка скорости по серверам
- логи и диагностика подключения
- светлая и тёмная темы
- два стиля интерфейса: fantasy и classic

### Поддерживаемые форматы импорта

Сейчас FrieRay умеет принимать несколько распространённых форматов подписок и конфигов:

- обычные текстовые подписки со строками `vless://`, `vmess://`, `trojan://`, `ss://`
- те же списки, если они закодированы в base64
- JSON-конфиги Xray / V2Ray с поддерживаемыми `outbounds`
- часть JSON-форматов в стиле sing-box, где используются поля `type`, `server`, `server_port`, `uuid`, `tls`, `transport`

Текущий импорт ориентирован на такие outbound-протоколы:

- VLESS
- VMess
- Trojan
- Shadowsocks

Важно:

- поддержка импорта не означает, что каждый протокол и каждый вариант транспорта одинаково хорошо проверен в полном сценарии подключения
- основной рабочий сценарий приложения сейчас в первую очередь протестирован на Xray-based VLESS-конфигах
- форматы вроде Clash YAML, TUIC, Hysteria, WireGuard и другие неподдерживаемые семейства протоколов пока не гарантируются

### Загрузка

Готовые сборки для macOS доступны во вкладке GitHub Releases.

Файл для скачивания:

- `FrieRay_0.2.1_aarch64.dmg`

### Текущий статус

Уже реализовано и доступно в приложении:

- управление подписками
- подключение и отключение
- запуск и остановка Xray
- установка TUN helper на macOS
- автоматическая очистка TUN и прокси при старте и выходе
- tray workflow для macOS с popup и быстрыми действиями
- быстрое подключение к лучшему серверу из трея
- запуск при входе в систему и работа в фоне на macOS
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
