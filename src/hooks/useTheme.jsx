import { createContext, useContext, useState, useEffect } from 'react';

const ThemeContext = createContext();
const THEME_KEY = 'frieray-theme';
const UI_STYLE_KEY = 'frieray-ui-style';

export function ThemeProvider({ children }) {
    const [theme, setTheme] = useState(() => {
        const saved = localStorage.getItem(THEME_KEY);
        return saved || 'dark';
    });
    const [uiStyle, setUiStyle] = useState(() => {
        const saved = localStorage.getItem(UI_STYLE_KEY);
        return saved || 'fantasy';
    });

    useEffect(() => {
        document.documentElement.setAttribute('data-theme', theme);
        localStorage.setItem(THEME_KEY, theme);
    }, [theme]);

    useEffect(() => {
        document.documentElement.setAttribute('data-ui-style', uiStyle);
        localStorage.setItem(UI_STYLE_KEY, uiStyle);
    }, [uiStyle]);

    const toggleTheme = () => setTheme(prev => prev === 'dark' ? 'light' : 'dark');
    const toggleUiStyle = () => setUiStyle(prev => prev === 'fantasy' ? 'classic' : 'fantasy');

    return (
        <ThemeContext.Provider
            value={{
                theme,
                setTheme,
                toggleTheme,
                uiStyle,
                setUiStyle,
                toggleUiStyle,
                isClassic: uiStyle === 'classic',
            }}
        >
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
