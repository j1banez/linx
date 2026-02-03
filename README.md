# Linx

Host and own your short links 🔗

Simple, lightweight, self hosted.

## ⚡ Quick start

## ⭐ Features

- Shorten URLs, allow base62 custom codes
- Basic stats: click counter and last-access date
- Minimal web UI plus JSON API
- Zero config SQLite storage

## Screenshots

## FAQ

>Is Linx multi-user?

Not yet. It's designed for single-owner/self-hosted use.

>Why is the click counter not working?

Browsers cache redirections when using http code 301 or 308 so if the same client
clicks multiple time, the counter will only update the first time.
