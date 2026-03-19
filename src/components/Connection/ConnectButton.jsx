import './ConnectButton.css';

export default function ConnectButton({ connected, connecting, onToggle }) {
    return (
        <div className="connect-wrapper">
            <div className="connect-area">
                {/* SVG Magic circles — fully transparent, no background */}
                <div className={`magic-circles ${connecting ? 'spinning' : ''} ${connected ? 'active' : ''}`}>
                    {/* Outer ring */}
                    <svg className="mc-ring mc-ring-1" viewBox="0 0 200 200" fill="none" xmlns="http://www.w3.org/2000/svg">
                        <circle cx="100" cy="100" r="96" stroke="url(#ring1)" strokeWidth="1" strokeDasharray="6 4" />
                        <circle cx="100" cy="100" r="88" stroke="url(#ring1)" strokeWidth="0.5" strokeDasharray="2 8" />
                        {/* Rune marks around the circle */}
                        {[0, 30, 60, 90, 120, 150, 180, 210, 240, 270, 300, 330].map((angle, i) => {
                            const rad = (angle * Math.PI) / 180;
                            const x = 100 + 92 * Math.cos(rad);
                            const y = 100 + 92 * Math.sin(rad);
                            return (
                                <text key={i} x={x} y={y} textAnchor="middle" dominantBaseline="middle"
                                    fill={i % 2 === 0 ? "rgba(139,106,255,0.6)" : "rgba(45,232,160,0.5)"}
                                    fontSize="6" fontFamily="serif"
                                    transform={`rotate(${angle + 90}, ${x}, ${y})`}>
                                    {['✦', '◇', '⟡', '✧', '◈', '⊹'][i % 6]}
                                </text>
                            );
                        })}
                        <defs>
                            <linearGradient id="ring1" x1="0" y1="0" x2="200" y2="200">
                                <stop offset="0%" stopColor="rgba(139,106,255,0.5)" />
                                <stop offset="50%" stopColor="rgba(45,232,160,0.4)" />
                                <stop offset="100%" stopColor="rgba(139,106,255,0.5)" />
                            </linearGradient>
                        </defs>
                    </svg>

                    {/* Middle ring */}
                    <svg className="mc-ring mc-ring-2" viewBox="0 0 200 200" fill="none" xmlns="http://www.w3.org/2000/svg">
                        <circle cx="100" cy="100" r="78" stroke="url(#ring2)" strokeWidth="1" />
                        <circle cx="100" cy="100" r="72" stroke="url(#ring2)" strokeWidth="0.5" strokeDasharray="4 6" />
                        {/* Geometric star pattern */}
                        <polygon
                            points="100,25 112,75 165,65 125,100 165,135 112,125 100,175 88,125 35,135 75,100 35,65 88,75"
                            stroke="rgba(139,106,255,0.25)" strokeWidth="0.5" fill="none"
                        />
                        <polygon
                            points="100,35 108,80 155,72 120,100 155,128 108,120 100,165 92,120 45,128 80,100 45,72 92,80"
                            stroke="rgba(45,232,160,0.2)" strokeWidth="0.5" fill="none"
                        />
                        <defs>
                            <linearGradient id="ring2" x1="200" y1="0" x2="0" y2="200">
                                <stop offset="0%" stopColor="rgba(45,232,160,0.4)" />
                                <stop offset="50%" stopColor="rgba(139,106,255,0.3)" />
                                <stop offset="100%" stopColor="rgba(45,232,160,0.4)" />
                            </linearGradient>
                        </defs>
                    </svg>

                    {/* Inner ring */}
                    <svg className="mc-ring mc-ring-3" viewBox="0 0 200 200" fill="none" xmlns="http://www.w3.org/2000/svg">
                        <circle cx="100" cy="100" r="58" stroke="rgba(139,106,255,0.3)" strokeWidth="0.8" />
                        <circle cx="100" cy="100" r="54" stroke="rgba(45,232,160,0.2)" strokeWidth="0.5" strokeDasharray="3 5" />
                        {/* Inner hexagon */}
                        <polygon
                            points="100,45 147,72.5 147,127.5 100,155 53,127.5 53,72.5"
                            stroke="rgba(139,106,255,0.2)" strokeWidth="0.8" fill="none"
                        />
                        {/* Cross pattern */}
                        <line x1="100" y1="42" x2="100" y2="58" stroke="rgba(139,106,255,0.2)" strokeWidth="0.5" />
                        <line x1="100" y1="142" x2="100" y2="158" stroke="rgba(139,106,255,0.2)" strokeWidth="0.5" />
                        <line x1="42" y1="100" x2="58" y2="100" stroke="rgba(139,106,255,0.2)" strokeWidth="0.5" />
                        <line x1="142" y1="100" x2="158" y2="100" stroke="rgba(139,106,255,0.2)" strokeWidth="0.5" />
                    </svg>
                </div>

                {/* The button */}
                <button
                    className={`connect-btn ${connected ? 'connected' : ''} ${connecting ? 'connecting' : ''}`}
                    onClick={onToggle}
                    disabled={connecting}
                >
                    <div className="connect-core">
                        {connecting ? (
                            <svg width="26" height="26" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="animate-spin">
                                <path d="M12 2v4M12 18v4M4.93 4.93l2.83 2.83M16.24 16.24l2.83 2.83M2 12h4M18 12h4M4.93 19.07l2.83-2.83M16.24 7.76l2.83-2.83" />
                            </svg>
                        ) : (
                            <span className="connect-rune">{connected ? '⟡' : '✦'}</span>
                        )}
                    </div>
                </button>
            </div>

            <span className="connect-label">
                {connecting ? '✦ Активация заклинания... ✦' : connected ? '✦ Связь установлена ✦' : '✦ Активировать ✦'}
            </span>
        </div>
    );
}
