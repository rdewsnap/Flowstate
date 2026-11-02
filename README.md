# Flowstate

**The local-first Wispr Flow alternative that knows your stuff.**

Hold Fn, talk, let go. Flowstate types what you meant: no ums, no false starts, "actually… instead" applied, and your project names, teammates and jargon spelled the way *you* spell them.

- **100% local by default.** Speech-to-text always runs on your Mac. No account, no subscription.
- **Knows your context.** Learns names from your projects folder, contacts and calendar, and reads the field you're typing into.
- **Fixes what you meant.** Removes fillers and repairs self-corrections across the whole dictation.
- **Honest fallbacks.** If the cleanup model is slow, missing or makes something up, you get the rule-cleaned text instead of a lost dictation.

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

Writing style follows the app: casual in Slack and Messages, prose in Mail and Docs, exact identifiers in editors and terminals.

## Quick start (macOS, Apple Silicon)

```bash
scripts/fetch-models.sh         # Whisper, Parakeet and a 4B cleanup model (~3.7 GB)
scripts/build-llama-server.sh   # self-contained llama-server for local cleanup
LLAMA_SERVER=build/llama-server scripts/build-mac.sh   # builds build/Flowstate.app
open build/Flowstate.app
```

Building needs Rust (stable, via rustup), Xcode 16 and CMake. Homebrew's current `llama.cpp` fails to compile its Metal shaders on macOS 15, so `build-llama-server.sh` builds a pinned version instead. On first launch Flowstate asks for microphone and Accessibility access and picks a cleanup mode from your hardware.

If Fn opens the emoji picker, set **System Settings → Keyboard → Press 🌐 key to → Do Nothing**.

| Shortcut | Action |
|---|---|
| Hold **Fn** | Dictate while held |
| Double-tap **Fn** | Hands-free; press Fn again to finish |
| **⌃⌥Z** | Swap the last dictation for exactly what you said |

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

## Connect your AI tools

Flowstate ships a small MCP server, `flowstate-mcp`, so Claude Desktop, Claude Code, Cursor and other MCP clients can search your notes, meeting transcripts, summaries, action items and dictation history. It runs over stdio, opens your databases read-only and never touches the network.

```json
{"mcpServers":{"flowstate":{"command":"/Applications/Flowstate.app/Contents/MacOS/flowstate-mcp"}}}
```

For Claude Code: `claude mcp add flowstate -- /Applications/Flowstate.app/Contents/MacOS/flowstate-mcp`. Pass `--data-dir` to read a different folder, or `--allow-write` to let the client save notes.

## Repository layout

| Path | What |
|---|---|
| `crates/flowstate-core` | Platform-independent pipeline (Rust) |
| `crates/flowstate-ffi` | UniFFI bindings for the native apps |
| `crates/flowstate-bench` | Benchmark CLI |
| `apps/mac` | SwiftUI/AppKit menu bar app |
| `prompts/` | The cleanup prompt, shared by the app and training |
| `training/` | Data generation and fine-tuning |

## License

MIT
