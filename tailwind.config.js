/** @type {import('tailwindcss').Config} */
module.exports = {
    content: {
        files: ["*.html", "./src/**/*.rs"],
    },
    theme: {
        extend: {
            dropShadow: {
                'tauri-s': '0 0 1em #24c8db',
                'tauri-xl': '0 0 4em #24c8db',
                'leptos-s': '0 0 1em #a82e20',
                'leptos-xl': '0 0 4em #a82e20'
            }
        },
    },
    plugins: [require("daisyui")],
    daisyui: {
        themes: [
            "light",
            "dark",
            "cupcake",
            "bumblebee",
            "nord",
            "sunset",
        ],
    },
};