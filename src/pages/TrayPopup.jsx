import { useEffect, useMemo, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import Card, { CardBody } from '../components/UI/Card';
import Button from '../components/UI/Button';
import { useTheme } from '../hooks/useTheme';
import * as api from '../api/tauri';
import './TrayPopup.css';

const AUTO_SELECT_PROGRESS_EVENT = 'tray-autoselect-progress';

export default function TrayPopup() {
    const { isClassic } = useTheme();
    const [servers, setServers] = useState([]);
    const [activeServer, setActiveServer] = useState(null);
    const [currentServer, setCurrentServer] = useState(null);
    const [connected, setConnected] = useState(false);
    const [busy, setBusy] = useState(false);
    const [error, setError] = useState(null);
    const [progress, setProgress] = useState(null);

    const refresh = async () => {
        const [status, active, current, loadedServers] = await Promise.all([
            api.getConnectionStatus(),
            api.getActiveServer(),
            api.getCurrentServer(),
            api.getServers(),
        ]);
        setConnected(status);
        setActiveServer(active || null);
        setCurrentServer(current || null);
        setServers(loadedServers || []);
    };

    useEffect(() => {
        const load = async () => {
            try {
                await refresh();
            } catch (e) {
                setError(String(e));
            }
        };

        load();
        const interval = setInterval(load, 3000);
        return () => clearInterval(interval);
    }, []);

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

    const orderedServers = useMemo(() => {
        return [...servers].sort((left, right) => {
            const leftCurrent = currentServer?.id === left.id;
            const rightCurrent = currentServer?.id === right.id;
            const leftActive = activeServer?.id === left.id;
            const rightActive = activeServer?.id === right.id;
            return rightCurrent - leftCurrent || rightActive - leftActive || left.name.localeCompare(right.name);
        });
    }, [servers, activeServer, currentServer]);

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

    const handleSelectServer = async (server) => {
        setBusy(true);
        setError(null);
        try {
            setProgress({ stage: 'switch', message: `Переключаюсь на ${server.name}...` });
            await api.setActiveServer(server.id);
            if (connected) {
                await api.disconnect();
                await api.connect(server);
            }
            await refresh();
        } catch (e) {
            setError(String(e));
        } finally {
            setBusy(false);
            setTimeout(() => setProgress(null), 500);
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

    const handleHide = async () => {
        try {
            await getCurrentWindow().hide();
        } catch (e) {
            setError(String(e));
        }
    };

    return (
        <div className={`tray-popup-shell ${isClassic ? 'classic' : 'fantasy'}`}>
            <div className="tray-popup-bg" />
            <Card variant="glass" hover={false} className="tray-popup-card">
                <CardBody className="tray-popup-body">
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
                        <button className="tray-popup-close" onClick={handleHide} aria-label="Скрыть popup">
                            ✕
                        </button>
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

                    <div className="tray-popup-section">
                        <div className="tray-popup-section-title">Серверы</div>
                        <div className="tray-popup-server-list">
                            {orderedServers.length === 0 ? (
                                <div className="tray-popup-empty">Нет серверов</div>
                            ) : (
                                orderedServers.map(server => {
                                    const isActive = activeServer?.id === server.id;
                                    const isCurrent = currentServer?.id === server.id;
                                    return (
                                        <button
                                            key={server.id}
                                            className={`tray-popup-server ${isActive ? 'active' : ''} ${isCurrent ? 'current' : ''}`}
                                            onClick={() => handleSelectServer(server)}
                                            disabled={busy}
                                        >
                                            <div className="tray-popup-server-main">
                                                <span className="tray-popup-server-name">{server.name}</span>
                                                <span className="tray-popup-server-protocol">
                                                    {typeof server.protocol === 'string' ? server.protocol.toUpperCase() : 'VLESS'}
                                                </span>
                                            </div>
                                            <div className="tray-popup-server-meta">
                                                <span>{server.address}:{server.port}</span>
                                                <span className="tray-popup-server-state">
                                                    {isCurrent ? 'Сейчас подключён' : isActive ? 'Выбран' : ' '}
                                                </span>
                                            </div>
                                        </button>
                                    );
                                })
                            )}
                        </div>
                    </div>
                </CardBody>
            </Card>
        </div>
    );
}
