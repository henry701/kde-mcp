# LLM-Facing Tool Design Principles

Captured during design of kde-mcp. The principles in the spec (§4) reference these and add concrete applications. This document records the underlying patterns and their evidence.

---

## 1. High signal-to-noise per token — Anthropic's master heuristic

> Every token in every tool response competes for attention against every other token. The goal is "the smallest possible set of high-signal tokens that maximize the likelihood of some desired outcome."

Anthropic explicitly favours `name` over `uuid`, `image_url` over `256px_image_url`, `file_type` over `mime_type`. Strip UUIDs, MIME types, internal IDs, ceremonial JSON unless they materially change a decision.

Sources:
- [Anthropic — Writing tools for agents](https://www.anthropic.com/engineering/writing-tools-for-agents)
- [Anthropic — Effective context engineering](https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents)
- pgEdge MCP retro
- PagerDuty MCP retro

Strongly shared practice.

---

## 2. Progressive disclosure / response_format tiers

Offer at least two response modes (`concise`/`detailed`, `summary`/`full`/`ids_only`); default cheap; let detail be opt-in. pgEdge's MCP team built a 3-tier ladder (~1000 / ~50 / ~10 tokens) and reports measured ~3× token reduction in practice.

Sources:
- Anthropic "Writing tools for agents" prescribes `response_format` enum
- pgEdge MCP retro
- PagerDuty MCP retro

---

## 3. Poka-yoke — make wrong actions structurally impossible

From manufacturing via Anthropic's "Building effective agents": when a class of mistake recurs, change the tool's *shape* so the mistake can't be expressed. Anthropic's example: agents kept passing relative paths; the fix was making the parameter type "absolute path only" — not adding warnings to descriptions.

Playwright-MCP's element-ref design is poka-yoke: model can pass only refs from a recent snapshot, can't hallucinate coordinates.

Sources:
- [Anthropic — Building effective agents](https://www.anthropic.com/engineering/building-effective-agents) uses the term explicitly
- Microsoft Playwright-MCP source

Becoming canonical.

---

## 4. Accessibility-tree-first over pixels

When a structured UI representation exists, prefer it categorically over screenshots. Deterministic IDs, 5–30× token efficiency, survives small visual changes.

Sources:
- [Microsoft Playwright-MCP README](https://github.com/microsoft/playwright-mcp/blob/main/README.md)
- Anthropic Computer Use docs treat screenshots as fallback

Shared by Playwright-MCP, Anthropic's MCP servers, most serious computer-use teams.

Tension with #7 (quarantine): the *structure* is trusted but the *strings inside* are not. Mark per-field.

---

## 5. Just-in-time references over pre-loaded payloads

Return references, let the agent fetch detail. Mirrors human cognition (we use file systems, not memory). Tradeoff: runtime exploration is slower than pre-computed data, but freshness and token efficiency usually win.

Sources:
- Anthropic "Effective context engineering": "maintain lightweight identifiers… and use these references to dynamically load data into context at runtime"
- MCP spec 2025-06-18 `resource_link` content type

Shared practice; baked into the spec.

---

## 6. Errors return state needed to recover, inline

An error saying "stale handle: window 0x4b2 no longer exists" forces a wasted round-trip. An error saying "stale handle; here are the current windows: [...]" is self-correcting in one turn.

Sources:
- Anthropic "Writing tools for agents": "actionable error messages [that] communicate specific and actionable improvements"
- pgEdge MCP retro: pagination response carries a "helpful nudge"
- PagerDuty MCP retro

Strongly shared.

---

## 7. Tools encode user journeys, not endpoints

Naive servers expose every primitive separately (`move_mouse` / `click_left` / `keydown` / ...). Thoughtful servers consolidate along workflow axes (`act_on_element` / `fill_form` / `open_app_and_wait_until_ready`). Tool-count sweet spot per multiple retros: 20–25.

Sources:
- Anthropic "Writing tools for agents": `schedule_event` example
- PagerDuty MCP retro
- Matt Adams — MCP Server Design Principles
- pgEdge MCP retro

Tension with #3 (poka-yoke split): the resolution is "consolidate along intent, split along safety."

---

## 8. Quarantine untrusted content

Any string read from the user's screen, a webpage, an email body, a file the agent didn't author is a prompt-injection vector. Wrap it structurally so the model treats it as data, not instruction. Meta's "Agents Rule of Two" (Oct 2025): in any one session, hold at most two of {untrusted input, sensitive data access, ability to externally communicate} — make the third structurally impossible.

Sources:
- Anthropic prompt-injection defense docs
- Simon Willison — Agents Rule of Two
- Claude Code sandboxing model

Active, evolving practice.

---

## 9. Read-only / mutating annotation surfaced on every tool

Every tool declares whether it changes state. MCP spec tool annotations: `readOnlyHint`, `destructiveHint`, `idempotentHint`. Serves two audiences: host (parallelize read-only, confirm destructive) and model (retry safety reasoning).

Sources:
- Playwright-MCP source marks each tool
- MCP spec 2025-06-18

Widely adopted.

---

## 10. State surfacing via diffs, not absolute snapshots

When the agent acts, the decision-relevant information is "what changed because of my action," not "what's the current full state." Return deltas. Anthropic's long-running-agents writeup describes maintaining `claude-progress.txt` whose deltas the next session reads first.

Sources:
- Anthropic "Effective harnesses for long-running agents"
- Inkeep "Fighting Context Rot"

Generalisation of #6.

---

## How these inform kde-mcp

The spec's principles section (§4) commits to:
- (4.1) Accessibility-tree first (principle #4 above)
- (4.2) Response-as-instruction (principles #6 + #10 above, generalised)
- (4.3) High signal-to-noise (principle #1 above)
- (4.4) Progressive disclosure (principle #2 above)
- (4.5) Poka-yoke (principle #3 above)
- (4.6) Just-in-time references (principle #5 above)
- (4.7) Quarantine untrusted (principle #8 above)

Annotation-on-every-tool (#9) is implementation-level, encoded in tool annotations.

Tool consolidation along intent (#7) is what informs the 15-tool surface (not 30 like kwin-mcp, not 3 like Anthropic native).

---

## Things deliberately NOT adopted

- **Journey tools** (`save_current_document()`, `open_app_and_wait_until_ready()`): premature for MVP. Worth revisiting once primitive use patterns are observed.
- **Elicitation prompts** mid-call: client support is uneven; would override the host's permission mode. Reconsider in v2.
- **Telemetry to phone home**: privacy boundary; not needed for personal use.
- **Server-side sampling** (where the MCP server asks the host LLM to plan something): no use case yet.

---

## Tensions to keep in mind for future tools

| Tension | Resolution rule |
|---|---|
| Signal-to-noise vs response-as-instruction + recovery state | Only add a field if it materially changes the next action. Empty diffs/arrays omitted. |
| Poka-yoke split vs journey-tool consolidation | Consolidate along intent; split along safety (different blast radii) |
| Just-in-time refs vs response-as-instruction | Return ref + minimal meta (title/role), not pure ref |
| A11y-tree primacy vs quarantine untrusted | Trust the structure (role, geometry, action names); mark strings (name, value) as external |

---

## Sources

- [Anthropic — Writing tools for agents](https://www.anthropic.com/engineering/writing-tools-for-agents)
- [Anthropic — Effective context engineering for AI agents](https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents)
- [Anthropic — Building effective agents](https://www.anthropic.com/engineering/building-effective-agents)
- [Anthropic — Advanced tool use](https://www.anthropic.com/engineering/advanced-tool-use)
- [Anthropic — Effective harnesses for long-running agents](https://www.anthropic.com/engineering/effective-harnesses-for-long-running-agents)
- [Anthropic — Mitigating prompt injection in browser use](https://www.anthropic.com/research/prompt-injection-defenses)
- [Anthropic — Claude Code sandboxing](https://www.anthropic.com/engineering/claude-code-sandboxing)
- [Microsoft Playwright-MCP README](https://github.com/microsoft/playwright-mcp/blob/main/README.md)
- [MCP spec 2025-06-18](https://modelcontextprotocol.io/specification/2025-06-18)
- [pgEdge — Lessons learned writing an MCP server for PostgreSQL](https://www.pgedge.com/blog/lessons-learned-writing-an-mcp-server-for-postgresql)
- [PagerDuty — Lessons learned while building PagerDuty's MCP server](https://www.pagerduty.com/eng/lessons-learned-while-building-pagerduty-mcp-server/)
- [Matt Adams — MCP Server Design Principles](https://www.matt-adams.co.uk/2025/08/30/mcp-design-principles.html)
- [Simon Willison — New prompt injection papers: Agents Rule of Two](https://simonw.substack.com/p/new-prompt-injection-papers-agents)
- [Inkeep — Fighting Context Rot](https://inkeep.com/blog/fighting-context-rot)
