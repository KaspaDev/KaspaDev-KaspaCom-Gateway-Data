/** @type {import('tailwindcss').Config} */
module.exports = {
    content: [
        "./dashboard/**/*.{html,js}",
        "./src/**/*.{rs,html}"
    ],
    theme: {
        extend: {
            colors: {
                bg: {
                    primary: '#050510',
                    secondary: 'rgba(20, 20, 40, 0.6)',
                    card: 'rgba(30, 30, 60, 0.4)',
                },
                text: {
                    primary: '#e0faff',
                    secondary: '#8899ac',
                },
                accent: {
                    DEFAULT: '#49eac4', // Main actinidia green
                    glow: 'rgba(73, 234, 196, 0.3)',
                    dim: '#2a8c7d',
                },
                status: {
                    positive: '#00ff9d',
                    negative: '#ff4d6d',
                },
                krc20: '#49eac4',
                krc721: '#bd68ee',
                kns: '#f39c12',
                trends: '#3498db',
            },
            borderColor: {
                DEFAULT: 'rgba(73, 234, 196, 0.2)',
            },
            boxShadow: {
                'glass': '0 4px 30px rgba(0, 0, 0, 0.3)',
                'glow': '0 0 20px rgba(73, 234, 196, 0.3)',
            },
            backdropBlur: {
                'xs': '2px',
            },
            fontFamily: {
                sans: ['Inter', 'sans-serif'],
            },
        },
    },
    plugins: [],
}
