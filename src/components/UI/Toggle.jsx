import './Toggle.css';

export default function Toggle({ checked, onChange, label, description, id }) {
    return (
        <label className="fr-toggle" htmlFor={id}>
            <div className="fr-toggle-info">
                {label && <span className="fr-toggle-label">{label}</span>}
                {description && <span className="fr-toggle-desc">{description}</span>}
            </div>
            <div className="fr-toggle-track-wrapper">
                <input
                    type="checkbox"
                    id={id}
                    className="fr-toggle-input"
                    checked={checked}
                    onChange={e => onChange(e.target.checked)}
                />
                <div className="fr-toggle-track">
                    <div className="fr-toggle-thumb" />
                </div>
            </div>
        </label>
    );
}
