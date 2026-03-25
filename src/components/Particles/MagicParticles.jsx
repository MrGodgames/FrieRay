import { useEffect, useRef, useCallback } from 'react';
import { useTheme } from '../../hooks/useTheme';
import './MagicParticles.css';

export default function MagicParticles() {
    const canvasRef = useRef(null);
    const animationRef = useRef(null);
    const particlesRef = useRef([]);
    const viewportRef = useRef({ width: 0, height: 0 });
    const { theme } = useTheme();

    const createParticle = useCallback((viewport, motionScale = 1) => {
        return {
            x: Math.random() * viewport.width,
            y: viewport.height + Math.random() * 120,
            size: Math.random() * 2.2 + 0.9,
            speedX: (Math.random() - 0.5) * 0.22 * motionScale,
            speedY: -(Math.random() * 0.55 + 0.18) * motionScale,
            opacity: Math.random() * 0.28 + 0.12,
            hue: Math.random() > 0.5 ? 160 : 270,
            pulse: Math.random() * Math.PI * 2,
            pulseSpeed: (Math.random() * 0.01 + 0.004) * motionScale,
        };
    }, []);

    useEffect(() => {
        const canvas = canvasRef.current;
        if (!canvas) return;

        const ctx = canvas.getContext('2d', { alpha: true, desynchronized: true });
        if (!ctx) return;

        let sceneConfig = {
            particleCount: 24,
            motionScale: 1,
            frameTime: 40,
            dpr: 1,
        };

        const mediaQuery = window.matchMedia('(prefers-reduced-motion: reduce)');

        const getSceneConfig = () => {
            const isCompact = window.innerWidth < 900;
            const prefersReducedMotion = mediaQuery.matches;

            return {
                particleCount: prefersReducedMotion ? 10 : isCompact ? 14 : 22,
                motionScale: prefersReducedMotion ? 0.65 : 1,
                frameTime: prefersReducedMotion ? 64 : 40,
                dpr: Math.min(window.devicePixelRatio || 1, isCompact ? 1.1 : 1.25),
            };
        };

        const rebuildScene = () => {
            sceneConfig = getSceneConfig();
            viewportRef.current = {
                width: window.innerWidth,
                height: window.innerHeight,
            };

            canvas.style.width = `${viewportRef.current.width}px`;
            canvas.style.height = `${viewportRef.current.height}px`;
            canvas.width = Math.floor(viewportRef.current.width * sceneConfig.dpr);
            canvas.height = Math.floor(viewportRef.current.height * sceneConfig.dpr);
            ctx.setTransform(sceneConfig.dpr, 0, 0, sceneConfig.dpr, 0, 0);
            particlesRef.current = Array.from(
                { length: sceneConfig.particleCount },
                () => createParticle(viewportRef.current, sceneConfig.motionScale)
            );
        };

        const handleResize = () => rebuildScene();
        const handleMotionChange = () => rebuildScene();

        rebuildScene();
        window.addEventListener('resize', handleResize);
        if (mediaQuery.addEventListener) {
            mediaQuery.addEventListener('change', handleMotionChange);
        } else {
            mediaQuery.addListener(handleMotionChange);
        }

        let lastFrameTime = 0;

        const animate = (timestamp = 0) => {
            animationRef.current = requestAnimationFrame(animate);

            if (document.hidden) return;
            if (timestamp - lastFrameTime < sceneConfig.frameTime) return;

            const delta = lastFrameTime ? (timestamp - lastFrameTime) / 16.67 : 1;
            lastFrameTime = timestamp;

            ctx.clearRect(0, 0, viewportRef.current.width, viewportRef.current.height);

            particlesRef.current.forEach((particle, index) => {
                particle.x += particle.speedX * delta;
                particle.y += particle.speedY * delta;
                particle.pulse += particle.pulseSpeed;

                const pulseSize = particle.size + Math.sin(particle.pulse) * 0.35;
                const pulseOpacity = particle.opacity + Math.sin(particle.pulse) * 0.06;
                const outerColor = particle.hue === 160
                    ? `hsla(${particle.hue}, 85%, 62%, ${pulseOpacity * 0.2})`
                    : `hsla(${particle.hue}, 72%, 66%, ${pulseOpacity * 0.18})`;
                const coreColor = particle.hue === 160
                    ? `hsla(${particle.hue}, 100%, 82%, ${pulseOpacity})`
                    : `hsla(${particle.hue}, 92%, 84%, ${pulseOpacity})`;

                ctx.beginPath();
                ctx.arc(particle.x, particle.y, pulseSize * 2.8, 0, Math.PI * 2);
                ctx.fillStyle = outerColor;
                ctx.fill();

                ctx.beginPath();
                ctx.arc(particle.x, particle.y, pulseSize, 0, Math.PI * 2);
                ctx.fillStyle = coreColor;
                ctx.fill();

                if (
                    particle.y < -20
                    || particle.x < -20
                    || particle.x > viewportRef.current.width + 20
                ) {
                    particlesRef.current[index] = createParticle(viewportRef.current, sceneConfig.motionScale);
                }
            });
        };

        animate();

        return () => {
            window.removeEventListener('resize', handleResize);
            if (mediaQuery.removeEventListener) {
                mediaQuery.removeEventListener('change', handleMotionChange);
            } else {
                mediaQuery.removeListener(handleMotionChange);
            }
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
