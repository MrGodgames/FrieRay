import { useState, useEffect } from 'react';
import Card, { CardHeader, CardBody } from '../components/UI/Card';
import Button from '../components/UI/Button';
import Toggle from '../components/UI/Toggle';
import * as api from '../api/tauri';
import './Servers.css';

export default function Servers() {
    const [servers, setServers] = useState([]);
    const [activeServerId, setActiveServerId] = useState(null);
    const [subscriptions, setSubscriptions] = useState([]);
    const [showAddSub, setShowAddSub] = useState(false);
    const [subUrl, setSubUrl] = useState('');
    const [subName, setSubName] = useState('');
    const [isUpdating, setIsUpdating] = useState(false);
    const [isPinging, setIsPinging] = useState(false);
    const [isTestingSpeed, setIsTestingSpeed] = useState(false);
    const [autoUpdate, setAutoUpdate] = useState(true);
    const [error, setError] = useState(null);

    // Load data on mount
    useEffect(() => {
        const load = async () => {
            try {
                const [subs, srvs, active] = await Promise.all([
                    api.getSubscriptions(),
                    api.getServers(),
                    api.getActiveServer(),
                ]);
                if (subs) setSubscriptions(subs);
                if (srvs) setServers(srvs);
                if (active) setActiveServerId(active.id);
            } catch (e) { }
        };
        load();
    }, []);

    useEffect(() => {
        if (!isPinging && !isTestingSpeed) return;
        const poll = async () => {
            try {
                const srvs = await api.getServers();
                if (srvs) setServers(srvs);
            } catch (e) { }
        };
        poll();
        const interval = setInterval(poll, 700);
        return () => clearInterval(interval);
    }, [isPinging, isTestingSpeed]);

    const getPingColor = (server) => {
        if (server.reachable === false) return 'var(--error)';
        const ping = server.ping;
        if (!ping && ping !== 0) return 'var(--text-muted)';
        if (ping < 80) return 'var(--accent-400)';
        if (ping < 150) return 'var(--warning)';
        return 'var(--error)';
    };

    const getPingLabel = (server) => {
        if (server.reachable === false) return 'Недоступен';
        const ping = server.ping;
        if (!ping && ping !== 0) return '—';
        return `${ping}ms`;
    };

    const formatSpeedLabel = (server) => {
        if (server.reachable === false) return 'Недоступен';
        const speedMbps = server.speed_mbps;
        if (speedMbps === null || speedMbps === undefined) return '—';
        if (speedMbps < 1) return `${speedMbps.toFixed(2)} Mb/s`;
        return `${speedMbps.toFixed(1)} Mb/s`;
    };

    const handleAddSubscription = async () => {
        if (!subUrl.trim()) return;
        setError(null);
        try {
            const sub = await api.addSubscription(
                subName.trim() || 'Подписка ' + (subscriptions.length + 1),
                subUrl.trim()
            );
            if (sub) setSubscriptions(prev => [...prev, sub]);
            setShowAddSub(false);
            setSubUrl('');
            setSubName('');
            handleUpdateAll();
        } catch (e) {
            setError(e?.toString() || 'Ошибка добавления подписки');
        }
    };

    const handleUpdateAll = async () => {
        setIsUpdating(true);
        setError(null);
        try {
            const result = await api.updateSubscriptions();
            if (result) setServers(result);
            const subs = await api.getSubscriptions();
            if (subs) setSubscriptions(subs);
        } catch (e) {
            setError(e?.toString() || 'Ошибка обновления');
        } finally {
            setIsUpdating(false);
        }
    };

    const handlePingAll = async () => {
        setIsPinging(true);
        setError(null);
        try {
            const result = await api.pingAllServers();
            if (result) setServers(result);
        } catch (e) {
            setError(e?.toString() || 'Ошибка пинга');
        } finally {
            setIsPinging(false);
        }
    };

    const handleSpeedTestAll = async () => {
        setIsTestingSpeed(true);
        setError(null);
        try {
            const result = await api.speedTestAllServers();
            if (result) setServers(result);
        } catch (e) {
            setError(e?.toString() || 'Ошибка теста скорости');
        } finally {
            setIsTestingSpeed(false);
        }
    };

    const handleSelectServer = async (server) => {
        try {
            const result = await api.setActiveServer(server.id);
            if (result) setActiveServerId(result.id);
        } catch (e) {
            setError(e?.toString());
        }
    };

    const handleRemoveSub = async (id) => {
        try {
            await api.removeSubscription(id);
            setSubscriptions(prev => prev.filter(s => s.id !== id));
            setServers(prev => prev.filter(s => s.subscription_id !== id));
        } catch (e) {
            setError(e?.toString());
        }
    };

    const handleAddLink = async () => {
        if (!subUrl.trim()) return;
        if (subUrl.startsWith('vless://') || subUrl.startsWith('vmess://') || subUrl.startsWith('trojan://')) {
            try {
                const server = await api.parseLink(subUrl.trim());
                if (server) {
                    setServers(prev => [...prev, server]);
                    setShowAddSub(false);
                    setSubUrl('');
                    setSubName('');
                    return;
                }
            } catch (e) { }
        }
        handleAddSubscription();
    };

    return (
        <div className="servers-page">
            <div className="page-header">
                <h1><span className="text-gradient">Серверы</span></h1>
                <p>Управление подписками и серверами</p>
            </div>

            {error && (
                <div className="dashboard-error animate-fade-in">
                    <span>⚠️ {error}</span>
                    <button onClick={() => setError(null)} className="error-close">✕</button>
                </div>
            )}

            {/* === SUBSCRIPTIONS === */}
            <div className="servers-section">
                <div className="servers-section-header">
                    <h3 className="servers-section-title">✦ Подписки</h3>
                    <Button variant="primary" size="sm" onClick={() => setShowAddSub(!showAddSub)}
                        icon={<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5"><line x1="12" y1="5" x2="12" y2="19" /><line x1="5" y1="12" x2="19" y2="12" /></svg>}>
                        Добавить
                    </Button>
                </div>

                {showAddSub && (
                    <Card variant="glass" hover={false} className="add-sub-card animate-fade-in-scale">
                        <CardBody>
                            <div className="add-sub-form">
                                <p className="add-sub-title">Добавить подписку или ссылку</p>
                                <p className="add-sub-desc">URL подписки или прямую ссылку vless://, vmess://, trojan://</p>
                                <div className="add-sub-fields">
                                    <input type="text" className="fr-input" placeholder="Название (необязательно)"
                                        value={subName} onChange={(e) => setSubName(e.target.value)} />
                                    <input type="text" className="fr-input" placeholder="https://... или vless://..."
                                        value={subUrl} onChange={(e) => setSubUrl(e.target.value)} autoFocus />
                                </div>
                                <div className="add-sub-actions">
                                    <Button variant="accent" size="sm" onClick={handleAddLink}>Добавить</Button>
                                    <Button variant="ghost" size="sm" onClick={() => { setShowAddSub(false); setSubUrl(''); setSubName(''); }}>Отмена</Button>
                                </div>
                            </div>
                        </CardBody>
                    </Card>
                )}

                {subscriptions.length === 0 && !showAddSub ? (
                    <Card variant="glass" hover={false} className="sub-empty-card">
                        <CardBody>
                            <div className="sub-empty">
                                <span className="sub-empty-icon">🔗</span>
                                <p>Нет добавленных подписок</p>
                                <span className="sub-empty-hint">Нажмите «Добавить» чтобы вставить URL-подписку от вашего VPN-провайдера</span>
                                <Button variant="primary" size="sm" onClick={() => setShowAddSub(true)} style={{ marginTop: '12px' }}>
                                    Добавить подписку
                                </Button>
                            </div>
                        </CardBody>
                    </Card>
                ) : (
                    <div className="sub-list">
                        {subscriptions.map(sub => (
                            <Card key={sub.id} variant="glass" className="sub-item">
                                <div className="sub-item-content">
                                    <div className="sub-item-left">
                                        <span className="sub-item-icon">🌐</span>
                                        <div className="sub-item-info">
                                            <span className="sub-item-name">{sub.name}</span>
                                            <span className="sub-item-url">{sub.url.length > 50 ? sub.url.slice(0, 50) + '...' : sub.url}</span>
                                        </div>
                                    </div>
                                    <div className="sub-item-right">
                                        <span className="sub-item-meta">{sub.server_count} серверов</span>
                                        <Button variant="ghost" size="sm" onClick={() => handleRemoveSub(sub.id)}>
                                            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><line x1="18" y1="6" x2="6" y2="18" /><line x1="6" y1="6" x2="18" y2="18" /></svg>
                                        </Button>
                                    </div>
                                </div>
                            </Card>
                        ))}
                    </div>
                )}

                {subscriptions.length > 0 && (
                    <Card variant="glass" hover={false} className="sub-auto-update">
                        <CardBody>
                            <Toggle id="auto-update" label="Автообновление подписок"
                                description="Обновлять список серверов каждые 6 часов"
                                checked={autoUpdate} onChange={setAutoUpdate} />
                        </CardBody>
                    </Card>
                )}
            </div>

            {/* === SERVERS === */}
            <div className="servers-section">
                <div className="servers-section-header">
                    <h3 className="servers-section-title">✦ Список серверов ({servers.length})</h3>
                    <div className="servers-section-actions">
                        <Button variant="secondary" size="sm" loading={isPinging} onClick={handlePingAll}
                            icon={<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><circle cx="12" cy="12" r="10" /><polyline points="12 6 12 12 16 14" /></svg>}>
                            Пинг
                        </Button>
                        <Button variant="secondary" size="sm" loading={isTestingSpeed} onClick={handleSpeedTestAll}
                            icon={<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><path d="M3 12h3l3-8 4 16 3-8h5" /></svg>}>
                            Скорость
                        </Button>
                        <Button variant="secondary" size="sm" loading={isUpdating} onClick={handleUpdateAll}
                            icon={<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><polyline points="23 4 23 10 17 10" /><path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10" /></svg>}>
                            Обновить
                        </Button>
                    </div>
                </div>

                {servers.length === 0 ? (
                    <Card variant="glass" hover={false}>
                        <CardBody>
                            <div className="sub-empty">
                                <span className="sub-empty-icon">📡</span>
                                <p>Нет серверов</p>
                                <span className="sub-empty-hint">Добавьте подписку и нажмите «Обновить»</span>
                            </div>
                        </CardBody>
                    </Card>
                ) : (
                    <div className="servers-list stagger-children">
                        {servers.map(server => (
                            <div
                                key={server.id}
                                className={`server-item fantasy-border ${activeServerId === server.id ? 'server-active' : ''}`}
                                onClick={() => handleSelectServer(server)}
                            >
                                <div className="server-item-content">
                                    <div className="server-item-left">
                                        <span className="server-country">{server.country || '🌍'}</span>
                                        <div className="server-info">
                                            <span className="server-name">{server.name}</span>
                                            <span className="server-address">{server.address}:{server.port}</span>
                                        </div>
                                    </div>
                                    <div className="server-item-right">
                                        <span className="server-protocol">
                                            {typeof server.protocol === 'string' ? server.protocol.toUpperCase() : 'VLESS'}
                                        </span>
                                        <span className="server-ping" style={{ color: getPingColor(server) }}>
                                            {getPingLabel(server)}
                                        </span>
                                        <span className="server-speed">
                                            {formatSpeedLabel(server)}
                                        </span>
                                        {activeServerId === server.id && (
                                            <span className="server-active-badge">✓ АКТИВНЫЙ</span>
                                        )}
                                    </div>
                                </div>
                            </div>
                        ))}
                    </div>
                )}
            </div>
        </div>
    );
}
