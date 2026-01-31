/** @type {import('tailwindcss').Config} */
export default {
  darkMode: ["class"],
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    screens: {
      'xs': '400px',
      'sm': '640px',
      'md': '768px',
      'lg': '1024px',
      'xl': '1280px',
      '2xl': '1536px',
    },
    extend: {
      fontFamily: {
        heading: ["Kalam", "cursive"],
        body: ["Patrick Hand", "cursive"],
      },
      borderRadius: {
        lg: "var(--radius)",
        md: "calc(var(--radius) - 2px)",
        sm: "calc(var(--radius) - 4px)",
        wobbly: "255px 15px 225px 15px / 15px 225px 15px 255px",
        wobblyMd: "25px 55px 20px 45px / 45px 20px 55px 25px",
      },
      boxShadow: {
        hard: "4px 4px 0px 0px var(--shadow-hard-color, #2d2d2d)",
        "hard-hover": "2px 2px 0px 0px var(--shadow-hard-color, #2d2d2d)",
        "hard-sm": "2px 2px 0px 0px var(--shadow-hard-color, #2d2d2d)",
      },
      colors: {
        background: "var(--background)",
        foreground: "var(--foreground)",
        card: {
          DEFAULT: "var(--card)",
          foreground: "var(--card-foreground)",
        },
        popover: {
          DEFAULT: "var(--popover)",
          foreground: "var(--popover-foreground)",
        },
        primary: {
          DEFAULT: "var(--primary)",
          foreground: "var(--primary-foreground)",
        },
        secondary: {
          DEFAULT: "var(--secondary)",
          foreground: "var(--secondary-foreground)",
        },
        muted: {
          DEFAULT: "var(--muted)",
          foreground: "var(--muted-foreground)",
        },
        accent: {
          DEFAULT: "var(--accent)",
          foreground: "var(--accent-foreground)",
        },
        destructive: {
          DEFAULT: "var(--destructive)",
          foreground: "var(--destructive-foreground)",
        },
        border: "var(--border)",
        input: "var(--input)",
        ring: "var(--ring)",
        "ring-focus": "var(--ring-focus)",
        chart: {
          "1": "var(--chart-1)",
          "2": "var(--chart-2)",
          "3": "var(--chart-3)",
          "4": "var(--chart-4)",
          "5": "var(--chart-5)",
        },
        success: {
          DEFAULT: "var(--success)",
          foreground: "var(--success-foreground)",
        },
        warning: {
          DEFAULT: "var(--warning)",
          foreground: "var(--warning-foreground)",
        },
        error: {
          DEFAULT: "var(--error)",
          foreground: "var(--error-foreground)",
        },
        info: {
          DEFAULT: "var(--info)",
          foreground: "var(--info-foreground)",
        },
        thinking: {
          DEFAULT: "var(--thinking)",
          foreground: "var(--thinking-foreground)",
        },
        overlay: "var(--overlay)",
        "shadow-hard-color": "var(--shadow-hard-color)",
        "diff-added": {
          DEFAULT: "var(--diff-added-bg)",
          text: "var(--diff-added-text)",
        },
        "diff-removed": {
          DEFAULT: "var(--diff-removed-bg)",
          text: "var(--diff-removed-text)",
        },
      },
    },
  },
  plugins: [],
}
