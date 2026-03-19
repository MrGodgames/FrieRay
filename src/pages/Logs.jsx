import { useState, useEffect, useRef } from 'react';
import Card, { CardBody } from '../components/UI/Card';
import Button from '../components/UI/Button';
import * as api from '../api/tauri';
import './Logs.css';

export default function Logs() {
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
                <h1><span className="text-gradient">Логи</span></h1>
                <p>Журнал событий и подключений</p>
            </div>

            <div className="logs-toolbar">
                <div className="logs-filters">
                    {['all', 'info', 'success', 'warn', 'error'].map(f => (
                        <button
                            key={f}
                            className={`log-filter-btn ${filter === f ? 'active' : ''}`}
                            onClick={() => setFilter(f)}
                        >
                            {f === 'all' ? 'Все' : f === 'info' ? 'Инфо' : f === 'success' ? 'Успех' : f === 'warn' ? 'Предупр.' : 'Ошибки'}
                        </button>
                    ))}
                </div>
                <Button variant="ghost" size="sm" onClick={clearLogs}>Очистить</Button>
            </div>

            <Card variant="glass" hover={false} className="logs-card">
                <CardBody>
                    <div className="logs-container">
                        {filteredLogs.length === 0 ? (
                            <div className="logs-empty">
                                {logs.length === 0 ? 'Нет записей — попробуйте подключиться к серверу' : 'Нет записей с этим фильтром'}
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
