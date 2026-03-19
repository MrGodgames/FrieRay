import { useEffect, useRef, useCallback } from 'react';
import { useTheme } from '../../hooks/useTheme';
import './MagicParticles.css';

export default function MagicParticles() {
    const canvasRef = useRef(null);
    const animationRef = useRef(null);
    const particlesRef = useRef([]);
    const { theme } = useTheme();

    const createParticle = useCallback((canvas) => {
        return {
            x: Math.random() * canvas.width,
            y: canvas.height + Math.random() * 100,
            size: Math.random() * 3 + 1,
            speedX: (Math.random() - 0.5) * 0.5,
            speedY: -(Math.random() * 1 + 0.3),
            opacity: Math.random() * 0.6 + 0.2,
            fadeSpeed: Math.random() * 0.003 + 0.001,
            hue: Math.random() > 0.5 ? 160 : 270, // green or purple
            pulse: Math.random() * Math.PI * 2,
            pulseSpeed: Math.random() * 0.02 + 0.01,
        };
    }, []);

    useEffect(() => {
        const canvas = canvasRef.current;
        if (!canvas) return;

        const ctx = canvas.getContext('2d');
        const maxParticles = 50;

        const handleResize = () => {
            canvas.width = window.innerWidth;
            canvas.height = window.innerHeight;
        };

        handleResize();
        window.addEventListener('resize', handleResize);

        // Initialize particles
        particlesRef.current = Array.from({ length: maxParticles }, () => createParticle(canvas));

        const animate = () => {
            ctx.clearRect(0, 0, canvas.width, canvas.height);

            particlesRef.current.forEach((p, i) => {
                p.x += p.speedX;
                p.y += p.speedY;
                p.pulse += p.pulseSpeed;

                const pulseSize = p.size + Math.sin(p.pulse) * 0.5;
                const pulseOpacity = p.opacity + Math.sin(p.pulse) * 0.1;

                // Draw glow
                const gradient = ctx.createRadialGradient(p.x, p.y, 0, p.x, p.y, pulseSize * 4);

                if (p.hue === 160) {
                    // Green/teal magic
                    gradient.addColorStop(0, `hsla(${p.hue}, 90%, 65%, ${pulseOpacity * 0.6})`);
                    gradient.addColorStop(0.5, `hsla(${p.hue}, 80%, 55%, ${pulseOpacity * 0.2})`);
                    gradient.addColorStop(1, `hsla(${p.hue}, 70%, 50%, 0)`);
                } else {
                    // Purple magic
                    gradient.addColorStop(0, `hsla(${p.hue}, 80%, 70%, ${pulseOpacity * 0.5})`);
                    gradient.addColorStop(0.5, `hsla(${p.hue}, 70%, 60%, ${pulseOpacity * 0.15})`);
                    gradient.addColorStop(1, `hsla(${p.hue}, 60%, 50%, 0)`);
                }

                ctx.beginPath();
                ctx.arc(p.x, p.y, pulseSize * 4, 0, Math.PI * 2);
                ctx.fillStyle = gradient;
                ctx.fill();

                // Draw core
                ctx.beginPath();
                ctx.arc(p.x, p.y, pulseSize, 0, Math.PI * 2);
                ctx.fillStyle = p.hue === 160
                    ? `hsla(${p.hue}, 100%, 80%, ${pulseOpacity})`
                    : `hsla(${p.hue}, 100%, 85%, ${pulseOpacity})`;
                ctx.fill();

                // Reset if out of bounds
                if (p.y < -20 || p.x < -20 || p.x > canvas.width + 20) {
                    particlesRef.current[i] = createParticle(canvas);
                }
            });

            animationRef.current = requestAnimationFrame(animate);
        };

        animate();

        return () => {
            window.removeEventListener('resize', handleResize);
            if (animationRef.current) {
                cancelAnimationFrame(animationRef.current);
            }
        };
    }, [createParticle, theme]);

    return (
        <canvas
            ref={canvasRef}
            className="magic-particles-canvas"
            aria-hidden="true"
        />
    );
}
