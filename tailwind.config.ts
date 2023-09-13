const withMT = require("@material-tailwind/react/utils/withMT");

const config = withMT({
  content: [
    "./pages/**/*.{js,ts,jsx,tsx,mdx}",
    "./components/**/*.{js,ts,jsx,tsx,mdx}",
    "./app/**/*.{js,ts,jsx,tsx,mdx}",
  ],
  theme: {
    extend: {
      colors: {
        "pm-primary": "var(--primary)",
        "pm-secondary": "var(--secondary)",
        "pm-danger": "var(--danger)",
        "pm-warning": "var(--warning)",
        "pm-success": "var(--success)",
        "pm-background": "var(--background)",
        "pm-foreground": "var(--foreground)",
      },
    },
  },
  plugins: [],
});
export default config;
