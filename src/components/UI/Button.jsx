import './Button.css';

export default function Button({
    children,
    variant = 'primary',
    size = 'md',
    icon,
    loading = false,
    disabled = false,
    className = '',
    ...props
}) {
    const classes = [
        'fr-btn',
        `fr-btn-${variant}`,
        `fr-btn-${size}`,
        loading && 'fr-btn-loading',
        className,
    ].filter(Boolean).join(' ');

    return (
        <button className={classes} disabled={disabled || loading} {...props}>
            {loading && (
                <span className="fr-btn-spinner">
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                        <path d="M12 2v4M12 18v4M4.93 4.93l2.83 2.83M16.24 16.24l2.83 2.83M2 12h4M18 12h4M4.93 19.07l2.83-2.83M16.24 7.76l2.83-2.83" />
                    </svg>
                </span>
            )}
            {icon && !loading && <span className="fr-btn-icon">{icon}</span>}
            {children && <span className="fr-btn-label">{children}</span>}
        </button>
    );
}
