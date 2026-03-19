import { useState } from 'react';
import Card, { CardHeader, CardBody } from '../components/UI/Card';
import Toggle from '../components/UI/Toggle';
import Button from '../components/UI/Button';
import './SplitTunnel.css';

const defaultApps = [
    { id: 'chrome', name: 'Google Chrome', icon: '🌐', category: 'Браузеры', enabled: true },
    { id: 'firefox', name: 'Firefox', icon: '🦊', category: 'Браузеры', enabled: false },
    { id: 'safari', name: 'Safari', icon: '🧭', category: 'Браузеры', enabled: false },
    { id: 'telegram', name: 'Telegram', icon: '💬', category: 'Мессенджеры', enabled: true },
    { id: 'discord', name: 'Discord', icon: '🎮', category: 'Мессенджеры', enabled: true },
    { id: 'spotify', name: 'Spotify', icon: '🎵', category: 'Медиа', enabled: false },
    { id: 'youtube', name: 'YouTube (в браузере)', icon: '📺', category: 'Медиа', enabled: true },
    { id: 'steam', name: 'Steam', icon: '🎮', category: 'Игры', enabled: false },
    { id: 'terminal', name: 'Terminal', icon: '⌨️', category: 'Системные', enabled: false },
    { id: 'vscode', name: 'VS Code', icon: '💻', category: 'Разработка', enabled: false },
];

export default function SplitTunnel() {
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
        app.name.toLowerCase().includes(searchQuery.toLowerCase())
    );

    const groupedApps = filteredApps.reduce((acc, app) => {
        if (!acc[app.category]) acc[app.category] = [];
        acc[app.category].push(app);
        return acc;
    }, {});

    return (
        <div className="split-tunnel-page">
            <div className="page-header">
                <h1><span className="text-gradient">Split Tunnel</span></h1>
                <p>Выберите приложения, которые будут использовать прокси</p>
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
                                <span className="split-mode-label">Белый список</span>
                                <span className="split-mode-desc">Только выбранные приложения через прокси</span>
                            </div>
                        </button>
                        <button
                            className={`split-mode-btn ${mode === 'blacklist' ? 'active' : ''}`}
                            onClick={() => setMode('blacklist')}
                        >
                            <span className="split-mode-icon">🚫</span>
                            <div className="split-mode-info">
                                <span className="split-mode-label">Чёрный список</span>
                                <span className="split-mode-desc">Все приложения кроме выбранных через прокси</span>
                            </div>
                        </button>
                    </div>
                </CardBody>
            </Card>

            {/* Stats */}
            <div className="split-stats">
                <span className="split-stat">
                    <strong>{enabledCount}</strong> из {apps.length} приложений
                    {mode === 'whitelist' ? ' через прокси' : ' исключено'}
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
                    placeholder="Поиск приложений..."
                    value={searchQuery}
                    onChange={(e) => setSearchQuery(e.target.value)}
                />
            </div>

            {/* App list */}
            <div className="split-app-list">
                {Object.entries(groupedApps).map(([category, categoryApps]) => (
                    <div key={category} className="split-category">
                        <h4 className="split-category-title">{category}</h4>
                        <Card variant="glass" hover={false}>
                            <CardBody>
                                <div className="split-category-apps">
                                    {categoryApps.map(app => (
                                        <div key={app.id} className={`split-app-item ${app.enabled ? 'enabled' : ''}`}>
                                            <div className="split-app-info">
                                                <span className="split-app-icon">{app.icon}</span>
                                                <span className="split-app-name">{app.name}</span>
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
