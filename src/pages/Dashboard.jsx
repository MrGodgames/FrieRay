import { useState, useEffect, useCallback, useRef } from 'react';
import ConnectButton from '../components/Connection/ConnectButton';
import { useTheme } from '../hooks/useTheme';
import * as api from '../api/tauri';
import './Dashboard.css';

const defaultSettings = {
    general: { auto_connect: false, start_minimized: false, launch_at_login: false, auto_update_subs: true, auto_update_interval_hours: 6 },
    proxy: { system_proxy: true, tun_mode: false, socks_port: 10808, http_port: 10809 },
    dns: { doh_server: 'https://dns.google/dns-query' },
    zapret: { enabled: false, strategy: 'auto', bypass_vpn: true, services: [] },
};

export default function Dashboard() {
    const { isClassic } = useTheme();
    const [connected, setConnected] = useState(false);
    const [connecting, setConnecting] = useState(false);
    const [currentServer, setCurrentServer] = useState(null);
    const [activeServer, setActiveServer] = useState(null);
    const [error, setError] = useState(null);
    const [startTime, setStartTime] = useState(null);
    const [duration, setDuration] = useState('00:00:00');
    const [loaded, setLoaded] = useState(false);
    const [settings, setSettings] = useState(defaultSettings);
    const [tunBusy, setTunBusy] = useState(false);
    const [ping, setPing] = useState(null);
    const mountedRef = useRef(true);

    useEffect(() => {
        mountedRef.current = true;
        const check = async () => {
            try {
                const [status, active, current, loadedSettings] = await Promise.all([
                    api.getConnectionStatus(),
                    api.getActiveServer(),
                    api.getCurrentServer(),
                    api.loadSettings(),
                ]);
                if (!mountedRef.current) return;
                setConnected(status);
                if (active) setActiveServer(active);
                if (current) {
                    setCurrentServer(current);
                    if (status && !startTime) setStartTime(Date.now());
                }
                if (loadedSettings) setSettings(loadedSettings);
                setLoaded(true);
            } catch (e) {
                if (mountedRef.current) setLoaded(true);
            }
        };
        check();
        const interval = setInterval(check, 3000);
        return () => {
            mountedRef.current = false;
            clearInterval(interval);
        };
    }, []);

    useEffect(() => {
        if (!connected) {
            setPing(null);
            return;
        }
        const doPing = async () => {
            const server = currentServer || activeServer;
            if (!server) return;
            try {
                const ms = await api.pingServer(server.address, server.port);
                if (mountedRef.current) setPing(ms);
            } catch (e) {
                if (mountedRef.current) setPing(null);
            }
        };
        doPing();
        const interval = setInterval(doPing, 10000);
        return () => clearInterval(interval);
    }, [connected, currentServer, activeServer]);

    useEffect(() => {
        if (!connected || !startTime) return;
        const tick = setInterval(() => {
            const diff = Math.floor((Date.now() - startTime) / 1000);
            const h = String(Math.floor(diff / 3600)).padStart(2, '0');
            const m = String(Math.floor((diff % 3600) / 60)).padStart(2, '0');
            const s = String(diff % 60).padStart(2, '0');
            setDuration(`${h}:${m}:${s}`);
        }, 1000);
        return () => clearInterval(tick);
    }, [connected, startTime]);

    const handleTunToggle = useCallback(async () => {
        setError(null);
        setTunBusy(true);
        try {
            const nextTunMode = !settings.proxy.tun_mode;
            const nextSettings = {
                ...settings,
                proxy: {
                    ...settings.proxy,
                    tun_mode: nextTunMode,
                    system_proxy: nextTunMode ? false : settings.proxy.system_proxy,
                },
            };

            if (nextTunMode) {
                const ready = await api.isTunReady();
                if (!ready) {
                    await api.installTunHelper();
                }
            }

            await api.saveSettings(nextSettings);
            if (!mountedRef.current) return;
            setSettings(nextSettings);

            if (connected) {
                setError('Режим TUN сохранён. Переподключись, чтобы применить изменение.');
            }
        } catch (e) {
            if (mountedRef.current) setError(String(e));
        } finally {
            if (mountedRef.current) setTunBusy(false);
        }
    }, [settings, connected]);

    const handleToggle = useCallback(async () => {
        setError(null);

        if (connected) {
            setConnecting(true);
            try {
                await api.disconnect();
                setConnected(false);
                setCurrentServer(null);
                setStartTime(null);
                setDuration('00:00:00');
            } catch (e) {
                setError(String(e));
            } finally {
                setConnecting(false);
            }
            return;
        }

        setConnecting(true);
        try {
            let server = activeServer;
            if (!server) {
                const servers = await api.getServers();
                if (!servers || servers.length === 0) {
                    setError('Нет серверов. Добавьте подписку в разделе «Серверы»');
                    setConnecting(false);
                    return;
                }
                server = servers[0];
            }
            await api.connect(server);
            setConnected(true);
            setCurrentServer(server);
            setStartTime(Date.now());
        } catch (e) {
            setError(String(e));
            setConnected(false);
        } finally {
            setConnecting(false);
        }
    }, [connected, activeServer]);

    const displayServer = currentServer || activeServer;
    const pingTone = ping === null ? 'muted' : ping < 100 ? 'good' : ping < 200 ? 'warn' : 'bad';
    const pingHint = ping === null
        ? 'Появится после подключения к серверу.'
        : ping < 100
            ? 'Низкая задержка.'
            : ping < 200
                ? 'Средняя задержка.'
                : 'Высокая задержка.';
    const connectionStatus = connected
        ? (isClassic ? 'Подключено' : '✦ Связь установлена ✦')
        : connecting
            ? (isClassic ? 'Подключение...' : '◌ Плетение заклинания...')
            : (isClassic ? 'Не подключено' : '○ Магия в покое');

    return (
        <div className="dashboard">
            <div className="page-header">
                <h1><span className="text-gradient">Панель Управления</span></h1>
                <p>Быстрый доступ к подключению и режиму TUN</p>
            </div>

            {error && (
                <div className="dashboard-error animate-fade-in">
                    <span>⚠️ {error}</span>
                    <button onClick={() => setError(null)} className="error-close">✕</button>
                </div>
            )}

            <div className="dashboard-hero">
                <div className="dashboard-connect-card fantasy-border corner-ornaments">
                    <div className="dashboard-connect-inner">
                        <ConnectButton
                            connected={connected}
                            connecting={connecting}
                            onToggle={handleToggle}
                        />

                        <div className="dashboard-server-info">
                            <div className="server-info-row">
                                <span className="server-info-label">✦ Сервер</span>
                                <span className="server-info-value">
                                    {displayServer ? displayServer.name : (loaded ? 'Не выбран — перейдите в «Серверы»' : 'Загрузка...')}
                                </span>
                            </div>
                            <div className="server-info-row">
                                <span className="server-info-label">✦ Протокол</span>
                                <span className="server-info-value">
                                    {displayServer
                                        ? (typeof displayServer.protocol === 'string' ? displayServer.protocol.toUpperCase() : 'VLESS')
                                        : '—'}
                                </span>
                            </div>
                            <div className="server-info-row">
                                <span className="server-info-label">✦ Адрес</span>
                                <span className="server-info-value server-info-mono">
                                    {displayServer ? `${displayServer.address}:${displayServer.port}` : '—'}
                                </span>
                            </div>
                            <div className="server-info-row">
                                <span className="server-info-label">✦ Статус</span>
                                <span className={`server-info-value status-${connected ? 'connected' : connecting ? 'connecting' : 'disconnected'}`}>
                                    {connectionStatus}
                                </span>
                            </div>
                            <div className="server-info-row">
                                <span className="server-info-label">✦ Сеанс</span>
                                <span className="server-info-value server-info-mono">
                                    {connected ? duration : '00:00:00'}
                                </span>
                            </div>
                            <div className="server-info-row server-info-row-tun">
                                <div className="server-info-tun-copy">
                                    <span className="server-info-label">✦ Пинг</span>
                                    <span className="server-info-tun-hint">{pingHint}</span>
                                </div>
                                <div className={`dashboard-tun-toggle dashboard-tun-toggle-static ping-${pingTone}`}>
                                    <span className="dashboard-tun-toggle-text">
                                        {ping === null ? '—' : `${ping} ms`}
                                    </span>
                                </div>
                            </div>
                            <div className="server-info-row server-info-row-tun">
                                <div className="server-info-tun-copy">
                                    <span className="server-info-label">✦ TUN режим</span>
                                    <span className="server-info-tun-hint">
                                        {settings.proxy.tun_mode
                                            ? 'Включён в быстрых настройках'
                                            : 'Выключен. Можно переключить отсюда.'}
                                    </span>
                                </div>
                                <button
                                    className={`dashboard-tun-toggle ${settings.proxy.tun_mode ? 'active' : ''}`}
                                    onClick={handleTunToggle}
                                    disabled={tunBusy}
                                    type="button"
                                >
                                    <span className="dashboard-tun-toggle-track">
                                        <span className="dashboard-tun-toggle-thumb" />
                                    </span>
                                    <span className="dashboard-tun-toggle-text">
                                        {tunBusy ? '...' : settings.proxy.tun_mode ? 'ВКЛ' : 'ВЫКЛ'}
                                    </span>
                                </button>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    );
}
