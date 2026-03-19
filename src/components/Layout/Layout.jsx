import Sidebar from './Sidebar';
import MagicParticles from '../Particles/MagicParticles';
import { useTheme } from '../../hooks/useTheme';
import characterImg from '../../assets/images/character.png';
import './Layout.css';

export default function Layout({ children }) {
    const { visualStyle } = useTheme();
    const isStrict = visualStyle === 'strict';

    return (
        <div className="app-layout">
            <div className="app-bg">
                <div className="app-bg-stars" />
                {isStrict && <div className="app-bg-grid" />}
            </div>

            {!isStrict && <MagicParticles />}
            <Sidebar />
            <main className="main-content">
                <div className="main-content-inner page-enter">
                    {children}
                </div>
            </main>

            {!isStrict && (
                <div className="character-decoration">
                    <img src={characterImg} alt="" className="character-img" />
                    <div className="character-glow" />
                </div>
            )}
        </div>
    );
}
