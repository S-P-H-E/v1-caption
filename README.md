# v1-caption

A monorepo for downloading YouTube transcripts.

## Structure

- **server** - Rust backend that fetches YouTube transcripts. Uses a proxy server in production to avoid rate limiting and IP blocks from YouTube.
- **frontend** - Svelte web app providing a simple interface for users to paste a YouTube URL and download the transcript.

*(These are located inside of the app folder.)*

## Overview

The Rust API handles transcript extraction from YouTube videos, routing requests through a proxy in production environments to ensure reliable access. The Svelte frontend offers a clean, user-friendly way to interact with the service without technical knowledge.
