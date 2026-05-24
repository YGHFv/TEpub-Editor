# TEpub Editor Flutter

This folder is a parallel rewrite of TEpub Editor. It intentionally keeps its
own application id, data directory, cache directory, and settings namespace so
it will not conflict with the current packaged Svelte/Tauri release.

## Data Isolation

The Flutter rewrite uses the namespace `com.yghfv.tepub_editor_flutter` and
stores data under a dedicated `TEpubEditorFlutter` folder created from the
platform application-support directory.

Planned subfolders:

- `library/` for the new book index and copied books.
- `cache/` for generated EPUB/TXT previews and temporary parse data.
- `history/` for editor snapshots.
- `logs/` for AI proofreading logs.
- `config/` for settings and provider profiles.

## Bootstrap

Flutter SDK is installed at `E:\MTool\SDK\flutter`. If a terminal cannot find
`flutter`, open a new terminal or add this path for the current session:

```powershell
$env:Path = "$env:Path;E:\MTool\SDK\flutter\bin"
```

Run the Flutter rewrite from this folder:

```powershell
cd flutter/tepub_editor_flutter
flutter pub get
flutter run -d windows
```

The initial Windows desktop shell has been verified with `flutter analyze`,
`flutter test`, and `flutter run -d windows`.

Keep the existing root Svelte/Tauri project untouched until the Flutter rewrite
has feature parity.
