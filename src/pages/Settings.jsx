import { useState, useEffect } from 'react';
import Card, { CardBody } from '../components/UI/Card';
import Toggle from '../components/UI/Toggle';
import Button from '../components/UI/Button';
import { useTheme } from '../hooks/useTheme';
import * as api from '../api/tauri';
import './Settings.css';

export default function Settings() {
    const { theme, setTheme, uiStyle, setUiStyle, isClassic } = useTheme();
    const [settings, setSettings] = useState({
        general: { auto_connect: false, start_minimized: false, launch_at_login: false, auto_update_subs: true, auto_update_interval_hours: 6 },
        proxy: { system_proxy: true, tun_mode: false, socks_port: 10808, http_port: 10809 },
        dns: { doh_server: 'https://dns.google/dns-query' },
        zapret: { enabled: false, strategy: 'auto', bypass_vpn: true, services: [] },
    });
    const [saved, setSaved] = useState(false);

    useEffect(() => {
        const load = async () => {
            try {
                const loaded = await api.loadSettings();
                if (loaded) setSettings(loaded);
            } catch (e) {
                // Tauri not available
            }
        };
        load();
    }, []);

    const update = (section, changes) => {
        // changes is { key: value } or just (section, key, value) for single change
        let changesObj = changes;
        if (typeof changes === 'string') {
            // Called as update('proxy', 'key', value) — 3 args
            return;
        }
        setSettings(prev => {
            const newSettings = {
                ...prev,
                [section]: { ...prev[section], ...changesObj }
            };
            // Auto-save to backend
            api.saveSettings(newSettings).then(() => {
                setSaved(true);
                setTimeout(() => setSaved(false), 1500);
            }).catch(() => { });
            return newSettings;
        });
    };

    const set = (section, key, value) => {
        update(section, { [key]: value });
    };

    const handleSave = async () => {
        try {
            await api.saveSettings(settings);
            setSaved(true);
            setTimeout(() => setSaved(false), 2000);
        } catch (e) { }
    };

    return (
        <div className="settings-page">
            <div className="page-header">
                <h1><span className="text-gradient">Настройки</span></h1>
                <p>Конфигурация приложения</p>
            </div>

            <div className="settings-section">
                <h4 className="settings-section-title">✦ Оформление</h4>
                <Card variant="glass" hover={false}>
                    <CardBody>
                        <div className="settings-list">
                            <div className="settings-choice-row">
                                <div className="settings-choice-copy">
                                    <span className="settings-choice-label">Цветовая тема</span>
                                    <span className="settings-choice-desc">Переключает светлую и тёмную палитру интерфейса.</span>
                                </div>
                                <div className="settings-segmented" role="tablist" aria-label="Выбор цветовой темы">
                                    <button
                                        type="button"
                                        className={`settings-segment ${theme === 'dark' ? 'active' : ''}`}
                                        onClick={() => setTheme('dark')}
                                    >
                                        Тёмная
                                    </button>
                                    <button
                                        type="button"
                                        className={`settings-segment ${theme === 'light' ? 'active' : ''}`}
                                        onClick={() => setTheme('light')}
                                    >
                                        Светлая
                                    </button>
                                </div>
                            </div>

                            <div className="settings-choice-row">
                                <div className="settings-choice-copy">
                                    <span className="settings-choice-label">Стиль интерфейса</span>
                                    <span className="settings-choice-desc">Классический режим убирает персонажа, частицы и фэнтези-декор.</span>
                                </div>
                                <div className="settings-segmented" role="tablist" aria-label="Выбор стиля интерфейса">
                                    <button
                                        type="button"
                                        className={`settings-segment ${uiStyle === 'fantasy' ? 'active' : ''}`}
                                        onClick={() => setUiStyle('fantasy')}
                                    >
                                        Фэнтези
                                    </button>
                                    <button
                                        type="button"
                                        className={`settings-segment ${uiStyle === 'classic' ? 'active' : ''}`}
                                        onClick={() => setUiStyle('classic')}
                                    >
                                        Классический
                                    </button>
                                </div>
                            </div>
                        </div>
                    </CardBody>
                </Card>
            </div>

            {/* General */}
            <div className="settings-section">
                <h4 className="settings-section-title">✦ Общие</h4>
                <Card variant="glass" hover={false}>
                    <CardBody>
                        <div className="settings-list">
                            <Toggle
                                id="auto-connect"
                                label="Автоподключение"
                                description="Подключаться к последнему серверу при запуске"
                                checked={settings.general.auto_connect}
                                onChange={(v) => set('general', 'auto_connect', v)}
                            />
                            <Toggle
                                id="start-minimized"
                                label="Запускать свёрнуто"
                                description="Стартует в tray и без открытия главного окна"
                                checked={settings.general.start_minimized}
                                onChange={(v) => set('general', 'start_minimized', v)}
                            />
                            <Toggle
                                id="launch-at-login"
                                label="Запускать при входе в систему"
                                description="На macOS создаёт login item и позволяет держать FrieRay в фоне"
                                checked={settings.general.launch_at_login}
                                onChange={(v) => set('general', 'launch_at_login', v)}
                            />
                            <Toggle
                                id="auto-update-subs"
                                label="Автообновление подписок"
                                description="Обновлять серверы каждые 6 часов"
                                checked={settings.general.auto_update_subs}
                                onChange={(v) => set('general', 'auto_update_subs', v)}
                            />
                        </div>
                    </CardBody>
                </Card>
            </div>

            {/* Proxy */}
            <div className="settings-section">
                <h4 className="settings-section-title">✦ Прокси</h4>
                <Card variant="glass" hover={false}>
                    <CardBody>
                        <div className="settings-list">
                            <Toggle
                                id="system-proxy"
                                label="Системный прокси"
                                description="Настраивает прокси в ОС (работает не для всех приложений)"
                                checked={settings.proxy.system_proxy && !settings.proxy.tun_mode}
                                onChange={(v) => {
                                    if (v) {
                                        update('proxy', { system_proxy: true, tun_mode: false });
                                    } else {
                                        set('proxy', 'system_proxy', false);
                                    }
                                }}
                            />
                            <Toggle
                                id="tun-mode"
                                label="TUN режим (рекомендуется)"
                                description="Перехват ВСЕГО трафика как настоящий VPN"
                                checked={settings.proxy.tun_mode}
                                onChange={(v) => {
                                    if (v) {
                                        update('proxy', { tun_mode: true, system_proxy: false });
                                        // Install helper if needed (password prompt)
                                        api.isTunReady().then(ready => {
                                            if (!ready) {
                                                api.installTunHelper().catch(e => {
                                                    console.warn('TUN helper install:', e);
                                                });
                                            }
                                        }).catch(() => { });
                                    } else {
                                        set('proxy', 'tun_mode', false);
                                    }
                                }}
                            />
                            <div className="settings-input-group">
                                <div className="settings-input-item">
                                    <label className="settings-input-label">SOCKS5 порт</label>
                                    <input
                                        type="number"
                                        className="fr-input fr-input-sm"
                                        value={settings.proxy.socks_port}
                                        onChange={(e) => set('proxy', 'socks_port', parseInt(e.target.value) || 10808)}
                                    />
                                </div>
                                <div className="settings-input-item">
                                    <label className="settings-input-label">HTTP порт</label>
                                    <input
                                        type="number"
                                        className="fr-input fr-input-sm"
                                        value={settings.proxy.http_port}
                                        onChange={(e) => set('proxy', 'http_port', parseInt(e.target.value) || 10809)}
                                    />
                                </div>
                            </div>
                        </div>
                    </CardBody>
                </Card>
            </div>

            {/* DNS */}
            <div className="settings-section">
                <h4 className="settings-section-title">✦ DNS</h4>
                <Card variant="glass" hover={false}>
                    <CardBody>
                        <div className="settings-list">
                            <div className="settings-input-item" style={{ width: '100%' }}>
                                <label className="settings-input-label">DoH сервер</label>
                                <input
                                    type="text"
                                    className="fr-input"
                                    value={settings.dns.doh_server}
                                    onChange={(e) => set('dns', 'doh_server', e.target.value)}
                                />
                            </div>
                        </div>
                    </CardBody>
                </Card>
            </div>

            {/* Save button */}
            <div style={{ display: 'flex', gap: '12px', alignItems: 'center' }}>
                <Button variant="accent" onClick={handleSave}>
                    {saved ? '✓ Сохранено' : 'Сохранить настройки'}
                </Button>
                {saved && <span style={{ color: 'var(--accent-400)', fontSize: '0.82rem' }}>Настройки сохранены!</span>}
            </div>

            {/* About */}
            <div className="settings-section">
                <h4 className="settings-section-title">✦ О программе</h4>
                <Card variant="glass" hover={false}>
                    <CardBody>
                        <div className="about-info">
                            <div className="about-logo text-gradient">FrieRay</div>
                            <div className="about-version">v0.2.0</div>
                            <p className="about-desc">
                                {isClassic
                                    ? 'Минималистичный V2Ray клиент без аниме-стилистики'
                                    : 'V2Ray клиент в стиле «Провожающая в последний путь Фрирен»'}
                            </p>
                            <p className="about-tech">Tauri v2 • React • Xray-core</p>
                        </div>
                    </CardBody>
                </Card>
            </div>
        </div>
    );
}
