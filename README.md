# Kiwi API

https://www.kiwijoinerydevon.co.uk/

Kiwi Joinery: A Brixham based manufacturer of bespoke staircases, doors, windows, cabinets, gates, and all other joinery for your needs, supplying to the Torbay and South Hams area.

This is the API which powers our website including the gallery and contact form etc.

## Technical

This is a HTTP server built using the Actix Web framework for Rust.

Working with the database: \
`cargo install diesel_cli --no-default-features --features "postgres"`

### Building on Windows

- Set `PQ_LIB_DIR` environment variable to `C:\Program Files\PostgreSQL\${VERSION}\lib`
- Add `C:\Program Files\PostgreSQL\${VERSION}\bin` to `PATH`
