# Changelog

## v0.1.0 (first preview)

Search your own files by meaning, fully offline.

### Highlights

- **Search by meaning as you type.** Hybrid semantic + keyword search, one
  result per file with a highlighted snippet. Unrelated files are filtered out.
- **PDFs, page by page.** Results show the matching page ("p. 2, also p. 5").
- **Screenshots and scanned PDFs.** On-device OCR reads text in images and
  image-only PDF pages.
- **Always at hand.** Alt+Shift+Space opens Trove from anywhere; closing keeps
  it in the tray.
- **Private.** No network requests. The search and OCR models are built into
  the app; the index lives in `%LOCALAPPDATA%\dev.ayushyadav.trove`.

### Install

1. Download **`Trove_0.1.0_x64-setup.exe`** (recommended) or the `.msi`.
2. The installers are not code-signed yet, so Windows SmartScreen may warn:
   choose **More info → Run anyway**.
3. Open Trove, choose **Add a folder**, and search once indexing finishes.

Requires 64-bit Windows 10 or 11.

### Known limitations

- New or changed files are picked up at startup or with **Re-index now**;
  live file watching is coming next.
- OCR reads English / basic Latin text only, takes ~1-3 s per image or
  scanned page the first time, and covers at most 100 scanned pages per PDF.
- Opening a PDF result doesn't jump to the matching page yet.

### Credits

- Engine: [quillrag](https://github.com/Ayush-yadav11/quillrag).
- Search model: [all-MiniLM-L6-v2](https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2)
  (Apache-2.0).
- OCR models: [ocrs](https://github.com/robertknight/ocrs) by Robert Knight
  (CC BY-SA 4.0).
