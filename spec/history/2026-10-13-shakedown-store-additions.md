# Shakedown's store, five additions, all deployed (2026-10-13)

For Shakedown (`~/Projects/Shakedown/spec/hosted.md` is the agreement; its
`spec/history/2026-10-13-*` tells the day). Every change in
`backend/src/shakedown.rs`, each with tests (53 in all), each deployed on
Jason's say by a push to `main`.

- **The box beside the dot:** a row's `addressed` field; a send says which
  screen it's for (`to`), carries a filter (`show`), and only a send to the
  phone is kept for later (`f5acf5b`).
- **Several changes from one post go as one `changes` event:** WebKit held
  back events that arrived together until the next ping, 15 s (`ddd1451`).
  Those two deployed by Jason at 01:24.
- **Part ids** (`UP-001.1`), and an app's card counting a row answered by
  its own answer or all its parts (`73f61dc`).
- **Test-build keys** (`sda_…`): the app's open run, answers and counts by
  key, as a page; an upload key only uploads; counts kept in a new table,
  **migration 0023** (additive; Jason: no backup needed) (`e0e1e56`). With
  the part ids, deployed in run 38384683913.
- **Carry over:** an open run takes a newer checklist and map, every
  answer kept; the run event says so (`5a93dfd`), run 38398133520.
