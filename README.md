# Trove

Search your own files by meaning, fully offline. A Windows-first desktop app
(Tauri 2 + Svelte 5) built on the [quillrag](https://github.com/Ayush-yadav11/quillrag)
engine: MiniLM embeddings and BM25 keyword search, fused, running locally.

- **Notes, docs and code:** `.md`, `.txt`, source files and more.
- **PDFs:** indexed per page; results show the matching page.
- **Screenshots and scanned PDFs:** on-device OCR (English / basic Latin).
- **Private:** no network requests; the search and OCR models are compiled in
  and the index lives in `%LOCALAPPDATA%\dev.ayushyadav.trove`.

Press **Alt+Shift+Space** anywhere to open the search box. Closing the window
keeps Trove in the tray; **Quit Trove** in the tray menu exits.

## Status

Early development. Folders are re-indexed at startup and on demand
("Re-index now"); live file watching is next.

## Development

quillrag is pulled from GitHub at a pinned commit (see `src-tauri/Cargo.toml`,
which also shows how to point it at a local checkout while hacking on both).

```sh
pnpm install
pnpm tauri dev      # dependencies are optimized even in dev; first build is slow
pnpm check          # Svelte/TypeScript
cd src-tauri && cargo clippy
```

## Credits

OCR uses the [ocrs](https://github.com/robertknight/ocrs) models by Robert
Knight (CC BY-SA 4.0), bundled via quillrag; see quillrag's
`assets/ocr/NOTICE.md`.
