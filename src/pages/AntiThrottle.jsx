import { useState } from 'react';
import Card, { CardHeader, CardBody } from '../components/UI/Card';
import Toggle from '../components/UI/Toggle';
import Button from '../components/UI/Button';
import { useTheme } from '../hooks/useTheme';
import './AntiThrottle.css';

const services = [
    {
        id: 'youtube',
        name: 'YouTube',
        icon: '📺',
        description: 'youtube.com, googlevideo.com, ytimg.com',
        domains: ['youtube.com', 'googlevideo.com', 'ytimg.com', 'ggpht.com', 'youtu.be'],
    },
    {
        id: 'discord',
        name: 'Discord',
        icon: '💬',
        description: 'discord.com, discord.gg, discordapp.com',
        domains: ['discord.com', 'discord.gg', 'discordapp.com', 'discord.media', 'discordapp.net'],
    },
    {
        id: 'telegram',
        name: 'Telegram',
        icon: '✈️',
        description: 'telegram.org, t.me, телеграм-серверы',
        domains: ['telegram.org', 't.me', 'core.telegram.org', 'web.telegram.org'],
    },
    {
        id: 'instagram',
        name: 'Instagram',
        icon: '📷',
        description: 'instagram.com, cdninstagram.com',
        domains: ['instagram.com', 'cdninstagram.com', 'instagram.net'],
    },
    {
        id: 'twitter',
        name: 'X (Twitter)',
        icon: '🐦',
        description: 'x.com, twitter.com, twimg.com',
        domains: ['x.com', 'twitter.com', 'twimg.com', 't.co', 'twitpic.com'],
    },
    {
        id: 'facebook',
        name: 'Facebook',
        icon: '👤',
        description: 'facebook.com, fbcdn.net, fb.com',
        domains: ['facebook.com', 'fbcdn.net', 'fb.com', 'fb.me', 'fbsbx.com'],
    },
    {
        id: 'spotify',
        name: 'Spotify',
        icon: '🎵',
        description: 'spotify.com, scdn.co, spotifycdn.com',
        domains: ['spotify.com', 'scdn.co', 'spotifycdn.com', 'audio-ak-spotify-com.akamaized.net'],
    },
];

const strategies = [
    { id: 'auto', name: 'Авто', description: 'Автоматический подбор стратегии' },
    { id: 'split', name: 'Фрагментация', description: 'Разбиение TLS ClientHello' },
    { id: 'fake', name: 'Fake-пакет', description: 'Отправка поддельного пакета перед реальным' },
    { id: 'desync', name: 'Десинхронизация', description: 'Полная десинхронизация DPI' },
];

export default function AntiThrottle() {
    const { visualStyle } = useTheme();
    const isStrict = visualStyle === 'strict';
    const [enabled, setEnabled] = useState(false);
    const [activeServices, setActiveServices] = useState({ youtube: true, discord: true, telegram: true });
    const [strategy, setStrategy] = useState('auto');
    const [bypassVpn, setBypassVpn] = useState(true);

    const toggleService = (id) => {
        setActiveServices(prev => ({ ...prev, [id]: !prev[id] }));
    };

    const enabledCount = Object.values(activeServices).filter(Boolean).length;

    return (
        <div className="anti-throttle-page">
            <div className="page-header">
                <h1><span className="text-gradient">Антизамедление</span></h1>
                <p>Обход DPI-замедления через Zapret</p>
            </div>

            {/* Master toggle */}
            <Card variant="glass" hover={false} className="fantasy-border corner-ornaments">
                <CardBody>
                    <div className="at-master">
                        <div className="at-master-info">
                            <div className="at-master-icon">⚡</div>
                            <div className="at-master-text">
                                <h3>Zapret DPI Bypass</h3>
                                <p>Убирает замедление без VPN — трафик идёт напрямую</p>
                            </div>
                        </div>
                        <Toggle
                            id="zapret-master"
                            checked={enabled}
                            onChange={setEnabled}
                        />
                    </div>

                    {enabled && (
                        <div className="at-master-status animate-fade-in">
                            <div className="at-status-indicator active">
                                <span className="at-status-dot" />
                                <span>Zapret активен</span>
                            </div>
                            <span className="at-status-count">{enabledCount} сервис(ов)</span>
                        </div>
                    )}
                </CardBody>
            </Card>

            {/* VPN bypass note */}
            <Card variant="glass" hover={false} className="at-bypass-card">
                <CardBody>
                    <Toggle
                        id="bypass-vpn"
                        label="Исключить из VPN"
                        description="Сервисы с Zapret не будут идти через VPN — только прямое подключение с обходом DPI"
                        checked={bypassVpn}
                        onChange={setBypassVpn}
                    />
                </CardBody>
            </Card>

            {/* DPI Strategy */}
            <div className="at-section">
                <h4 className="at-section-title">{isStrict ? 'Стратегия обхода' : '✦ Стратегия обхода'}</h4>
                <div className="at-strategies">
                    {strategies.map(s => (
                        <button
                            key={s.id}
                            className={`at-strategy-btn ${strategy === s.id ? 'active' : ''}`}
                            onClick={() => setStrategy(s.id)}
                        >
                            <span className="at-strategy-name">{s.name}</span>
                            <span className="at-strategy-desc">{s.description}</span>
                        </button>
                    ))}
                </div>
            </div>

            {/* Services */}
            <div className="at-section">
                <h4 className="at-section-title">{isStrict ? 'Сервисы' : '✦ Сервисы'}</h4>
                <div className="at-services stagger-children">
                    {services.map(service => (
                        <Card
                            key={service.id}
                            variant="glass"
                            className={`at-service-card ${activeServices[service.id] ? 'at-service-active' : ''}`}
                            hover={true}
                        >
                            <div className="at-service-content">
                                <div className="at-service-left">
                                    <span className="at-service-icon">{service.icon}</span>
                                    <div className="at-service-info">
                                        <span className="at-service-name">{service.name}</span>
                                        <span className="at-service-domains">{service.description}</span>
                                    </div>
                                </div>
                                <div className="at-service-right">
                                    {activeServices[service.id] && bypassVpn && (
                                        <span className="at-badge at-badge-direct">DIRECT</span>
                                    )}
                                    {activeServices[service.id] && (
                                        <span className="at-badge at-badge-zapret">ZAPRET</span>
                                    )}
                                    <Toggle
                                        id={`service-${service.id}`}
                                        checked={activeServices[service.id] || false}
                                        onChange={() => toggleService(service.id)}
                                    />
                                </div>
                            </div>
                        </Card>
                    ))}
                </div>
            </div>

            {/* Info */}
            <Card variant="glass" hover={false} className="at-info-card">
                <CardBody>
                    <div className="at-info">
                        <span className="at-info-icon">💡</span>
                        <div className="at-info-text">
                            <p><strong>Как это работает?</strong></p>
                            <p>Zapret модифицирует сетевые пакеты, обходя DPI-систему провайдера. Трафик идёт напрямую к серверам сервиса — без VPN, без шифрования тоннеля. Это быстрее чем VPN, но не скрывает ваш IP.</p>
                            <p className="at-info-note">Если «Исключить из VPN» включено, сервисы с Zapret будут работать напрямую, даже если VPN активен.</p>
                        </div>
                    </div>
                </CardBody>
            </Card>
        </div>
    );
}
