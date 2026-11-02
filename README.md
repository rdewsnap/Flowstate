# Flowstate

**The local-first Wispr Flow alternative that knows your stuff.**

Hold Fn, talk, let go. Flowstate types what you meant: no ums, no false starts, "actually… instead" applied, and your project names, teammates and jargon spelled the way *you* spell them.

- **100% local by default.** Speech-to-text always runs on your Mac. No account, no subscription.
- **Knows your context.** Learns names from your projects folder, contacts and calendar, and reads the field you're typing into.
- **Fixes what you meant.** Removes fillers and repairs self-corrections across the whole dictation.
- **Honest fallbacks.** If the cleanup model is slow, missing or makes something up, you get the rule-cleaned text instead of a lost dictation.
- **Learns from your edits.** Fix a word Flowstate got wrong and the next dictation spells it your way. Undo it from the pill if it guessed wrong.
- **Spoken commands.** End a dictation with "scratch that", "new line" or "new paragraph". "Send it" presses Return, if you turn it on.
- **Notes bar.** ⌥Space opens a floating bar for quick notes, typed or dictated, with search across everything. Notes can be mirrored to a folder as Markdown for Obsidian.
- **Meetings.** Records your mic and the call's audio, transcribes on your Mac, and writes a summary with decisions and action items. No audio is kept.
- **Integrations.** Send notes and summaries to Apple Notes, post events to a webhook, add meeting summaries to the calendar event, and drive Flowstate from Shortcuts or `flowstate://` links.
- **Works with your AI tools.** A bundled MCP server lets MCP clients search your notes and meetings.

## How it works

```
mic ─▶ speech-to-text ─▶ filler filter ─▶ vocabulary fixes ─▶ cleanup model ─▶ guardrail ─▶ paste
       (Whisper/Parakeet)  (rules, instant)  (phonetic match)   (local or cloud)  (no new words)
```

1. **Speech-to-text** runs locally with Whisper (whisper.cpp, which also gets your vocabulary as a prompt) or Parakeet (sherpa-onnx, faster).
2. **Filler filter** drops um/uh, stutters (`I-I`) and repeats (`the the`). Instant, always on.
3. **Vocabulary fixes** match misheard names by sound: `kin ju` → `Kinju`, `PG vector` → `pgvector`.
4. **Cleanup model** handles the judgment calls: `like`, `you know`, restarts, and corrections that reach back several sentences. Runs locally through llama.cpp, or in the cloud on slower machines (text only, never audio).
5. **Guardrail** rejects any rewrite that adds words you didn't say.

Writing style follows the app: casual in Slack and Messages, prose in Mail and Docs, exact identifiers in editors and terminals. You can change the style for a kind of app, or for one app, in Settings → Cleanup.

## Quick start (macOS, Apple Silicon)

```bash
scripts/fetch-models.sh         # Whisper, Parakeet and a 4B cleanup model (~3.7 GB)
scripts/build-llama-server.sh   # self-contained llama-server for local cleanup
LLAMA_SERVER=build/llama-server scripts/build-mac.sh   # builds build/Flowstate.app
open build/Flowstate.app
```

Building needs Rust (stable, via rustup), Xcode 16 and CMake. Homebrew's current `llama.cpp` fails to compile its Metal shaders on macOS 15, so `build-llama-server.sh` builds a pinned version instead. On first launch Flowstate asks for microphone and Accessibility access and picks a cleanup mode from your hardware.

Signed builds are published on the Releases page as a `.dmg` and `.zip` once a tagged release exists. A Homebrew tap is coming; once it exists:

```bash
brew tap rdewsnap/flowstate
brew install --cask flowstate
```

If Fn opens the emoji picker, set **System Settings → Keyboard → Press 🌐 key to → Do Nothing**.

| Shortcut | Action |
|---|---|
| Hold **Fn** | Dictate while held |
| Double-tap **Fn** | Hands-free; press Fn again to finish |
| **⌃⌥Z** | Swap the last dictation for exactly what you said |
| **⌥Space** | Open the notes bar |

## Meetings

Choose **Start meeting…** from the menu, or click Start on the notification Flowstate shows when a call starts during a calendar event. Your microphone is transcribed as "Me" and the call's audio (with Screen Recording permission) as "Others". When you stop, the cleanup model writes a summary with decisions, action items and open questions. Transcripts and summaries are searchable from the notes bar and the MCP server.

## Integrations

Settings → Integrations:

- **Apple Notes**: Send to Apple Notes on any note or meeting puts it in a Flowstate folder.
- **Markdown**: notes, and optionally every finished meeting, are written to a folder with front matter.
- **Calendar**: a few lines of the summary and a link back to the meeting are added to its event.
- **Webhook**: `note.created`, `note.updated` and `meeting.finished` as JSON, signed with HMAC-SHA256 in `X-Flowstate-Signature` when you set a secret.
- **Shortcuts and links**: `flowstate://note?text=…`, `flowstate://meeting/start?title=…`, `flowstate://meeting/stop` and `flowstate://dictate`, plus Shortcuts actions for the same things and Get Last Dictation.

## Cleanup modes

| Mode | What runs | Good for |
|---|---|---|
| Local | 3–4B model via `llama-server` on localhost | Most Apple Silicon Macs with 16 GB+ |
| Cloud | Your own key: Anthropic, Groq, or any OpenAI-compatible endpoint | Slower machines |
| Filter only | Rules + vocabulary fixes | Instant, no rewriting |
| Raw | Nothing | Exactly what the speech engine heard |

In cloud mode, dictations with no self-corrections go to a cheaper "light" model and only repairs go to the stronger one.

## Benchmark

`flowstate-bench` scores systems on the things Flowstate claims to do better: word error rate, your vocabulary spelled right, fillers left behind, and repair accuracy. It also scores other tools' output pasted into the manifest, so you can compare against Wispr Flow on the same clips.

```bash
# Cleanup only, from raw transcripts (no audio needed):
cargo run -p flowstate-bench -- --manifest bench/manifest.example.jsonl --vocab bench/vocab.example.txt

# Full pipeline on your own recordings:
cargo run --release -p flowstate-bench --features whisper,parakeet -- \
  --manifest bench/manifest.jsonl --projects ~/Projects \
  --whisper "$HOME/Library/Application Support/Flowstate/Models/ggml-large-v3-turbo-q5_0.bin" \
  --llama-server build/llama-server --local-model path/to/model.gguf
```

Record clips as 16 kHz mono WAV, e.g. `ffmpeg -f avfoundation -i ":0" -ar 16000 -ac 1 bench/clips/vocab-01.wav`.

## Training a small cleanup model

`training/` generates messy/clean pairs with Claude, mixes in Google's Disfl-QA (CC BY 4.0), and LoRA-trains a ~1–2B model with mlx-lm so slower Macs can run good cleanup locally. See the script headers; your own benchmark recordings are never used for training.

It also works with no API key: `prepare_*.py` pull free datasets, `corrupt.py` turns clean text into messy transcripts, and `generate_local.py` uses the local 4B model for the hard cases. Then `build_dataset.py data/*/pairs.jsonl --cap ...` and `LLAMA_CPP=build/llama.cpp-src ./train_mlx.sh`. Results of the first run are in `training/RESULTS.md`.

## Connect your AI tools

Flowstate ships a small MCP server, `flowstate-mcp`, so Claude Desktop, Claude Code, Cursor and other MCP clients can search your notes, meeting transcripts, summaries, action items and dictation history. It runs over stdio, opens your databases read-only and never touches the network.

```json
{"mcpServers":{"flowstate":{"command":"/Applications/Flowstate.app/Contents/MacOS/flowstate-mcp"}}}
```

For Claude Code: `claude mcp add flowstate -- /Applications/Flowstate.app/Contents/MacOS/flowstate-mcp`. Pass `--data-dir` to read a different folder, or `--allow-write` to let the client save notes.

## Sync, teams and self-hosting

Everything above works with no account. If you want notes and meetings on more than one Mac, a glossary shared with your team, or cleanup without running a model or managing a key, the app can sign in to a Flowstate server. The server is in this repo (`crates/flowstate-server`) and runs from one binary with SQLite or Postgres; `crates/flowstate-server/README.md` has a docker compose quickstart. Audio never leaves your Mac in any mode; the server only ever sees text you choose to sync.

## Repository layout

| Path | What |
|---|---|
| `crates/flowstate-core` | Platform-independent pipeline (Rust): speech-to-text, filter, vocabulary and learned corrections, cleanup, spoken commands, notes and meeting storage, summaries |
| `crates/flowstate-ffi` | UniFFI bindings for the native apps |
| `crates/flowstate-mcp` | MCP server for notes, meetings and dictation history |
| `crates/flowstate-bench` | Benchmark CLI |
| `crates/flowstate-server` | Optional backend: accounts, teams, sync, shared glossary, hosted cleanup, MCP over HTTP |
| `deploy/` | Dockerfile, docker compose and Fly config for the server |
| `apps/mac` | SwiftUI/AppKit menu bar app |
| `apps/mac/Sources/Flowstate/Notes` | Notes bar, Notes window, Markdown export |
| `apps/mac/Sources/Flowstate/Meetings` | Meeting recording, call detection, summaries view |
| `apps/mac/Sources/Flowstate/Integrations` | Apple Notes, webhook, calendar, `flowstate://` links, Shortcuts actions |
| `prompts/` | Model prompts: dictation cleanup (shared with training), meeting summaries, note titles |
| `training/` | Data generation and fine-tuning |

## License

MIT
