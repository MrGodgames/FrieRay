import './Card.css';

export default function Card({
    children,
    className = '',
    variant = 'default',
    glow = false,
    hover = true,
    onClick,
    ...props
}) {
    const classes = [
        'fr-card',
        `fr-card-${variant}`,
        glow && 'fr-card-glow',
        hover && 'hover-lift',
        className,
    ].filter(Boolean).join(' ');

    return (
        <div className={classes} onClick={onClick} {...props}>
            {children}
        </div>
    );
}

export function CardHeader({ children, className = '' }) {
    return <div className={`fr-card-header ${className}`}>{children}</div>;
}

export function CardBody({ children, className = '' }) {
    return <div className={`fr-card-body ${className}`}>{children}</div>;
}

export function CardFooter({ children, className = '' }) {
    return <div className={`fr-card-footer ${className}`}>{children}</div>;
}
