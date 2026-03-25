import { useEffect, useRef, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import Button from '../components/UI/Button';
import { useTheme } from '../hooks/useTheme';
import * as api from '../api/tauri';
import './TrayPopup.css';

const AUTO_SELECT_PROGRESS_EVENT = 'tray-autoselect-progress';

export default function TrayPopup() {
    const { isClassic } = useTheme();
    const [activeServer, setActiveServer] = useState(null);
    const [currentServer, setCurrentServer] = useState(null);
    const [connected, setConnected] = useState(false);
    const [busy, setBusy] = useState(false);
    const [error, setError] = useState(null);
    const [progress, setProgress] = useState(null);
    const [ping, setPing] = useState(null);
    const [startTime, setStartTime] = useState(null);
    const [duration, setDuration] = useState('00:00:00');
    const mountedRef = useRef(true);

    const refresh = async () => {
        const [status, active, current] = await Promise.all([
            api.getConnectionStatus(),
            api.getActiveServer(),
            api.getCurrentServer(),
        ]);
        setConnected(status);
        setActiveServer(active || null);
        setCurrentServer(current || null);
        if (status && !startTime) setStartTime(Date.now());
        if (!status) {
            setStartTime(null);
            setDuration('00:00:00');
        }
    };

    useEffect(() => {
        mountedRef.current = true;
        const load = async () => {
            try {
                await refresh();
            } catch (e) {
                if (mountedRef.current) setError(String(e));
            }
        };

        load();
        const interval = setInterval(load, 3000);
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
            } catch {
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

    useEffect(() => {
        let unsubscribe;

        const setup = async () => {
            unsubscribe = await listen(AUTO_SELECT_PROGRESS_EVENT, (event) => {
                setProgress(event.payload || null);
            });
        };

        setup();
        return () => {
            if (unsubscribe) {
                unsubscribe();
            }
        };
    }, []);

    const pingTone = ping === null ? 'muted' : ping < 100 ? 'good' : ping < 200 ? 'warn' : 'bad';

    const handleConnectToggle = async () => {
        setBusy(true);
        setError(null);
        try {
            if (connected) {
                setProgress({ stage: 'disconnect', message: 'Отключаю текущее соединение...' });
                await api.disconnect();
            } else {
                setProgress({ stage: 'prepare', message: 'Подбираю лучший сервер...' });
                await api.connectBestServer();
            }
            await refresh();
        } catch (e) {
            setError(String(e));
        } finally {
            setBusy(false);
            setTimeout(() => setProgress(null), 900);
        }
    };



    const handleOpenApp = async () => {
        setError(null);
        try {
            await api.showMainWindow();
        } catch (e) {
            setError(String(e));
        }
    };



    return (
        <div className={`tray-popup-shell ${isClassic ? 'classic' : 'fantasy'}`}>
            <div className="tray-popup-header">
                <div>
                    <div className="tray-popup-title-row">
                        <span className="tray-popup-brand">FrieRay</span>
                        <span className={`tray-popup-status ${connected ? 'connected' : 'idle'}`}>
                            {connected ? 'Подключено' : 'Отключено'}
                        </span>
                    </div>
                    <p className="tray-popup-subtitle">
                        {currentServer ? currentServer.name : activeServer ? activeServer.name : 'Выбери сервер для быстрого подключения'}
                    </p>
                </div>
            </div>

            {error && <div className="tray-popup-error">{error}</div>}

            {busy && progress && (
                <div className={`tray-popup-progress stage-${progress.stage || 'working'}`}>
                    <div className="tray-popup-progress-orb">
                        <span />
                        <span />
                        <span />
                    </div>
                    <div className="tray-popup-progress-copy">
                        <div className="tray-popup-progress-title">Подключение в процессе</div>
                        <div className="tray-popup-progress-text">{progress.message}</div>
                    </div>
                </div>
            )}

            <div className="tray-popup-actions">
                <Button
                    variant={connected ? 'ghost' : 'accent'}
                    size="sm"
                    loading={busy}
                    onClick={handleConnectToggle}
                >
                    {connected ? 'Отключить' : 'Подключить лучший'}
                </Button>
                <Button variant="secondary" size="sm" onClick={handleOpenApp}>
                    Открыть приложение
                </Button>
            </div>

            <div className="tray-popup-stats">
                <div className={`tray-popup-stat ping-${pingTone}`}>
                    <span className="tray-popup-stat-label">Пинг</span>
                    <span className="tray-popup-stat-value">
                        {ping === null ? '—' : `${ping} ms`}
                    </span>
                </div>
                <div className="tray-popup-stat">
                    <span className="tray-popup-stat-label">Время подключения</span>
                    <span className="tray-popup-stat-value">
                        {connected ? duration : '—'}
                    </span>
                </div>
            </div>
        </div>
    );
}
