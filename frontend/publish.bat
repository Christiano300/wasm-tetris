wasm-pack build --target web --out-name lib --release --no-opt ./lib
pnpm build
scp -r dist/* deploy@coder.patzl.dev:upload/tetris/
ssh deploy@coder.patzl.dev './deploy tetris'