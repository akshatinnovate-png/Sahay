# Sahay

**Sahay** (सहाय, "help") does things on your computer for you. Tell it what you want in plain words, by typing or speaking, and it moves its own cursor, clicks and types until the job is done, showing every step as it goes.

It started as a byproduct of [Saathi](https://github.com/akshatinnovate-png/Saathi-Alexa), an Alexa+ companion for elders and their families, and lives here as its own project. Saathi's Windows installer includes Sahay.

## Download

| | |
|---|---|
| **Sahay for Windows** | [**Sahay.exe**](https://github.com/akshatinnovate-png/Sahay/releases/latest/download/Sahay.exe) from the [latest release](https://github.com/akshatinnovate-png/Sahay/releases/latest). Run it, paste a free Groq key from [console.groq.com/keys](https://console.groq.com/keys), and tell it what to do. |
| **Sahay for Android** | **Sahay-Android-x.y.z.apk** from the [releases](https://github.com/akshatinnovate-png/Sahay/releases). Built on Senku by Steve Aldrin, with every charge removed, running on your own free Gemini key. See [android/](android/). |

Windows may say "Windows protected your PC" for a new download: choose **More info**, then **Run anyway**.

## What it can do

- **Instant commands with no AI call at all**: play a song or video (it finds the real video, skips ads, unmutes and turns the volume up), open sites and apps in the browser you name, search any shop or site, close tabs by topic, media and volume, full screen, timers and reminders, routines ("every day at 9 am play bhajans"), weather, news, currency, Wikipedia, maths, files, settings, installing apps.
- **Plans for everything else.** Groq (`gpt-oss-120b` first, then Groq's other free models when it is busy) sees a short summary of the screen and answers with a whole plan. The Mover on your PC carries it out: it reads the screen with Windows UI Automation and Windows OCR, finds targets by their words, looks at a picture of the screen for icons, and checks that the job really worked before saying so.
- **A brain on your own PC.** Right-click the sphere, **Set up the local brain**: Ollama and Qwen3 4B run offline on your graphics chip, and Sahay uses them whenever Groq is busy (or always, if you choose).
- **It learns.** Drop a dataset on the chat (or `learn from <file or folder>`) and it learns at once: templates run instantly, and similar examples and lessons guide the brain. Every task it finishes cleanly is added to its own knowledge file. You can teach it rules too: "teach: on Amazon, sort by customer reviews".
- **It talks** in Windows' voice, listens through the 🎤 button, remembers what you tell it, reads and summarises the screen, and works on text you select.
- **Safe by design**: it never pays, deletes or shares personal data without asking, never closes Windows' own windows, and never invents prices or facts.

Ctrl+Alt+S opens Sahay from anywhere; Ctrl+Alt+M opens it listening. Right-click the sphere for keys, voice, brain mode, datasets, start with Windows and the log.

## Datasets

[`training/`](training/) holds ready-made datasets and the scripts that build them:

| File | Size | What's in it |
|---|---|---|
| [`sahay-starter.jsonl`](training/sahay-starter.jsonl) | 134 lines | 104 task templates and 30 lessons |
| [`sahay-massive.jsonl`](training/sahay-massive.jsonl) | 2,552 lines | 779 templates across 70 sites, music, shopping, messaging, maps, settings, files and system jobs, plus 100 lessons |
| [`sahay-50k.jsonl`](training/sahay-50k.jsonl) | 51,126 lines | the massive set filled in with real products, people, cities and topics |
| [`sahay-800k/`](training/sahay-800k) | 813,094 lines in 9 parts | 151 new templates (food, jobs, local services, doctors, travel, groceries, 20 languages, the Bhagavad Gita, devotional music, health, cars, colleges, coding, maths, bills, government services, banking) filled in for about 250 cities |

The format, how to write your own, and how to fine-tune the local brain on free Colab GPUs are in [`training/README.md`](training/README.md).

## Build it yourself

Windows, with Rust 1.97:

```
cargo test --locked
cargo build --locked --release
```

The exe is `target\release\sahay.exe`. The pure parts (planning, matching, datasets, settings) also build and test on Linux; the Windows parts are checked with `cargo clippy --target x86_64-pc-windows-gnu`.

Android: `gradle -p android assembleRelease` with JDK 21.

## Releases

Releases are published by the **Release** workflow (Actions → Release → Run workflow, with a version like `2.9.0`). Each one has `Sahay-x.y.z.exe`, a stable `Sahay.exe` (the link Saathi's installer downloads), checksums, build provenance, the starter dataset and the Android APK. Release history from v2.0.0 onwards is in [RELEASE_NOTES.md](RELEASE_NOTES.md).

## Licence

Sahay for Windows (everything outside `android/`) is MIT licensed; see [LICENSE](LICENSE). Sahay for Android in [`android/`](android/) is based on Senku by Steve Aldrin and is under the PolyForm Noncommercial licence; see [android/LICENSE.md](android/LICENSE.md), [android/NOTICE](android/NOTICE) and [android/PERMISSION.md](android/PERMISSION.md).
