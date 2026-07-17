# What Claude Built in Rust for The Livery

The new OG Overlay Studio needed a server-side image compositor — something that takes a card's hero photo and renders styled text overlays on top of it to produce a 1200×630 PNG for social sharing (Discord, Reddit, iMessage unfurls, etc.).

The compositor lives in `backend/src/compositor.rs` and does the following:

1. **Photo loading** — opens the card's hero photo via the `image` crate, fill-crops it to the canvas size using Lanczos3 resampling so it always fills the frame without letterboxing.

2. **Glyph rasterization** — uses `fontdue` (a pure-Rust TTF parser and rasterizer) to render text at a size that fills the box height, then measures the natural rendered width.

3. **Text-to-box fill** — the natural glyph buffer gets scaled to exactly the box's pixel dimensions using `imageops::resize`. This intentionally distorts the text horizontally to fill the width — same behavior as CSS `transform: scaleX()` — which is the design intent for a travel-postcard-style stamp.

4. **Affine transforms** — rotation and horizontal shear are applied via inverse pixel mapping. For each output pixel in the bounding region, the code applies the inverse rotation and inverse shear to find the corresponding source pixel, then samples from the text buffer. This means the compositor supports arbitrary rotation angles and shear values without any separate matrix library.

5. **Alpha compositing** — text pixels are blended over the photo using standard Porter-Duff "over" compositing. The GHOST style uses partial alpha (170/255); the others are fully opaque white.

6. **Two call sites, one function** — `GET /share/:id/card.png` (public, serves the card's saved config) and `POST /share/preview` (auth-gated, accepts raw config JSON for the live editor) both call the same `compose()` function. The whole point of the architecture is that there's no separate "preview renderer" that could drift from the real output.

Bundled fonts are Bebas Neue and Oswald Variable — both OFL-licensed, embedded at compile time with `include_bytes!`.

---

He'll probably have opinions about the pixel loop.
