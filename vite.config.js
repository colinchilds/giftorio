import {defineConfig} from 'vite';
import solidPlugin from 'vite-plugin-solid';
import {viteStaticCopy} from 'vite-plugin-static-copy';

export default defineConfig({
    plugins: [
        solidPlugin(),
        viteStaticCopy({
            targets: [
                {
                    src: 'robots.txt',
                    dest: '.'
                }
            ]
        })
    ],
    server: {
        port: 3000,
    },
    build: {
        target: 'esnext',
    },
    assetsInclude: ['**/*.png', '**/*.jpg', '**/*.gif', '**/*.jpeg'],
});
