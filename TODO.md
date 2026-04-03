# TODO
- as a user I want delete of session to cascade to the messages

## Streaming — bugs found in code review

### High
- **Concurrency guard per session** — parallel `/stream` requests to the same session corrupt conversation history (both insert user messages and generate interleaved assistant messages)
- **No auth/ownership check on stream endpoint** — any caller can trigger LLM API calls against any session UUID (real API key spend)
- **Mid-stream LLM errors silently dropped** — malformed/error JSON lines in the SSE stream are swallowed; partial content is saved as a successful response

### Medium
- **Max tool iterations leaves frontend broken** — `SseEvent::Error` is yielded after 10 rounds but `SseEvent::Done` never fires; `invalidateQueries` is never called and the streaming bubble persists
- **`streaming` state not cleared on error** — `setStreaming(null)` only runs in the `done` handler; on error the stale streaming bubble stays visible
- **Abort race condition in `useStreamChat`** — the aborted stream's `finally` block can run after the new `send()` has set its state, leaving the UI inconsistent
- **Last delta dropped on `finish_reason`** — `parse_stream_chunk` returns early when `finish_reason` is present, discarding any `delta.content` or `delta.tool_calls` in the same chunk
- **No HTTP timeout on LLM requests** — a hung provider holds the connection, tokio task, and browser SSE open indefinitely
- **Dangling user message on mid-stream error** — if the LLM stream errors, the user message is in the DB with no corresponding assistant message

### Low
- **`JSON.parse` on SSE lines has no try/catch** — silently stops streaming with no user feedback on malformed data
- **Final SSE buffer not flushed** — remaining buffer after stream ends is discarded if the last event lacks a trailing `\n\n`
