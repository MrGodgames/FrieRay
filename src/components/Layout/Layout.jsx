import Sidebar from './Sidebar';
import MagicParticles from '../Particles/MagicParticles';
import characterImg from '../../assets/images/character.png';
import './Layout.css';

export default function Layout({ children }) {
    return (
        <div className="app-layout">
            {/* Pure CSS gradient background — no image */}
            <div className="app-bg">
                <div className="app-bg-stars" />
            </div>

            <MagicParticles />
            <Sidebar />
            <main className="main-content">
                <div className="main-content-inner page-enter">
                    {children}
                </div>
            </main>

            {/* Frieren character — right side */}
            <div className="character-decoration">
                <img src={characterImg} alt="" className="character-img" />
                <div className="character-glow" />
            </div>
        </div>
    );
}
