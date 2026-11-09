/** @type {import('tailwindcss').Config} */
export default {
  content: ['./src/**/*.{html,js,svelte,ts}'],
  theme: {
    extend: {
      colors: {
        // macOS-style dark theme (matching Swift version)
        // Base colors - neutral dark grays
        'base': '#111113',           // Main background
        'mantle': '#0c0c0e',         // Darker background
        'crust': '#08080a',          // Darkest background

        // Surface colors
        'surface-0': '#18181b',      // Card background
        'surface-1': '#222227',      // Hover state
        'surface-2': '#303037',      // Active state

        // Overlay colors
        'overlay-0': '#8b8b94',      // Muted text (good contrast)
        'overlay-1': '#a1a1aa',      // Lighter muted
        'overlay-2': '#b8b8c0',      // Lightest muted

        // Text colors
        'text': '#f4f4f5',           // Primary text
        'subtext-1': '#d4d4d8',      // Secondary text
        'subtext-0': '#a1a1aa',      // Tertiary text

        // Accent colors - matching Swift app
        'pink': '#f472b6',
        'mauve': '#a78bfa',          // Purple (for donut chart)
        'red': '#f87171',            // Softer red
        'peach': '#fb923c',          // Orange
        'yellow': '#fbbf24',         // Yellow
        'green': '#34d399',          // Cyan-green (Swift uses cyan for green accent)
        'teal': '#2dd4bf',           // Teal/Cyan
        'sky': '#38bdf8',            // Cyan (primary accent)
        'blue': '#5b8def',           // macOS blue
        'lavender': '#a78bfa',       // Purple

        // Semantic aliases for the app
        'bg-primary': '#111113',      // Main background
        'bg-secondary': '#0c0c0e',    // Darker
        'bg-tertiary': '#08080a',     // Darkest
        'bg-card': '#18181b',         // Card background
        'bg-card-hover': '#222227',   // Card hover

        'border-primary': '#2a2a30',  // Primary border
        'border-secondary': '#26262b', // Secondary border
        'border-focus': '#5b8def',    // Focus ring

        'text-primary': '#f4f4f5',    // Primary text
        'text-secondary': '#d4d4d8',  // Secondary text
        'text-muted': '#8b8b94',      // Muted text

        // Accent colors mapped to semantic names
        'accent-primary': '#7c9cff',  // Cyan
        'accent-green': '#34d399',    // Bright green for positive
        'accent-cyan': '#38bdf8',     // Cyan
        'accent-purple': '#a78bfa',   // Purple
        'accent-orange': '#fb923c',   // Orange
        'accent-yellow': '#fbbf24',   // Yellow
        'accent-red': '#f87171',      // Red for negative
        'accent-blue': '#5b8def',     // macOS blue
        'accent-pink': '#f472b6',     // Pink
        'accent-teal': '#2dd4bf',     // Teal

        // Legacy aliases (for compatibility)
        surface: '#111113',
        'surface-light': '#0c0c0e',
        'surface-lighter': '#18181b',
        accent: '#7c9cff',
      },
      fontFamily: {
        sans: [
          'Inter',
          '-apple-system',
          'BlinkMacSystemFont',
          'Segoe UI',
          'Roboto',
          'Oxygen',
          'Ubuntu',
          'sans-serif',
        ],
        mono: [
          'JetBrains Mono',
          'SF Mono',
          'Consolas',
          'Monaco',
          'Liberation Mono',
          'monospace',
        ],
      },
      // Consistent typography scale
      fontSize: {
        'xs': ['0.75rem', { lineHeight: '1rem' }],      // 12px
        'sm': ['0.8125rem', { lineHeight: '1.25rem' }], // 13px
        'base': ['0.875rem', { lineHeight: '1.5rem' }], // 14px
        'lg': ['1rem', { lineHeight: '1.5rem' }],       // 16px
        'xl': ['1.125rem', { lineHeight: '1.75rem' }],  // 18px
        '2xl': ['1.25rem', { lineHeight: '1.75rem' }],  // 20px
      },
      // Consistent spacing scale
      spacing: {
        '0.5': '0.125rem',  // 2px
        '1': '0.25rem',     // 4px
        '1.5': '0.375rem',  // 6px
        '2': '0.5rem',      // 8px
        '2.5': '0.625rem',  // 10px
        '3': '0.75rem',     // 12px
        '3.5': '0.875rem',  // 14px
        '4': '1rem',        // 16px
        '5': '1.25rem',     // 20px
        '6': '1.5rem',      // 24px
      },
      borderRadius: {
        'sm': '0.25rem',    // 4px
        'DEFAULT': '0.375rem', // 6px
        'md': '0.5rem',     // 8px
        'lg': '0.75rem',    // 12px
      },
      boxShadow: {
        'card': '0 2px 8px rgba(0, 0, 0, 0.4)',
        'card-hover': '0 4px 12px rgba(0, 0, 0, 0.5)',
        'glow-sm': '0 0 10px rgba(124, 156, 255, 0.2)',     // Cyan glow
        'glow-md': '0 0 20px rgba(124, 156, 255, 0.3)',     // Cyan glow
        'glow-green': '0 0 15px rgba(52, 211, 153, 0.25)', // Green glow
        'glow-purple': '0 0 15px rgba(167, 139, 250, 0.25)', // Purple glow
      },
      animation: {
        'pulse-slow': 'pulse 3s cubic-bezier(0.4, 0, 0.6, 1) infinite',
        'fade-in': 'fadeIn 0.2s ease-out',
      },
      keyframes: {
        fadeIn: {
          '0%': { opacity: '0', transform: 'translateY(-4px)' },
          '100%': { opacity: '1', transform: 'translateY(0)' },
        },
      },
    },
  },
  plugins: [],
};
