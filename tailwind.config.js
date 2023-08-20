/** @type {import('tailwindcss').Config} */
module.exports = {
  content: ["./src/**/*"],
  theme: {
    extend: {},
  },
  plugins: [require("@tailwindcss/typography"), require("@tailwindcss/forms")],
};
