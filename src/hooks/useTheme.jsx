import { createContext, useContext, useState, useEffect } from 'react';

const ThemeContext = createContext();

export function ThemeProvider({ children }) {
    const [theme, setTheme] = useState(() => {
        const saved = localStorage.getItem('frieray-theme');
        return saved || 'dark';
    });
    const [visualStyle, setVisualStyle] = useState(() => {
        const saved = localStorage.getItem('frieray-visual-style');
        return saved || 'fantasy';
    });

    useEffect(() => {
        document.documentElement.setAttribute('data-theme', theme);
        localStorage.setItem('frieray-theme', theme);
    }, [theme]);

    useEffect(() => {
        document.documentElement.setAttribute('data-ui-style', visualStyle);
        localStorage.setItem('frieray-visual-style', visualStyle);
    }, [visualStyle]);

    const toggleTheme = () => {
        setTheme(prev => prev === 'dark' ? 'light' : 'dark');
    };

    const toggleVisualStyle = () => {
        setVisualStyle(prev => prev === 'strict' ? 'fantasy' : 'strict');
    };

    return (
        <ThemeContext.Provider value={{ theme, setTheme, toggleTheme, visualStyle, setVisualStyle, toggleVisualStyle }}>
            {children}
        </ThemeContext.Provider>
    );
}

export function useTheme() {
    const context = useContext(ThemeContext);
    if (!context) {
        throw new Error('useTheme must be used within ThemeProvider');
    }
    return context;
}
