// @ts-check
import { defineConfig } from 'astro/config';
import tailwindcss from "@tailwindcss/vite";
import { ViteImageOptimizer } from 'vite-plugin-image-optimizer';

// https://astro.build/config
export default defineConfig({
  vite: {
    plugins: [
        tailwindcss(),
        ViteImageOptimizer({
          // Options for PNG, JPEG, SVG, WebP, AVIF
          png: {quality: 80},
          jpeg: {quality: 75},
          // ...
        }),
    ]
  }
});