# v1-caption

A monorepo for downloading YouTube transcripts.

## Structure

- **server** - Rust backend that fetches YouTube transcripts. Uses a proxy server in production to avoid rate limiting and IP blocks from YouTube.
- **frontend** - Svelte web app providing a simple interface for users to paste a YouTube URL and download the transcript.

*(These are located inside of the app folder.)*

## To-do

- [x] Add proxy server
- [ ] Add redis caching
- [ ] Add rate limiting
- [ ] Create Svelte UI

---
Developed By Sphe