import { useState, useEffect, useRef } from 'react';
import Card, { CardBody } from '../components/UI/Card';
import Button from '../components/UI/Button';
import { useI18n } from '../hooks/useI18n';
import * as api from '../api/tauri';
import './Logs.css';

export default function Logs() {
    const { t } = useI18n();
    const [logs, setLogs] = useState([]);
    const [filter, setFilter] = useState('all');
    const logEndRef = useRef(null);

    // Poll logs from backend every second
    useEffect(() => {
        const fetch = async () => {
            try {
                const result = await api.getLogs();
                if (result && result.length > 0) {
                    setLogs(result);
                }
            } catch (e) { }
        };
        fetch();
        const interval = setInterval(fetch, 1000);
        return () => clearInterval(interval);
    }, []);

    // Auto-scroll to bottom
    useEffect(() => {
        logEndRef.current?.scrollIntoView({ behavior: 'smooth' });
    }, [logs]);

    const filteredLogs = filter === 'all'
        ? logs
        : logs.filter(l => l.level === filter);

    const clearLogs = async () => {
        try {
            await api.clearLogs();
            setLogs([]);
        } catch (e) { }
    };

    return (
        <div className="logs-page">
            <div className="page-header">
                <h1><span className="text-gradient">{t('logsTitle')}</span></h1>
                <p>{t('logsSubtitle')}</p>
            </div>

            <div className="logs-toolbar">
                <div className="logs-filters">
                    {['all', 'info', 'success', 'warn', 'error'].map(f => (
                        <button
                            key={f}
                            className={`log-filter-btn ${filter === f ? 'active' : ''}`}
                            onClick={() => setFilter(f)}
                        >
                            {f === 'all' ? t('filterAll') : f === 'info' ? t('filterInfo') : f === 'success' ? t('filterSuccess') : f === 'warn' ? t('filterWarn') : t('filterError')}
                        </button>
                    ))}
                </div>
                <Button variant="ghost" size="sm" onClick={clearLogs}>{t('clear')}</Button>
            </div>

            <Card variant="glass" hover={false} className="logs-card">
                <CardBody>
                    <div className="logs-container">
                        {filteredLogs.length === 0 ? (
                            <div className="logs-empty">
                                {logs.length === 0 ? t('logsEmpty') : t('logsFilterEmpty')}
                            </div>
                        ) : (
                            filteredLogs.map((log, i) => (
                                <div key={i} className={`log-entry log-level-${log.level}`}>
                                    <span className="log-time">{log.time}</span>
                                    <span className="log-badge">{log.level}</span>
                                    <span className="log-message">{log.message}</span>
                                </div>
                            ))
                        )}
                        <div ref={logEndRef} />
                    </div>
                </CardBody>
            </Card>
        </div>
    );
}
