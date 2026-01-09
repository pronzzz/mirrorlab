/** @type {import('tailwindcss').Config} */
export default {
    content: [
        "./index.html",
        "./*.{js,ts,jsx,tsx}",
        "./components/**/*.{js,ts,jsx,tsx}",
        "./context/**/*.{js,ts,jsx,tsx}",
        "./services/**/*.{js,ts,jsx,tsx}",
    ],
    darkMode: 'class',
    theme: {
        extend: {
            fontFamily: {
                sans: ['Inter', 'sans-serif'],
            },
            colors: {
                // Coolors Palette: https://coolors.co/333333-643173-7d5ba6-86a59c-89ce94
                background: '#333333', // Dark Charcoal
                surface: '#3d3d3d',     // Slightly lighter for cards/surfaces
                primary: '#643173',     // Purple
                secondary: '#7d5ba6',   // Light Purple
                accent: '#86a59c',      // Sage
                highlight: '#89ce94',   // Green
                text: '#e0e0e0',        // Off-white text
            },
            animation: {
                'fade-in': 'fadeIn 0.6s ease-out forwards',
                'slide-up': 'slideUp 0.6s ease-out forwards',
                'scale-in': 'scaleIn 0.4s cubic-bezier(0.25, 0.46, 0.45, 0.94) both',
            },
            keyframes: {
                fadeIn: {
                    '0%': { opacity: '0' },
                    '100%': { opacity: '1' },
                },
                slideUp: {
                    '0%': { transform: 'translateY(20px)', opacity: '0' },
                    '100%': { transform: 'translateY(0)', opacity: '1' },
                },
                scaleIn: {
                    '0%': { transform: 'scale(0.9)', opacity: '0' },
                    '100%': { transform: 'scale(1)', opacity: '1' },
                }
            }
        }
    },
    plugins: [],
}
