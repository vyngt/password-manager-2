import type { Config } from "tailwindcss";
const withMT = require("@material-tailwind/react/utils/withMT");

const config: Config = {
  content: [
    "./components/**/*.{js,ts,jsx,tsx,mdx}",
    "./app/**/*.{js,ts,jsx,tsx,mdx}",
  ],
  theme: {
    extend: {
      colors: {
        primary: "rgb(var(--primary) / <alpha-value> )",
        secondary: "rgb(var(--secondary) / <alpha-value> )",
        danger: "rgb(var(--danger) / <alpha-value> )",
        warning: "rgb(var(--warning) / <alpha-value> )",
        success: "rgb(var(--success) / <alpha-value> )",
        background: "rgb(var(--background) / <alpha-value>)",
        foreground: "rgb(var(--foreground) / <alpha-value>)",
        selection: "rgb(var(--selection) / <alpha-value>)",
      },
    },
  },
  plugins: [],
};

export default withMT(config);
