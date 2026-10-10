// True on a device without hover / with a coarse pointer (phones, iPads), read
// once. For behaviour that can't be done in CSS — e.g. dropping webkitdirectory
// from an upload input, which a phone's photo library can't satisfy.
export const isTouch = typeof window !== 'undefined' && window.matchMedia('(hover: none), (pointer: coarse)').matches
