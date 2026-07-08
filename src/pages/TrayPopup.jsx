import { useEffect, useRef, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { useTheme } from '../hooks/useTheme';
import { useI18n } from '../hooks/useI18n';
import * as api from '../api/tauri';
import './TrayPopup.css';

const AUTO_SELECT_PROGRESS_EVENT = 'tray-autoselect-progress';
const PING_FAILURES_BEFORE_FAILOVER = 2;

export default function TrayPopup() {
    const { isClassic } = useTheme();
    const { t } = useI18n();
    const [activeServer, setActiveServer] = useState(null);
    const [currentServer, setCurrentServer] = useState(null);
    const [connected, setConnected] = useState(false);
    const [busy, setBusy] = useState(false);
    const [error, setError] = useState(null);
    const [progress, setProgress] = useState(null);
    const [ping, setPing] = useState(null);
    const [pingFailures, setPingFailures] = useState(0);
    const [startTime, setStartTime] = useState(null);
    const [duration, setDuration] = useState('00:00:00');
    const mountedRef = useRef(true);
    const busyRef = useRef(false);
    const autoReconnectRef = useRef(false);

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

    const reconnectWithFreshScan = async (auto = false) => {
        setBusy(true);
        setError(null);
        setProgress({
            stage: 'rescan',
            message: auto ? t('trayAutoReconnectMessage') : t('trayRescanSelecting'),
        });
        try {
            if (connected || auto) {
                await api.reconnectBestServerRescan();
            } else {
                await api.connectBestServerRescan();
            }
            setPingFailures(0);
            autoReconnectRef.current = false;
            await refresh();
        } catch (e) {
            setError(String(e));
        } finally {
            setBusy(false);
            setTimeout(() => setProgress(null), 1200);
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
        busyRef.current = busy;
    }, [busy]);

    useEffect(() => {
        if (!connected) {
            setPing(null);
            setPingFailures(0);
            autoReconnectRef.current = false;
            return;
        }
        const doPing = async () => {
            const server = currentServer || activeServer;
            if (!server) return;
            try {
                const ms = await api.pingServer(server.address, server.port);
                if (mountedRef.current) {
                    setPing(ms);
                    setPingFailures(0);
                    autoReconnectRef.current = false;
                }
            } catch {
                if (!mountedRef.current) return;
                setPing(null);
                setPingFailures(prev => {
                    const next = Math.min(prev + 1, PING_FAILURES_BEFORE_FAILOVER);
                    if (
                        next >= PING_FAILURES_BEFORE_FAILOVER &&
                        !busyRef.current &&
                        !autoReconnectRef.current
                    ) {
                        autoReconnectRef.current = true;
                        reconnectWithFreshScan(true);
                    }
                    return next;
                });
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
    const displayedServer = currentServer || activeServer;
    const healthLabel = ping !== null
        ? t('trayHealthGood')
        : pingFailures > 0
            ? t('trayHealthFailing', { count: pingFailures })
            : t('trayHealthUnknown');

    const handleConnectToggle = async () => {
        setBusy(true);
        setError(null);
        try {
            if (connected) {
                setProgress({ stage: 'disconnect', message: t('trayDisconnecting') });
                await api.disconnect();
            } else {
                setProgress({ stage: 'rescan', message: t('trayRescanSelecting') });
                await api.connectBestServerRescan();
            }
            setPingFailures(0);
            autoReconnectRef.current = false;
            await refresh();
        } catch (e) {
            setError(String(e));
        } finally {
            setBusy(false);
            setTimeout(() => setProgress(null), 1200);
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
        <div className={`tray-popup-shell ${isClassic ? 'classic' : 'fantasy'} ${connected ? 'is-connected' : 'is-idle'}`}>
            <div className="tray-popup-caret" />
            <div className="tray-popup-header">
                <div className="tray-popup-heading">
                    <div className="tray-popup-title-row">
                        <span className={`tray-popup-dot ${connected ? 'connected' : 'idle'}`} />
                        <span className="tray-popup-brand">FrieRay</span>
                        <span className={`tray-popup-status ${connected ? 'connected' : 'idle'}`}>
                            {connected ? t('trayStatusConnected') : t('trayStatusIdle')}
                        </span>
                    </div>
                    <div className="tray-popup-server-card">
                        <span className="tray-popup-server-icon">✦</span>
                        <div className="tray-popup-server-copy">
                            <span className="tray-popup-server-label">{t('dashboardServer')}</span>
                            <span className="tray-popup-server-name">
                                {displayedServer ? displayedServer.name : t('trayChooseServer')}
                            </span>
                        </div>
                    </div>
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
                        <div className="tray-popup-progress-title">
                            {progress.stage === 'rescan' ? t('trayAutoReconnectTitle') : t('trayConnectingTitle')}
                        </div>
                        <div className="tray-popup-progress-text">{progress.message}</div>
                    </div>
                </div>
            )}

            <div className={`tray-popup-health-card ping-${pingTone}`}>
                <div className="tray-popup-health-copy">
                    <div className="tray-popup-health-main">
                        <span className="tray-popup-health-dot" />
                        <span>{healthLabel}</span>
                    </div>
                </div>
                {(connected || ping !== null) && (
                    <div className="tray-popup-meta">
                        <span>{ping === null ? '—' : `${ping} ms`}</span>
                        <span>{connected ? duration : '—'}</span>
                    </div>
                )}
            </div>

            <div className="tray-popup-actions">
                <button
                    className={`tray-menu-action tray-menu-action-primary ${connected ? 'danger' : 'accent'}`}
                    disabled={busy}
                    onClick={handleConnectToggle}
                >
                    <span className="tray-menu-action-icon">{connected ? '⏻' : '↻'}</span>
                    <span>{connected ? t('trayDisconnect') : t('trayConnectBest')}</span>
                </button>
                {connected && (
                    <button
                        className="tray-menu-action"
                        disabled={busy}
                        onClick={() => reconnectWithFreshScan(false)}
                    >
                        <span className="tray-menu-action-icon">⇄</span>
                        <span>{t('traySwitchBest')}</span>
                    </button>
                )}
                <button className="tray-menu-action" disabled={busy} onClick={handleOpenApp}>
                    <span className="tray-menu-action-icon">⌘</span>
                    <span>{t('trayOpenApp')}</span>
                </button>
            </div>
        </div>
    );
}
