import Card, { CardHeader, CardBody } from '../components/UI/Card';
import Button from '../components/UI/Button';
import Toggle from '../components/UI/Toggle';
import { useState } from 'react';
import './Routing.css';

const defaultRules = [
    { id: 1, name: 'Bypass LAN', type: 'ip', value: '192.168.0.0/16, 10.0.0.0/8, 172.16.0.0/12', action: 'direct', enabled: true },
    { id: 2, name: 'Bypass localhost', type: 'domain', value: 'localhost, *.local', action: 'direct', enabled: true },
    { id: 3, name: 'Block ads', type: 'domain', value: 'geosite:category-ads-all', action: 'block', enabled: false },
    { id: 4, name: 'Direct RU sites', type: 'domain', value: 'geosite:category-ru', action: 'direct', enabled: false },
];

export default function Routing() {
    const [rules, setRules] = useState(defaultRules);

    const toggleRule = (id) => {
        setRules(prev => prev.map(r =>
            r.id === id ? { ...r, enabled: !r.enabled } : r
        ));
    };

    const getActionBadge = (action) => {
        const styles = {
            direct: { bg: 'rgba(45, 232, 160, 0.1)', color: 'var(--accent-400)', label: 'Напрямую' },
            proxy: { bg: 'rgba(139, 106, 255, 0.1)', color: 'var(--primary-400)', label: 'Прокси' },
            block: { bg: 'rgba(255, 107, 138, 0.1)', color: 'var(--error)', label: 'Блок' },
        };
        const s = styles[action] || styles.proxy;
        return <span className="rule-action-badge" style={{ background: s.bg, color: s.color }}>{s.label}</span>;
    };

    return (
        <div className="routing-page">
            <div className="page-header">
                <h1><span className="text-gradient">Маршрутизация</span></h1>
                <p>Правила маршрутизации трафика</p>
            </div>

            <div className="routing-actions">
                <Button variant="primary" icon={
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                        <line x1="12" y1="5" x2="12" y2="19" /><line x1="5" y1="12" x2="19" y2="12" />
                    </svg>
                }>
                    Добавить правило
                </Button>
            </div>

            <div className="routing-rules stagger-children">
                {rules.map(rule => (
                    <Card key={rule.id} variant="glass" className="rule-item">
                        <div className="rule-content">
                            <div className="rule-left">
                                <div className="rule-info">
                                    <span className="rule-name">{rule.name}</span>
                                    <span className="rule-value">{rule.value}</span>
                                </div>
                            </div>
                            <div className="rule-right">
                                {getActionBadge(rule.action)}
                                <Toggle
                                    id={`rule-${rule.id}`}
                                    checked={rule.enabled}
                                    onChange={() => toggleRule(rule.id)}
                                />
                            </div>
                        </div>
                    </Card>
                ))}
            </div>
        </div>
    );
}
