# WoW Profile Manager

Configuration profile manager for **WoW Forever**, on Windows. It exports a character's setup (addons, their
settings, macros, key bindings and action bar layout) to a single `.wowprofile` file. What sits inside each button
does not travel: the game server stores it, and each player places their own abilities. Another player, on another
computer, chooses what to import and which of their own characters to apply it to. Before writing anything, the app
makes a backup and shows a preview of the changes; afterwards, it lets you undo them.

> **Status: Phase 3 (MVP), not released yet.** The app already detects installations, accounts, realms and
> characters; exports and imports macros, key bindings and addon settings; and backs up, previews and undoes every
> change. Stack: Tauri 2 + Rust, with `full_moon` to read and write the `SavedVariables`, and React for the
> interface. The interface is available in English and Spanish.
>
> The technical documentation in [`docs/`](docs/) is in Spanish: what was measured on WoW Forever in
> [`docs/compatibilidad-wow-forever.md`](docs/compatibilidad-wow-forever.md), the package format in
> [`docs/formato-wowprofile.md`](docs/formato-wowprofile.md), the data model in
> [`docs/modelo-de-datos.md`](docs/modelo-de-datos.md), the addon adapters in
> [`docs/adaptadores.md`](docs/adaptadores.md), the export and import flows, with backups, in
> [`docs/flujos.md`](docs/flujos.md), and the architecture in [`docs/arquitectura.md`](docs/arquitectura.md).

## Development

```powershell
git clone https://github.com/SergioVertedor/wow-profile-manager.git
cd wow-profile-manager
git switch develop
npm ci
```

Requirements (Windows): [Node.js](https://nodejs.org/) 24, stable [Rust](https://rustup.rs/) and the
[Visual Studio C++ build tools](https://tauri.app/start/prerequisites/). WebView2 already ships with Windows 11.

| what | command |
|---|---|
| Run in development mode | `npm run tauri dev` |
| Interface: types, linter and tests | `npm run typecheck`, `npm run lint`, `npm test` |
| Core: formatting, linter and tests (in `src-tauri/`) | `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` |

CI runs all of the above on Windows for every PR.

## Workflow

1. Issue on GitHub.
2. Branch `feature/<issue>-<slug>` from `develop`.
3. Implement, with tests.
4. PR to `develop`.
5. Releases: `release/x.y.z` → `main` + tag.

## How it is made

This project is developed with the help of AI tools. A person makes the decisions and does the in-game testing, and
everything the project says about WoW Forever has been measured on a real installation, not assumed.

## Support the project

If you find it useful, you can buy me a coffee on Ko-fi.

<a href="https://ko-fi.com/tonotabidev"><img src="https://storage.ko-fi.com/cdn/kofi2.png?v=3" height="36" alt="Support me on Ko-fi"></a>

## Author

Sergio Vertedor — [tonotabi.dev](https://tonotabi.dev)

## License

[GNU GPL v3](LICENSE), with an additional attribution term (section 7(b)): modified versions that are distributed
must show credit to the original project in their interface. Details in [`NOTICE.md`](NOTICE.md).
