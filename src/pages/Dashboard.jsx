import { useState, useEffect, useCallback, useRef } from 'react';
import ConnectButton from '../components/Connection/ConnectButton';
import Card, { CardHeader, CardBody } from '../components/UI/Card';
import * as api from '../api/tauri';
import './Dashboard.css';

function formatSpeed(bytesPerSec) {
    if (!bytesPerSec || bytesPerSec < 1) return '0 B/s';
    if (bytesPerSec < 1024) return `${Math.round(bytesPerSec)} B/s`;
    if (bytesPerSec < 1024 * 1024) return `${(bytesPerSec / 1024).toFixed(1)} KB/s`;
    return `${(bytesPerSec / (1024 * 1024)).toFixed(1)} MB/s`;
}

function formatBytes(bytes) {
    if (!bytes) return '0 B';
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

export default function Dashboard() {
    const [connected, setConnected] = useState(false);
    const [connecting, setConnecting] = useState(false);
    const [currentServer, setCurrentServer] = useState(null);
    const [activeServer, setActiveServer] = useState(null);
    const [error, setError] = useState(null);
    const [startTime, setStartTime] = useState(null);
    const [duration, setDuration] = useState('00:00:00');
    const [loaded, setLoaded] = useState(false);
    const [traffic, setTraffic] = useState({ down_speed: 0, up_speed: 0, downlink: 0, uplink: 0 });
    const [ping, setPing] = useState(null);
    const mountedRef = useRef(true);

    // Load state on mount
    useEffect(() => {
        mountedRef.current = true;
        const check = async () => {
            try {
                const [status, active, current] = await Promise.all([
                    api.getConnectionStatus(),
                    api.getActiveServer(),
                    api.getCurrentServer(),
                ]);
                if (!mountedRef.current) return;
                setConnected(status);
                if (active) setActiveServer(active);
                if (current) {
                    setCurrentServer(current);
                    if (status && !startTime) setStartTime(Date.now());
                }
                setLoaded(true);
            } catch (e) {
                if (mountedRef.current) setLoaded(true);
            }
        };
        check();
        const interval = setInterval(check, 3000);
        return () => { mountedRef.current = false; clearInterval(interval); };
    }, []);

    // Poll traffic stats when connected
    useEffect(() => {
        if (!connected) return;
        const poll = async () => {
            try {
                const stats = await api.getTrafficStats();
                if (stats && mountedRef.current) setTraffic(stats);
            } catch (e) { }
        };
        poll();
        const interval = setInterval(poll, 1500);
        return () => clearInterval(interval);
    }, [connected]);

    // Poll ping when connected
    useEffect(() => {
        if (!connected) { setPing(null); return; }
        const doPing = async () => {
            const server = currentServer || activeServer;
            if (!server) return;
            try {
                const ms = await api.pingServer(server.address, server.port);
                if (mountedRef.current) setPing(ms);
            } catch (e) { }
        };
        doPing();
        const interval = setInterval(doPing, 10000);
        return () => clearInterval(interval);
    }, [connected, currentServer, activeServer]);

    // Duration timer
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
                setTraffic({ down_speed: 0, up_speed: 0, downlink: 0, uplink: 0 });
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

    const pingColor = ping !== null ? (ping < 100 ? 'var(--accent-400)' : ping < 200 ? 'var(--gold-400)' : 'var(--danger)') : 'var(--info)';

    const stats = [
        { label: 'Загрузка', value: connected ? formatSpeed(traffic.down_speed) : '—', icon: '↓', color: 'var(--accent-400)' },
        { label: 'Отдача', value: connected ? formatSpeed(traffic.up_speed) : '—', icon: '↑', color: 'var(--primary-400)' },
        { label: 'Пинг', value: ping !== null ? `${ping}ms` : '—', icon: '◷', color: pingColor },
        { label: 'Трафик', value: connected ? formatBytes(traffic.downlink + traffic.uplink) : '—', icon: '◈', color: 'var(--gold-400)' },
    ];

    return (
        <div className="dashboard">
            <div className="page-header">
                <h1><span className="text-gradient">Панель Управления</span></h1>
                <p>Магия связи в твоих руках</p>
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
                                    {connected ? '✦ Связь установлена ✦' : connecting ? '◌ Плетение заклинания...' : '○ Магия в покое'}
                                </span>
                            </div>
                        </div>
                    </div>
                </div>
            </div>

            <div className="dashboard-stats stagger-children">
                {stats.map((stat, i) => (
                    <div key={i} className="stat-card fantasy-border">
                        <div className="stat-content">
                            <span className="stat-icon" style={{ color: stat.color, textShadow: `0 0 10px ${stat.color}` }}>{stat.icon}</span>
                            <div className="stat-info">
                                <span className="stat-value">{stat.value}</span>
                                <span className="stat-label">{stat.label}</span>
                            </div>
                        </div>
                    </div>
                ))}
            </div>


        </div>
    );
}
