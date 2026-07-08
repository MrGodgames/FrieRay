import { useState } from 'react';
import Card, { CardHeader, CardBody } from '../components/UI/Card';
import Toggle from '../components/UI/Toggle';
import Button from '../components/UI/Button';
import { useI18n } from '../hooks/useI18n';
import './SplitTunnel.css';

const defaultApps = [
    { id: 'chrome', name: 'Google Chrome', icon: '🌐', categoryKey: 'categoryBrowsers', enabled: true },
    { id: 'firefox', name: 'Firefox', icon: '🦊', categoryKey: 'categoryBrowsers', enabled: false },
    { id: 'safari', name: 'Safari', icon: '🧭', categoryKey: 'categoryBrowsers', enabled: false },
    { id: 'telegram', name: 'Telegram', icon: '💬', categoryKey: 'categoryMessengers', enabled: true },
    { id: 'discord', name: 'Discord', icon: '🎮', categoryKey: 'categoryMessengers', enabled: true },
    { id: 'spotify', name: 'Spotify', icon: '🎵', categoryKey: 'categoryMedia', enabled: false },
    { id: 'youtube', nameKey: 'youtubeBrowser', icon: '📺', categoryKey: 'categoryMedia', enabled: true },
    { id: 'steam', name: 'Steam', icon: '🎮', categoryKey: 'categoryGames', enabled: false },
    { id: 'terminal', name: 'Terminal', icon: '⌨️', categoryKey: 'categorySystem', enabled: false },
    { id: 'vscode', name: 'VS Code', icon: '💻', categoryKey: 'categoryDev', enabled: false },
];

export default function SplitTunnel() {
    const { t } = useI18n();
    const [mode, setMode] = useState('whitelist'); // whitelist = only selected apps use proxy
    const [apps, setApps] = useState(defaultApps);
    const [searchQuery, setSearchQuery] = useState('');

    const toggleApp = (id) => {
        setApps(prev => prev.map(app =>
            app.id === id ? { ...app, enabled: !app.enabled } : app
        ));
    };

    const enabledCount = apps.filter(a => a.enabled).length;

    const filteredApps = apps.filter(app =>
        (app.nameKey ? t(app.nameKey) : app.name).toLowerCase().includes(searchQuery.toLowerCase())
    );

    const groupedApps = filteredApps.reduce((acc, app) => {
        if (!acc[app.categoryKey]) acc[app.categoryKey] = [];
        acc[app.categoryKey].push(app);
        return acc;
    }, {});

    return (
        <div className="split-tunnel-page">
            <div className="page-header">
                <h1><span className="text-gradient">{t('splitTitle')}</span></h1>
                <p>{t('splitSubtitle')}</p>
            </div>

            {/* Mode selector */}
            <Card variant="glass" hover={false}>
                <CardBody>
                    <div className="split-mode-selector">
                        <button
                            className={`split-mode-btn ${mode === 'whitelist' ? 'active' : ''}`}
                            onClick={() => setMode('whitelist')}
                        >
                            <span className="split-mode-icon">✅</span>
                            <div className="split-mode-info">
                                <span className="split-mode-label">{t('whitelist')}</span>
                                <span className="split-mode-desc">{t('whitelistDesc')}</span>
                            </div>
                        </button>
                        <button
                            className={`split-mode-btn ${mode === 'blacklist' ? 'active' : ''}`}
                            onClick={() => setMode('blacklist')}
                        >
                            <span className="split-mode-icon">🚫</span>
                            <div className="split-mode-info">
                                <span className="split-mode-label">{t('blacklist')}</span>
                                <span className="split-mode-desc">{t('blacklistDesc')}</span>
                            </div>
                        </button>
                    </div>
                </CardBody>
            </Card>

            {/* Stats */}
            <div className="split-stats">
                <span className="split-stat">
                    {t('splitStats', { enabled: enabledCount, total: apps.length, mode })}
                </span>
            </div>

            {/* Search */}
            <div className="split-search">
                <svg className="split-search-icon" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                    <circle cx="11" cy="11" r="8" /><line x1="21" y1="21" x2="16.65" y2="16.65" />
                </svg>
                <input
                    type="text"
                    className="fr-input split-search-input"
                    placeholder={t('searchApps')}
                    value={searchQuery}
                    onChange={(e) => setSearchQuery(e.target.value)}
                />
            </div>

            {/* App list */}
            <div className="split-app-list">
                {Object.entries(groupedApps).map(([categoryKey, categoryApps]) => (
                    <div key={categoryKey} className="split-category">
                        <h4 className="split-category-title">{t(categoryKey)}</h4>
                        <Card variant="glass" hover={false}>
                            <CardBody>
                                <div className="split-category-apps">
                                    {categoryApps.map(app => (
                                        <div key={app.id} className={`split-app-item ${app.enabled ? 'enabled' : ''}`}>
                                            <div className="split-app-info">
                                                <span className="split-app-icon">{app.icon}</span>
                                                <span className="split-app-name">{app.nameKey ? t(app.nameKey) : app.name}</span>
                                            </div>
                                            <Toggle
                                                id={`app-${app.id}`}
                                                checked={app.enabled}
                                                onChange={() => toggleApp(app.id)}
                                            />
                                        </div>
                                    ))}
                                </div>
                            </CardBody>
                        </Card>
                    </div>
                ))}
            </div>
        </div>
    );
}
